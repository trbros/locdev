//! Mapping `Host` headers to project names.

/// Local suffixes the proxy answers for. `.localhost` resolves to loopback in
/// every major browser without any DNS setup, so it works before `.test` does.
pub const SUFFIXES: [&str; 2] = ["test", "localhost"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target<'a> {
    /// `<name>.test`, `<sub>.<name>.test`, `<name>.localhost`, ...
    Site {
        name: &'a str,
        suffix: &'a str,
        /// What comes before the site (`api`, `acme.tenants`); empty for the site itself.
        sub: &'a str,
    },
    /// A bare loopback host (`localhost`, `127.0.0.1`, `[::1]`).
    Index,
    Unknown,
}

/// Removes a trailing `:port` (also from bracketed IPv6) and a trailing dot.
pub fn strip_port(host: &str) -> &str {
    let host = match host.rsplit_once(':') {
        Some((head, port)) if port.bytes().all(|b| b.is_ascii_digit()) && !head.ends_with(':') => {
            // An unbracketed IPv6 address (`::1`) has no port to strip.
            if head.contains(':') && !head.starts_with('[') {
                host
            } else {
                head
            }
        }
        _ => host,
    };
    host.trim_end_matches('.')
}

/// Classifies an already lower-cased host.
pub fn target(host: &str) -> Target<'_> {
    let host = strip_port(host);

    if matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1") {
        return Target::Index;
    }

    let Some((rest, suffix)) = host.rsplit_once('.') else {
        return Target::Unknown;
    };
    let (sub, name) = rest.rsplit_once('.').unwrap_or(("", rest));

    match SUFFIXES.iter().find(|s| **s == suffix) {
        Some(_) if !name.is_empty() && (sub.is_empty() || !sub.split('.').any(str::is_empty)) => {
            Target::Site { name, suffix, sub }
        }
        _ => Target::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sites() {
        assert_eq!(
            target("blog.test"),
            Target::Site {
                name: "blog",
                suffix: "test",
                sub: "",
            }
        );
        assert_eq!(
            target("blog.test:7080"),
            Target::Site {
                name: "blog",
                suffix: "test",
                sub: "",
            }
        );
        assert_eq!(
            target("api.blog.localhost."),
            Target::Site {
                name: "blog",
                suffix: "localhost",
                sub: "api",
            }
        );
        assert_eq!(
            target("a.b.blog.test"),
            Target::Site {
                name: "blog",
                suffix: "test",
                sub: "a.b",
            }
        );
        assert_eq!(target("a..blog.test"), Target::Unknown);
    }

    #[test]
    fn index_and_unknown() {
        assert_eq!(target("localhost:7080"), Target::Index);
        assert_eq!(target("127.0.0.1"), Target::Index);
        assert_eq!(target("[::1]:80"), Target::Index);
        assert_eq!(target("::1"), Target::Index);
        assert_eq!(target("example.com"), Target::Unknown);
        assert_eq!(target(".test"), Target::Unknown);
        assert_eq!(target("test"), Target::Unknown);
    }
}
