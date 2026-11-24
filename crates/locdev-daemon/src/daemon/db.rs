//! The database manager. Clients send a connection id; the daemon resolves
//! host, user and password itself every time (from the project's .env and
//! shared variables, or a LocDev instance), so secrets never leave it.
//! Remote connections need an explicit `allow_remote`; destructive SQL
//! needs the database's name typed back.

use std::collections::BTreeMap;
use std::time::Duration;

use locdev_core::db::{DbConnection, DbEngine, RedisEdit, RowEdit};
use locdev_core::model::ProcessRole;
use locdev_core::protocol::{Request, Response};
use locdev_core::services::ServiceKind;
use locdev_core::{AppError, ErrorCode};
use locdev_db::{Client, DbError, Resolved};
use tracing::info;

use super::Daemon;

/// Any single database call gives up after this long.
const TIMEOUT: Duration = Duration::from_secs(35);

fn db_error(e: DbError) -> AppError {
    if e.missing_database {
        AppError::new(
            ErrorCode::NotFound,
            format!("missing database: {}", e.message),
        )
    } else {
        AppError::new(ErrorCode::Remote, e.message)
    }
}

async fn timed<T>(f: impl std::future::Future<Output = Result<T, DbError>>) -> Result<T, AppError> {
    tokio::time::timeout(TIMEOUT, f)
        .await
        .map_err(|_| AppError::new(ErrorCode::Remote, "the database didn't answer in 35 s"))?
        .map_err(db_error)
}

impl Daemon {
    fn instance_connections(&self) -> Vec<Resolved> {
        self.services
            .list()
            .into_iter()
            .filter_map(|i| {
                let (engine, user, database) = match i.kind {
                    ServiceKind::Postgres => (DbEngine::Postgres, Some("locdev"), Some("postgres")),
                    ServiceKind::Mariadb | ServiceKind::Mysql => {
                        (DbEngine::Mysql, Some("root"), None)
                    }
                    ServiceKind::Valkey | ServiceKind::Redis => (DbEngine::Redis, None, None),
                    ServiceKind::Mongodb | ServiceKind::Sqlite => return None,
                };
                Some(Resolved {
                    public: DbConnection {
                        id: format!("instance:{}", i.name),
                        label: format!("{} · {}", i.name, i.kind.label()),
                        engine,
                        host: Some("127.0.0.1".into()),
                        port: Some(i.port),
                        database: database.map(str::to_string),
                        username: user.map(str::to_string),
                        has_password: false,
                        source: "LocDev instance".into(),
                        project: None,
                        remote: false,
                    },
                    password: None,
                    sqlite_path: None,
                })
            })
            .collect()
    }

    /// A site's connections, from its files and the shared variables it gets.
    pub(super) fn site_connections(&self, project: &locdev_core::model::Project) -> Vec<Resolved> {
        let shared: BTreeMap<String, String> = self
            .env_resolve(project, ProcessRole::Main)
            .resolution
            .env
            .into_iter()
            .collect();
        let env = locdev_db::detect::project_env(&project.path, &shared);
        locdev_db::detect::project_connections(&project.name, &project.path, &env)
    }

    async fn resolve_connection(&self, id: &str, allow_remote: bool) -> Result<Resolved, AppError> {
        let found = if let Some(name) = id.strip_prefix("instance:") {
            self.instance_connections()
                .into_iter()
                .find(|r| r.public.id == format!("instance:{name}"))
        } else if let Some(rest) = id.strip_prefix("project:") {
            let site = rest.rsplit_once(':').map(|(s, _)| s).unwrap_or(rest);
            let project = self.project(site).await?;
            self.site_connections(&project)
                .into_iter()
                .find(|r| r.public.id == id)
        } else {
            None
        };
        let resolved =
            found.ok_or_else(|| AppError::not_found(format!("no database connection {id}")))?;
        if resolved.public.remote && !allow_remote {
            return Err(AppError::new(
                ErrorCode::Unauthorized,
                format!(
                    "{} is on {}, not this machine (it may be production); open it explicitly to continue",
                    resolved.public.label,
                    resolved.public.host.as_deref().unwrap_or("another host")
                ),
            ));
        }
        Ok(resolved)
    }

    async fn open_db(&self, id: &str, allow_remote: bool) -> Result<(Resolved, Client), AppError> {
        let resolved = self.resolve_connection(id, allow_remote).await?;
        let client = timed(locdev_db::open(&resolved, None)).await?;
        Ok((resolved, client))
    }

    pub(super) async fn db_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::DbConnections { name } => {
                let projects = match name {
                    Some(name) => vec![self.project(&name).await?],
                    None => self.registry.lock().await.all().to_vec(),
                };
                let mut connections: Vec<DbConnection> = projects
                    .iter()
                    .flat_map(|p| self.site_connections(p))
                    .map(|r| r.public)
                    .collect();
                connections.extend(self.instance_connections().into_iter().map(|r| r.public));
                Ok(Response::DbConnections { connections })
            }

            Request::DbSchema {
                connection,
                allow_remote,
            } => {
                let (_, mut client) = self.open_db(&connection, allow_remote).await?;
                let databases = timed(client.databases()).await?;
                let tables = timed(client.tables()).await?;
                Ok(Response::DbSchema { databases, tables })
            }

            Request::DbTableInfo {
                connection,
                table,
                schema,
                allow_remote,
            } => {
                let (_, mut client) = self.open_db(&connection, allow_remote).await?;
                Ok(Response::DbTableInfo(
                    timed(client.table_info(&schema, &table)).await?,
                ))
            }

            Request::DbRows {
                connection,
                table,
                schema,
                page,
                page_size,
                order_by,
                desc,
                filters,
                allow_remote,
            } => {
                let (_, mut client) = self.open_db(&connection, allow_remote).await?;
                let info = timed(client.table_info(&schema, &table)).await?;
                let rows = timed(client.rows(
                    &info,
                    page,
                    page_size.unwrap_or(100),
                    order_by.as_deref(),
                    desc,
                    &filters,
                ))
                .await?;
                Ok(Response::DbRows(rows))
            }

            Request::DbCheck { connection, sql } => {
                let resolved = self.resolve_connection(&connection, true).await?;
                Ok(Response::DbCheck(locdev_db::sql::check(
                    &sql,
                    &database_name(&resolved),
                )))
            }

            Request::DbQuery {
                connection,
                sql,
                confirm,
                allow_remote,
            } => {
                let (resolved, mut client) = self.open_db(&connection, allow_remote).await?;
                let check = locdev_db::sql::check(&sql, &database_name(&resolved));
                if let Some(phrase) = &check.confirm
                    && confirm.as_deref() != Some(phrase.as_str())
                {
                    return Err(AppError::new(
                        ErrorCode::Unauthorized,
                        format!(
                            "this would destroy data ({}); type the database's name ({phrase}) to run it",
                            check.destructive.join("; ")
                        ),
                    ));
                }
                let rows = timed(client.query(&sql)).await?;
                if check.writes {
                    info!(connection = %connection, destructive = !check.destructive.is_empty(), "SQL that writes ran");
                }
                Ok(Response::DbRows(rows))
            }

            Request::DbEditRow {
                connection,
                table,
                schema,
                edit,
                allow_remote,
            } => {
                let (_, mut client) = self.open_db(&connection, allow_remote).await?;
                let info = timed(client.table_info(&schema, &table)).await?;
                let rows = match edit {
                    RowEdit::Update { key, changes } => {
                        timed(client.update_row(&info, &key, &changes)).await?
                    }
                    RowEdit::Insert { values } => timed(client.insert_row(&info, &values)).await?,
                    RowEdit::Delete { key } => timed(client.delete_row(&info, &key)).await?,
                };
                Ok(Response::DbAffected { rows })
            }

            Request::DbCreateDatabase {
                connection,
                database,
            } => {
                let resolved = self.resolve_connection(&connection, false).await?;
                let name = database.unwrap_or_else(|| database_name(&resolved));
                timed(locdev_db::create_database(&resolved, &name)).await?;
                info!(%connection, database = %name, "database created");
                Ok(Response::Ok)
            }

            Request::RedisKeys {
                connection,
                cursor,
                pattern,
                allow_remote,
            } => {
                let (_, mut client) = self.open_db(&connection, allow_remote).await?;
                let redis = client.redis().map_err(db_error)?;
                let (cursor, keys) = tokio::time::timeout(
                    TIMEOUT,
                    locdev_db::redis_scan(
                        redis,
                        cursor.as_deref().unwrap_or("0"),
                        pattern.as_deref().unwrap_or("*"),
                    ),
                )
                .await
                .map_err(|_| AppError::new(ErrorCode::Remote, "timed out"))?
                .map_err(|e| AppError::new(ErrorCode::Remote, e))?;
                Ok(Response::RedisKeys { cursor, keys })
            }

            Request::RedisGet {
                connection,
                key,
                allow_remote,
            } => {
                let (_, mut client) = self.open_db(&connection, allow_remote).await?;
                let redis = client.redis().map_err(db_error)?;
                let value = locdev_db::redis_get(redis, &key)
                    .await
                    .map_err(|e| AppError::new(ErrorCode::NotFound, e))?;
                Ok(Response::RedisValue(value))
            }

            Request::RedisEdit {
                connection,
                edit,
                allow_remote,
            } => {
                let (_, mut client) = self.open_db(&connection, allow_remote).await?;
                let redis = client.redis().map_err(db_error)?;
                let result = match &edit {
                    RedisEdit::Set { key, value } => redis.set(key, value).await,
                    RedisEdit::Expire { key, seconds } => redis.expire(key, *seconds).await,
                    RedisEdit::Delete { key } => redis.delete(key).await.map(|_| ()),
                };
                result.map_err(|e| AppError::new(ErrorCode::Remote, e))?;
                Ok(Response::Ok)
            }

            other => Err(AppError::internal(format!(
                "not a database request: {other:?}"
            ))),
        }
    }
}

/// The name to type back: the database, or the SQLite file's name.
fn database_name(r: &Resolved) -> String {
    r.public
        .database
        .clone()
        .filter(|d| !d.is_empty())
        .or_else(|| {
            r.sqlite_path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| r.public.label.clone())
}
