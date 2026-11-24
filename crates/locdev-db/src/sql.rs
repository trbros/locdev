//! SQL the manager writes itself (quoted identifiers, parameters for every
//! value) and a conservative reading of SQL the user writes, to stop
//! statements that destroy data until the database's name is typed back.

use locdev_core::db::{DbEngine, DbFilter, QueryCheck};

/// Statements in `sql`, split on `;` outside quotes and comments.
pub fn statements(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                current.push(c);
                if c == q {
                    quote = None;
                }
            }
            None => match c {
                '\'' | '"' | '`' => {
                    quote = Some(c);
                    current.push(c);
                }
                '-' if chars.peek() == Some(&'-') => {
                    for c in chars.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                    current.push(' ');
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    let mut prev = ' ';
                    for c in chars.by_ref() {
                        if prev == '*' && c == '/' {
                            break;
                        }
                        prev = c;
                    }
                    current.push(' ');
                }
                ';' => {
                    if !current.trim().is_empty() {
                        out.push(current.trim().to_string());
                    }
                    current.clear();
                }
                _ => current.push(c),
            },
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

/// The words of a statement outside string literals, lowercased.
fn words(statement: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quote: Option<char> = None;
    for c in statement.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '\'' => quote = Some(c),
            None if c.is_alphanumeric() || c == '_' => word.push(c.to_ascii_lowercase()),
            None => {
                if !word.is_empty() {
                    out.push(std::mem::take(&mut word));
                }
            }
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

/// What running `sql` would do. `database` is what must be typed back.
pub fn check(sql: &str, database: &str) -> QueryCheck {
    let mut result = QueryCheck::default();
    for statement in statements(sql) {
        let w = words(&statement);
        let first = w.first().map(String::as_str).unwrap_or("");
        let has = |word: &str| w.iter().any(|x| x == word);
        let reads = matches!(
            first,
            "select" | "show" | "explain" | "describe" | "desc" | "pragma" | "with" | "values"
        ) && !has("insert")
            && !has("update")
            && !has("delete")
            && !has("drop");
        if !reads {
            result.writes = true;
        }
        let destructive = match first {
            "drop" | "truncate" => true,
            "delete" | "update" => !has("where") || always_true(&statement),
            "alter" => has("drop"),
            "grant" | "revoke" => true,
            // `WITH x AS (…) DELETE FROM t` and friends.
            "with" => {
                (has("delete") || has("update")) && (!has("where") || always_true(&statement))
                    || has("drop")
            }
            _ => false,
        };
        if destructive {
            let preview: String = statement.chars().take(120).collect();
            result.destructive.push(preview);
        }
    }
    if !result.destructive.is_empty() {
        result.confirm = Some(database.to_string());
    }
    result
}

/// A WHERE that matches every row (`1=1`, `true`, `id = id`, `… or 1=1`):
/// the usual way a "safe" DELETE or UPDATE still empties a table.
fn always_true(statement: &str) -> bool {
    let lower = statement.to_lowercase();
    let Some(at) = lower.find(" where ") else {
        return false;
    };
    let mut condition = &lower[at + 7..];
    for end in [" order by ", " limit ", " returning "] {
        if let Some(i) = condition.find(end) {
            condition = &condition[..i];
        }
    }
    let padded = format!(
        " {} ",
        condition.split_whitespace().collect::<Vec<_>>().join(" ")
    );
    padded.split(" or ").any(|term| {
        let t: String = term.chars().filter(|c| !c.is_whitespace() && *c != '(' && *c != ')').collect();
        if matches!(t.as_str(), "1" | "true" | "notfalse" | "1<>0" | "1!=0") {
            return true;
        }
        // Both sides the same: `1=1`, `'a'='a'`, `id=id`.
        matches!(t.split_once('='), Some((a, b)) if !a.is_empty() && a == b && !a.ends_with(['<', '>', '!']))
    })
}

/// An identifier quoted for the engine.
pub fn ident(engine: DbEngine, name: &str) -> String {
    match engine {
        DbEngine::Mysql => format!("`{}`", name.replace('`', "``")),
        _ => format!("\"{}\"", name.replace('"', "\"\"")),
    }
}

/// `schema.table`, quoted.
pub fn table_ref(engine: DbEngine, schema: &str, table: &str) -> String {
    if schema.is_empty() {
        ident(engine, table)
    } else {
        format!("{}.{}", ident(engine, schema), ident(engine, table))
    }
}

/// The n-th parameter placeholder.
pub fn param(engine: DbEngine, n: usize) -> String {
    match engine {
        DbEngine::Postgres => format!("${n}"),
        _ => "?".into(),
    }
}

/// A WHERE clause from filters on known columns, with its parameters.
/// Unknown columns and operators are refused.
pub fn where_clause(
    engine: DbEngine,
    filters: &[DbFilter],
    columns: &[(String, String)],
    first_param: usize,
) -> Result<(String, Vec<String>), String> {
    let mut parts = Vec::new();
    let mut params = Vec::new();
    for filter in filters {
        let Some((_, data_type)) = columns.iter().find(|(c, _)| *c == filter.column) else {
            return Err(format!("no column {}", filter.column));
        };
        // Compare as text: works for every type without knowing them.
        let column = match engine {
            DbEngine::Postgres => format!("{}::text", ident(engine, &filter.column)),
            DbEngine::Mysql => format!("CAST({} AS CHAR)", ident(engine, &filter.column)),
            _ => format!("CAST({} AS TEXT)", ident(engine, &filter.column)),
        };
        let _ = data_type;
        let op = match filter.op.as_str() {
            "is_null" => {
                parts.push(format!("{} IS NULL", ident(engine, &filter.column)));
                continue;
            }
            "not_null" => {
                parts.push(format!("{} IS NOT NULL", ident(engine, &filter.column)));
                continue;
            }
            "=" | "!=" | "<" | ">" | "<=" | ">=" => filter.op.clone(),
            "like" => "LIKE".into(),
            other => return Err(format!("unknown operator {other}")),
        };
        params.push(filter.value.clone().unwrap_or_default());
        parts.push(format!(
            "{column} {op} {}",
            param(engine, first_param + params.len() - 1)
        ));
    }
    Ok(if parts.is_empty() {
        (String::new(), params)
    } else {
        (format!(" WHERE {}", parts.join(" AND ")), params)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_statements_outside_quotes_and_comments() {
        let s = statements(
            "select ';' from t; -- drop table x;\nupdate t set a = 1 where id = 2; /* ; */ select 1",
        );
        assert_eq!(s.len(), 3);
        assert!(s[0].starts_with("select ';'"));
        assert!(!s[1].contains("drop"));
    }

    #[test]
    fn destructive_statements_need_confirmation() {
        for sql in [
            "DROP TABLE users",
            "truncate sessions",
            "delete from users",
            "UPDATE users SET admin = true",
            "alter table users drop column email",
            "select 1; drop database app",
            "delete from users where 1=1",
            "DELETE FROM users WHERE (1 = 1)",
            "update users set admin = true where true",
            "delete from users where id = 3 or 1=1",
            "delete from users where id = id",
        ] {
            let c = check(sql, "app");
            assert!(!c.destructive.is_empty(), "{sql}");
            assert_eq!(c.confirm.as_deref(), Some("app"));
        }
        for sql in [
            "select * from users where name = 'drop table'",
            "delete from users where id = 3",
            "delete from users where id >= 1",
            "delete from users where a = b or id = 2",
            "update users set name = 'x' where id = 1",
            "insert into t values (1)",
        ] {
            assert!(check(sql, "app").destructive.is_empty(), "{sql}");
        }
        assert!(!check("select 1", "app").writes);
        assert!(check("insert into t values (1)", "app").writes);
    }

    #[test]
    fn identifiers_and_filters_are_safe() {
        assert_eq!(ident(DbEngine::Postgres, "we\"ird"), "\"we\"\"ird\"");
        assert_eq!(ident(DbEngine::Mysql, "a`b"), "`a``b`");
        let columns = vec![("name".to_string(), "text".to_string())];
        let (sql, params) = where_clause(
            DbEngine::Postgres,
            &[DbFilter {
                column: "name".into(),
                op: "like".into(),
                value: Some("%x'; drop%".into()),
            }],
            &columns,
            1,
        )
        .unwrap();
        assert_eq!(sql, " WHERE \"name\"::text LIKE $1");
        assert_eq!(params, vec!["%x'; drop%".to_string()]);
        assert!(
            where_clause(
                DbEngine::Postgres,
                &[DbFilter {
                    column: "nope".into(),
                    op: "=".into(),
                    value: None
                }],
                &columns,
                1
            )
            .is_err()
        );
        assert!(
            where_clause(
                DbEngine::Postgres,
                &[DbFilter {
                    column: "name".into(),
                    op: "; drop".into(),
                    value: None
                }],
                &columns,
                1
            )
            .is_err()
        );
    }
}
