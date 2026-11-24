//! `locdev db …`: the databases sites use, their tables and SQL.

use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::Subcommand;
use locdev_core::db::{DbConnection, DbEngine, DbFilter, RedisEdit, RowEdit};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::output::Output;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum DbCommand {
    /// Databases sites point at (from their .env) and LocDev's instances.
    #[command(visible_alias = "ls")]
    List { site: Option<String> },
    /// How a site connects (host, port, database, user; never the password).
    Connect {
        site: Option<String>,
        /// A connection id from `locdev db list` instead of the site's first database.
        #[arg(long = "conn")]
        connection: Option<String>,
    },
    /// Databases on the server and the tables of the current one.
    Tables {
        site: Option<String>,
        /// A connection id from `locdev db list` instead of the site's first database.
        #[arg(long = "conn")]
        connection: Option<String>,
        /// Open a database on another host (it may be production).
        #[arg(long)]
        allow_remote: bool,
    },
    /// A table's columns, indexes and foreign keys.
    Describe {
        /// `table` or `schema.table`.
        table: String,
        site: Option<String>,
        /// A connection id from `locdev db list` instead of the site's first database.
        #[arg(long = "conn")]
        connection: Option<String>,
        /// Open a database on another host (it may be production).
        #[arg(long)]
        allow_remote: bool,
    },
    /// Rows of a table, a page at a time.
    Rows {
        /// `table` or `schema.table`.
        table: String,
        site: Option<String>,
        /// A connection id from `locdev db list` instead of the site's first database.
        #[arg(long = "conn")]
        connection: Option<String>,
        /// `column=value` (repeatable).
        #[arg(long = "where", value_name = "COLUMN=VALUE")]
        filters: Vec<String>,
        /// Sort by this column.
        #[arg(long)]
        order_by: Option<String>,
        /// Sort descending.
        #[arg(long)]
        desc: bool,
        /// Page number, from 0.
        #[arg(long, default_value_t = 0)]
        page: u64,
        /// Rows per page (at most 1000).
        #[arg(long, default_value_t = 50)]
        limit: u64,
        /// Open a database on another host (it may be production).
        #[arg(long)]
        allow_remote: bool,
    },
    /// Run SQL. Destructive statements ask for the database's name.
    Query {
        /// One or more statements; quote them for the shell.
        sql: String,
        site: Option<String>,
        /// A connection id from `locdev db list` instead of the site's first database.
        #[arg(long = "conn")]
        connection: Option<String>,
        /// Open a database on another host (it may be production).
        #[arg(long)]
        allow_remote: bool,
    },
    /// Insert a row: `--set column=value` for each column you give.
    Insert {
        /// `table` or `schema.table`.
        table: String,
        site: Option<String>,
        #[arg(long = "set", value_name = "COLUMN=VALUE", required = true)]
        values: Vec<String>,
        /// Columns set to NULL (repeatable).
        #[arg(long = "null", value_name = "COLUMN")]
        nulls: Vec<String>,
        #[command(flatten)]
        target: Target,
    },
    /// Change one row, found by its primary key (`--key id=5`).
    Update {
        /// `table` or `schema.table`.
        table: String,
        site: Option<String>,
        /// Primary key column=value (repeat for composite keys).
        #[arg(long = "key", value_name = "COLUMN=VALUE", required = true)]
        key: Vec<String>,
        #[arg(long = "set", value_name = "COLUMN=VALUE")]
        values: Vec<String>,
        #[arg(long = "null", value_name = "COLUMN")]
        nulls: Vec<String>,
        #[command(flatten)]
        target: Target,
    },
    /// Delete one row, found by its primary key (asks first).
    Delete {
        /// `table` or `schema.table`.
        table: String,
        site: Option<String>,
        #[arg(long = "key", value_name = "COLUMN=VALUE", required = true)]
        key: Vec<String>,
        #[arg(short, long)]
        yes: bool,
        #[command(flatten)]
        target: Target,
    },
    /// Redis/Valkey keys, a page at a time (SCAN).
    Keys {
        /// Glob pattern (`session:*`).
        pattern: Option<String>,
        site: Option<String>,
        /// The cursor `locdev db keys` printed, for the next page.
        #[arg(long)]
        cursor: Option<String>,
        #[command(flatten)]
        target: Target,
    },
    /// A Redis/Valkey key's type, TTL and value.
    Get {
        key: String,
        site: Option<String>,
        #[command(flatten)]
        target: Target,
    },
    /// Set a Redis/Valkey string key.
    Set {
        key: String,
        value: String,
        site: Option<String>,
        #[command(flatten)]
        target: Target,
    },
    /// Set or remove (`none`) a Redis/Valkey key's expiry, in seconds.
    Expire {
        key: String,
        /// Seconds, or `none` to keep the key forever.
        seconds: String,
        site: Option<String>,
        #[command(flatten)]
        target: Target,
    },
    /// Delete a Redis/Valkey key (asks first).
    Del {
        key: String,
        site: Option<String>,
        #[arg(short, long)]
        yes: bool,
        #[command(flatten)]
        target: Target,
    },
    /// Create a database on the site's server (the one its config names by default).
    Create {
        site: Option<String>,
        /// Name to create (default: the one the site's config names).
        database: Option<String>,
        /// A connection id from `locdev db list` instead of the site's first database.
        #[arg(long = "conn")]
        connection: Option<String>,
    },
}

/// Which connection, and whether a remote one may be opened.
#[derive(clap::Args)]
pub(crate) struct Target {
    /// A connection id from `locdev db list` instead of the site's first one.
    #[arg(long = "conn")]
    connection: Option<String>,
    /// Open a database on another host (it may be production).
    #[arg(long)]
    allow_remote: bool,
}

/// `column=value` pairs plus columns set to NULL.
fn pairs(values: &[String], nulls: &[String]) -> anyhow::Result<Vec<(String, Option<String>)>> {
    let mut out = values
        .iter()
        .map(|v| {
            let (c, v) = v
                .split_once('=')
                .with_context(|| format!("`{v}`: use column=value"))?;
            Ok((c.to_string(), Some(v.to_string())))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    out.extend(nulls.iter().map(|c| (c.clone(), None)));
    Ok(out)
}

async fn connections(paths: &Paths, site: Option<String>) -> anyhow::Result<Vec<DbConnection>> {
    let Response::DbConnections { connections } =
        daemon_call(paths, Request::DbConnections { name: site }).await?
    else {
        bail!("unexpected response from daemon");
    };
    Ok(connections)
}

/// The connection to use: `--conn`, else the site's first SQL database.
async fn pick(
    paths: &Paths,
    site: Option<String>,
    explicit: Option<String>,
) -> anyhow::Result<DbConnection> {
    pick_kind(paths, site, explicit, false).await
}

/// Like [`pick`], for the site's Redis/Valkey connection.
async fn pick_redis(
    paths: &Paths,
    site: Option<String>,
    explicit: Option<String>,
) -> anyhow::Result<DbConnection> {
    let conn = pick_kind(paths, site, explicit, true).await?;
    if conn.engine != DbEngine::Redis {
        bail!("{} is not Redis or Valkey", conn.label);
    }
    Ok(conn)
}

async fn pick_kind(
    paths: &Paths,
    site: Option<String>,
    explicit: Option<String>,
    redis: bool,
) -> anyhow::Result<DbConnection> {
    let name = site::resolve(paths, site).await?.project.name;
    let list = connections(paths, Some(name.clone())).await?;
    match explicit {
        Some(id) => list
            .into_iter()
            .find(|c| c.id == id)
            .with_context(|| format!("no connection {id}")),
        None => list
            .into_iter()
            .find(|c| {
                c.project.as_deref() == Some(name.as_str())
                    && if redis {
                        c.engine == DbEngine::Redis
                    } else {
                        c.engine.is_sql()
                    }
            })
            .with_context(|| {
                format!(
                    "{name} has no {} in its .env; pass --conn (see `locdev db list`)",
                    if redis { "Redis" } else { "SQL database" }
                )
            }),
    }
}

pub async fn run(command: DbCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        DbCommand::List { site } => {
            let site = match site {
                Some(s) => Some(site::resolve(paths, Some(s)).await?.project.name),
                None => None,
            };
            out.db_connections(&connections(paths, site).await?);
        }
        DbCommand::Connect { site, connection } => {
            out.db_connections(&[pick(paths, site, connection).await?]);
        }
        DbCommand::Tables {
            site,
            connection,
            allow_remote,
        } => {
            let conn = pick(paths, site, connection).await?;
            let Response::DbSchema { databases, tables } = daemon_call(
                paths,
                Request::DbSchema {
                    connection: conn.id,
                    allow_remote,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.db_schema(&databases, &tables);
        }
        DbCommand::Describe {
            table,
            site,
            connection,
            allow_remote,
        } => {
            let conn = pick(paths, site, connection).await?;
            let (schema, table) = split_table(&table);
            let Response::DbTableInfo(info) = daemon_call(
                paths,
                Request::DbTableInfo {
                    connection: conn.id,
                    table,
                    schema,
                    allow_remote,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.db_table(&info);
        }
        DbCommand::Rows {
            table,
            site,
            connection,
            filters,
            order_by,
            desc,
            page,
            limit,
            allow_remote,
        } => {
            let conn = pick(paths, site, connection).await?;
            let (schema, table) = split_table(&table);
            let filters = filters
                .iter()
                .map(|f| {
                    let (c, v) = f
                        .split_once('=')
                        .with_context(|| format!("`{f}`: use column=value"))?;
                    Ok(DbFilter {
                        column: c.into(),
                        op: "=".into(),
                        value: Some(v.into()),
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            let Response::DbRows(rows) = daemon_call(
                paths,
                Request::DbRows {
                    connection: conn.id,
                    table,
                    schema,
                    page,
                    page_size: Some(limit),
                    order_by,
                    desc,
                    filters,
                    allow_remote,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.db_rows(&rows);
        }
        DbCommand::Query {
            sql,
            site,
            connection,
            allow_remote,
        } => {
            let conn = pick(paths, site, connection).await?;
            if conn.engine == DbEngine::Redis {
                bail!(
                    "{} is Redis; use `locdev db keys` / `get` / `set`",
                    conn.label
                );
            }
            let Response::DbCheck(check) = daemon_call(
                paths,
                Request::DbCheck {
                    connection: conn.id.clone(),
                    sql: sql.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            let confirm = match &check.confirm {
                Some(phrase) => {
                    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                        bail!(
                            "this would destroy data ({}); run it on a terminal to confirm",
                            check.destructive.join("; ")
                        );
                    }
                    eprintln!(
                        "This destroys data in {}:\n  {}",
                        conn.label,
                        check.destructive.join("\n  ")
                    );
                    eprint!("Type the database's name ({phrase}) to run it: ");
                    Some(crate::vcs::read_secret()?)
                }
                None => None,
            };
            let Response::DbRows(rows) = daemon_call(
                paths,
                Request::DbQuery {
                    connection: conn.id,
                    sql,
                    confirm,
                    allow_remote,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.db_rows(&rows);
        }
        DbCommand::Insert {
            table,
            site,
            values,
            nulls,
            target,
        } => {
            let values = pairs(&values, &nulls)?;
            edit_row(paths, out, site, target, &table, RowEdit::Insert { values }).await?;
        }
        DbCommand::Update {
            table,
            site,
            key,
            values,
            nulls,
            target,
        } => {
            let changes = pairs(&values, &nulls)?;
            if changes.is_empty() {
                bail!("nothing to change; pass --set column=value or --null column");
            }
            let key = pairs(&key, &[])?;
            edit_row(
                paths,
                out,
                site,
                target,
                &table,
                RowEdit::Update { key, changes },
            )
            .await?;
        }
        DbCommand::Delete {
            table,
            site,
            key,
            yes,
            target,
        } => {
            let key = pairs(&key, &[])?;
            if !yes {
                let shown: Vec<String> = key
                    .iter()
                    .map(|(c, v)| format!("{c}={}", v.as_deref().unwrap_or("NULL")))
                    .collect();
                crate::work::confirm(&format!(
                    "Delete the row {} from {table}?",
                    shown.join(", ")
                ))?;
            }
            edit_row(paths, out, site, target, &table, RowEdit::Delete { key }).await?;
        }
        DbCommand::Keys {
            pattern,
            site,
            cursor,
            target,
        } => {
            let conn = pick_redis(paths, site, target.connection).await?;
            let Response::RedisKeys { cursor, keys } = daemon_call(
                paths,
                Request::RedisKeys {
                    connection: conn.id,
                    cursor,
                    pattern,
                    allow_remote: target.allow_remote,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.redis_keys(&keys, &cursor);
        }
        DbCommand::Get { key, site, target } => {
            let conn = pick_redis(paths, site, target.connection).await?;
            let Response::RedisValue(value) = daemon_call(
                paths,
                Request::RedisGet {
                    connection: conn.id,
                    key,
                    allow_remote: target.allow_remote,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.redis_value(&value);
        }
        DbCommand::Set {
            key,
            value,
            site,
            target,
        } => {
            let done = format!("set {key}");
            redis_edit(paths, site, target, RedisEdit::Set { key, value }).await?;
            out.message(&done);
        }
        DbCommand::Expire {
            key,
            seconds,
            site,
            target,
        } => {
            let seconds = match seconds.as_str() {
                "none" | "never" | "persist" => None,
                s => Some(
                    s.parse::<i64>()
                        .with_context(|| format!("`{s}`: seconds, or `none`"))?,
                ),
            };
            let done = match seconds {
                Some(s) => format!("{key} expires in {s}s"),
                None => format!("{key} no longer expires"),
            };
            redis_edit(paths, site, target, RedisEdit::Expire { key, seconds }).await?;
            out.message(&done);
        }
        DbCommand::Del {
            key,
            site,
            yes,
            target,
        } => {
            if !yes {
                crate::work::confirm(&format!("Delete the key {key}?"))?;
            }
            let done = format!("deleted {key}");
            redis_edit(paths, site, target, RedisEdit::Delete { key }).await?;
            out.message(&done);
        }
        DbCommand::Create {
            site,
            database,
            connection,
        } => {
            let conn = pick(paths, site, connection).await?;
            daemon_call(
                paths,
                Request::DbCreateDatabase {
                    connection: conn.id.clone(),
                    database: database.clone(),
                },
            )
            .await?;
            out.message(&format!(
                "created {} on {}",
                database
                    .or(conn.database)
                    .unwrap_or_else(|| "the database".into()),
                conn.label
            ));
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn edit_row(
    paths: &Paths,
    out: &Output,
    site: Option<String>,
    target: Target,
    table: &str,
    edit: RowEdit,
) -> anyhow::Result<()> {
    let conn = pick(paths, site, target.connection).await?;
    let (schema, table) = split_table(table);
    let Response::DbAffected { rows } = daemon_call(
        paths,
        Request::DbEditRow {
            connection: conn.id,
            table,
            schema,
            edit,
            allow_remote: target.allow_remote,
        },
    )
    .await?
    else {
        bail!("unexpected response from daemon");
    };
    out.message(&format!("{rows} row(s) changed"));
    Ok(())
}

async fn redis_edit(
    paths: &Paths,
    site: Option<String>,
    target: Target,
    edit: RedisEdit,
) -> anyhow::Result<()> {
    let conn = pick_redis(paths, site, target.connection).await?;
    daemon_call(
        paths,
        Request::RedisEdit {
            connection: conn.id,
            edit,
            allow_remote: target.allow_remote,
        },
    )
    .await?;
    Ok(())
}

/// `schema.table` → (`schema`, `table`); a plain name keeps an empty schema.
fn split_table(table: &str) -> (String, String) {
    match table.split_once('.') {
        Some((s, t)) => (s.to_string(), t.to_string()),
        None => (String::new(), table.to_string()),
    }
}
