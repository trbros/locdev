//! Package search for the "add packages" step: the registries' own search
//! APIs (`registry.npmjs.org/-/v1/search`, `packagist.org/search.json`).
//! The daemon calls this through its request limiter and never offline.

use std::time::Duration;

use locdev_core::scaffold::{PackageHit, PackageRegistry};
use serde_json::Value;

const NPM: &str = "https://registry.npmjs.org/-/v1/search";
const PACKAGIST: &str = "https://packagist.org/search.json";

pub fn search(registry: PackageRegistry, query: &str) -> Result<Vec<PackageHit>, String> {
    let query = query.trim();
    if query.is_empty() || query.len() > 100 || query.chars().any(char::is_control) {
        return Err("type 1 to 100 characters to search".into());
    }
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .user_agent(concat!("LocDev/", env!("CARGO_PKG_VERSION")))
        .build()
        .new_agent();
    let (url, key, size) = match registry {
        PackageRegistry::Npm => (NPM, "text", "size"),
        PackageRegistry::Packagist => (PACKAGIST, "q", "per_page"),
    };
    let doc: Value = agent
        .get(url)
        .query(key, query)
        .query(size, "20")
        .call()
        .map_err(|e| format!("{url}: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| format!("{url}: {e}"))?;
    Ok(match registry {
        PackageRegistry::Npm => parse_npm(&doc),
        PackageRegistry::Packagist => parse_packagist(&doc),
    })
}

pub fn parse_npm(doc: &Value) -> Vec<PackageHit> {
    doc["objects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|o| {
            let p = &o["package"];
            Some(PackageHit {
                name: p["name"].as_str()?.to_string(),
                description: p["description"].as_str().map(str::to_string),
                version: p["version"].as_str().map(str::to_string),
                downloads: o["downloads"]["monthly"].as_u64(),
                url: p["links"]["npm"].as_str().map(str::to_string),
            })
        })
        .collect()
}

pub fn parse_packagist(doc: &Value) -> Vec<PackageHit> {
    doc["results"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["virtual"].as_bool() != Some(true))
        .filter_map(|r| {
            Some(PackageHit {
                name: r["name"].as_str()?.to_string(),
                description: r["description"].as_str().map(str::to_string),
                version: None,
                downloads: r["downloads"].as_u64(),
                url: r["url"].as_str().map(str::to_string),
            })
        })
        .collect()
}
