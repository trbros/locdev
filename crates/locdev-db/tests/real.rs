//! The drivers against real databases: SQLite always; PostgreSQL and
//! Redis from this machine's binaries with `-- --ignored`.

use locdev_core::db::{DbConnection, DbEngine, DbFilter};
use locdev_db::{Resolved, open};

fn conn(
    engine: DbEngine,
    port: Option<u16>,
    database: Option<&str>,
    sqlite: Option<std::path::PathBuf>,
) -> Resolved {
    Resolved {
        public: DbConnection {
            id: "test".into(),
            label: "test".into(),
            engine,
            host: port.map(|_| "127.0.0.1".into()),
            port,
            database: database.map(str::to_string),
            username: if engine == DbEngine::Postgres {
                Some("locdev".into())
            } else {
                None
            },
            has_password: false,
            source: "test".into(),
            project: None,
            remote: false,
        },
        password: None,
        sqlite_path: sqlite,
    }
}

async fn exercise_sql(r: &Resolved) {
    let mut c = open(r, None).await.expect("open");
    c.query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT)")
        .await
        .unwrap();
    c.query("INSERT INTO users (id, name, email) VALUES (1, 'Ana', 'a@x.test'), (2, 'Bruno', NULL), (3, 'Carla', 'c@x.test')")
        .await
        .unwrap();
    let tables = c.tables().await.unwrap();
    assert!(tables.iter().any(|t| t.name == "users"), "{tables:?}");
    let info = c.table_info("", "users").await.unwrap();
    assert_eq!(info.columns.iter().filter(|c| c.primary_key).count(), 1);

    // A page, filtered and ordered.
    let page = c
        .rows(
            &info,
            0,
            2,
            Some("name"),
            true,
            &[DbFilter {
                column: "email".into(),
                op: "not_null".into(),
                value: None,
            }],
        )
        .await
        .unwrap();
    assert_eq!(page.total, Some(2));
    assert_eq!(page.rows[0][1].as_deref(), Some("Carla"));
    let like = c
        .rows(
            &info,
            0,
            10,
            None,
            false,
            &[DbFilter {
                column: "name".into(),
                op: "like".into(),
                value: Some("%run%".into()),
            }],
        )
        .await
        .unwrap();
    assert_eq!(like.rows.len(), 1);
    assert_eq!(like.rows[0][2], None, "NULL stays NULL");

    // Edits by primary key, values as parameters.
    let key = vec![("id".to_string(), Some("2".to_string()))];
    let n = c
        .update_row(
            &info,
            &key,
            &[(
                "email".into(),
                Some("b'; DROP TABLE users; --@x.test".into()),
            )],
        )
        .await
        .unwrap();
    assert_eq!(n, 1);
    c.insert_row(
        &info,
        &[
            ("id".into(), Some("4".into())),
            ("name".into(), Some("Davi".into())),
        ],
    )
    .await
    .unwrap();
    c.delete_row(&info, &[("id".into(), Some("1".into()))])
        .await
        .unwrap();
    let all = c
        .query("SELECT id, email FROM users ORDER BY id")
        .await
        .unwrap();
    assert_eq!(all.rows.len(), 3);
    assert_eq!(
        all.rows[0][1].as_deref(),
        Some("b'; DROP TABLE users; --@x.test")
    );
    let changed = c
        .query("UPDATE users SET name = 'x' WHERE id = 3")
        .await
        .unwrap();
    assert_eq!(changed.affected, Some(1));
}

#[tokio::test]
async fn sqlite_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let r = conn(DbEngine::Sqlite, None, None, Some(path.clone()));
    let missing = open(&r, None).await.err().unwrap();
    assert!(missing.missing_database);
    locdev_db::create_database(&r, "app").await.unwrap();
    exercise_sql(&r).await;
}

#[cfg(unix)]
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "needs a system PostgreSQL"]
async fn postgres_end_to_end() {
    use std::process::{Command, Stdio};
    let bin = std::fs::read_dir("/usr/lib/postgresql")
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("bin");
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    assert!(
        Command::new(bin.join("initdb"))
            .arg("-D")
            .arg(&data)
            .args(["-U", "locdev", "-A", "trust", "--no-instructions"])
            .stdout(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    let port = free_port();
    let mut server = Command::new(bin.join("postgres"))
        .arg("-D")
        .arg(&data)
        .args([
            "-p",
            &port.to_string(),
            "-c",
            "listen_addresses=127.0.0.1",
            "-c",
            "unix_socket_directories=",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let r = conn(DbEngine::Postgres, Some(port), Some("shop"), None);
    let missing = open(&r, None).await.err().unwrap();
    assert!(missing.missing_database, "{}", missing.message);
    locdev_db::create_database(&r, "shop").await.unwrap();
    exercise_sql(&r).await;
    let mut c = open(&r, None).await.unwrap();
    assert!(c.databases().await.unwrap().contains(&"shop".to_string()));
    let _ = server.kill();
    let _ = server.wait();
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "needs a system redis-server"]
async fn redis_end_to_end() {
    use std::process::{Command, Stdio};
    let port = free_port();
    let dir = tempfile::tempdir().unwrap();
    let mut server = Command::new("redis-server")
        .args([
            "--port",
            &port.to_string(),
            "--bind",
            "127.0.0.1",
            "--save",
            "",
            "--dir",
        ])
        .arg(dir.path())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let r = conn(DbEngine::Redis, Some(port), None, None);
    let mut c = open(&r, None).await.unwrap();
    let redis = c.redis().unwrap();
    redis.set("greeting", "olá").await.unwrap();
    redis.expire("greeting", Some(600)).await.unwrap();
    redis
        .call(&["RPUSH", "queue", "a", "b", "c"])
        .await
        .unwrap();
    redis
        .call(&["HSET", "user:1", "name", "Ana", "role", "admin"])
        .await
        .unwrap();
    let (_, keys) = locdev_db::redis_scan(redis, "0", "*").await.unwrap();
    assert_eq!(keys.len(), 3);
    let greeting = keys.iter().find(|k| k.key == "greeting").unwrap();
    assert!(greeting.kind == "string" && greeting.ttl > 0);
    let value = locdev_db::redis_get(redis, "greeting").await.unwrap();
    assert_eq!(value.items[0].0, "olá");
    let hash = locdev_db::redis_get(redis, "user:1").await.unwrap();
    assert!(
        hash.items
            .contains(&("role".to_string(), Some("admin".to_string())))
    );
    let list = locdev_db::redis_get(redis, "queue").await.unwrap();
    assert_eq!(list.items.len(), 3);
    assert_eq!(redis.delete("queue").await.unwrap(), 1);
    let _ = server.kill();
    let _ = server.wait();
}
