//! The database manager's drivers: PostgreSQL, MySQL/MariaDB, SQLite and
//! Redis/Valkey behind one small interface. Values travel as text (or
//! NULL); identifiers the manager writes come from the database's own
//! catalog, and every value is a parameter.

pub mod detect;
pub mod resp;
pub mod sql;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use locdev_core::db::{
    DbColumn, DbEngine, DbFilter, DbForeignKey, DbIndex, DbRows, DbTable, DbTableInfo, RedisKey,
    RedisValue,
};
use mysql_async::prelude::Queryable;

pub use detect::Resolved;
use sql::{ident, param, table_ref};

/// Rows returned at most by one request.
pub const MAX_ROWS: usize = 1000;

#[derive(Debug, Clone)]
pub struct DbError {
    pub message: String,
    /// The server is there but the database isn't (offer to create it).
    pub missing_database: bool,
}

impl DbError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            missing_database: false,
        }
    }
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

type Result<T> = std::result::Result<T, DbError>;

fn pg_err(e: tokio_postgres::Error) -> DbError {
    let missing = e.code().is_some_and(|c| c.code() == "3D000");
    let message = match e.as_db_error() {
        Some(db) => db.message().to_string(),
        None => e.to_string(),
    };
    DbError {
        message,
        missing_database: missing,
    }
}

fn my_err(e: mysql_async::Error) -> DbError {
    let missing = matches!(&e, mysql_async::Error::Server(s) if s.code == 1049);
    DbError {
        message: e.to_string(),
        missing_database: missing,
    }
}

fn lite_err(e: rusqlite::Error) -> DbError {
    DbError::new(e.to_string())
}

pub enum Client {
    Postgres(tokio_postgres::Client),
    Mysql(mysql_async::Conn),
    Sqlite(Arc<Mutex<rusqlite::Connection>>),
    Redis(resp::Resp),
}

/// Opens a connection; `database` overrides the configured one.
pub async fn open(r: &Resolved, database: Option<&str>) -> Result<Client> {
    let c = &r.public;
    let host = c.host.clone().unwrap_or_else(|| "127.0.0.1".into());
    let db = database.map(str::to_string).or_else(|| c.database.clone());
    match c.engine {
        DbEngine::Postgres => {
            let mut config = tokio_postgres::Config::new();
            config
                .host(&host)
                .port(c.port.unwrap_or(5432))
                .user(c.username.as_deref().unwrap_or("postgres"))
                .dbname(db.as_deref().unwrap_or("postgres"))
                .connect_timeout(Duration::from_secs(5))
                .application_name("locdev");
            if let Some(p) = &r.password {
                config.password(p);
            }
            let (client, connection) = config
                .connect(tokio_postgres::NoTls)
                .await
                .map_err(pg_err)?;
            tokio::spawn(async move {
                let _ = connection.await;
            });
            client
                .batch_execute("SET statement_timeout = 30000")
                .await
                .map_err(pg_err)?;
            Ok(Client::Postgres(client))
        }
        DbEngine::Mysql => {
            let mut opts = mysql_async::OptsBuilder::default()
                .ip_or_hostname(host)
                .tcp_port(c.port.unwrap_or(3306))
                .user(c.username.clone())
                .pass(r.password.clone())
                .prefer_socket(false);
            if let Some(db) = db.filter(|d| !d.is_empty()) {
                opts = opts.db_name(Some(db));
            }
            let conn = tokio::time::timeout(Duration::from_secs(5), mysql_async::Conn::new(opts))
                .await
                .map_err(|_| DbError::new("timed out connecting"))?
                .map_err(my_err)?;
            Ok(Client::Mysql(conn))
        }
        DbEngine::Sqlite => {
            let path: PathBuf = r
                .sqlite_path
                .clone()
                .ok_or_else(|| DbError::new("no database file"))?;
            if !path.is_file() {
                return Err(DbError {
                    message: format!("{} doesn't exist", path.display()),
                    missing_database: true,
                });
            }
            let conn = tokio::task::spawn_blocking(move || {
                let conn = rusqlite::Connection::open_with_flags(
                    &path,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                        | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
                )?;
                conn.busy_timeout(Duration::from_secs(5))?;
                Ok::<_, rusqlite::Error>(conn)
            })
            .await
            .map_err(|e| DbError::new(e.to_string()))?
            .map_err(lite_err)?;
            Ok(Client::Sqlite(Arc::new(Mutex::new(conn))))
        }
        DbEngine::Redis => {
            let index = db.and_then(|d| d.parse().ok()).unwrap_or(0);
            let resp = resp::Resp::connect(
                &host,
                c.port.unwrap_or(6379),
                c.username.as_deref(),
                r.password.as_deref(),
                index,
            )
            .await
            .map_err(DbError::new)?;
            Ok(Client::Redis(resp))
        }
    }
}

/// Creates a database on the server (PostgreSQL, MySQL); SQLite creates the file.
pub async fn create_database(r: &Resolved, name: &str) -> Result<()> {
    let server = matches!(r.public.engine, DbEngine::Postgres | DbEngine::Mysql);
    if server
        && (name.is_empty()
            || name.len() > 63
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
    {
        return Err(DbError::new(format!(
            "`{name}` is not a valid database name"
        )));
    }
    match r.public.engine {
        DbEngine::Postgres => {
            let Client::Postgres(client) = open(r, Some("postgres")).await? else {
                unreachable!()
            };
            client
                .batch_execute(&format!(
                    "CREATE DATABASE {}",
                    ident(DbEngine::Postgres, name)
                ))
                .await
                .map_err(pg_err)
        }
        DbEngine::Mysql => {
            let mut stripped = r.clone();
            stripped.public.database = None;
            let Client::Mysql(mut conn) = open(&stripped, None).await? else {
                unreachable!()
            };
            conn.query_drop(format!("CREATE DATABASE {}", ident(DbEngine::Mysql, name)))
                .await
                .map_err(my_err)
        }
        DbEngine::Sqlite => {
            let path = r
                .sqlite_path
                .clone()
                .ok_or_else(|| DbError::new("no database file"))?;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| DbError::new(e.to_string()))?;
            }
            rusqlite::Connection::open(&path)
                .map(|_| ())
                .map_err(lite_err)
        }
        DbEngine::Redis => Err(DbError::new(
            "Redis has numbered databases; nothing to create",
        )),
    }
}

fn my_value(v: mysql_async::Value) -> Option<String> {
    use mysql_async::Value;
    match v {
        Value::NULL => None,
        Value::Bytes(b) => Some(String::from_utf8_lossy(&b).into_owned()),
        Value::Int(i) => Some(i.to_string()),
        Value::UInt(u) => Some(u.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Double(d) => Some(d.to_string()),
        Value::Date(y, mo, d, h, mi, s, us) => Some(if h == 0 && mi == 0 && s == 0 && us == 0 {
            format!("{y:04}-{mo:02}-{d:02}")
        } else {
            format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
        }),
        Value::Time(neg, d, h, mi, s, _) => Some(format!(
            "{}{:02}:{mi:02}:{s:02}",
            if neg { "-" } else { "" },
            d * 24 + u32::from(h)
        )),
    }
}

fn lite_value(v: rusqlite::types::ValueRef) -> Option<String> {
    use rusqlite::types::ValueRef;
    match v {
        ValueRef::Null => None,
        ValueRef::Integer(i) => Some(i.to_string()),
        ValueRef::Real(f) => Some(f.to_string()),
        ValueRef::Text(t) => Some(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Some(format!("<{} bytes>", b.len())),
    }
}

fn lite<T: Send + 'static>(
    conn: &Arc<Mutex<rusqlite::Connection>>,
    f: impl FnOnce(&rusqlite::Connection) -> rusqlite::Result<T> + Send + 'static,
) -> impl std::future::Future<Output = Result<T>> {
    let conn = conn.clone();
    async move {
        tokio::task::spawn_blocking(move || f(&conn.lock().unwrap()))
            .await
            .map_err(|e| DbError::new(e.to_string()))?
            .map_err(lite_err)
    }
}

impl Client {
    pub fn engine(&self) -> DbEngine {
        match self {
            Self::Postgres(_) => DbEngine::Postgres,
            Self::Mysql(_) => DbEngine::Mysql,
            Self::Sqlite(_) => DbEngine::Sqlite,
            Self::Redis(_) => DbEngine::Redis,
        }
    }

    pub async fn databases(&mut self) -> Result<Vec<String>> {
        match self {
            Self::Postgres(c) => Ok(c
                .query(
                    "SELECT datname FROM pg_database WHERE NOT datistemplate ORDER BY 1",
                    &[],
                )
                .await
                .map_err(pg_err)?
                .iter()
                .map(|r| r.get(0))
                .collect()),
            Self::Mysql(c) => c.query("SHOW DATABASES").await.map_err(my_err),
            Self::Sqlite(_) => Ok(vec!["main".into()]),
            Self::Redis(_) => Ok((0..16).map(|i| i.to_string()).collect()),
        }
    }

    pub async fn tables(&mut self) -> Result<Vec<DbTable>> {
        match self {
            Self::Postgres(c) => Ok(c
                .query(
                    "SELECT t.table_schema, t.table_name, CASE WHEN t.table_type = 'VIEW' THEN 'view' ELSE 'table' END,
                            (SELECT c.reltuples::bigint FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                             WHERE n.nspname = t.table_schema AND c.relname = t.table_name)
                     FROM information_schema.tables t
                     WHERE t.table_schema NOT IN ('pg_catalog', 'information_schema')
                     ORDER BY 1, 2",
                    &[],
                )
                .await
                .map_err(pg_err)?
                .iter()
                .map(|r| DbTable {
                    schema: r.get(0),
                    name: r.get(1),
                    kind: r.get(2),
                    rows_estimate: r.get::<_, Option<i64>>(3).filter(|n| *n >= 0),
                })
                .collect()),
            Self::Mysql(c) => {
                let rows: Vec<(String, String, Option<i64>)> = c
                    .query(
                        "SELECT TABLE_NAME, IF(TABLE_TYPE = 'VIEW', 'view', 'table'), TABLE_ROWS
                         FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() ORDER BY 1",
                    )
                    .await
                    .map_err(my_err)?;
                Ok(rows
                    .into_iter()
                    .map(|(name, kind, rows)| DbTable { name, schema: String::new(), kind, rows_estimate: rows })
                    .collect())
            }
            Self::Sqlite(conn) => {
                lite(conn, |c| {
                    let mut stmt = c.prepare(
                        "SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY 1",
                    )?;
                    let rows = stmt.query_map([], |r| {
                        Ok(DbTable { name: r.get(0)?, schema: String::new(), kind: r.get(1)?, rows_estimate: None })
                    })?;
                    rows.collect()
                })
                .await
            }
            Self::Redis(_) => Ok(Vec::new()),
        }
    }

    pub async fn table_info(&mut self, schema: &str, name: &str) -> Result<DbTableInfo> {
        let tables = self.tables().await?;
        let table = tables
            .into_iter()
            .find(|t| t.name == name && (schema.is_empty() || t.schema == schema))
            .ok_or_else(|| DbError::new(format!("no table {name}")))?;
        let (schema, name) = (table.schema.clone(), table.name.clone());
        let (columns, indexes, foreign_keys) = match self {
            Self::Postgres(c) => {
                let columns = c
                    .query(
                        "SELECT a.attname, format_type(a.atttypid, a.atttypmod), NOT a.attnotnull,
                                pg_get_expr(d.adbin, d.adrelid),
                                COALESCE((SELECT true FROM pg_index i WHERE i.indrelid = a.attrelid AND i.indisprimary AND a.attnum = ANY(i.indkey)), false)
                         FROM pg_attribute a
                         JOIN pg_class t ON t.oid = a.attrelid JOIN pg_namespace n ON n.oid = t.relnamespace
                         LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
                         WHERE n.nspname = $1 AND t.relname = $2 AND a.attnum > 0 AND NOT a.attisdropped
                         ORDER BY a.attnum",
                        &[&schema, &name],
                    )
                    .await
                    .map_err(pg_err)?
                    .iter()
                    .map(|r| DbColumn {
                        name: r.get(0),
                        data_type: r.get(1),
                        nullable: r.get(2),
                        default: r.get(3),
                        primary_key: r.get(4),
                    })
                    .collect();
                let indexes = c
                    .query(
                        "SELECT i.relname, ix.indisunique,
                                array_to_string(ARRAY(SELECT a.attname FROM unnest(ix.indkey) k JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = k), ',')
                         FROM pg_class t JOIN pg_index ix ON t.oid = ix.indrelid JOIN pg_class i ON i.oid = ix.indexrelid
                         JOIN pg_namespace n ON n.oid = t.relnamespace
                         WHERE n.nspname = $1 AND t.relname = $2 ORDER BY 1",
                        &[&schema, &name],
                    )
                    .await
                    .map_err(pg_err)?
                    .iter()
                    .map(|r| DbIndex {
                        name: r.get(0),
                        unique: r.get(1),
                        columns: r.get::<_, String>(2).split(',').filter(|s| !s.is_empty()).map(str::to_string).collect(),
                    })
                    .collect();
                let fks = c
                    .query(
                        "SELECT con.conname,
                                array_to_string(ARRAY(SELECT a.attname FROM unnest(con.conkey) k JOIN pg_attribute a ON a.attrelid = con.conrelid AND a.attnum = k), ','),
                                ref.relname,
                                array_to_string(ARRAY(SELECT a.attname FROM unnest(con.confkey) k JOIN pg_attribute a ON a.attrelid = con.confrelid AND a.attnum = k), ',')
                         FROM pg_constraint con JOIN pg_class t ON t.oid = con.conrelid JOIN pg_namespace n ON n.oid = t.relnamespace
                         JOIN pg_class ref ON ref.oid = con.confrelid
                         WHERE con.contype = 'f' AND n.nspname = $1 AND t.relname = $2 ORDER BY 1",
                        &[&schema, &name],
                    )
                    .await
                    .map_err(pg_err)?
                    .iter()
                    .map(|r| DbForeignKey {
                        name: r.get(0),
                        columns: r.get::<_, String>(1).split(',').map(str::to_string).collect(),
                        references_table: r.get(2),
                        references_columns: r.get::<_, String>(3).split(',').map(str::to_string).collect(),
                    })
                    .collect();
                (columns, indexes, fks)
            }
            Self::Mysql(c) => {
                let cols: Vec<(String, String, String, Option<String>, String)> = c
                    .exec(
                        "SELECT COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE, COLUMN_DEFAULT, COLUMN_KEY FROM information_schema.COLUMNS
                         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
                        (name.clone(),),
                    )
                    .await
                    .map_err(my_err)?;
                let columns = cols
                    .into_iter()
                    .map(|(n, t, nullable, default, key)| DbColumn {
                        name: n,
                        data_type: t,
                        nullable: nullable == "YES",
                        default,
                        primary_key: key == "PRI",
                    })
                    .collect();
                let idx: Vec<(String, i64, String)> = c
                    .exec(
                        "SELECT INDEX_NAME, MAX(NON_UNIQUE), GROUP_CONCAT(COLUMN_NAME ORDER BY SEQ_IN_INDEX)
                         FROM information_schema.STATISTICS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? GROUP BY INDEX_NAME",
                        (name.clone(),),
                    )
                    .await
                    .map_err(my_err)?;
                let indexes = idx
                    .into_iter()
                    .map(|(n, non_unique, cols)| DbIndex {
                        name: n,
                        unique: non_unique == 0,
                        columns: cols.split(',').map(str::to_string).collect(),
                    })
                    .collect();
                let fk: Vec<(String, String, String, String)> = c
                    .exec(
                        "SELECT CONSTRAINT_NAME, GROUP_CONCAT(COLUMN_NAME ORDER BY ORDINAL_POSITION), MAX(REFERENCED_TABLE_NAME),
                                GROUP_CONCAT(REFERENCED_COLUMN_NAME ORDER BY ORDINAL_POSITION)
                         FROM information_schema.KEY_COLUMN_USAGE
                         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND REFERENCED_TABLE_NAME IS NOT NULL GROUP BY CONSTRAINT_NAME",
                        (name.clone(),),
                    )
                    .await
                    .map_err(my_err)?;
                let fks = fk
                    .into_iter()
                    .map(|(n, cols, rt, rc)| DbForeignKey {
                        name: n,
                        columns: cols.split(',').map(str::to_string).collect(),
                        references_table: rt,
                        references_columns: rc.split(',').map(str::to_string).collect(),
                    })
                    .collect();
                (columns, indexes, fks)
            }
            Self::Sqlite(conn) => {
                let table_name = name.clone();
                lite(conn, move |c| {
                    let quoted = ident(DbEngine::Sqlite, &table_name);
                    let mut stmt = c.prepare(&format!("PRAGMA table_info({quoted})"))?;
                    let columns: Vec<DbColumn> = stmt
                        .query_map([], |r| {
                            Ok(DbColumn {
                                name: r.get(1)?,
                                data_type: r.get::<_, String>(2)?,
                                nullable: r.get::<_, i64>(3)? == 0,
                                default: r.get(4)?,
                                primary_key: r.get::<_, i64>(5)? > 0,
                            })
                        })?
                        .collect::<rusqlite::Result<_>>()?;
                    let mut stmt = c.prepare(&format!("PRAGMA index_list({quoted})"))?;
                    let list: Vec<(String, bool)> = stmt
                        .query_map([], |r| {
                            Ok((r.get::<_, String>(1)?, r.get::<_, i64>(2)? == 1))
                        })?
                        .collect::<rusqlite::Result<_>>()?;
                    let mut indexes = Vec::new();
                    for (index, unique) in list {
                        let mut stmt = c.prepare(&format!(
                            "PRAGMA index_info({})",
                            ident(DbEngine::Sqlite, &index)
                        ))?;
                        let cols: Vec<String> = stmt
                            .query_map([], |r| r.get::<_, Option<String>>(2))?
                            .filter_map(|c| c.ok().flatten())
                            .collect();
                        indexes.push(DbIndex {
                            name: index,
                            columns: cols,
                            unique,
                        });
                    }
                    let mut stmt = c.prepare(&format!("PRAGMA foreign_key_list({quoted})"))?;
                    let fks: Vec<DbForeignKey> = stmt
                        .query_map([], |r| {
                            Ok(DbForeignKey {
                                name: format!("fk_{}", r.get::<_, i64>(0)?),
                                references_table: r.get(2)?,
                                columns: vec![r.get(3)?],
                                references_columns: vec![
                                    r.get::<_, Option<String>>(4)?.unwrap_or_default(),
                                ],
                            })
                        })?
                        .collect::<rusqlite::Result<_>>()?;
                    Ok((columns, indexes, fks))
                })
                .await?
            }
            Self::Redis(_) => return Err(DbError::new("Redis has keys, not tables")),
        };
        Ok(DbTableInfo {
            table,
            columns,
            indexes,
            foreign_keys,
        })
    }

    /// One page of a table, filtered and ordered by known columns.
    pub async fn rows(
        &mut self,
        info: &DbTableInfo,
        page: u64,
        page_size: u64,
        order_by: Option<&str>,
        desc: bool,
        filters: &[DbFilter],
    ) -> Result<DbRows> {
        let engine = self.engine();
        let started = Instant::now();
        let page_size = page_size.clamp(1, 500);
        let columns: Vec<(String, String)> = info
            .columns
            .iter()
            .map(|c| (c.name.clone(), c.data_type.clone()))
            .collect();
        let (where_sql, params) =
            sql::where_clause(engine, filters, &columns, 1).map_err(DbError::new)?;
        let order = match order_by {
            Some(col) if columns.iter().any(|(c, _)| c == col) => {
                format!(
                    " ORDER BY {}{}",
                    ident(engine, col),
                    if desc { " DESC" } else { "" }
                )
            }
            Some(col) => return Err(DbError::new(format!("no column {col}"))),
            None => String::new(),
        };
        let table = table_ref(engine, &info.table.schema, &info.table.name);
        let select_list = match engine {
            // Every column as text, so any type reads the same way.
            DbEngine::Postgres => columns
                .iter()
                .map(|(c, _)| format!("{}::text", ident(engine, c)))
                .collect::<Vec<_>>()
                .join(", "),
            _ => "*".into(),
        };
        let page_sql = format!(
            "SELECT {select_list} FROM {table}{where_sql}{order} LIMIT {page_size} OFFSET {}",
            page * page_size
        );
        let count_sql = format!("SELECT COUNT(*) FROM {table}{where_sql}");
        let names: Vec<String> = columns.iter().map(|(c, _)| c.clone()).collect();
        let (rows, total) = match self {
            Self::Postgres(c) => {
                let p: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                    params.iter().map(|s| s as _).collect();
                let rows = c.query(&page_sql, &p).await.map_err(pg_err)?;
                let total: i64 = c.query_one(&count_sql, &p).await.map_err(pg_err)?.get(0);
                (
                    rows.iter()
                        .map(|r| {
                            (0..names.len())
                                .map(|i| r.get::<_, Option<String>>(i))
                                .collect()
                        })
                        .collect(),
                    total as u64,
                )
            }
            Self::Mysql(c) => {
                let p: Vec<mysql_async::Value> = params
                    .iter()
                    .map(|s| mysql_async::Value::from(s.as_str()))
                    .collect();
                let rows: Vec<mysql_async::Row> =
                    c.exec(&page_sql, p.clone()).await.map_err(my_err)?;
                let total: Option<u64> = c.exec_first(&count_sql, p).await.map_err(my_err)?;
                (
                    rows.into_iter()
                        .map(|r| r.unwrap().into_iter().map(my_value).collect())
                        .collect(),
                    total.unwrap_or(0),
                )
            }
            Self::Sqlite(conn) => {
                let n = names.len();
                lite(conn, move |c| {
                    let mut stmt = c.prepare(&page_sql)?;
                    let rows: Vec<Vec<Option<String>>> = stmt
                        .query_map(rusqlite::params_from_iter(params.iter()), |r| {
                            Ok((0..n)
                                .map(|i| r.get_ref(i).ok().and_then(lite_value))
                                .collect())
                        })?
                        .collect::<rusqlite::Result<_>>()?;
                    let total: i64 =
                        c.query_row(&count_sql, rusqlite::params_from_iter(params.iter()), |r| {
                            r.get(0)
                        })?;
                    Ok((rows, total as u64))
                })
                .await?
            }
            Self::Redis(_) => return Err(DbError::new("Redis has keys, not tables")),
        };
        Ok(DbRows {
            columns: names,
            rows,
            total: Some(total),
            affected: None,
            truncated: false,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    }

    /// Runs SQL the user wrote; returns the last statement's result.
    pub async fn query(&mut self, text: &str) -> Result<DbRows> {
        let started = Instant::now();
        let mut out = DbRows::default();
        match self {
            Self::Postgres(c) => {
                for message in c.simple_query(text).await.map_err(pg_err)? {
                    match message {
                        tokio_postgres::SimpleQueryMessage::RowDescription(cols) => {
                            out = DbRows {
                                columns: cols.iter().map(|c| c.name().to_string()).collect(),
                                ..Default::default()
                            };
                        }
                        tokio_postgres::SimpleQueryMessage::Row(row) => {
                            if out.columns.is_empty() {
                                out.columns =
                                    row.columns().iter().map(|c| c.name().to_string()).collect();
                            }
                            if out.rows.len() < MAX_ROWS {
                                out.rows.push(
                                    (0..row.len())
                                        .map(|i| row.get(i).map(str::to_string))
                                        .collect(),
                                );
                            } else {
                                out.truncated = true;
                            }
                        }
                        tokio_postgres::SimpleQueryMessage::CommandComplete(n)
                            if out.columns.is_empty() =>
                        {
                            out.affected = Some(n);
                        }
                        _ => {}
                    }
                }
            }
            Self::Mysql(c) => {
                let mut result = c.query_iter(text).await.map_err(my_err)?;
                loop {
                    let columns: Vec<String> = result
                        .columns()
                        .map(|cols| cols.iter().map(|c| c.name_str().into_owned()).collect())
                        .unwrap_or_default();
                    let rows: Vec<mysql_async::Row> = result.collect().await.map_err(my_err)?;
                    if !columns.is_empty() {
                        out = DbRows {
                            columns,
                            ..Default::default()
                        };
                        out.truncated = rows.len() > MAX_ROWS;
                        out.rows = rows
                            .into_iter()
                            .take(MAX_ROWS)
                            .map(|r| r.unwrap().into_iter().map(my_value).collect())
                            .collect();
                    } else {
                        out = DbRows {
                            affected: Some(result.affected_rows()),
                            ..Default::default()
                        };
                    }
                    if result.is_empty() {
                        break;
                    }
                }
            }
            Self::Sqlite(conn) => {
                let statements = sql::statements(text);
                out = lite(conn, move |c| {
                    let mut out = DbRows::default();
                    for statement in statements {
                        let mut stmt = c.prepare(&statement)?;
                        let columns: Vec<String> =
                            stmt.column_names().map(str::to_string).collect();
                        if columns.is_empty() {
                            let n = stmt.execute([])?;
                            out = DbRows {
                                affected: Some(n as u64),
                                ..Default::default()
                            };
                            continue;
                        }
                        let n = columns.len();
                        let mut rows = stmt.query([])?;
                        out = DbRows {
                            columns,
                            ..Default::default()
                        };
                        while let Some(row) = rows.next()? {
                            if out.rows.len() >= MAX_ROWS {
                                out.truncated = true;
                                break;
                            }
                            out.rows.push(
                                (0..n)
                                    .map(|i| row.get_ref(i).ok().and_then(lite_value))
                                    .collect(),
                            );
                        }
                    }
                    Ok(out)
                })
                .await?;
            }
            Self::Redis(_) => return Err(DbError::new("use the key browser for Redis")),
        }
        out.elapsed_ms = started.elapsed().as_millis() as u64;
        Ok(out)
    }

    /// The `WHERE` that pins one row by its primary key.
    fn key_clause(
        engine: DbEngine,
        info: &DbTableInfo,
        key: &[(String, Option<String>)],
        first: usize,
    ) -> Result<(String, Vec<Option<String>>)> {
        let pk: Vec<&DbColumn> = info.columns.iter().filter(|c| c.primary_key).collect();
        if pk.is_empty() {
            return Err(DbError::new(
                "this table has no primary key; edit it with SQL",
            ));
        }
        let mut parts = Vec::new();
        let mut params = Vec::new();
        for col in &pk {
            let value = key
                .iter()
                .find(|(k, _)| *k == col.name)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| DbError::new(format!("missing key column {}", col.name)))?;
            params.push(value);
            parts.push(format!(
                "{} = {}",
                ident(engine, &col.name),
                typed(
                    engine,
                    &param(engine, first + params.len() - 1),
                    &col.data_type
                )
            ));
        }
        Ok((parts.join(" AND "), params))
    }

    async fn execute(&mut self, statement: &str, params: Vec<Option<String>>) -> Result<u64> {
        match self {
            Self::Postgres(c) => {
                let p: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                    params.iter().map(|s| s as _).collect();
                c.execute(statement, &p).await.map_err(pg_err)
            }
            Self::Mysql(c) => {
                let p: Vec<mysql_async::Value> = params
                    .into_iter()
                    .map(|v| {
                        v.map(mysql_async::Value::from)
                            .unwrap_or(mysql_async::Value::NULL)
                    })
                    .collect();
                c.exec_drop(statement, p).await.map_err(my_err)?;
                Ok(c.affected_rows())
            }
            Self::Sqlite(conn) => {
                let statement = statement.to_string();
                lite(conn, move |c| {
                    c.execute(&statement, rusqlite::params_from_iter(params.iter()))
                        .map(|n| n as u64)
                })
                .await
            }
            Self::Redis(_) => Err(DbError::new("not a SQL database")),
        }
    }

    /// Changes one row (by primary key); exactly one row must match.
    pub async fn update_row(
        &mut self,
        info: &DbTableInfo,
        key: &[(String, Option<String>)],
        changes: &[(String, Option<String>)],
    ) -> Result<u64> {
        let engine = self.engine();
        let mut sets = Vec::new();
        let mut params = Vec::new();
        for (col, value) in changes {
            let column = info
                .columns
                .iter()
                .find(|c| &c.name == col)
                .ok_or_else(|| DbError::new(format!("no column {col}")))?;
            params.push(value.clone());
            sets.push(format!(
                "{} = {}",
                ident(engine, col),
                typed(engine, &param(engine, params.len()), &column.data_type)
            ));
        }
        if sets.is_empty() {
            return Ok(0);
        }
        let (where_sql, key_params) = Self::key_clause(engine, info, key, params.len() + 1)?;
        params.extend(key_params);
        let statement = format!(
            "UPDATE {} SET {} WHERE {where_sql}",
            table_ref(engine, &info.table.schema, &info.table.name),
            sets.join(", ")
        );
        self.execute(&statement, params).await
    }

    pub async fn insert_row(
        &mut self,
        info: &DbTableInfo,
        values: &[(String, Option<String>)],
    ) -> Result<u64> {
        let engine = self.engine();
        let mut cols = Vec::new();
        let mut placeholders = Vec::new();
        let mut params = Vec::new();
        for (col, value) in values {
            let column = info
                .columns
                .iter()
                .find(|c| &c.name == col)
                .ok_or_else(|| DbError::new(format!("no column {col}")))?;
            params.push(value.clone());
            cols.push(ident(engine, col));
            placeholders.push(typed(
                engine,
                &param(engine, params.len()),
                &column.data_type,
            ));
        }
        let statement = if cols.is_empty() {
            format!(
                "INSERT INTO {} DEFAULT VALUES",
                table_ref(engine, &info.table.schema, &info.table.name)
            )
        } else {
            format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_ref(engine, &info.table.schema, &info.table.name),
                cols.join(", "),
                placeholders.join(", ")
            )
        };
        self.execute(&statement, params).await
    }

    pub async fn delete_row(
        &mut self,
        info: &DbTableInfo,
        key: &[(String, Option<String>)],
    ) -> Result<u64> {
        let engine = self.engine();
        let (where_sql, params) = Self::key_clause(engine, info, key, 1)?;
        let statement = format!(
            "DELETE FROM {} WHERE {where_sql}",
            table_ref(engine, &info.table.schema, &info.table.name)
        );
        self.execute(&statement, params).await
    }

    pub fn redis(&mut self) -> Result<&mut resp::Resp> {
        match self {
            Self::Redis(r) => Ok(r),
            _ => Err(DbError::new("not a Redis connection")),
        }
    }
}

/// A text parameter converted to the column's type (PostgreSQL needs it;
/// MySQL and SQLite convert by themselves).
fn typed(engine: DbEngine, placeholder: &str, data_type: &str) -> String {
    match engine {
        // `format_type` output from the catalog, never user input.
        DbEngine::Postgres => format!("CAST({placeholder}::text AS {data_type})"),
        _ => placeholder.to_string(),
    }
}

pub use resp::{redis_get, redis_scan};

/// Re-exported for the daemon's Redis requests.
pub type RedisKeys = Vec<RedisKey>;
pub type RedisEntry = RedisValue;
