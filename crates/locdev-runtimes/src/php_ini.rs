//! LocDev's own php.ini per PHP version line: the directives the form
//! offers, their validation, the file (edited line by line, so comments and
//! anything written by hand stay), and reading what PHP actually uses.
//!
//! The file is never a system php.ini: it is added to the processes LocDev
//! runs through PHP_INI_SCAN_DIR, after PHP's own scan folder.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use locdev_core::phpini::{DirectiveKind, PhpDirective};
use serde::Deserialize;

/// The file's name inside its folder; sorts after the usual `NN-name.ini`.
pub const FILE_NAME: &str = "90-locdev.ini";

const HEADER: &str = "; Managed by LocDev: PHP settings for the sites it runs (Runtimes → PHP).\n; Edit here or with `locdev php set`; a php.ini of your system is never changed.\n";

struct Def {
    key: &'static str,
    label: &'static str,
    kind: DirectiveKind,
    description: &'static str,
    docs: &'static str,
}

const DEFS: &[Def] = &[
    Def {
        key: "memory_limit",
        label: "Memory limit",
        kind: DirectiveKind::Size,
        description: "The most memory a script may use; -1 for no limit.",
        docs: "https://www.php.net/manual/en/ini.core.php#ini.memory-limit",
    },
    Def {
        key: "upload_max_filesize",
        label: "Largest upload",
        kind: DirectiveKind::Size,
        description: "The largest file a form can upload.",
        docs: "https://www.php.net/manual/en/ini.core.php#ini.upload-max-filesize",
    },
    Def {
        key: "post_max_size",
        label: "Largest POST",
        kind: DirectiveKind::Size,
        description: "The largest request body; must be at least the largest upload.",
        docs: "https://www.php.net/manual/en/ini.core.php#ini.post-max-size",
    },
    Def {
        key: "max_file_uploads",
        label: "Files per request",
        kind: DirectiveKind::Int,
        description: "How many files one request may upload.",
        docs: "https://www.php.net/manual/en/ini.core.php#ini.max-file-uploads",
    },
    Def {
        key: "max_execution_time",
        label: "Execution time limit",
        kind: DirectiveKind::Int,
        description: "Seconds a request may run; 0 for no limit. Commands (artisan, composer) always run without one.",
        docs: "https://www.php.net/manual/en/info.configuration.php#ini.max-execution-time",
    },
    Def {
        key: "max_input_time",
        label: "Input time limit",
        kind: DirectiveKind::Int,
        description: "Seconds to read a request's input; -1 uses the execution limit.",
        docs: "https://www.php.net/manual/en/info.configuration.php#ini.max-input-time",
    },
    Def {
        key: "max_input_vars",
        label: "Input variables",
        kind: DirectiveKind::Int,
        description: "How many GET, POST and cookie variables a request may have.",
        docs: "https://www.php.net/manual/en/info.configuration.php#ini.max-input-vars",
    },
    Def {
        key: "display_errors",
        label: "Show errors",
        kind: DirectiveKind::Bool,
        description: "Print errors in the page.",
        docs: "https://www.php.net/manual/en/errorfunc.configuration.php#ini.display-errors",
    },
    Def {
        key: "display_startup_errors",
        label: "Show startup errors",
        kind: DirectiveKind::Bool,
        description: "Print errors that happen while PHP starts.",
        docs: "https://www.php.net/manual/en/errorfunc.configuration.php#ini.display-startup-errors",
    },
    Def {
        key: "log_errors",
        label: "Log errors",
        kind: DirectiveKind::Bool,
        description: "Write errors to the log (the site's output in LocDev).",
        docs: "https://www.php.net/manual/en/errorfunc.configuration.php#ini.log-errors",
    },
    Def {
        key: "error_reporting",
        label: "Error level",
        kind: DirectiveKind::ErrorLevel,
        description: "Which errors are reported, as constants (E_ALL & ~E_DEPRECATED) or a number.",
        docs: "https://www.php.net/manual/en/errorfunc.configuration.php#ini.error-reporting",
    },
    Def {
        key: "date.timezone",
        label: "Timezone",
        kind: DirectiveKind::Timezone,
        description: "The default timezone of date functions (America/Sao_Paulo, UTC).",
        docs: "https://www.php.net/manual/en/datetime.configuration.php#ini.date.timezone",
    },
    Def {
        key: "default_socket_timeout",
        label: "Socket timeout",
        kind: DirectiveKind::Int,
        description: "Seconds to wait on socket-based streams (HTTP requests from PHP).",
        docs: "https://www.php.net/manual/en/filesystem.configuration.php#ini.default-socket-timeout",
    },
    Def {
        key: "realpath_cache_size",
        label: "Realpath cache",
        kind: DirectiveKind::Size,
        description: "Cache of resolved file paths; bigger helps large projects.",
        docs: "https://www.php.net/manual/en/ini.core.php#ini.realpath-cache-size",
    },
    Def {
        key: "opcache.enable_cli",
        label: "OPcache for the built-in server",
        kind: DirectiveKind::Bool,
        description: "Cache compiled code in PHP's built-in server and commands (needs OPcache).",
        docs: "https://www.php.net/manual/en/opcache.configuration.php#ini.opcache.enable-cli",
    },
];

/// The CLI forces these over php.ini (`max_execution_time=0`,
/// `max_input_time=-1`), but PHP's built-in server, which runs the sites,
/// reads them from the files: what counts is the files' value.
const SERVER_READS_FILES: &[&str] = &["max_execution_time", "max_input_time"];

/// PHP's own defaults for those, when no file sets them (main.c).
fn builtin_default(key: &str) -> Option<&'static str> {
    match key {
        "max_execution_time" => Some("30"),
        "max_input_time" => Some("-1"),
        _ => None,
    }
}

pub fn is_known(key: &str) -> bool {
    DEFS.iter().any(|d| d.key == key)
}

/// Checks a value for a directive and writes it the way the file stores it.
pub fn validate(key: &str, raw: &str) -> Result<String, String> {
    let def = DEFS.iter().find(|d| d.key == key).ok_or_else(|| {
        format!("LocDev doesn't offer '{key}' in the form; use the advanced editor")
    })?;
    let value = raw.trim();
    let bad = |what: &str| Err(format!("{key}: '{value}' is not {what}"));
    match def.kind {
        DirectiveKind::Size => {
            if value == "-1" || size_bytes(value).is_some() {
                Ok(value.to_ascii_uppercase())
            } else {
                bad("a size like 128M, 2G or -1")
            }
        }
        DirectiveKind::Int => {
            let digits = value.strip_prefix('-').unwrap_or(value);
            if !digits.is_empty()
                && digits.len() <= 10
                && digits.chars().all(|c| c.is_ascii_digit())
                && (!value.starts_with('-') || value == "-1")
            {
                Ok(value.into())
            } else {
                bad("a whole number")
            }
        }
        DirectiveKind::Bool => match value.to_ascii_lowercase().as_str() {
            "on" | "1" | "true" | "yes" => Ok("On".into()),
            "off" | "0" | "false" | "no" | "" => Ok("Off".into()),
            _ => bad("On or Off"),
        },
        DirectiveKind::Timezone => {
            let ok = !value.is_empty()
                && value.len() <= 64
                && value.split('/').all(|part| {
                    !part.is_empty()
                        && part
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || "_-+".contains(c))
                });
            if ok {
                Ok(value.into())
            } else {
                bad("a timezone like America/Sao_Paulo")
            }
        }
        DirectiveKind::ErrorLevel => {
            let ok = !value.is_empty()
                && value.len() <= 120
                && value.chars().all(|c| {
                    c.is_ascii_uppercase() || c.is_ascii_digit() || "_ &|~^()".contains(c)
                })
                && value
                    .split(|c: char| " &|~^()".contains(c))
                    .filter(|t| !t.is_empty())
                    .all(|t| t.starts_with("E_") || t.chars().all(|c| c.is_ascii_digit()));
            if ok {
                Ok(value.into())
            } else {
                bad("an error level like E_ALL & ~E_DEPRECATED")
            }
        }
    }
}

/// `128M` → bytes. `None` for anything else (including `-1`).
pub fn size_bytes(value: &str) -> Option<u64> {
    let value = value.trim();
    let (digits, unit) = match value.char_indices().last()? {
        (i, c) if c.is_ascii_alphabetic() => (&value[..i], c.to_ascii_uppercase()),
        _ => (value, ' '),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) || digits.len() > 12 {
        return None;
    }
    let n: u64 = digits.parse().ok()?;
    let factor = match unit {
        ' ' => 1,
        'K' => 1 << 10,
        'M' => 1 << 20,
        'G' => 1 << 30,
        _ => return None,
    };
    n.checked_mul(factor)
}

/// How a value is written in the file (timezones quoted).
fn line_for(key: &str, value: &str) -> String {
    let def = DEFS.iter().find(|d| d.key == key);
    if def.is_some_and(|d| d.kind == DirectiveKind::Timezone) {
        format!("{key} = \"{value}\"")
    } else {
        format!("{key} = {value}")
    }
}

/// The key of an ini line (`key = value`), if it is one.
fn key_of(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(';') || line.starts_with('[') || line.starts_with('#') {
        return None;
    }
    let (key, _) = line.split_once('=')?;
    let key = key.trim();
    (!key.is_empty()).then_some(key)
}

/// The values the file sets (`key = value`, quotes removed).
pub fn parse(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        if let Some(key) = key_of(line) {
            let value = line.split_once('=').map(|(_, v)| v).unwrap_or("").trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or(value);
            out.insert(key.to_string(), value.to_string());
        }
    }
    out
}

/// The file with `changes` applied: an existing line is replaced in place,
/// `None` removes it, a new key is appended. Everything else stays.
pub fn apply(text: &str, changes: &BTreeMap<String, Option<String>>) -> Result<String, String> {
    let mut validated = BTreeMap::new();
    for (key, value) in changes {
        let value = value.as_deref().map(|v| validate(key, v)).transpose()?;
        validated.insert(key.as_str(), value);
    }
    let base = if text.trim().is_empty() { HEADER } else { text };
    let mut out = Vec::new();
    let mut done = Vec::new();
    for line in base.lines() {
        match key_of(line).and_then(|k| validated.get_key_value(k)) {
            Some((key, Some(value))) => {
                if !done.contains(key) {
                    out.push(line_for(key, value));
                    done.push(*key);
                }
            }
            Some((_, None)) => {}
            None => out.push(line.to_string()),
        }
    }
    for (key, value) in &validated {
        if let Some(value) = value
            && !done.contains(key)
        {
            out.push(line_for(key, value));
        }
    }
    let mut text = out.join("\n");
    text.push('\n');
    Ok(text)
}

/// sha256 of the file's text, to save only over the version that was shown.
pub fn hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A new file for a line with no settings yet.
pub fn empty() -> String {
    HEADER.to_string()
}

/// What `php` says about itself and the catalog's directives.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Probe {
    pub version: String,
    pub sapi: String,
    #[serde(default)]
    pub loaded: Option<String>,
    /// Comma-separated (PHP's format), possibly with newlines.
    #[serde(default)]
    pub scanned: Option<String>,
    /// `ini_get` (null when PHP doesn't know the directive).
    #[serde(default)]
    pub values: BTreeMap<String, Option<String>>,
    /// `get_cfg_var`: the value from the files (null when no file sets it).
    #[serde(default)]
    pub files: BTreeMap<String, Option<String>>,
    #[serde(default)]
    pub timezone_ok: Option<bool>,
}

impl Probe {
    pub fn scanned_files(&self) -> Vec<PathBuf> {
        self.scanned
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect()
    }

    /// The value a site served by PHP's built-in server gets.
    pub fn value(&self, key: &str) -> Option<String> {
        if SERVER_READS_FILES.contains(&key) {
            return self
                .files
                .get(key)
                .cloned()
                .flatten()
                .or_else(|| builtin_default(key).map(str::to_string));
        }
        let value = self.values.get(key).cloned().flatten()?;
        Some(display(key, &value))
    }
}

/// Booleans the way people write them (PHP reports "1"/"").
fn display(key: &str, value: &str) -> String {
    match DEFS.iter().find(|d| d.key == key).map(|d| d.kind) {
        Some(DirectiveKind::Bool) => match value.to_ascii_lowercase().as_str() {
            "1" | "on" | "true" | "yes" | "stderr" | "stdout" => "On".into(),
            _ => "Off".into(),
        },
        _ => value.to_string(),
    }
}

/// The PHP code that reports the catalog (keys are our constants, never
/// user text).
fn probe_code() -> String {
    let keys: Vec<String> = DEFS.iter().map(|d| format!("'{}'", d.key)).collect();
    format!(
        "$k=[{}];$o=['version'=>PHP_VERSION,'sapi'=>PHP_SAPI,'loaded'=>php_ini_loaded_file()?:null,'scanned'=>php_ini_scanned_files()?:null,'values'=>[],'files'=>[]];\
foreach($k as $x){{$v=ini_get($x);$o['values'][$x]=$v===false?null:$v;$c=get_cfg_var($x);$o['files'][$x]=$c===false?null:(string)$c;}}\
$tz=ini_get('date.timezone');$o['timezone_ok']=$tz?(@timezone_open($tz)!==false):null;echo json_encode($o);",
        keys.join(",")
    )
}

/// Runs `php` with `scan_dir` as PHP_INI_SCAN_DIR (`None`: as inherited).
/// Blocking. An error from the ini files themselves (a syntax error in
/// LocDev's file) fails the probe with PHP's message.
pub fn probe(php: &Path, scan_dir: Option<&str>) -> Result<Probe, String> {
    let mut cmd = locdev_core::process::command(php);
    cmd.arg("-r").arg(probe_code());
    if let Some(dir) = scan_dir {
        cmd.env("PHP_INI_SCAN_DIR", dir);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("could not run {}: {e}", php.display()))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    for text in [&stderr, &stdout] {
        if let Some(line) = text.lines().find(|l| l.contains("syntax error")) {
            return Err(line.trim().trim_start_matches("PHP:").trim().to_string());
        }
    }
    let json = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or_else(|| format!("php printed no report: {}", stderr.trim()))?;
    let mut report: Probe = serde_json::from_str(json).map_err(|e| format!("php's report: {e}"))?;
    read_from_files(&mut report);
    Ok(report)
}

/// The CLI hides the files' values of [`SERVER_READS_FILES`] behind its
/// forced ones (`get_cfg_var` included), so they're read from the files PHP
/// loaded, in its order: php.ini, then the scan folders; the last one wins.
fn read_from_files(report: &mut Probe) {
    let files: Vec<PathBuf> = report
        .loaded
        .iter()
        .map(PathBuf::from)
        .chain(report.scanned_files())
        .collect();
    for key in SERVER_READS_FILES {
        let mut found = None;
        for file in &files {
            if let Ok(text) = std::fs::read_to_string(file)
                && let Some(value) = parse(&text).remove(*key)
            {
                found = Some(value);
            }
        }
        report.files.insert((*key).to_string(), found);
    }
}

/// PHP_INI_SCAN_DIR with `dir` after what's already there (the inherited
/// value, else PHP's default folder via a leading separator).
pub fn scan_dir_with(existing: Option<&str>, dir: &Path) -> String {
    let sep = if cfg!(windows) { ";" } else { ":" };
    match existing {
        Some(existing) if !existing.is_empty() => format!("{existing}{sep}{}", dir.display()),
        _ => format!("{sep}{}", dir.display()),
    }
}

/// The form: every directive with its original, effective and LocDev values.
pub fn directives(original: &Probe, effective: &Probe, text: &str) -> Vec<PhpDirective> {
    let ours = parse(text);
    DEFS.iter()
        .map(|d| PhpDirective {
            key: d.key.into(),
            label: d.label.into(),
            kind: d.kind,
            description: d.description.into(),
            docs: d.docs.into(),
            original: original.value(d.key),
            effective: effective.value(d.key),
            value: ours.get(d.key).cloned(),
        })
        .collect()
}

/// Settings that work against each other, and values PHP refused.
pub fn warnings(effective: &Probe, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let size = |k: &str| effective.value(k).and_then(|v| size_bytes(&v));
    if let (Some(upload), Some(post)) = (size("upload_max_filesize"), size("post_max_size"))
        && post > 0
        && upload > post
    {
        out.push("upload_max_filesize is larger than post_max_size: uploads above post_max_size are dropped whole.".into());
    }
    if let (Some(post), Some(memory)) = (size("post_max_size"), size("memory_limit"))
        && post > memory
    {
        out.push(
            "post_max_size is larger than memory_limit: a request that big can't be handled."
                .into(),
        );
    }
    if effective.timezone_ok == Some(false) {
        out.push(
            "date.timezone isn't a timezone PHP knows; it falls back to UTC with a warning.".into(),
        );
    }
    for (key, value) in parse(text) {
        if is_known(&key) && validate(&key, &value).is_err() {
            out.push(format!(
                "{key} = {value} in LocDev's file isn't a valid value."
            ));
        }
    }
    out
}

/// Keys whose effective value differs from what LocDev's file asks (PHP
/// ignored them: a typo, or something later in the scan folder wins).
pub fn not_applied(effective: &Probe, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (key, wanted) in parse(text) {
        if !is_known(&key) {
            continue;
        }
        let Ok(wanted) = validate(&key, &wanted) else {
            continue;
        };
        let def = DEFS.iter().find(|d| d.key == key).unwrap();
        let got = effective.value(&key);
        let same = match def.kind {
            // PHP evaluates the expression; any number means it was read.
            DirectiveKind::ErrorLevel => got.as_deref().is_some_and(|g| g.parse::<i64>().is_ok()),
            DirectiveKind::Size => got.as_deref().is_some_and(|g| {
                g.eq_ignore_ascii_case(&wanted) || size_bytes(g) == size_bytes(&wanted)
            }),
            _ => got.as_deref() == Some(wanted.as_str()),
        };
        if !same {
            out.push(format!(
                "{key}: LocDev's file says {wanted}, PHP uses {}",
                got.unwrap_or_else(|| "nothing (unknown directive)".into())
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_checked_and_normalized() {
        assert_eq!(validate("memory_limit", "512m").unwrap(), "512M");
        assert_eq!(validate("memory_limit", "-1").unwrap(), "-1");
        assert!(validate("memory_limit", "lots").is_err());
        assert!(validate("memory_limit", "12T").is_err());
        assert_eq!(validate("display_errors", "true").unwrap(), "On");
        assert_eq!(validate("display_errors", "0").unwrap(), "Off");
        assert!(validate("display_errors", "maybe").is_err());
        assert_eq!(validate("max_execution_time", "0").unwrap(), "0");
        assert!(validate("max_execution_time", "-5").is_err());
        assert!(validate("max_execution_time", "1.5").is_err());
        assert_eq!(
            validate("error_reporting", "E_ALL & ~E_DEPRECATED").unwrap(),
            "E_ALL & ~E_DEPRECATED"
        );
        assert!(validate("error_reporting", "E_ALL; extension=evil").is_err());
        assert!(validate("error_reporting", "system(1)").is_err());
        assert_eq!(
            validate("date.timezone", "America/Sao_Paulo").unwrap(),
            "America/Sao_Paulo"
        );
        assert!(validate("date.timezone", "UTC\"\nextension=x").is_err());
        assert!(validate("extension", "x.so").is_err(), "only catalog keys");
    }

    #[test]
    fn sizes() {
        assert_eq!(size_bytes("128M"), Some(128 << 20));
        assert_eq!(size_bytes("2g"), Some(2 << 30));
        assert_eq!(size_bytes("1024"), Some(1024));
        assert_eq!(size_bytes("-1"), None);
        assert_eq!(size_bytes("M"), None);
    }

    #[test]
    fn edits_keep_everything_else() {
        let text = "; mine\nmemory_limit = 256M\n\n; keep this\nxdebug.mode = debug\ndisplay_errors = On\n";
        let mut changes = BTreeMap::new();
        changes.insert("memory_limit".to_string(), Some("1G".to_string()));
        changes.insert("display_errors".to_string(), None);
        changes.insert("date.timezone".to_string(), Some("UTC".to_string()));
        let out = apply(text, &changes).unwrap();
        assert_eq!(
            out,
            "; mine\nmemory_limit = 1G\n\n; keep this\nxdebug.mode = debug\ndate.timezone = \"UTC\"\n"
        );
        let parsed = parse(&out);
        assert_eq!(parsed["date.timezone"], "UTC");
        assert_eq!(parsed["xdebug.mode"], "debug");

        let fresh = apply("", &changes).unwrap();
        assert!(fresh.starts_with("; Managed by LocDev"));
        assert!(fresh.contains("memory_limit = 1G"));

        let mut bad = BTreeMap::new();
        bad.insert("memory_limit".to_string(), Some("huge".to_string()));
        assert!(apply(text, &bad).is_err());
    }

    fn probe_with(values: &[(&str, &str)], files: &[(&str, &str)]) -> Probe {
        Probe {
            version: "8.3.0".into(),
            sapi: "cli".into(),
            values: values
                .iter()
                .map(|(k, v)| ((*k).into(), Some((*v).into())))
                .collect(),
            files: files
                .iter()
                .map(|(k, v)| ((*k).into(), Some((*v).into())))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn the_server_reads_time_limits_from_the_files() {
        // `php -r` reports the CLI's forced 0/-1; the built-in server uses
        // the files' value, or PHP's default.
        let p = probe_with(
            &[("max_execution_time", "0"), ("display_errors", "")],
            &[("max_execution_time", "60")],
        );
        assert_eq!(p.value("max_execution_time").as_deref(), Some("60"));
        assert_eq!(p.value("max_input_time").as_deref(), Some("-1"));
        assert_eq!(p.value("display_errors").as_deref(), Some("Off"));
        let none = probe_with(&[], &[]);
        assert_eq!(none.value("max_execution_time").as_deref(), Some("30"));
    }

    #[test]
    fn warnings_and_ignored_values() {
        let p = probe_with(
            &[
                ("upload_max_filesize", "64M"),
                ("post_max_size", "8M"),
                ("memory_limit", "128M"),
            ],
            &[],
        );
        let w = warnings(&p, "");
        assert!(w.iter().any(|w| w.contains("upload_max_filesize")), "{w:?}");
        let ignored = not_applied(&p, "memory_limit = 256M\npost_max_size = 8M\n");
        assert_eq!(ignored.len(), 1, "{ignored:?}");
        assert!(ignored[0].contains("memory_limit"));
    }

    #[test]
    fn scan_dir_keeps_what_was_there() {
        let dir = Path::new("/x/8.3");
        let sep = if cfg!(windows) { ";" } else { ":" };
        assert_eq!(scan_dir_with(None, dir), format!("{sep}/x/8.3"));
        assert_eq!(
            scan_dir_with(Some("/etc/php"), dir),
            format!("/etc/php{sep}/x/8.3")
        );
    }

    fn php() -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .map(|d| d.join(if cfg!(windows) { "php.exe" } else { "php" }))
            .find(|p| p.is_file())
    }

    #[test]
    fn probes_a_real_php_with_locdevs_folder() {
        let Some(php) = php() else {
            eprintln!("no php on PATH; skipped");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(FILE_NAME),
            "memory_limit = 321M\nmax_execution_time = 45\ndate.timezone = \"America/Sao_Paulo\"\n",
        )
        .unwrap();
        let scan = scan_dir_with(None, dir.path());
        let report = probe(&php, Some(&scan)).unwrap();
        assert_eq!(report.value("memory_limit").as_deref(), Some("321M"));
        assert_eq!(report.value("max_execution_time").as_deref(), Some("45"));
        assert_eq!(report.timezone_ok, Some(true));
        assert!(
            report
                .scanned_files()
                .iter()
                .any(|f| f.ends_with(FILE_NAME))
        );
        assert!(
            not_applied(
                &report,
                &std::fs::read_to_string(dir.path().join(FILE_NAME)).unwrap()
            )
            .is_empty()
        );

        std::fs::write(dir.path().join(FILE_NAME), "memory_limit == \"3\n").unwrap();
        let error = probe(&php, Some(&scan)).unwrap_err();
        assert!(error.contains("syntax error"), "{error}");
    }
}
