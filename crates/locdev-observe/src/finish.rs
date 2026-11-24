//! What the pipeline adds to every event: the fingerprint that groups
//! repeats, and a likely cause for errors LocDev recognizes.

use locdev_core::observe::Event;

/// Same kind, same error type, same message once numbers/ids/quoted values
/// are blanked, same place in the project's own code → same problem.
pub fn fingerprint(e: &Event) -> String {
    let place = e
        .stack
        .iter()
        .find(|f| !f.vendor && f.file.is_some())
        .map(|f| {
            format!(
                "{}:{}",
                f.file.as_deref().unwrap_or(""),
                f.line.unwrap_or(0)
            )
        })
        .or_else(|| {
            e.file
                .as_ref()
                .map(|f| format!("{f}:{}", e.line.unwrap_or(0)))
        })
        .unwrap_or_default();
    let text = format!(
        "{:?}|{}|{}|{}|{}",
        e.kind,
        e.site.as_deref().unwrap_or(""),
        e.code.as_deref().unwrap_or(""),
        normalize(&e.message),
        place
    );
    format!("{:016x}", fnv(text.as_bytes()))
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Blanks what changes between repeats: numbers, hex ids, quoted values.
pub fn normalize(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut chars = message.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
                out.push(c);
            }
            continue;
        }
        if matches!(c, '"' | '\'' | '`') {
            quote = Some(c);
            out.push(c);
            out.push('?');
            continue;
        }
        if c.is_ascii_digit() {
            while chars
                .peek()
                .is_some_and(|n| n.is_ascii_alphanumeric() || *n == '-' || *n == '.')
            {
                chars.next();
            }
            out.push('#');
            continue;
        }
        out.push(c);
    }
    out.chars().take(300).collect()
}

/// (all of these appear, what to try). Checked against the message, the
/// error type and the raw text.
const HINTS: &[(&[&str], &str)] = &[
    (
        &["EADDRINUSE"],
        "Another program is using the port. `locdev doctor` shows which one.",
    ),
    (
        &["address already in use"],
        "Another program is using the port. `locdev doctor` shows which one.",
    ),
    (
        &["SQLSTATE[HY000] [2002]"],
        "The database isn't running or listens elsewhere: check Services and the site's DB_HOST/DB_PORT.",
    ),
    (
        &["ECONNREFUSED", ":5432"],
        "PostgreSQL isn't running or listens elsewhere: check Services and the connection settings.",
    ),
    (
        &["ECONNREFUSED", ":3306"],
        "MySQL isn't running or listens elsewhere: check Services and the connection settings.",
    ),
    (
        &["ECONNREFUSED", ":6379"],
        "Redis isn't running or listens elsewhere: check Services and the connection settings.",
    ),
    (
        &["SQLSTATE[HY000] [1045]"],
        "The database refused the user or password in the site's .env.",
    ),
    (
        &["SQLSTATE[42S02]"],
        "A table is missing: run the migrations (`php artisan migrate`).",
    ),
    (
        &["SQLSTATE[42P01]"],
        "A table is missing: run the migrations.",
    ),
    (
        &["No application encryption key has been specified"],
        "Run `php artisan key:generate` to set APP_KEY in .env.",
    ),
    (
        &["MissingAppKeyException"],
        "Run `php artisan key:generate` to set APP_KEY in .env.",
    ),
    (
        &["Target class [", "] does not exist"],
        "The container can't find that class: check the controller's name and namespace.",
    ),
    (
        &["View [", "] not found"],
        "The Blade view doesn't exist: check its name and resources/views.",
    ),
    (
        &["Class \"", "\" not found"],
        "PHP can't find the class: check the namespace and `use`, then `composer dump-autoload`.",
    ),
    (
        &["Allowed memory size of"],
        "PHP ran out of memory: raise memory_limit (Runtimes → PHP) or look for something loading too much.",
    ),
    (
        &["Maximum execution time of"],
        "The request ran longer than max_execution_time (Runtimes → PHP).",
    ),
    (
        &["Vite manifest not found"],
        "Start the site's Vite helper process, or build the assets (`npm run build`).",
    ),
    (
        &["Unable to locate file in Vite manifest"],
        "Start the site's Vite helper process, or build the assets (`npm run build`).",
    ),
    (
        &["CSRF token mismatch"],
        "The session cookie didn't come back: check SESSION_DOMAIN and that the page and the form use the same host.",
    ),
    (
        &["Permission denied", "storage"],
        "PHP can't write to storage/ or bootstrap/cache: fix those folders' permissions.",
    ),
    (
        &["Cannot find module"],
        "A dependency is missing: install the packages (npm/pnpm/yarn install) or fix the import path.",
    ),
    (
        &["ERR_MODULE_NOT_FOUND"],
        "A dependency is missing: install the packages (npm/pnpm/yarn install) or fix the import path.",
    ),
    (
        &["Module not found"],
        "A dependency is missing or the import path is wrong.",
    ),
    (
        &["Failed to resolve import"],
        "The imported file doesn't exist: check the path and the extension.",
    ),
    (
        &["is not allowed", "allowedHosts"],
        "Vite blocked the host: restart the site so LocDev's allowed-hosts variable reaches it.",
    ),
    (
        &["ModuleNotFoundError"],
        "The package isn't installed in this site's Python environment (.venv): `pip install -r requirements.txt`.",
    ),
    (
        &["No module named"],
        "The package isn't installed in this site's Python environment (.venv): `pip install -r requirements.txt`.",
    ),
    (
        &["unwrap()", "panicked"],
        "An `unwrap()` met an error or `None`: handle that case (or use `?`).",
    ),
    (
        &["called `Option::unwrap()`"],
        "An `unwrap()` met `None`: handle that case.",
    ),
    (
        &["called `Result::unwrap()`"],
        "An `unwrap()` met an error: handle it (or use `?`).",
    ),
];

pub fn hint(e: &Event) -> Option<String> {
    let hay = format!(
        "{}\n{}\n{}",
        e.code.as_deref().unwrap_or(""),
        e.message,
        e.raw
    );
    HINTS
        .iter()
        .find(|(needles, _)| needles.iter().all(|n| hay.contains(n)))
        .map(|(_, h)| (*h).to_string())
}

/// Fingerprint and hint, once the event has its site.
pub fn finish(e: &mut Event) {
    e.fingerprint = fingerprint(e);
    if e.hint.is_none() {
        e.hint = hint(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::blank;
    use locdev_core::observe::{EventKind, EventSeverity};

    #[test]
    fn repeats_share_a_fingerprint() {
        let mut a = blank(
            EventKind::Exception,
            EventSeverity::Error,
            "User 42 not found (id 'abc-1')",
            1,
        );
        let mut b = blank(
            EventKind::Exception,
            EventSeverity::Error,
            "User 7 not found (id 'zz')",
            2,
        );
        a.code = Some("ModelNotFound".into());
        b.code = a.code.clone();
        assert_eq!(fingerprint(&a), fingerprint(&b));
        b.site = Some("other".into());
        assert_ne!(
            fingerprint(&a),
            fingerprint(&b),
            "another site is another problem"
        );
        assert_eq!(
            normalize("took 1.5s at 0x7ffd, \"x\""),
            "took # at #, \"?\""
        );
    }

    #[test]
    fn known_causes() {
        let e = blank(
            EventKind::Exception,
            EventSeverity::Error,
            "SQLSTATE[HY000] [2002] Connection refused (Connection: mysql)",
            1,
        );
        assert!(hint(&e).unwrap().contains("database"));
        let e = blank(
            EventKind::Log,
            EventSeverity::Error,
            "listen EADDRINUSE: address already in use :::3000",
            1,
        );
        assert!(hint(&e).unwrap().contains("port"));
        let e = blank(EventKind::Log, EventSeverity::Error, "all good", 1);
        assert_eq!(hint(&e), None);
    }
}
