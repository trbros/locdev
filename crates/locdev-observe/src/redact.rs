//! Hides secrets before anything is stored or shown: values that came from
//! the site's `.env` and LocDev's vault, and what looks like credentials
//! (auth headers, cookies, bearer tokens, JWTs, `password=`, URL
//! passwords, keys named like secrets in dumps and JSON).

use locdev_core::observe::{DumpNode, Event};

pub const MASK: &str = "[redacted]";

/// Key names that hold secrets (compared lowercase, `-` as `_`).
pub fn is_secret_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase().replace('-', "_");
    const PARTS: &[&str] = &[
        "password",
        "passwd",
        "secret",
        "token",
        "apikey",
        "api_key",
        "authorization",
        "cookie",
        "credential",
        "private_key",
        "privatekey",
        "session",
        "access_key",
        "signature",
    ];
    PARTS.iter().any(|p| k.contains(p))
        || k.ends_with("_key")
        || matches!(k.as_str(), "pwd" | "pass" | "auth")
}

pub struct Redactor {
    /// Known secret values, longest first so a shorter one inside a longer
    /// one doesn't leave a piece behind.
    values: Vec<String>,
}

/// Shorter values would hide ordinary words.
const MIN_SECRET_LEN: usize = 6;

impl Redactor {
    pub fn new(values: impl IntoIterator<Item = String>) -> Self {
        let mut values: Vec<String> = values
            .into_iter()
            .map(|v| v.trim().to_string())
            .filter(|v| v.len() >= MIN_SECRET_LEN)
            .collect();
        values.sort_by_key(|v| std::cmp::Reverse(v.len()));
        values.dedup();
        Self { values }
    }

    pub fn text(&self, s: &str) -> String {
        let mut out = s.to_string();
        for v in &self.values {
            if out.contains(v.as_str()) {
                out = out.replace(v.as_str(), MASK);
            }
        }
        let out = header_lines(&out);
        let out = after_markers(&out);
        let out = url_passwords(&out);
        let out = json_secret_values(&out);
        jwts(&out)
    }

    pub fn node(&self, node: &mut DumpNode) {
        match node {
            DumpNode::Str { v, .. } | DumpNode::Text { v } => *v = self.text(v),
            DumpNode::Array { items, .. }
            | DumpNode::Object { items, .. }
            | DumpNode::Resource { items, .. } => {
                for entry in items {
                    if is_secret_key(&entry.key) && !matches!(entry.value, DumpNode::Null) {
                        entry.value = DumpNode::Redacted;
                    } else {
                        self.node(&mut entry.value);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn event(&self, e: &mut Event) {
        e.message = self.text(&e.message);
        e.raw = self.text(&e.raw);
        for (k, v) in e.context.iter_mut() {
            *v = if is_secret_key(k) {
                MASK.into()
            } else {
                self.text(v)
            };
        }
        if let Some(r) = e.request.as_mut()
            && let Some(url) = r.url.as_mut()
        {
            *url = self.text(url);
        }
        if let Some(d) = e.dump.as_mut() {
            self.node(d);
        }
    }
}

fn find_ci(hay: &str, needle: &str, from: usize) -> Option<usize> {
    hay.get(from..)?
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
        .map(|i| i + from)
}

/// `Authorization: …`, `Cookie: …`, `Set-Cookie: …`, `X-Api-Key: …` lines.
fn header_lines(s: &str) -> String {
    s.split_inclusive('\n')
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            for h in [
                "authorization:",
                "cookie:",
                "set-cookie:",
                "x-api-key:",
                "proxy-authorization:",
            ] {
                if let Some(i) = lower.find(h) {
                    let end = line.ends_with('\n') as usize;
                    return format!(
                        "{} {MASK}{}",
                        &line[..i + h.len()],
                        if end == 1 { "\n" } else { "" }
                    );
                }
            }
            line.to_string()
        })
        .collect()
}

/// `Bearer abc`, `password=abc`, `token: abc`… up to a delimiter.
fn after_markers(s: &str) -> String {
    let mut out = s.to_string();
    for marker in [
        "bearer ",
        "password=",
        "passwd=",
        "pwd=",
        "token=",
        "secret=",
        "api_key=",
        "apikey=",
        "access_token=",
        "client_secret=",
        "password: ",
        "token: ",
    ] {
        let mut from = 0;
        while let Some(i) = find_ci(&out, marker, from) {
            let start = i + marker.len();
            let end = out[start..]
                .find(|c: char| {
                    c.is_whitespace() || matches!(c, '&' | ',' | ';' | '"' | '\'' | ')' | '}' | ']')
                })
                .map_or(out.len(), |e| start + e);
            if end > start && &out[start..end] != MASK {
                out.replace_range(start..end, MASK);
            }
            from = start + MASK.len().min(out.len() - start);
        }
    }
    out
}

/// `scheme://user:password@host` → `scheme://user:[redacted]@host`.
fn url_passwords(s: &str) -> String {
    let mut out = s.to_string();
    let mut from = 0;
    while let Some(i) = out[from..].find("://").map(|i| i + from) {
        let start = i + 3;
        let end = out[start..]
            .find(|c: char| c.is_whitespace() || c == '/' || c == '"' || c == '\'')
            .map_or(out.len(), |e| start + e);
        let authority = &out[start..end];
        if let Some(at) = authority.rfind('@')
            && let Some(colon) = authority[..at].find(':')
        {
            let (a, b) = (start + colon + 1, start + at);
            if &out[a..b] != MASK {
                out.replace_range(a..b, MASK);
            }
        }
        from = start;
    }
    out
}

/// `"password":"x"` / `"api_token": "x"` in JSON-ish text.
fn json_secret_values(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(q) = rest.find('"') {
        out.push_str(&rest[..=q]);
        rest = &rest[q + 1..];
        let Some(close) = rest.find('"') else { break };
        let key = &rest[..close];
        out.push_str(&rest[..=close]);
        rest = &rest[close + 1..];
        let after = rest.trim_start();
        if is_secret_key(key)
            && let Some(v) = after.strip_prefix(':')
            && let Some(v) = v.trim_start().strip_prefix('"')
            && let Some(end) = v.find('"')
        {
            let skipped = rest.len() - v.len();
            out.push_str(&rest[..skipped]);
            out.push_str(MASK);
            out.push('"');
            rest = &v[end + 1..];
        }
    }
    out.push_str(rest);
    out
}

/// `eyJ….….…` (JSON Web Tokens).
fn jwts(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("eyJ") {
        out.push_str(&rest[..i]);
        let token_len = rest[i..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
            .unwrap_or(rest.len() - i);
        let token = &rest[i..i + token_len];
        if token.matches('.').count() == 2 && token.len() > 20 {
            out.push_str(MASK);
        } else {
            out.push_str(token);
        }
        rest = &rest[i + token_len..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use locdev_core::observe::{DumpEntry, KeyKind};

    #[test]
    fn known_values_and_credentials_are_hidden() {
        let r = Redactor::new(["s3cr3t-db-pass".to_string(), "abc".to_string()]);
        let out = r.text(
            "connect failed with s3cr3t-db-pass\nAuthorization: Bearer xyz.123\nGET /cb?code=1&access_token=AAA&x=2\npostgres://app:hunter2@localhost/db abc",
        );
        assert!(!out.contains("s3cr3t-db-pass"));
        assert!(out.contains("Authorization: [redacted]"));
        assert!(!out.contains("AAA") && out.contains("x=2"));
        assert!(out.contains("postgres://app:[redacted]@localhost/db"));
        assert!(out.ends_with("abc"), "values under 6 characters stay");
    }

    #[test]
    fn json_and_jwts() {
        let r = Redactor::new([]);
        let out = r.text(r#"{"email":"a@b.c","password":"pw1234","api_token": "tok"} eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.c2lnbmF0dXJlLXRleHQ"#);
        assert!(out.contains(r#""email":"a@b.c""#));
        assert!(out.contains(r#""password":"[redacted]""#));
        assert!(out.contains(r#""api_token": "[redacted]""#));
        assert!(!out.contains("eyJhbGci"));
    }

    #[test]
    fn dump_keys_named_like_secrets() {
        let r = Redactor::new(["k-123456".to_string()]);
        let mut node = DumpNode::Array {
            count: 3,
            cut: 0,
            items: vec![
                DumpEntry {
                    key: "token".into(),
                    kind: KeyKind::Key,
                    value: DumpNode::Str {
                        v: "abc".into(),
                        cut: 0,
                        binary: false,
                    },
                },
                DumpEntry {
                    key: "name".into(),
                    kind: KeyKind::Key,
                    value: DumpNode::Str {
                        v: "Ana".into(),
                        cut: 0,
                        binary: false,
                    },
                },
                DumpEntry {
                    key: "note".into(),
                    kind: KeyKind::Key,
                    value: DumpNode::Str {
                        v: "key k-123456".into(),
                        cut: 0,
                        binary: false,
                    },
                },
            ],
        };
        r.node(&mut node);
        let DumpNode::Array { items, .. } = node else {
            panic!()
        };
        assert_eq!(items[0].value, DumpNode::Redacted);
        assert_eq!(
            items[1].value,
            DumpNode::Str {
                v: "Ana".into(),
                cut: 0,
                binary: false
            }
        );
        assert_eq!(
            items[2].value,
            DumpNode::Str {
                v: "key [redacted]".into(),
                cut: 0,
                binary: false
            }
        );
    }

    #[test]
    fn secret_key_names() {
        for k in [
            "password",
            "DB_PASSWORD",
            "api-key",
            "APP_KEY",
            "remember_token",
            "Cookie",
            "pwd",
        ] {
            assert!(is_secret_key(k), "{k}");
        }
        for k in ["name", "email", "monkey", "keyboard", "id"] {
            assert!(!is_secret_key(k), "{k}");
        }
    }
}
