//! Rendering for `locdev db …`.

use locdev_core::db::{DbConnection, DbRows, DbTable, DbTableInfo, RedisKey, RedisValue};
use serde_json::json;

use super::{Output, Style, truncate};

impl Output {
    pub fn db_connections(&self, list: &[DbConnection]) {
        if self.json {
            self.print_json(list);
            return;
        }
        if list.is_empty() {
            println!(
                "No databases found. Sites declare them in .env (DB_*, DATABASE_URL, REDIS_URL); instances come from `locdev service create`."
            );
        }
        for c in list {
            let place = match (&c.host, c.port) {
                (Some(h), Some(p)) => format!("{h}:{p}"),
                _ => String::new(),
            };
            println!(
                "{:<22} {:<28} {:<22} {}{}",
                c.id,
                c.label,
                place,
                self.paint(Style::Dim, &c.source),
                if c.remote {
                    self.paint(Style::Yellow, "  remote (--allow-remote)")
                } else {
                    String::new()
                }
            );
            if let Some(user) = &c.username {
                println!(
                    "{:<22} user {user}{}",
                    "",
                    if c.has_password {
                        self.paint(Style::Dim, ", password set (not shown)")
                    } else {
                        String::new()
                    }
                );
            }
        }
    }

    pub fn db_schema(&self, databases: &[String], tables: &[DbTable]) {
        if self.json {
            self.print_json(json!({ "databases": databases, "tables": tables }));
            return;
        }
        println!(
            "{} {}",
            self.paint(Style::Dim, "databases:"),
            databases.join(", ")
        );
        for t in tables {
            let name = if t.schema.is_empty() {
                t.name.clone()
            } else {
                format!("{}.{}", t.schema, t.name)
            };
            println!(
                "  {:<40} {:<6} {}",
                name,
                t.kind,
                t.rows_estimate
                    .map(|n| format!("~{n} rows"))
                    .unwrap_or_default()
            );
        }
    }

    pub fn db_table(&self, info: &DbTableInfo) {
        if self.json {
            self.print_json(info);
            return;
        }
        for c in &info.columns {
            println!(
                "{} {:<28} {:<28} {}{}",
                if c.primary_key {
                    self.paint(Style::Yellow, "PK")
                } else {
                    "  ".into()
                },
                c.name,
                c.data_type,
                if c.nullable { "null" } else { "not null" },
                c.default
                    .as_ref()
                    .map(|d| self.paint(Style::Dim, &format!("  default {d}")))
                    .unwrap_or_default()
            );
        }
        for i in &info.indexes {
            println!(
                "{} {} ({}){}",
                self.paint(Style::Dim, "index"),
                i.name,
                i.columns.join(", "),
                if i.unique { " unique" } else { "" }
            );
        }
        for f in &info.foreign_keys {
            println!(
                "{} {} ({}) → {} ({})",
                self.paint(Style::Dim, "fk"),
                f.name,
                f.columns.join(", "),
                f.references_table,
                f.references_columns.join(", ")
            );
        }
    }

    pub fn db_rows(&self, r: &DbRows) {
        if self.json {
            self.print_json(r);
            return;
        }
        if r.columns.is_empty() {
            println!(
                "{} row(s) affected  {}",
                r.affected.unwrap_or(0),
                self.paint(Style::Dim, &format!("{} ms", r.elapsed_ms))
            );
            return;
        }
        let cell = |v: &Option<String>| {
            v.as_deref()
                .map(|s| truncate(&s.replace('\n', "↵"), 40))
                .unwrap_or_else(|| "NULL".into())
        };
        let widths: Vec<usize> = (0..r.columns.len())
            .map(|i| {
                r.rows
                    .iter()
                    .map(|row| cell(&row[i]).chars().count())
                    .chain([r.columns[i].len()])
                    .max()
                    .unwrap_or(4)
                    .min(40)
            })
            .collect();
        let line = |values: Vec<String>| {
            values
                .iter()
                .zip(&widths)
                .map(|(v, w)| format!("{v:<w$}"))
                .collect::<Vec<_>>()
                .join("  ")
        };
        println!("{}", self.paint(Style::Bold, &line(r.columns.clone())));
        for row in &r.rows {
            println!("{}", line(row.iter().map(cell).collect()));
        }
        let mut footer = format!("{} row(s)", r.rows.len());
        if let Some(total) = r.total {
            footer.push_str(&format!(" of {total}"));
        }
        if r.truncated {
            footer.push_str(", more not shown");
        }
        println!(
            "{}",
            self.paint(Style::Dim, &format!("{footer}  {} ms", r.elapsed_ms))
        );
    }

    pub fn redis_keys(&self, keys: &[RedisKey], cursor: &str) {
        if self.json {
            self.print_json(json!({ "keys": keys, "cursor": cursor }));
            return;
        }
        if keys.is_empty() {
            println!("no keys on this page");
        }
        for k in keys {
            let ttl = if k.ttl < 0 {
                String::new()
            } else {
                format!("  ttl {}s", k.ttl)
            };
            println!(
                "{}  {}{}",
                k.key,
                self.paint(Style::Dim, &k.kind),
                self.paint(Style::Dim, &ttl)
            );
        }
        if cursor != "0" {
            self.note(&format!("more keys: --cursor {cursor}"));
        }
    }

    pub fn redis_value(&self, v: &RedisValue) {
        if self.json {
            self.print_json(v);
            return;
        }
        let ttl = if v.ttl < 0 {
            "no expiry".to_string()
        } else {
            format!("expires in {}s", v.ttl)
        };
        println!(
            "{}  {}",
            self.paint(Style::Bold, &v.key),
            self.paint(Style::Dim, &format!("{} · {ttl}", v.kind))
        );
        for (a, b) in &v.items {
            match b {
                Some(b) => println!("  {a}  {}", self.paint(Style::Dim, b)),
                None => println!("  {a}"),
            }
        }
        if v.truncated {
            self.note("(first page only)");
        }
    }
}
