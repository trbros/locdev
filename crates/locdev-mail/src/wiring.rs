//! How a site's framework is pointed at LocDev's capture server: through
//! the variables it reads (put in the process environment only, never in
//! the project's files), or a snippet when it only reads settings from code.
//!
//! A project that points at a real mail server (another host, an API
//! transport like SES or Postmark) is flagged: replacing that needs the
//! user's explicit `override`, so a real SMTP is never swapped silently.

use std::collections::BTreeMap;
use std::path::Path;

use locdev_core::mail::{MailHeader, MailSetup, MailWiring};

/// The password LocDev gives every site; the server accepts any.
pub const PASSWORD: &str = "locdev";

pub struct Input<'a> {
    pub site: &'a str,
    pub framework: &'a str,
    /// Component ids from the stack detection (`laravel`, `django`…).
    pub components: &'a [String],
    pub root: &'a Path,
    pub port: u16,
    /// The project's own variables (its .env files), as the framework sees them.
    pub env: &'a BTreeMap<String, String>,
    /// `locdevd sendmail …` for PHP's mail(); `None` when not known.
    pub sendmail: Option<String>,
    /// Where LocDev may write a php.ini fragment for PHP's mail().
    pub ini_dir: &'a Path,
}

pub struct Plan {
    pub setup: MailSetup,
    pub env: Vec<(String, String)>,
}

fn local_host(host: &str) -> bool {
    let host = host.trim().trim_matches(['"', '\'']).to_ascii_lowercase();
    matches!(
        host.as_str(),
        "" | "127.0.0.1"
            | "localhost"
            | "::1"
            | "[::1]"
            | "0.0.0.0"
            | "mailpit"
            | "mailhog"
            | "host.docker.internal"
            | "null"
    ) || host.ends_with(".test")
        || host.ends_with(".localhost")
}

fn read(root: &Path, file: &str) -> String {
    std::fs::read_to_string(root.join(file)).unwrap_or_default()
}

fn pairs(list: &[(&str, String)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn shown(env: &[(String, String)]) -> Vec<MailHeader> {
    env.iter()
        .map(|(k, v)| MailHeader {
            name: k.clone(),
            value: v.clone(),
        })
        .collect()
}

fn current(env: &BTreeMap<String, String>, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .filter_map(|k| env.get(*k).map(|v| (k, v)))
        .map(|(k, v)| {
            if k.contains("PASS") || k.contains("SECRET") {
                format!("{k}=…")
            } else {
                format!("{k}={v}")
            }
        })
        .collect()
}

pub fn plan(input: &Input) -> Plan {
    let has = |id: &str| input.components.iter().any(|c| c == id);
    if has("laravel") {
        laravel(input)
    } else if has("wordpress") || (has("php") && !has("laravel")) {
        php_mail(input)
    } else {
        by_names(input)
    }
}

fn laravel(input: &Input) -> Plan {
    const KEYS: &[&str] = &[
        "MAIL_MAILER",
        "MAIL_HOST",
        "MAIL_PORT",
        "MAIL_URL",
        "MAIL_SCHEME",
        "MAIL_ENCRYPTION",
        "MAIL_USERNAME",
    ];
    let config = read(input.root, "config/mail.php");
    let mailer = input
        .env
        .get("MAIL_MAILER")
        .map(|m| m.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "log".into());
    let host = input.env.get("MAIL_HOST").cloned().unwrap_or_default();
    let url = input
        .env
        .get("MAIL_URL")
        .filter(|u| !u.trim().is_empty() && u.trim() != "null");
    let real = match mailer.as_str() {
        "log" | "array" | "null" | "" => false,
        "smtp" => {
            !local_host(&host)
                || url.is_some_and(|u| {
                    !local_host(
                        u.split('@')
                            .next_back()
                            .unwrap_or(u)
                            .split(':')
                            .next()
                            .unwrap_or(""),
                    )
                })
        }
        // ses, postmark, mailgun, resend, sendmail, failover…: they deliver for real.
        _ => true,
    };
    let mut env = pairs(&[
        ("MAIL_MAILER", "smtp".into()),
        ("MAIL_HOST", "127.0.0.1".into()),
        ("MAIL_PORT", input.port.to_string()),
        ("MAIL_USERNAME", input.site.to_string()),
        ("MAIL_PASSWORD", PASSWORD.into()),
    ]);
    // Laravel 11+ reads MAIL_SCHEME; 10 and older MAIL_ENCRYPTION (default "tls").
    if config.contains("MAIL_SCHEME") {
        env.push(("MAIL_SCHEME".into(), "smtp".into()));
    }
    if config.contains("MAIL_ENCRYPTION") {
        env.push(("MAIL_ENCRYPTION".into(), "null".into()));
    }
    if url.is_some() {
        // A MAIL_URL wins over MAIL_HOST in config/mail.php.
        env.push(("MAIL_URL".into(), String::new()));
    }
    let mut notes = vec![
        "Queue workers and the scheduler get the same variables, so queued mail is captured too."
            .to_string(),
    ];
    if config.is_empty() {
        notes.push(
            "config/mail.php wasn't found; these are the names a default Laravel install reads."
                .into(),
        );
    }
    Plan {
        setup: MailSetup {
            site: input.site.into(),
            framework: input.framework.into(),
            enabled: false,
            wiring: MailWiring::Automatic,
            current: current(input.env, KEYS),
            real_smtp: real,
            injects: shown(&env),
            snippet: None,
            notes,
        },
        env,
    }
}

/// PHP's mail() (WordPress, plain PHP): a php.ini fragment, added through
/// PHP_INI_SCAN_DIR so the system's own ini files still load.
fn php_mail(input: &Input) -> Plan {
    let ini = if cfg!(windows) {
        // On Windows mail() talks SMTP itself and ignores sendmail_path.
        format!(
            "; Managed by LocDev: PHP mail() goes to LocDev's Mailbox.\nSMTP = 127.0.0.1\nsmtp_port = {}\n",
            input.port
        )
    } else {
        match &input.sendmail {
            Some(cmd) => format!(
                "; Managed by LocDev: PHP mail() goes to LocDev's Mailbox.\nsendmail_path = \"{cmd}\"\n"
            ),
            None => String::new(),
        }
    };
    let dir = input.ini_dir.join(input.site);
    let mut env = Vec::new();
    let mut notes = vec!["PHP's mail() is sent to the Mailbox. An SMTP plugin (WP Mail SMTP and the like) bypasses mail(): point it at 127.0.0.1 too.".to_string()];
    if !ini.is_empty()
        && std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(dir.join("zz-locdev-mail.ini"), &ini))
            .is_ok()
    {
        let sep = if cfg!(windows) { ";" } else { ":" };
        // A leading separator keeps PHP's default scan directory.
        let value = match std::env::var("PHP_INI_SCAN_DIR") {
            Ok(existing) if !existing.is_empty() => format!("{existing}{sep}{}", dir.display()),
            _ => format!("{sep}{}", dir.display()),
        };
        env.push(("PHP_INI_SCAN_DIR".to_string(), value));
    } else {
        notes.push("LocDev couldn't write its php.ini fragment; set sendmail_path by hand.".into());
    }
    Plan {
        setup: MailSetup {
            site: input.site.into(),
            framework: input.framework.into(),
            enabled: false,
            wiring: MailWiring::Automatic,
            current: Vec::new(),
            real_smtp: false,
            injects: shown(&env),
            snippet: Some(ini),
            notes,
        },
        env,
    }
}

/// Stacks that read SMTP settings from code: the variables the project
/// already uses for it (from its .env and .env.example), when there are some.
fn by_names(input: &Input) -> Plan {
    let mut names: Vec<String> = input.env.keys().cloned().collect();
    for file in [".env.example", ".env.sample", ".env.dist"] {
        if let Ok(bytes) = std::fs::read(input.root.join(file))
            && let Ok(doc) = locdev_env::dotenv::parse(&bytes)
        {
            names.extend(doc.entries.into_iter().map(|e| e.key));
        }
    }
    names.sort();
    names.dedup();
    let prefix = |n: &str| {
        ["SMTP_", "MAIL_", "EMAIL_", "MAILER_"]
            .iter()
            .any(|p| n.starts_with(p))
    };
    let mut env = Vec::new();
    for name in names.iter().filter(|n| prefix(n)) {
        let rest = name.split_once('_').map(|(_, r)| r).unwrap_or("");
        let value = match rest {
            "HOST" | "SERVER" => Some("127.0.0.1".to_string()),
            "PORT" => Some(input.port.to_string()),
            "USER" | "USERNAME" | "HOST_USER" | "LOGIN" => Some(input.site.to_string()),
            "PASS" | "PASSWORD" | "HOST_PASSWORD" => Some(PASSWORD.to_string()),
            "SECURE" | "USE_TLS" | "USE_SSL" | "TLS" | "SSL" | "ENABLE_STARTTLS_AUTO" => {
                Some("false".to_string())
            }
            "URL" | "DSN" => Some(format!(
                "smtp://{}:{PASSWORD}@127.0.0.1:{}",
                input.site, input.port
            )),
            _ => None,
        };
        if let Some(value) = value {
            env.push((name.clone(), value));
        }
    }
    let host_key = env
        .iter()
        .find(|(k, _)| k.ends_with("_HOST") || k.ends_with("_SERVER"))
        .map(|(k, _)| k.clone());
    let real = host_key
        .and_then(|k| input.env.get(&k))
        .is_some_and(|h| !local_host(h));
    let has = |id: &str| input.components.iter().any(|c| c == id);
    let snippet = if has("django") {
        format!(
            "# settings.py (development)\nEMAIL_BACKEND = \"django.core.mail.backends.smtp.EmailBackend\"\nEMAIL_HOST = \"127.0.0.1\"\nEMAIL_PORT = {port}\nEMAIL_HOST_USER = \"{site}\"\nEMAIL_HOST_PASSWORD = \"{PASSWORD}\"\nEMAIL_USE_TLS = False\n",
            port = input.port,
            site = input.site
        )
    } else if has("rails") {
        format!(
            "# config/environments/development.rb\nconfig.action_mailer.delivery_method = :smtp\nconfig.action_mailer.smtp_settings = {{\n  address: \"127.0.0.1\", port: {port},\n  user_name: \"{site}\", password: \"{PASSWORD}\", authentication: :plain,\n  enable_starttls_auto: false\n}}\n",
            port = input.port,
            site = input.site
        )
    } else {
        format!(
            "// Nodemailer\nconst transport = nodemailer.createTransport({{\n  host: \"127.0.0.1\",\n  port: {port},\n  secure: false,\n  auth: {{ user: \"{site}\", pass: \"{PASSWORD}\" }},\n}});\n",
            port = input.port,
            site = input.site
        )
    };
    let automatic = !env.is_empty();
    let mut notes = Vec::new();
    if automatic {
        let used: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
        notes.push(format!(
            "The project's .env files name {}; LocDev sets those in the process. Check that the app reads them for its mail settings.",
            used.join(", ")
        ));
    } else {
        notes.push(format!(
            "{} reads its SMTP settings from code; add the snippet (development only).",
            input.framework
        ));
    }
    let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
    Plan {
        setup: MailSetup {
            site: input.site.into(),
            framework: input.framework.into(),
            enabled: false,
            wiring: if automatic {
                MailWiring::Automatic
            } else {
                MailWiring::Manual
            },
            current: current(input.env, &keys),
            real_smtp: real,
            injects: shown(&env),
            snippet: Some(snippet),
            notes,
        },
        env,
    }
}
