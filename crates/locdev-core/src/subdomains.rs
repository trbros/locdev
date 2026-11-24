//! Per-site subdomains: which `<sub>.<site>.test` names reach the site, and
//! where specific ones go instead (another local port, an auxiliary process).
//!
//! A pattern is the part before the site's domain: `api`, `admin.eu`, or a
//! leading `*` (`*.tenants`) that stands for one or more labels. Exact
//! patterns win over wildcard ones; among wildcards, the longer suffix wins.

use serde::{Deserialize, Serialize};

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subdomains {
    /// Unlisted subdomains go to the site (`*.shop.test`). On by default,
    /// which is how every site behaved before rules existed.
    #[serde(default = "yes")]
    pub wildcard: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<SubdomainRule>,
}

impl Default for Subdomains {
    fn default() -> Self {
        Self {
            wildcard: true,
            rules: Vec::new(),
        }
    }
}

impl Subdomains {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubdomainRule {
    pub pattern: String,
    pub target: SubdomainTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubdomainTarget {
    /// The site's own dev server (useful with the wildcard off).
    Site,
    /// Another server on this machine (127.0.0.1).
    Port { port: u16 },
    /// One of the site's auxiliary processes, on the port LocDev gave it.
    Process { id: String },
}

/// How one hostname is handled, step by step (`locdev route <host>`).
/// DNS and proxy routing are separate: both must work for a browser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostExplain {
    pub host: String,
    pub dns: HostDns,
    pub route: HostRoute,
    pub tls: HostTls,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HostDns {
    /// Addresses the OS resolver returned.
    pub answers: Vec<String>,
    /// LocDev's DNS server was the one asked (`None`: it isn't running).
    #[serde(default)]
    pub via_locdev: Option<bool>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HostRoute {
    #[serde(default)]
    pub site: Option<String>,
    /// The part before the site's domain.
    pub sub: String,
    /// The rule that matched (`*.tenants`), if any.
    #[serde(default)]
    pub rule: Option<String>,
    /// Where it goes, in words (`the site`, `port 9000`, `the reverb process`).
    pub target: String,
    #[serde(default)]
    pub port: Option<u16>,
    /// Something answered on that port just now.
    #[serde(default)]
    pub reachable: Option<bool>,
    /// Why it isn't routed, when it isn't.
    #[serde(default)]
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HostTls {
    /// Names the certificate for this host covers.
    pub names: Vec<String>,
    #[serde(default)]
    pub https_port: Option<u16>,
    pub trusted: bool,
}

fn label_ok(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !label.starts_with('-')
        && !label.ends_with('-')
}

/// Lower-cases and checks a pattern (`API` → `api`, `*.Tenants` → `*.tenants`).
pub fn normalize(pattern: &str) -> Result<String, String> {
    let pattern = pattern.trim().trim_end_matches('.').to_ascii_lowercase();
    if pattern == "*" {
        return Err("`*` alone is the wildcard setting; use `*.<name>` for a wildcard rule".into());
    }
    let labels: Vec<&str> = pattern.split('.').collect();
    for (i, label) in labels.iter().enumerate() {
        if *label == "*" && i == 0 {
            continue;
        }
        if !label_ok(label) {
            return Err(format!(
                "`{pattern}` isn't a subdomain: use letters, digits and hyphens, with `*` only as the first label"
            ));
        }
    }
    if pattern.len() > 200 {
        return Err("the subdomain is too long".into());
    }
    Ok(pattern)
}

/// Whether `pattern` matches `sub` (both lower-case, `sub` without the site).
pub fn matches(pattern: &str, sub: &str) -> bool {
    match pattern.strip_prefix("*.") {
        // `*` is one or more labels: `a.tenants` and `a.b.tenants`, not `tenants`.
        Some(rest) => {
            sub.len() > rest.len() + 1
                && sub.ends_with(rest)
                && sub.as_bytes()[sub.len() - rest.len() - 1] == b'.'
        }
        None => pattern == sub,
    }
}

/// How specific a pattern is: exact beats wildcard, then more labels.
fn rank(pattern: &str) -> (bool, usize) {
    (!pattern.starts_with("*."), pattern.split('.').count())
}

/// The rule that handles `sub`, if any.
pub fn best<'a>(rules: &'a [SubdomainRule], sub: &str) -> Option<&'a SubdomainRule> {
    rules
        .iter()
        .filter(|r| matches(&r.pattern, sub))
        .max_by_key(|r| rank(&r.pattern))
}

/// Validates a whole configuration: patterns, ports, no duplicates.
pub fn validate(subdomains: &Subdomains) -> Result<Subdomains, String> {
    let mut out = Subdomains {
        wildcard: subdomains.wildcard,
        rules: Vec::new(),
    };
    for rule in &subdomains.rules {
        let pattern = normalize(&rule.pattern)?;
        if out.rules.iter().any(|r| r.pattern == pattern) {
            return Err(format!("`{pattern}` is listed twice"));
        }
        match &rule.target {
            SubdomainTarget::Port { port: 0 } => {
                return Err("port must be between 1 and 65535".into());
            }
            SubdomainTarget::Process { id } if id.trim().is_empty() => {
                return Err("name the process".into());
            }
            _ => {}
        }
        out.rules.push(SubdomainRule {
            pattern,
            target: rule.target.clone(),
        });
    }
    if out.rules.len() > 64 {
        return Err("at most 64 subdomain rules per site".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(pattern: &str, port: u16) -> SubdomainRule {
        SubdomainRule {
            pattern: pattern.into(),
            target: SubdomainTarget::Port { port },
        }
    }

    #[test]
    fn patterns_are_checked_and_normalized() {
        assert_eq!(normalize(" API. ").unwrap(), "api");
        assert_eq!(normalize("*.Tenants").unwrap(), "*.tenants");
        assert_eq!(normalize("admin.eu").unwrap(), "admin.eu");
        for bad in ["*", "a.*", "-x", "x_y", "", "a..b", "é"] {
            assert!(normalize(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn wildcards_match_one_or_more_labels() {
        assert!(matches("*.tenants", "acme.tenants"));
        assert!(matches("*.tenants", "a.b.tenants"));
        assert!(!matches("*.tenants", "tenants"));
        assert!(!matches("*.tenants", "xtenants"));
        assert!(matches("api", "api"));
        assert!(!matches("api", "v2.api"));
    }

    #[test]
    fn the_most_specific_rule_wins() {
        let rules = vec![
            rule("*.tenants", 1),
            rule("vip.tenants", 2),
            rule("*.eu.tenants", 3),
        ];
        let port = |sub: &str| match best(&rules, sub).map(|r| &r.target) {
            Some(SubdomainTarget::Port { port }) => Some(*port),
            _ => None,
        };
        assert_eq!(port("acme.tenants"), Some(1));
        assert_eq!(port("vip.tenants"), Some(2));
        assert_eq!(port("acme.eu.tenants"), Some(3));
        assert_eq!(port("api"), None);
    }

    #[test]
    fn duplicates_and_bad_targets_are_refused() {
        let dup = Subdomains {
            wildcard: true,
            rules: vec![rule("api", 1), rule("API", 2)],
        };
        assert!(validate(&dup).unwrap_err().contains("twice"));
        let zero = Subdomains {
            wildcard: true,
            rules: vec![rule("api", 0)],
        };
        assert!(validate(&zero).is_err());
    }
}
