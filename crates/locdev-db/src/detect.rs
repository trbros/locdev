//! The databases a project points at, read from its `.env.local`/`.env`
//! (and the shared variables it gets): Laravel's `DB_*`, `DATABASE_URL`
//! (Prisma, Rails, Django-environ…), `REDIS_URL`/`REDIS_*`, and SQLite files
//! in the usual places. Secrets stay in [`Resolved`], never in the
//! client-facing [`DbConnection`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use locdev_core::db::{DbConnection, DbEngine};

/// A connection with what's needed to open it.
#[derive(Clone)]
pub struct Resolved {
    pub public: DbConnection,
    pub password: Option<String>,
    pub sqlite_path: Option<PathBuf>,
}

impl std::fmt::Debug for Resolved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resolved")
            .field("public", &self.public)
            .field("password", &self.password.as_ref().map(|_| "********"))
            .field("sqlite_path", &self.sqlite_path)
            .finish()
    }
}

pub fn is_local(host: &str) -> bool {
    matches!(
        host,
        "" | "127.0.0.1" | "localhost" | "::1" | "[::1]" | "0.0.0.0"
    )
}

/// The project's variables: shared ones, then `.env`, then `.env.local`
/// (a later source wins).
pub fn project_env(root: &Path, shared: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut env = shared.clone();
    for file in [".env", ".env.local"] {
        if let Ok(bytes) = std::fs::read(root.join(file))
            && let Ok(doc) = locdev_env::dotenv::parse(&bytes)
        {
            for entry in doc.entries {
                env.insert(entry.key, entry.value);
            }
        }
    }
    env
}

/// Percent-decoding for URL user, password and path.
fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 3 <= bytes.len()
            && let Some(b) = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `scheme://user:pass@host:port/db?…` → its parts.
pub struct Url {
    pub scheme: String,
    pub user: Option<String>,
    pub password: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
}

pub fn parse_url(url: &str) -> Option<Url> {
    let (scheme, rest) = url.split_once("://")?;
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (credentials, host_port) = match authority.rsplit_once('@') {
        Some((c, h)) => (Some(c), h),
        None => (None, authority),
    };
    let (user, password) = match credentials {
        Some(c) => match c.split_once(':') {
            Some((u, p)) => (Some(decode(u)), Some(decode(p))),
            None => (Some(decode(c)), None),
        },
        None => (None, None),
    };
    let (host, port) = if let Some(stripped) = host_port.strip_prefix('[') {
        let (h, rest) = stripped.split_once(']')?;
        (
            h.to_string(),
            rest.strip_prefix(':').and_then(|p| p.parse().ok()),
        )
    } else {
        match host_port.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), p.parse().ok()),
            None => (host_port.to_string(), None),
        }
    };
    Some(Url {
        scheme: scheme.to_lowercase(),
        user: user.filter(|u| !u.is_empty()),
        password: password.filter(|p| !p.is_empty()),
        host,
        port,
        path: decode(path),
    })
}

fn engine_of(scheme_or_driver: &str) -> Option<DbEngine> {
    Some(match scheme_or_driver {
        "pgsql" | "postgres" | "postgresql" => DbEngine::Postgres,
        "mysql" | "mariadb" | "mysql2" => DbEngine::Mysql,
        "sqlite" | "sqlite3" | "file" => DbEngine::Sqlite,
        "redis" | "rediss" | "valkey" => DbEngine::Redis,
        _ => return None,
    })
}

/// The project's connections, ids `project:<site>:<n>` in a stable order.
pub fn project_connections(
    site: &str,
    root: &Path,
    env: &BTreeMap<String, String>,
) -> Vec<Resolved> {
    let mut out: Vec<Resolved> = Vec::new();
    let mut add = |engine: DbEngine,
                   host: Option<String>,
                   port: Option<u16>,
                   database: Option<String>,
                   username: Option<String>,
                   password: Option<String>,
                   sqlite: Option<PathBuf>,
                   source: &str| {
        let label = match (&sqlite, &database) {
            (Some(p), _) => format!(
                "{} · {}",
                engine.label(),
                p.file_name().unwrap_or_default().to_string_lossy()
            ),
            (None, Some(db)) if !db.is_empty() => format!("{} · {db}", engine.label()),
            _ => engine.label().to_string(),
        };
        let remote = host.as_deref().is_some_and(|h| !is_local(h));
        // The same database found twice (DB_* and DATABASE_URL): keep one.
        if out.iter().any(|r| {
            r.public.engine == engine
                && r.public.host == host
                && r.public.port == port
                && r.public.database == database
                && r.sqlite_path == sqlite
        }) {
            return;
        }
        let id = format!("project:{site}:{}", out.len());
        out.push(Resolved {
            public: DbConnection {
                id,
                label,
                engine,
                host,
                port,
                database,
                username,
                has_password: password.is_some(),
                source: source.into(),
                project: Some(site.into()),
                remote,
            },
            password,
            sqlite_path: sqlite,
        });
    };
    let get = |k: &str| env.get(k).cloned().filter(|v| !v.is_empty());

    // Laravel's DB_*.
    if let Some(engine) = get("DB_CONNECTION").and_then(|d| engine_of(&d)) {
        if engine == DbEngine::Sqlite {
            let path = get("DB_DATABASE")
                .map(PathBuf::from)
                .map(|p| if p.is_absolute() { p } else { root.join(p) })
                .unwrap_or_else(|| root.join("database/database.sqlite"));
            add(
                engine,
                None,
                None,
                None,
                None,
                None,
                Some(path),
                ".env (DB_CONNECTION=sqlite)",
            );
        } else {
            add(
                engine,
                Some(get("DB_HOST").unwrap_or_else(|| "127.0.0.1".into())),
                get("DB_PORT").and_then(|p| p.parse().ok()).or(Some(
                    if engine == DbEngine::Postgres {
                        5432
                    } else {
                        3306
                    },
                )),
                get("DB_DATABASE"),
                get("DB_USERNAME"),
                get("DB_PASSWORD"),
                None,
                ".env (DB_*)",
            );
        }
    }
    // DATABASE_URL (Prisma, Rails, Django-environ, Drizzle…).
    if let Some(url) = get("DATABASE_URL") {
        if let Some(rest) = url
            .strip_prefix("file:")
            .or_else(|| url.strip_prefix("sqlite:"))
        {
            let rest = rest.trim_start_matches("//").trim_start_matches("./");
            let path = PathBuf::from(rest);
            // Prisma resolves `file:` next to prisma/schema.prisma.
            let path = if path.is_absolute() {
                path
            } else if root.join("prisma").is_dir() {
                root.join("prisma").join(path)
            } else {
                root.join(path)
            };
            add(
                DbEngine::Sqlite,
                None,
                None,
                None,
                None,
                None,
                Some(path),
                ".env (DATABASE_URL)",
            );
        } else if let Some(u) = parse_url(&url)
            && let Some(engine) = engine_of(&u.scheme)
        {
            let default = if engine == DbEngine::Postgres {
                5432
            } else {
                3306
            };
            add(
                engine,
                Some(u.host),
                u.port.or(Some(default)),
                Some(u.path).filter(|p| !p.is_empty()),
                u.user,
                u.password,
                None,
                ".env (DATABASE_URL)",
            );
        }
    }
    // Redis.
    if let Some(u) = get("REDIS_URL").and_then(|u| parse_url(&u)) {
        add(
            DbEngine::Redis,
            Some(u.host),
            u.port.or(Some(6379)),
            Some(u.path).filter(|p| !p.is_empty()),
            u.user,
            u.password,
            None,
            ".env (REDIS_URL)",
        );
    } else if let Some(host) = get("REDIS_HOST") {
        add(
            DbEngine::Redis,
            Some(host),
            get("REDIS_PORT")
                .and_then(|p| p.parse().ok())
                .or(Some(6379)),
            None,
            get("REDIS_USERNAME"),
            get("REDIS_PASSWORD").filter(|p| p != "null"),
            None,
            ".env (REDIS_*)",
        );
    }
    // SQLite files where frameworks put them.
    for file in [
        "database/database.sqlite",
        "db.sqlite3",
        "prisma/dev.db",
        "storage/database.sqlite",
        "db/development.sqlite3",
    ] {
        let path = root.join(file);
        if path.is_file() {
            add(
                DbEngine::Sqlite,
                None,
                None,
                None,
                None,
                None,
                Some(path),
                "SQLite file",
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn laravel_and_url_dedupe_and_remote() {
        let dir = tempfile::tempdir().unwrap();
        let found = project_connections(
            "shop",
            dir.path(),
            &env(&[
                ("DB_CONNECTION", "pgsql"),
                ("DB_HOST", "127.0.0.1"),
                ("DB_DATABASE", "shop"),
                ("DB_USERNAME", "app"),
                ("DB_PASSWORD", "s3cret"),
                (
                    "DATABASE_URL",
                    "postgresql://app:s3cret@127.0.0.1:5432/shop",
                ),
                ("REDIS_URL", "redis://:p%40ss@cache.example.com:6380/0"),
            ]),
        );
        assert_eq!(found.len(), 2);
        let pg = &found[0];
        assert_eq!(pg.public.id, "project:shop:0");
        assert!(pg.public.has_password && !pg.public.remote);
        assert_eq!(pg.password.as_deref(), Some("s3cret"));
        assert!(!format!("{pg:?}").contains("s3cret"));
        let redis = &found[1];
        assert!(redis.public.remote);
        assert_eq!(redis.password.as_deref(), Some("p@ss"));
        assert_eq!(redis.public.port, Some(6380));
        // What a client sees has no secret.
        assert!(
            !serde_json::to_string(&pg.public)
                .unwrap()
                .contains("s3cret")
        );
    }

    #[test]
    fn sqlite_from_laravel_prisma_and_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("prisma")).unwrap();
        std::fs::write(dir.path().join("prisma/dev.db"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("database")).unwrap();
        std::fs::write(dir.path().join("database/database.sqlite"), "").unwrap();
        let found = project_connections(
            "app",
            dir.path(),
            &env(&[
                ("DB_CONNECTION", "sqlite"),
                ("DATABASE_URL", "file:./dev.db"),
            ]),
        );
        let paths: Vec<_> = found
            .iter()
            .map(|r| r.sqlite_path.clone().unwrap())
            .collect();
        assert_eq!(
            paths,
            vec![
                dir.path().join("database/database.sqlite"),
                dir.path().join("prisma/dev.db")
            ]
        );
    }

    #[test]
    fn urls() {
        let u = parse_url("mysql://root@[::1]:3307/app?ssl=false").unwrap();
        assert_eq!(
            (u.host.as_str(), u.port, u.path.as_str(), u.user.as_deref()),
            ("::1", Some(3307), "app", Some("root"))
        );
        assert!(is_local("localhost") && !is_local("db.internal"));
    }
}
