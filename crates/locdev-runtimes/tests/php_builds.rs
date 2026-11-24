//! PHP downloads end to end against a local server laid out like
//! dl.static-php.dev: the publisher's index (or `.sha256` file) → SHA-256 of
//! the archive → install. A mismatch must be refused before anything lands.

#![cfg(unix)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use locdev_core::model::RuntimeKind;
use locdev_runtimes::{PhpSource, RuntimeError, Store};

type Files = Arc<Mutex<HashMap<String, Vec<u8>>>>;

/// Serves `files` by path (query string ignored for the listing) until the
/// test ends.
fn serve(files: Files) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 2 {
                line.clear();
            }
            let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
            let body = files.lock().unwrap().get(&path).cloned();
            let response = match body {
                Some(body) => {
                    let mut head = format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        body.len()
                    )
                    .into_bytes();
                    head.extend(body);
                    head
                }
                None => b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                    .to_vec(),
            };
            let _ = stream.write_all(&response);
        }
    });
    base
}

fn platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "x86_64") => "macos-x86_64",
        ("macos", "aarch64") => "macos-aarch64",
        other => panic!("no StaticPHP platform for {other:?}"),
    }
}

/// Like StaticPHP's archives: a lone `php` at the root.
fn fake_php_tarball(version: &str) -> Vec<u8> {
    let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(gz);
    let script = format!("#!/bin/sh\necho {version}\n");
    let mut header = tar::Header::new_gnu();
    header.set_size(script.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    tar.append_data(&mut header, "php", script.as_bytes())
        .unwrap();
    tar.into_inner().unwrap().finish().unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

struct Fixture {
    files: Files,
    base: String,
}

impl Fixture {
    /// Publishes 8.4.99 (newest of its line, hash in `version_84.json`) and
    /// 8.4.98 (older, hash only in its `.sha256` file).
    fn new() -> Self {
        let files: Files = Arc::default();
        let base = serve(files.clone());
        let f = Self { files, base };
        let p = platform();
        let newest = fake_php_tarball("8.4.99");
        let older = fake_php_tarball("8.4.98");
        let newest_file = format!("php-8.4.99-cli-{p}.tar.gz");
        let older_file = format!("php-8.4.98-cli-{p}.tar.gz");

        f.put(
            "/common/versions.json",
            br#"{"schema-version":1,"version-files":{"8.4":"version_84.json"}}"#,
        );
        f.put(
            "/common/version_84.json",
            format!(
                r#"{{"php-version":"8.4.99","built-at":"2026-11-22T12:00:00Z","artifacts":{{
                  "{p}":{{"cli":{{"file":"{newest_file}","size":{},"sha256":"{}"}}}}}}}}"#,
                newest.len(),
                sha256(&newest)
            )
            .as_bytes(),
        );
        f.put(
            "/common/?format=json",
            format!(
                r#"[{{"name":"{newest_file}","last_modified":"2026-11-22 12:00:00"}},
                   {{"name":"{newest_file}.sha256","last_modified":"2026-11-22 12:00:00"}},
                   {{"name":"{older_file}","last_modified":"2026-10-01 12:00:00"}},
                   {{"name":"{older_file}.sha256","last_modified":"2026-10-01 12:00:00"}}]"#
            )
            .as_bytes(),
        );
        f.put(
            &format!("/common/{older_file}.sha256"),
            format!("{}  {older_file}\n", sha256(&older)).as_bytes(),
        );
        f.put(&format!("/common/{newest_file}"), &newest);
        f.put(&format!("/common/{older_file}"), &older);
        f
    }

    fn put(&self, path: &str, body: &[u8]) {
        self.files
            .lock()
            .unwrap()
            .insert(path.into(), body.to_vec());
    }

    fn store(&self, dir: &std::path::Path) -> Store {
        Store::new(
            dir.to_path_buf(),
            PhpSource {
                static_url: self.base.clone(),
                flavor: "common".into(),
                windows_url: self.base.clone(),
            },
        )
    }

    fn install(&self, version: &str) -> Result<std::path::PathBuf, RuntimeError> {
        let dir = tempfile::tempdir().unwrap().keep();
        self.store(&dir)
            .install(RuntimeKind::Php, version, &|_, _, _| {})
            .map(|v| v.executable)
    }
}

#[test]
fn installs_verified_builds_from_the_index_and_from_sha256_files() {
    let f = Fixture::new();
    let dir = tempfile::tempdir().unwrap();
    let store = f.store(dir.path());
    assert!(store.downloads(RuntimeKind::Php));

    let catalog = store.catalog(RuntimeKind::Php).unwrap();
    let versions: Vec<&str> = catalog.iter().map(|r| r.version.as_str()).collect();
    assert_eq!(versions, ["8.4.99", "8.4.98"]);
    assert_eq!(catalog[0].date, "2026-11-22");

    for version in ["8.4.99", "8.4.98"] {
        let installed = store
            .install(RuntimeKind::Php, version, &|_, _, _| {})
            .unwrap();
        let output = std::process::Command::new(&installed.executable)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), version);
        // The php.ini next to the binary is LocDev's.
        let ini = std::fs::read_to_string(installed.executable.with_file_name("php.ini")).unwrap();
        assert!(ini.starts_with("; Managed by LocDev"), "{ini}");
    }
    let installed: Vec<String> = store
        .installed(RuntimeKind::Php)
        .into_iter()
        .map(|v| v.version)
        .collect();
    assert_eq!(installed, ["8.4.99", "8.4.98"]);
}

#[test]
fn refuses_an_archive_that_does_not_match_the_index() {
    let f = Fixture::new();
    f.put(
        &format!("/common/php-8.4.99-cli-{}.tar.gz", platform()),
        &fake_php_tarball("6.6.6"),
    );
    assert!(matches!(
        f.install("8.4.99"),
        Err(RuntimeError::Checksum { .. })
    ));
}

#[test]
fn refuses_an_archive_that_does_not_match_its_sha256_file() {
    let f = Fixture::new();
    f.put(
        &format!("/common/php-8.4.98-cli-{}.tar.gz", platform()),
        &fake_php_tarball("6.6.6"),
    );
    assert!(matches!(
        f.install("8.4.98"),
        Err(RuntimeError::Checksum { .. })
    ));
}

#[test]
fn refuses_versions_without_a_published_checksum() {
    let f = Fixture::new();
    // Listed, but its `.sha256` is gone: not offered, not installed.
    f.files.lock().unwrap().remove(&format!(
        "/common/php-8.4.98-cli-{}.tar.gz.sha256",
        platform()
    ));
    f.put(
        "/common/?format=json",
        format!(
            r#"[{{"name":"php-8.4.98-cli-{}.tar.gz","last_modified":""}}]"#,
            platform()
        )
        .as_bytes(),
    );
    assert!(matches!(f.install("8.4.98"), Err(RuntimeError::Catalog(_))));
    assert!(f.install("8.4.99").is_ok(), "the indexed build still works");
}
