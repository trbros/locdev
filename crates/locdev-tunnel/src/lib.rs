//! Tunnel providers: the program that shares a local site on the internet,
//! where it comes from and how it's run. LocDev never relays traffic
//! itself; it downloads the provider's own program (verified), starts it
//! for one site, and reads the public address back.
//!
//! Only Cloudflare's Quick Tunnels for now: free, no account, no domain.

pub mod cloudflare;

/// `name@example.com` or `*@example.com`, as `--allowed-mail` takes them.
pub fn valid_email(value: &str) -> bool {
    let Some((user, domain)) = value.split_once('@') else {
        return false;
    };
    let user_ok = user == "*"
        || (!user.is_empty()
            && user.len() <= 64
            && user
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._%+-".contains(c)));
    let domain_ok = domain.len() <= 253
        && domain.contains('.')
        && domain.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    user_ok && domain_ok
}

/// `2026.10.7` ≥ `2026.9.2`.
pub fn version_at_least(version: &str, min: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter(|p| !p.is_empty())
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parse(version) >= parse(min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails() {
        assert!(valid_email("ana@example.com"));
        assert!(valid_email("*@example.com.br"));
        assert!(valid_email("a.b+c@sub.example.org"));
        assert!(!valid_email("example.com"));
        assert!(!valid_email("ana@localhost"));
        assert!(!valid_email("ana@exa mple.com"));
        assert!(!valid_email("--url@example.com,evil"));
        assert!(!valid_email("*@*"));
    }

    #[test]
    fn versions() {
        assert!(version_at_least("2026.10.7", "2026.9.2"));
        assert!(version_at_least("2026.9.2", "2026.9.2"));
        assert!(!version_at_least("2026.8.3", "2026.9.2"));
        assert!(!version_at_least("2025.12.0", "2026.9.2"));
    }
}
