//! Package metadata from the npm registry and Packagist, and advisories from
//! OSV (osv.dev, which aggregates GitHub's advisory database among others).
//! Blocking calls; the daemon runs them off its async threads, with its own
//! cache, rate limit and backoff around them.

use std::time::Duration;

use locdev_core::deps::{Advisory, Ecosystem, PackageLinks};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const NPM: &str = "https://registry.npmjs.org";
const PACKAGIST: &str = "https://repo.packagist.org/p2";
const OSV: &str = "https://api.osv.dev/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// Offline, DNS, TLS, timeouts, 5xx: worth retrying later.
    Network(String),
    /// 429 or 403 with a rate-limit hint; seconds to wait, when given.
    RateLimited(Option<u64>),
    NotFound,
    Invalid(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::RateLimited(Some(s)) => write!(f, "rate limited; retry in {s}s"),
            Self::RateLimited(None) => write!(f, "rate limited"),
            Self::NotFound => write!(f, "not found in the registry"),
            Self::Invalid(e) => write!(f, "unexpected registry response: {e}"),
        }
    }
}

impl FetchError {
    /// Whether trying again later could succeed.
    pub fn transient(&self) -> bool {
        matches!(self, Self::Network(_) | Self::RateLimited(_))
    }
}

/// One published version, reduced to what compatibility checks need.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub deprecated: Option<String>,
    /// npm `engines.node`.
    pub node: Option<String>,
    /// Composer `require.php`.
    pub php: Option<String>,
    /// npm peer dependencies: name, range, optional.
    pub peers: Vec<(String, String, bool)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub ecosystem: Ecosystem,
    pub versions: Vec<VersionInfo>,
    /// The registry's own "latest" (npm dist-tag), if any.
    pub latest_tag: Option<String>,
    pub links: PackageLinks,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .user_agent(concat!(
            "locdev/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/trbros/locdev)"
        ))
        .build()
        .into()
}

fn get_json(url: &str, accept: &str) -> Result<Value, FetchError> {
    let response = agent()
        .get(url)
        .header("Accept", accept)
        .call()
        .map_err(|e| FetchError::Network(format!("{url}: {e}")))?;
    read_json(url, response)
}

fn read_json(
    url: &str,
    mut response: ureq::http::Response<ureq::Body>,
) -> Result<Value, FetchError> {
    let status = response.status().as_u16();
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());
    match status {
        200..=299 => response
            .body_mut()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_json()
            .map_err(|e| FetchError::Invalid(format!("{url}: {e}"))),
        404 => Err(FetchError::NotFound),
        429 => Err(FetchError::RateLimited(retry_after)),
        403 if retry_after.is_some() => Err(FetchError::RateLimited(retry_after)),
        500..=599 => Err(FetchError::Network(format!("{url}: HTTP {status}"))),
        other => Err(FetchError::Invalid(format!("{url}: HTTP {other}"))),
    }
}

/// Registry names are validated before they reach a URL.
fn valid_name(ecosystem: Ecosystem, name: &str) -> bool {
    let ok = |c: char| c.is_ascii_alphanumeric() || "-_.".contains(c);
    match ecosystem {
        Ecosystem::Npm => {
            let bare = name.strip_prefix('@').unwrap_or(name);
            let parts: Vec<&str> = bare.split('/').collect();
            !name.is_empty()
                && name.len() <= 214
                && parts.len() == if name.starts_with('@') { 2 } else { 1 }
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && !p.starts_with('.') && p.chars().all(ok))
        }
        Ecosystem::Composer => {
            let parts: Vec<&str> = name.split('/').collect();
            parts.len() == 2 && parts.iter().all(|p| !p.is_empty() && p.chars().all(ok))
        }
    }
}

pub fn fetch(ecosystem: Ecosystem, name: &str) -> Result<PackageInfo, FetchError> {
    if !valid_name(ecosystem, name) {
        return Err(FetchError::Invalid(format!(
            "'{name}' is not a package name"
        )));
    }
    match ecosystem {
        Ecosystem::Npm => {
            let url = format!("{NPM}/{}", name.replace('/', "%2f"));
            npm_info(
                name,
                &get_json(&url, "application/vnd.npm.install-v1+json")?,
            )
        }
        Ecosystem::Composer => {
            let url = format!("{PACKAGIST}/{name}.json");
            packagist_info(name, &get_json(&url, "application/json")?)
        }
    }
}

pub fn npm_info(name: &str, doc: &Value) -> Result<PackageInfo, FetchError> {
    let versions = doc["versions"]
        .as_object()
        .ok_or_else(|| FetchError::Invalid(format!("{name}: no versions")))?
        .iter()
        .map(|(version, v)| VersionInfo {
            version: version.clone(),
            deprecated: v["deprecated"].as_str().map(str::to_string),
            node: v["engines"]["node"].as_str().map(str::to_string),
            php: None,
            peers: v["peerDependencies"]
                .as_object()
                .into_iter()
                .flatten()
                .filter_map(|(peer, range)| {
                    let optional = v["peerDependenciesMeta"][peer]["optional"]
                        .as_bool()
                        .unwrap_or(false);
                    Some((peer.clone(), range.as_str()?.to_string(), optional))
                })
                .collect(),
        })
        .collect();
    Ok(PackageInfo {
        name: name.to_string(),
        ecosystem: Ecosystem::Npm,
        versions,
        latest_tag: doc["dist-tags"]["latest"].as_str().map(str::to_string),
        links: PackageLinks {
            registry: format!("https://www.npmjs.com/package/{name}"),
            ..Default::default()
        },
    })
}

/// Links for one npm version (its full document has `repository`).
pub fn npm_links(name: &str, version: &str) -> Result<PackageLinks, FetchError> {
    if !valid_name(Ecosystem::Npm, name) {
        return Err(FetchError::Invalid(format!(
            "'{name}' is not a package name"
        )));
    }
    let doc = get_json(
        &format!("{NPM}/{}/{}", name.replace('/', "%2f"), urlencode(version)),
        "application/json",
    )?;
    let repository = doc["repository"]["url"]
        .as_str()
        .or(doc["repository"].as_str())
        .and_then(repo_url);
    Ok(PackageLinks {
        registry: format!("https://www.npmjs.com/package/{name}"),
        releases: repository.as_deref().and_then(releases_url),
        repository,
        homepage: doc["homepage"].as_str().map(str::to_string),
    })
}

/// Packagist's p2 format is "minified": each version only lists what
/// changed since the previous one, and `"__unset"` removes a key.
pub fn packagist_info(name: &str, doc: &Value) -> Result<PackageInfo, FetchError> {
    let list = doc["packages"][name]
        .as_array()
        .ok_or_else(|| FetchError::Invalid(format!("{name}: no versions")))?;
    let mut current = serde_json::Map::new();
    let mut versions = Vec::new();
    let mut links = PackageLinks {
        registry: format!("https://packagist.org/packages/{name}"),
        ..Default::default()
    };
    for (i, entry) in list.iter().enumerate() {
        for (key, value) in entry.as_object().into_iter().flatten() {
            if value.as_str() == Some("__unset") {
                current.remove(key);
            } else {
                current.insert(key.clone(), value.clone());
            }
        }
        let Some(version) = current.get("version").and_then(Value::as_str) else {
            continue;
        };
        if i == 0 {
            // The newest version carries the current links.
            let repository = current
                .get("source")
                .and_then(|s| s["url"].as_str())
                .and_then(repo_url);
            links.releases = repository.as_deref().and_then(releases_url);
            links.repository = repository;
            links.homepage = current
                .get("homepage")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        versions.push(VersionInfo {
            version: version.trim_start_matches('v').to_string(),
            deprecated: current.get("abandoned").and_then(|a| match a {
                Value::Bool(true) => Some("abandoned".to_string()),
                Value::String(by) => Some(format!("abandoned; use {by}")),
                _ => None,
            }),
            node: None,
            php: current
                .get("require")
                .and_then(|r| r["php"].as_str())
                .map(str::to_string),
            peers: Vec::new(),
        });
    }
    Ok(PackageInfo {
        name: name.to_string(),
        ecosystem: Ecosystem::Composer,
        versions,
        latest_tag: None,
        links,
    })
}

/// `git+https://github.com/x/y.git`, `git@github.com:x/y.git`, `github:x/y`
/// → `https://github.com/x/y`. Anything not https-reachable is dropped.
pub fn repo_url(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let url = if let Some(rest) = raw.strip_prefix("github:") {
        format!("https://github.com/{rest}")
    } else if let Some(rest) = raw.strip_prefix("git@") {
        format!("https://{}", rest.replacen(':', "/", 1))
    } else {
        raw.trim_start_matches("git+")
            .replacen("git://", "https://", 1)
            .replacen("ssh://git@", "https://", 1)
    };
    let url = url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_string();
    (url.starts_with("https://") && !url.contains(' ')).then_some(url)
}

/// The releases page of a GitHub or GitLab repository.
pub fn releases_url(repository: &str) -> Option<String> {
    if repository.starts_with("https://github.com/") {
        Some(format!("{repository}/releases"))
    } else if repository.starts_with("https://gitlab.com/") {
        Some(format!("{repository}/-/releases"))
    } else {
        None
    }
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '.' | '_' | '~' => c.to_string(),
            other => format!("%{:02X}", other as u32),
        })
        .collect()
}

/// Known advisories for each `(ecosystem, name, version)`, in one batch
/// request plus one detail request per distinct advisory.
pub fn advisories(
    queries: &[(Ecosystem, String, String)],
) -> Result<Vec<Vec<Advisory>>, FetchError> {
    if queries.is_empty() {
        return Ok(Vec::new());
    }
    let body = serde_json::json!({
        "queries": queries.iter().map(|(eco, name, version)| serde_json::json!({
            "package": { "name": name, "ecosystem": match eco { Ecosystem::Npm => "npm", Ecosystem::Composer => "Packagist" } },
            "version": version,
        })).collect::<Vec<_>>()
    });
    let url = format!("{OSV}/querybatch");
    let response = agent()
        .post(&url)
        .header("Content-Type", "application/json")
        .send(body.to_string())
        .map_err(|e| FetchError::Network(format!("{url}: {e}")))?;
    let batch = read_json(&url, response)?;

    let mut details: std::collections::HashMap<String, Advisory> = std::collections::HashMap::new();
    let mut out = Vec::new();
    for (i, result) in batch["results"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let mut found = Vec::new();
        for vuln in result["vulns"].as_array().into_iter().flatten() {
            let Some(id) = vuln["id"].as_str() else {
                continue;
            };
            if !details.contains_key(id) {
                let detail = get_json(
                    &format!("{OSV}/vulns/{}", urlencode(id)),
                    "application/json",
                )?;
                details.insert(id.to_string(), advisory(id, &detail, &queries[i].1));
            }
            found.push(details[id].clone());
        }
        out.push(found);
    }
    out.resize(queries.len(), Vec::new());
    Ok(out)
}

fn advisory(id: &str, detail: &Value, package: &str) -> Advisory {
    let fixed = detail["affected"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["package"]["name"].as_str() == Some(package))
        .flat_map(|a| a["ranges"].as_array().into_iter().flatten())
        .flat_map(|r| r["events"].as_array().into_iter().flatten())
        .find_map(|e| e["fixed"].as_str().map(str::to_string));
    Advisory {
        id: id.to_string(),
        summary: detail["summary"]
            .as_str()
            .or(detail["details"].as_str())
            .unwrap_or("no summary")
            .chars()
            .take(240)
            .collect(),
        severity: detail["database_specific"]["severity"]
            .as_str()
            .map(str::to_string),
        url: format!("https://osv.dev/vulnerability/{id}"),
        fixed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_names_before_building_urls() {
        for ok in ["react", "@vitejs/plugin-react", "lodash.merge"] {
            assert!(valid_name(Ecosystem::Npm, ok), "{ok}");
        }
        for bad in ["", "../x", "@scope", "a/b", ".hidden", "a b"] {
            assert!(!valid_name(Ecosystem::Npm, bad), "{bad}");
        }
        assert!(valid_name(Ecosystem::Composer, "laravel/framework"));
        assert!(!valid_name(Ecosystem::Composer, "laravel"));
        assert!(!valid_name(Ecosystem::Composer, "a/b/c"));
    }

    #[test]
    fn reads_abbreviated_npm_metadata() {
        let doc = serde_json::json!({
            "dist-tags": { "latest": "2.0.0" },
            "versions": {
                "1.0.0": { "engines": { "node": ">=14" } },
                "2.0.0": {
                    "engines": { "node": ">=20" },
                    "peerDependencies": { "react": "^19.0.0", "react-dom": "^19.0.0" },
                    "peerDependenciesMeta": { "react-dom": { "optional": true } }
                },
                "0.9.0": { "deprecated": "use 1.x" }
            }
        });
        let info = npm_info("lib", &doc).unwrap();
        assert_eq!(info.latest_tag.as_deref(), Some("2.0.0"));
        let v2 = info.versions.iter().find(|v| v.version == "2.0.0").unwrap();
        assert_eq!(v2.node.as_deref(), Some(">=20"));
        assert!(
            v2.peers
                .contains(&("react-dom".into(), "^19.0.0".into(), true))
        );
        assert!(
            info.versions
                .iter()
                .any(|v| v.deprecated.as_deref() == Some("use 1.x"))
        );
    }

    #[test]
    fn expands_minified_packagist_metadata() {
        let doc = serde_json::json!({ "packages": { "acme/lib": [
            { "version": "v2.1.0", "require": { "php": "^8.2" }, "source": { "url": "https://github.com/acme/lib.git" }, "homepage": "https://acme.dev" },
            { "version": "v2.0.0" },
            { "version": "v1.5.0", "require": { "php": "^7.4" }, "homepage": "__unset" }
        ] } });
        let info = packagist_info("acme/lib", &doc).unwrap();
        let php: Vec<_> = info
            .versions
            .iter()
            .map(|v| (v.version.as_str(), v.php.as_deref()))
            .collect();
        assert_eq!(
            php,
            vec![
                ("2.1.0", Some("^8.2")),
                ("2.0.0", Some("^8.2")),
                ("1.5.0", Some("^7.4"))
            ]
        );
        assert_eq!(
            info.links.repository.as_deref(),
            Some("https://github.com/acme/lib")
        );
        assert_eq!(
            info.links.releases.as_deref(),
            Some("https://github.com/acme/lib/releases")
        );
    }

    #[test]
    fn normalizes_repository_urls() {
        assert_eq!(
            repo_url("git+https://github.com/vitejs/vite.git").as_deref(),
            Some("https://github.com/vitejs/vite")
        );
        assert_eq!(
            repo_url("git@github.com:x/y.git").as_deref(),
            Some("https://github.com/x/y")
        );
        assert_eq!(
            repo_url("github:x/y").as_deref(),
            Some("https://github.com/x/y")
        );
        assert_eq!(
            repo_url("git://github.com/x/y").as_deref(),
            Some("https://github.com/x/y")
        );
        assert_eq!(repo_url("javascript:alert(1)"), None);
        assert_eq!(
            releases_url("https://gitlab.com/a/b").as_deref(),
            Some("https://gitlab.com/a/b/-/releases")
        );
        assert_eq!(releases_url("https://example.com/a"), None);
    }
}
