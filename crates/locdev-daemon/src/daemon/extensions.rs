//! PHP extensions: what a site needs (composer.json/lock, its framework,
//! the drivers its .env names) against what the site's own PHP loads, and
//! how to get the missing ones for that PHP. Nothing is installed here:
//! LocDev's PHP builds are static, and system PHP belongs to the system.

use std::path::{Path, PathBuf};

use locdev_core::model::{
    ExtRequirement, PhpExtensionsReport, PhpModule, PhpModules, Project, RuntimeKind,
};
use locdev_core::{AppError, ErrorCode};
use serde_json::Value;

use super::Daemon;

/// Laravel's documented server requirements (12.x).
const LARAVEL: &[&str] = &[
    "ctype",
    "curl",
    "dom",
    "fileinfo",
    "filter",
    "hash",
    "mbstring",
    "openssl",
    "pcre",
    "pdo",
    "session",
    "tokenizer",
    "xml",
];

fn push(out: &mut Vec<ExtRequirement>, name: &str, from: &str) {
    let name = name.to_lowercase();
    if !out.iter().any(|r| r.name == name) {
        out.push(ExtRequirement {
            name,
            from: from.into(),
            reason: None,
        });
    }
}

fn dotenv_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let (k, v) = l.trim().split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches(['"', '\'']).to_string())
    })
}

/// What the project's files ask for.
pub fn required(path: &Path, laravel: bool) -> Vec<ExtRequirement> {
    let mut out = Vec::new();
    let read = |f: &str| std::fs::read_to_string(path.join(f)).ok();
    if let Some(json) = read("composer.json").and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        for section in ["require", "require-dev"] {
            for key in json[section]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(k, _)| k)
            {
                if let Some(ext) = key.strip_prefix("ext-") {
                    push(&mut out, ext, "composer.json");
                }
            }
        }
    }
    if let Some(json) = read("composer.lock").and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        for package in ["packages", "packages-dev"]
            .iter()
            .flat_map(|k| json[*k].as_array().into_iter().flatten())
        {
            let name = package["name"].as_str().unwrap_or("a package");
            for key in package["require"]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(k, _)| k)
            {
                if let Some(ext) = key.strip_prefix("ext-") {
                    push(&mut out, ext, name);
                }
            }
        }
    }
    if laravel {
        for ext in LARAVEL {
            push(&mut out, ext, "Laravel");
        }
    }
    if let Some(env) = read(".env") {
        if let Some(driver) = dotenv_value(&env, "DB_CONNECTION") {
            let ext = match driver.as_str() {
                "pgsql" => Some("pdo_pgsql"),
                "mysql" | "mariadb" => Some("pdo_mysql"),
                "sqlite" => Some("pdo_sqlite"),
                "sqlsrv" => Some("pdo_sqlsrv"),
                _ => None,
            };
            if let Some(ext) = ext {
                push(&mut out, ext, &format!(".env (DB_CONNECTION={driver})"));
            }
        }
        let uses_redis = [
            "CACHE_STORE",
            "QUEUE_CONNECTION",
            "SESSION_DRIVER",
            "CACHE_DRIVER",
        ]
        .iter()
        .any(|k| dotenv_value(&env, k).as_deref() == Some("redis"));
        let client = dotenv_value(&env, "REDIS_CLIENT");
        // Laravel defaults to phpredis when REDIS_CLIENT isn't set.
        if uses_redis && client.as_deref().unwrap_or("phpredis") == "phpredis" {
            push(&mut out, "redis", ".env (Redis with the phpredis client)");
        }
    }
    out
}

/// Extensions the installed packages suggest (`suggest` in composer.lock and
/// composer.json) or the framework uses for some features; never the ones
/// already required.
pub fn recommended(path: &Path, laravel: bool, required: &[ExtRequirement]) -> Vec<ExtRequirement> {
    let mut out: Vec<ExtRequirement> = Vec::new();
    let mut add = |name: &str, from: &str, reason: Option<&str>| {
        let name = name.to_lowercase();
        if required.iter().any(|r| r.name == name) || out.iter().any(|r| r.name == name) {
            return;
        }
        out.push(ExtRequirement {
            name,
            from: from.into(),
            reason: reason
                .map(str::trim)
                .filter(|r| !r.is_empty())
                .map(|r| r.chars().take(200).collect()),
        });
    };
    let read = |f: &str| {
        std::fs::read_to_string(path.join(f))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
    };
    if let Some(json) = read("composer.json") {
        for (key, why) in json["suggest"].as_object().into_iter().flatten() {
            if let Some(ext) = key.strip_prefix("ext-") {
                add(ext, "composer.json", why.as_str());
            }
        }
    }
    if let Some(json) = read("composer.lock") {
        for package in ["packages", "packages-dev"]
            .iter()
            .flat_map(|k| json[*k].as_array().into_iter().flatten())
        {
            let name = package["name"].as_str().unwrap_or("a package");
            for (key, why) in package["suggest"].as_object().into_iter().flatten() {
                if let Some(ext) = key.strip_prefix("ext-") {
                    add(ext, name, why.as_str());
                }
            }
        }
    }
    if laravel {
        // Illuminate\Support\Number refuses to format without it.
        add(
            "intl",
            "Laravel",
            Some("The Number helper (format, currency, spell…) needs it."),
        );
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// `php -m` output, lowercased (`Zend OPcache` → `zend opcache` and `opcache`).
fn parse_modules(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        let name = line.to_lowercase();
        if let Some(short) = name.strip_prefix("zend ") {
            out.push(short.to_string());
        }
        out.push(name);
    }
    out.sort();
    out.dedup();
    out
}

/// `PDO::getAvailableDrivers()` (one per line) as extension names.
fn pdo_drivers(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
        .map(|d| format!("pdo_{}", d.to_lowercase()))
        .collect()
}

/// The package that brings an extension, in Debian/Ubuntu's naming.
fn apt_package(ext: &str) -> &str {
    match ext {
        "pdo_pgsql" | "pgsql" => "pgsql",
        "pdo_mysql" | "mysqli" | "mysqlnd" => "mysql",
        "pdo_sqlite" | "sqlite3" => "sqlite3",
        "dom" | "xml" | "simplexml" | "xmlreader" | "xmlwriter" | "xsl" => "xml",
        other => other,
    }
}

/// Extensions in StaticPHP's "bulk" set (its build-manifest.json, 2026-11-22).
const BULK: &[&str] = &[
    "apcu",
    "bcmath",
    "bz2",
    "calendar",
    "ctype",
    "curl",
    "dba",
    "dom",
    "event",
    "exif",
    "fileinfo",
    "filter",
    "ftp",
    "gd",
    "gmp",
    "iconv",
    "igbinary",
    "imagick",
    "imap",
    "intl",
    "libxml",
    "mbregex",
    "mbstring",
    "msgpack",
    "mysqli",
    "mysqlnd",
    "opcache",
    "openssl",
    "opentelemetry",
    "pcntl",
    "pdo",
    "pdo_mysql",
    "pgsql",
    "phar",
    "posix",
    "protobuf",
    "readline",
    "redis",
    "session",
    "shmop",
    "simplexml",
    "soap",
    "sockets",
    "sodium",
    "sqlite3",
    "swoole",
    "sysvmsg",
    "sysvsem",
    "sysvshm",
    "tokenizer",
    "xml",
    "xmlreader",
    "xmlwriter",
    "xsl",
    "zip",
    "zlib",
];

fn remedy(
    origin: &str,
    version: Option<&str>,
    missing: &[ExtRequirement],
    project: &str,
) -> Option<String> {
    if missing.is_empty() {
        return None;
    }
    let names: Vec<&str> = missing.iter().map(|m| m.name.as_str()).collect();
    let line: String = version
        .map(|v| v.split('.').take(2).collect::<Vec<_>>().join("."))
        .unwrap_or_default();
    Some(match origin {
        "locdev" if names.iter().all(|n| BULK.contains(n)) => format!(
            "LocDev's PHP (StaticPHP) has a fixed set of extensions; {} come(s) in its \"bulk\" set (the default): set php_flavor = \"bulk\" under [runtimes] in config.toml, `locdev restart`, then reinstall PHP {line} (`locdev runtime uninstall php {}` and `locdev runtime install php {line}`). A system PHP (`locdev edit {project} --php <version>`) is the other way.",
            names.join(", "),
            version.unwrap_or(line.as_str()),
        ),
        "locdev" => format!(
            "LocDev's PHP (StaticPHP) has a fixed set of extensions and none of its sets has {}; use a system PHP that has it (`locdev edit {project} --php <version>`).",
            names.join(", ")
        ),
        "apt" => {
            let mut packages: Vec<String> = names
                .iter()
                .map(|n| format!("php{line}-{}", apt_package(n)))
                .collect();
            packages.dedup();
            format!(
                "sudo apt install {} (then `locdev reload {project}`)",
                packages.join(" ")
            )
        }
        "homebrew" => format!(
            "Homebrew's php has most extensions built in; for {} use `pecl install <name>` with that php, then `locdev reload {project}`.",
            names.join(", ")
        ),
        _ if cfg!(windows) => format!(
            "Enable them in php.ini (`extension={}`) if the DLLs are in the ext folder, then `locdev reload {project}`.",
            names.join("`, `extension=")
        ),
        other => format!(
            "This PHP comes from {other}; add {} through it, then `locdev reload {project}`.",
            names.join(", ")
        ),
    })
}

impl Daemon {
    pub(super) async fn php_extensions(
        &self,
        project: &Project,
    ) -> Result<PhpExtensionsReport, AppError> {
        let laravel = self.stack_of(project).adapter.as_deref() == Some("laravel");
        let required = required(&project.path, laravel);
        let mut report = PhpExtensionsReport {
            recommended: recommended(&project.path, laravel, &required),
            required,
            ..Default::default()
        };
        let php = self
            .runtimes
            .for_project(project)
            .into_iter()
            .find(|r| r.kind == RuntimeKind::Php)
            .and_then(|r| r.version)
            // No pin or default: the PHP on PATH, as the site runs it.
            .or_else(|| self.runtimes.on_path(RuntimeKind::Php));
        let executable: PathBuf = match &php {
            Some(v) => v.executable.clone(),
            None => PathBuf::from("php"),
        };
        report.version = php.as_ref().map(|v| v.version.clone());
        report.origin = php.as_ref().map(|v| v.origin.clone());
        report.executable = Some(executable.clone());
        let output = tokio::process::Command::from(locdev_core::process::command(&executable))
            .arg("-m")
            .current_dir(&project.path)
            .output()
            .await;
        match output {
            Ok(o) if o.status.success() => {
                report.loaded = parse_modules(&String::from_utf8_lossy(&o.stdout));
                // PDO drivers can come from elsewhere than `pdo_<driver>`
                // (StaticPHP's bulk set gets sqlite/pgsql from Swoole's hooks):
                // a driver PDO offers counts as that extension.
                let drivers = tokio::process::Command::from(locdev_core::process::command(
                    &executable,
                ))
                .args([
                    "-r",
                    "if (class_exists('PDO')) echo implode(\"\\n\", PDO::getAvailableDrivers());",
                ])
                .current_dir(&project.path)
                .output()
                .await;
                if let Ok(d) = drivers
                    && d.status.success()
                {
                    report
                        .loaded
                        .extend(pdo_drivers(&String::from_utf8_lossy(&d.stdout)));
                    report.loaded.sort();
                    report.loaded.dedup();
                }
            }
            Ok(o) => report.error = Some(String::from_utf8_lossy(&o.stderr).trim().to_string()),
            Err(e) => report.error = Some(format!("could not run {}: {e}", executable.display())),
        }
        if report.error.is_none() {
            report.missing = report
                .required
                .iter()
                .filter(|r| !report.loaded.contains(&r.name))
                .cloned()
                .collect();
            report.remedy = remedy(
                report.origin.as_deref().unwrap_or("PATH"),
                report.version.as_deref(),
                &report.missing,
                &project.name,
            );
        }
        Ok(report)
    }
}

/// Prints each extension (`e`) and Zend extension (`z`) with its version,
/// then the extension folder, php.ini and scanned files.
const MODULES_SCRIPT: &str = r#"foreach (get_loaded_extensions() as $e) echo "e\t", $e, "\t", phpversion($e), "\n";
foreach (get_loaded_extensions(true) as $e) echo "z\t", $e, "\t", phpversion($e), "\n";
echo "d\t", ini_get('extension_dir'), "\n";
echo "i\t", php_ini_loaded_file(), "\n";
echo "s\t", str_replace("\n", "", (string) php_ini_scanned_files()), "\n";"#;

/// `opcache`, `Zend OPcache` → `opcache`; `php_pdo_pgsql.dll` → `pdo_pgsql`.
fn ext_key(name: &str) -> String {
    let lower = name.to_lowercase();
    let lower = lower.strip_prefix("zend ").unwrap_or(&lower);
    let lower = lower.strip_prefix("php_").unwrap_or(lower);
    lower
        .trim_end_matches(".dll")
        .trim_end_matches(".so")
        .to_string()
}

fn parse_modules_script(text: &str) -> PhpModules {
    let mut out = PhpModules::default();
    for line in text.lines() {
        let mut parts = line.splitn(3, '\t');
        let (Some(kind), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let version = parts
            .next()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let value = value.trim();
        match kind {
            "e" | "z" => {
                let zend = kind == "z";
                match out.extensions.iter_mut().find(|m| m.name == value) {
                    Some(m) => m.zend |= zend,
                    None => out.extensions.push(PhpModule {
                        name: value.into(),
                        version,
                        zend,
                    }),
                }
            }
            "d" if !value.is_empty() => out.extension_dir = Some(value.into()),
            "i" if !value.is_empty() => out.ini = Some(value.into()),
            "s" => {
                out.scanned = value
                    .split(',')
                    .map(str::trim)
                    .filter(|f| !f.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            _ => {}
        }
    }
    out.extensions.sort_by_key(|m| m.name.to_lowercase());
    out
}

/// Extension files in `dir` that aren't loaded.
fn available_in(dir: &Path, loaded: &[PhpModule]) -> Vec<String> {
    let loaded: Vec<String> = loaded.iter().map(|m| ext_key(&m.name)).collect();
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let ok = if cfg!(windows) {
                name.starts_with("php_") && name.ends_with(".dll")
            } else {
                name.ends_with(".so")
            };
            ok.then(|| ext_key(&name))
        })
        .filter(|k| !loaded.contains(k))
        .collect();
    out.sort();
    out.dedup();
    out
}

impl Daemon {
    pub(super) async fn php_modules(&self, executable: &Path) -> Result<PhpModules, AppError> {
        self.runtimes.wait_detected().await;
        // Only a PHP LocDev detected runs here, never a path from the client.
        let php = self
            .runtimes
            .versions(RuntimeKind::Php)
            .into_iter()
            .find(|v| v.executable == executable)
            .ok_or_else(|| {
                AppError::not_found(format!("{} is not a detected PHP", executable.display()))
            })?;
        let output = tokio::process::Command::from(locdev_core::process::command(&php.executable))
            .args(["-r", MODULES_SCRIPT])
            .output()
            .await
            .map_err(|e| {
                AppError::internal(format!("could not run {}: {e}", php.executable.display()))
            })?;
        if !output.status.success() {
            return Err(AppError::new(
                ErrorCode::NotRunnable,
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        let mut modules = parse_modules_script(&String::from_utf8_lossy(&output.stdout));
        if let Some(dir) = &modules.extension_dir {
            // A relative extension_dir is relative to the binary's folder.
            let dir = Path::new(dir);
            let dir = if dir.is_absolute() {
                dir.to_path_buf()
            } else {
                php.executable.parent().unwrap_or(Path::new(".")).join(dir)
            };
            modules.available = available_in(&dir, &modules.extensions);
        }
        modules.executable = php.executable;
        modules.version = php.version;
        modules.origin = php.origin;
        Ok(modules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirements_from_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("composer.json"),
            r#"{"require":{"php":"^8.2","ext-intl":"*","laravel/framework":"^12"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("composer.lock"),
            r#"{"packages":[{"name":"league/flysystem-ftp","require":{"ext-ftp":"*"}}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "DB_CONNECTION=pgsql\nCACHE_STORE=redis\n",
        )
        .unwrap();
        let req = required(dir.path(), true);
        let names: Vec<&str> = req.iter().map(|r| r.name.as_str()).collect();
        for want in ["intl", "ftp", "mbstring", "pdo_pgsql", "redis"] {
            assert!(names.contains(&want), "{want} in {names:?}");
        }
        assert_eq!(
            req.iter().find(|r| r.name == "ftp").unwrap().from,
            "league/flysystem-ftp"
        );
    }

    #[test]
    fn modules_and_remedies() {
        let loaded =
            parse_modules("[PHP Modules]\nCore\nPDO\npdo_mysql\n\n[Zend Modules]\nZend OPcache\n");
        assert!(loaded.contains(&"pdo".to_string()) && loaded.contains(&"opcache".to_string()));
        let missing = vec![ExtRequirement {
            name: "pdo_pgsql".into(),
            from: ".env".into(),
            reason: None,
        }];
        assert_eq!(
            remedy("apt", Some("8.3.14"), &missing, "shop").unwrap(),
            "sudo apt install php8.3-pgsql (then `locdev reload shop`)"
        );
        // pdo_pgsql is in no other StaticPHP set: a system PHP.
        let none = remedy("locdev", Some("8.4.1"), &missing, "shop").unwrap();
        assert!(
            none.contains("none of its sets") && !none.contains("bulk"),
            "{none}"
        );
        // intl is in "bulk": say how to switch, and what that costs.
        let intl = vec![ExtRequirement {
            name: "intl".into(),
            from: "composer.json".into(),
            reason: None,
        }];
        let bulk = remedy("locdev", Some("8.4.28"), &intl, "shop").unwrap();
        assert!(bulk.contains("php_flavor = \"bulk\""), "{bulk}");
        assert!(
            bulk.contains("locdev runtime uninstall php 8.4.28"),
            "{bulk}"
        );
        assert!(bulk.contains("locdev runtime install php 8.4"), "{bulk}");
        assert!(remedy("apt", None, &[], "shop").is_none());
        assert_eq!(
            pdo_drivers("mysql\npgsql\nodbc\nsqlite\n"),
            ["pdo_mysql", "pdo_pgsql", "pdo_odbc", "pdo_sqlite"]
        );
        assert!(pdo_drivers("bad name\n").is_empty());
    }

    #[test]
    fn recommendations_from_suggests() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("composer.json"),
            r#"{"require":{"ext-intl":"*"},"suggest":{"ext-gd":"For images"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("composer.lock"),
            r#"{"packages":[{"name":"intervention/image","suggest":{"ext-imagick":"Faster","ext-intl":"x","psr/log":"no"}}],
               "packages-dev":[{"name":"phpunit/phpunit","suggest":{"ext-pcov":"Coverage"}}]}"#,
        )
        .unwrap();
        let req = required(dir.path(), false);
        let rec = recommended(dir.path(), true, &req);
        let names: Vec<&str> = rec.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            ["gd", "imagick", "pcov"],
            "intl is required, not recommended"
        );
        let imagick = rec.iter().find(|r| r.name == "imagick").unwrap();
        assert_eq!(imagick.from, "intervention/image");
        assert_eq!(imagick.reason.as_deref(), Some("Faster"));
    }

    #[test]
    fn modules_listing() {
        let m = parse_modules_script(
            "e\tCore\t8.4.1\ne\tZend OPcache\t8.4.1\ne\tpdo_mysql\t\nz\tZend OPcache\t8.4.1\nd\tC:\\php\\ext\ni\tC:\\php\\php.ini\ns\t/a.ini,\n/b.ini\n",
        );
        assert_eq!(m.extensions.len(), 3);
        assert!(
            m.extensions
                .iter()
                .any(|e| e.name == "Zend OPcache" && e.zend)
        );
        assert_eq!(
            m.extensions
                .iter()
                .find(|e| e.name == "pdo_mysql")
                .unwrap()
                .version,
            None
        );
        assert_eq!(m.ini.as_deref(), Some("C:\\php\\php.ini"));
        let dir = tempfile::tempdir().unwrap();
        for f in [
            "php_opcache.dll",
            "php_pdo_mysql.dll",
            "php_gd.dll",
            "opcache.so",
            "gd.so",
            "readme.txt",
        ] {
            std::fs::write(dir.path().join(f), "").unwrap();
        }
        assert_eq!(available_in(dir.path(), &m.extensions), ["gd"]);
    }
}
