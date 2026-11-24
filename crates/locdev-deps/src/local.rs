//! What a project declares, what its lockfile resolved and what is
//! installed, read from disk only. Malformed files become errors in the
//! result, never panics, and never stop the other files from being read.

use std::collections::HashMap;
use std::path::Path;

use locdev_core::deps::{DepKind, Ecosystem};
use locdev_core::model::RuntimeKind;
use serde_json::Value;

/// Larger manifests or lockfiles are refused rather than read into memory.
const MAX_FILE: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    pub name: String,
    pub ecosystem: Ecosystem,
    pub kind: DepKind,
    pub constraint: String,
    pub locked: Option<String>,
    pub installed: Option<String>,
}

#[derive(Debug, Default)]
pub struct Local {
    pub packages: Vec<Declared>,
    pub manager: Option<RuntimeKind>,
    pub files: Vec<String>,
    pub errors: Vec<String>,
}

pub fn read(root: &Path) -> Local {
    let mut local = Local::default();
    read_npm(root, &mut local);
    read_composer(root, &mut local);
    local
}

fn load(root: &Path, file: &str, local: &mut Local) -> Option<String> {
    let path = root.join(file);
    let meta = std::fs::metadata(&path).ok()?;
    if meta.len() > MAX_FILE {
        local
            .errors
            .push(format!("{file} is larger than 64 MB; skipped"));
        return None;
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            local.files.push(file.to_string());
            Some(text)
        }
        Err(e) => {
            local.errors.push(format!("{file}: {e}"));
            None
        }
    }
}

fn json(text: &str, file: &str, local: &mut Local) -> Option<Value> {
    match serde_json::from_str(text) {
        Ok(v) => Some(v),
        Err(e) => {
            local.errors.push(format!("{file} is not valid JSON: {e}"));
            None
        }
    }
}

fn read_npm(root: &Path, local: &mut Local) {
    let Some(text) = load(root, "package.json", local) else {
        return;
    };
    let Some(manifest) = json(&text, "package.json", local) else {
        return;
    };

    let mut declared = Vec::new();
    for (section, kind) in [
        ("dependencies", DepKind::Prod),
        ("devDependencies", DepKind::Dev),
        ("optionalDependencies", DepKind::Optional),
        ("peerDependencies", DepKind::Peer),
    ] {
        for (name, constraint) in manifest[section].as_object().into_iter().flatten() {
            // A package listed twice keeps its first (most important) kind.
            if declared.iter().any(|d: &Declared| &d.name == name) {
                continue;
            }
            declared.push(Declared {
                name: name.clone(),
                ecosystem: Ecosystem::Npm,
                kind,
                constraint: constraint.as_str().unwrap_or_default().to_string(),
                locked: None,
                installed: None,
            });
        }
    }

    let (manager, locked) = npm_lock(root, &declared, local);
    local.manager = local.manager.or(manager).or(Some(RuntimeKind::Npm));
    for mut dep in declared {
        dep.locked = locked.get(&dep.name).cloned();
        dep.installed = installed_npm(root, &dep.name);
        local.packages.push(dep);
    }
}

/// Locked versions of the declared packages, from whichever lockfile exists.
fn npm_lock(
    root: &Path,
    declared: &[Declared],
    local: &mut Local,
) -> (Option<RuntimeKind>, HashMap<String, String>) {
    if root.join("pnpm-lock.yaml").is_file() {
        let map = load(root, "pnpm-lock.yaml", local)
            .map(|text| pnpm_lock(&text, local))
            .unwrap_or_default();
        return (Some(RuntimeKind::Pnpm), map);
    }
    if root.join("yarn.lock").is_file() {
        let map = load(root, "yarn.lock", local)
            .map(|text| yarn_lock(&text, declared))
            .unwrap_or_default();
        return (Some(RuntimeKind::Yarn), map);
    }
    if root.join("bun.lock").is_file() {
        let map = load(root, "bun.lock", local)
            .and_then(|text| json(&strip_trailing_commas(&text), "bun.lock", local))
            .map(|lock| bun_lock(&lock))
            .unwrap_or_default();
        return (Some(RuntimeKind::Bun), map);
    }
    if root.join("bun.lockb").is_file() {
        local.files.push("bun.lockb".into());
        local
            .errors
            .push("bun.lockb is binary; locked versions are read from node_modules only (bun ≥1.2 writes a text bun.lock)".into());
        return (Some(RuntimeKind::Bun), HashMap::new());
    }
    for file in ["package-lock.json", "npm-shrinkwrap.json"] {
        if root.join(file).is_file() {
            let map = load(root, file, local)
                .and_then(|text| json(&text, file, local))
                .map(|lock| package_lock(&lock))
                .unwrap_or_default();
            return (Some(RuntimeKind::Npm), map);
        }
    }
    (None, HashMap::new())
}

/// package-lock v2/v3 (`packages["node_modules/x"]`) or v1 (`dependencies`).
pub fn package_lock(lock: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Some(packages) = lock["packages"].as_object() {
        for (path, entry) in packages {
            // Only top-level installs: nested ones belong to other packages.
            if let Some(name) = path.strip_prefix("node_modules/")
                && !name.contains("/node_modules/")
                && let Some(version) = entry["version"].as_str()
            {
                out.insert(name.to_string(), version.to_string());
            }
        }
    } else if let Some(deps) = lock["dependencies"].as_object() {
        for (name, entry) in deps {
            if let Some(version) = entry["version"].as_str() {
                out.insert(name.clone(), version.to_string());
            }
        }
    }
    out
}

/// pnpm-lock.yaml: `importers["."]` (v6+) or the root maps (v5).
pub fn pnpm_lock(text: &str, local: &mut Local) -> HashMap<String, String> {
    let lock: serde_norway::Value = match serde_norway::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            local
                .errors
                .push(format!("pnpm-lock.yaml is not valid YAML: {e}"));
            return HashMap::new();
        }
    };
    let root = lock
        .get("importers")
        .and_then(|i| i.get("."))
        .unwrap_or(&lock);
    let mut out = HashMap::new();
    for section in ["dependencies", "devDependencies", "optionalDependencies"] {
        let Some(map) = root.get(section).and_then(|m| m.as_mapping()) else {
            continue;
        };
        for (name, entry) in map {
            let version = entry
                .get("version")
                .and_then(|v| v.as_str())
                .or_else(|| entry.as_str());
            if let (Some(name), Some(version)) = (name.as_str(), version) {
                // `1.2.3(react@18.2.0)`: peer context after the version.
                let version = version.split('(').next().unwrap_or(version);
                if !version.starts_with("link:") && !version.starts_with("file:") {
                    out.insert(name.to_string(), version.to_string());
                }
            }
        }
    }
    out
}

/// yarn.lock: classic (v1 text) or berry (YAML). Entries are keyed by the
/// specs that resolved to them, `name@range`.
pub fn yarn_lock(text: &str, declared: &[Declared]) -> HashMap<String, String> {
    let mut by_spec: HashMap<String, String> = HashMap::new();
    if text.contains("__metadata:") {
        if let Ok(serde_norway::Value::Mapping(map)) =
            serde_norway::from_str::<serde_norway::Value>(text)
        {
            for (key, entry) in &map {
                let (Some(key), Some(version)) =
                    (key.as_str(), entry.get("version").and_then(|v| v.as_str()))
                else {
                    continue;
                };
                for spec in key.split(", ") {
                    by_spec.insert(spec.replace("@npm:", "@"), version.to_string());
                }
            }
        }
    } else {
        let mut specs: Vec<String> = Vec::new();
        for line in text.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if !line.starts_with(' ') && line.ends_with(':') {
                specs = line
                    .trim_end_matches(':')
                    .split(", ")
                    .map(|s| s.trim_matches('"').to_string())
                    .collect();
            } else if let Some(version) = line.trim().strip_prefix("version ") {
                let version = version.trim_matches('"');
                for spec in specs.drain(..) {
                    by_spec.insert(spec, version.to_string());
                }
            }
        }
    }
    declared
        .iter()
        .filter_map(|d| {
            let version = by_spec.get(&format!("{}@{}", d.name, d.constraint))?;
            Some((d.name.clone(), version.clone()))
        })
        .collect()
}

/// bun.lock (text, JSON with trailing commas): `packages[name][0]` is
/// `name@version`.
pub fn bun_lock(lock: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for (key, entry) in lock["packages"].as_object().into_iter().flatten() {
        // Nested installs are keyed `parent/child`; keep top-level ones.
        if key.matches('/').count() > usize::from(key.starts_with('@')) {
            continue;
        }
        if let Some(resolved) = entry[0].as_str()
            && let Some((_, version)) = resolved.rsplit_once('@')
        {
            out.insert(key.clone(), version.to_string());
        }
    }
    out
}

/// Removes commas right before `}` or `]` outside strings (bun.lock is JSONC).
pub fn strip_trailing_commas(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if in_string {
            out.push(c);
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
            continue;
        }
        if c == '"' {
            in_string = true;
        }
        if c == ',' {
            let next = chars[i + 1..].iter().find(|c| !c.is_whitespace());
            if matches!(next, Some('}' | ']')) {
                continue;
            }
        }
        out.push(c);
    }
    out
}

fn installed_npm(root: &Path, name: &str) -> Option<String> {
    // Scoped names (`@scope/pkg`) are two path segments; anything else odd
    // (`..`, absolute) is not a package name.
    if name.contains("..") || name.starts_with('/') || name.contains('\\') {
        return None;
    }
    let raw =
        std::fs::read_to_string(root.join("node_modules").join(name).join("package.json")).ok()?;
    serde_json::from_str::<Value>(&raw).ok()?["version"]
        .as_str()
        .map(str::to_string)
}

fn read_composer(root: &Path, local: &mut Local) {
    let Some(text) = load(root, "composer.json", local) else {
        return;
    };
    let Some(manifest) = json(&text, "composer.json", local) else {
        return;
    };

    let locked = load(root, "composer.lock", local)
        .and_then(|text| json(&text, "composer.lock", local))
        .map(|lock| composer_lock(&lock))
        .unwrap_or_default();
    let installed = installed_composer(root);
    if local.manager.is_none() {
        local.manager = Some(RuntimeKind::Composer);
    }

    for (section, kind) in [("require", DepKind::Prod), ("require-dev", DepKind::Dev)] {
        for (name, constraint) in manifest[section].as_object().into_iter().flatten() {
            // Platform requirements (php, ext-*, lib-*) aren't packages.
            if !name.contains('/') {
                continue;
            }
            local.packages.push(Declared {
                name: name.clone(),
                ecosystem: Ecosystem::Composer,
                kind,
                constraint: constraint.as_str().unwrap_or_default().to_string(),
                locked: locked.get(name).cloned(),
                installed: installed.get(name).cloned(),
            });
        }
    }
}

/// composer.lock: `packages` and `packages-dev`, versions without `v`.
pub fn composer_lock(lock: &Value) -> HashMap<String, String> {
    ["packages", "packages-dev"]
        .iter()
        .flat_map(|section| lock[section].as_array().into_iter().flatten())
        .filter_map(|p| {
            Some((
                p["name"].as_str()?.to_string(),
                p["version"].as_str()?.trim_start_matches('v').to_string(),
            ))
        })
        .collect()
}

/// vendor/composer/installed.json: `{ "packages": [...] }` (Composer 2) or
/// a bare array (Composer 1).
fn installed_composer(root: &Path) -> HashMap<String, String> {
    let Ok(raw) = std::fs::read_to_string(root.join("vendor/composer/installed.json")) else {
        return HashMap::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return HashMap::new();
    };
    let list = value["packages"].as_array().or(value.as_array());
    list.into_iter()
        .flatten()
        .filter_map(|p| {
            Some((
                p["name"].as_str()?.to_string(),
                p["version"].as_str()?.trim_start_matches('v').to_string(),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, files: &[(&str, &str)]) {
        for (file, body) in files {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        }
    }

    const MANIFEST: &str = r#"{"dependencies":{"react":"^18.2.0","@vitejs/plugin-react":"^4.0.0"},"devDependencies":{"vite":"^5.0.0"}}"#;

    #[test]
    fn npm_lock_v3_and_installed() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            &[
                ("package.json", MANIFEST),
                (
                    "package-lock.json",
                    r#"{"lockfileVersion":3,"packages":{"":{},"node_modules/react":{"version":"18.3.1"},"node_modules/vite":{"version":"5.4.2"},"node_modules/vite/node_modules/esbuild":{"version":"0.21.0"},"node_modules/@vitejs/plugin-react":{"version":"4.3.1"}}}"#,
                ),
                ("node_modules/react/package.json", r#"{"version":"18.3.0"}"#),
            ],
        );
        let local = read(dir.path());
        assert_eq!(local.manager, Some(RuntimeKind::Npm));
        let react = local.packages.iter().find(|p| p.name == "react").unwrap();
        assert_eq!(
            (react.locked.as_deref(), react.installed.as_deref()),
            (Some("18.3.1"), Some("18.3.0"))
        );
        let vite = local.packages.iter().find(|p| p.name == "vite").unwrap();
        assert_eq!(vite.kind, DepKind::Dev);
        assert_eq!(vite.locked.as_deref(), Some("5.4.2"));
        let scoped = local
            .packages
            .iter()
            .find(|p| p.name == "@vitejs/plugin-react")
            .unwrap();
        assert_eq!(scoped.locked.as_deref(), Some("4.3.1"));
        assert!(local.errors.is_empty(), "{:?}", local.errors);
    }

    #[test]
    fn pnpm_v9_with_peer_suffix() {
        let mut local = Local::default();
        let lock = "lockfileVersion: '9.0'\nimporters:\n  .:\n    dependencies:\n      react:\n        specifier: ^18.2.0\n        version: 18.3.1\n      '@vitejs/plugin-react':\n        specifier: ^4.0.0\n        version: 4.3.1(vite@5.4.2)\n    devDependencies:\n      vite:\n        specifier: ^5.0.0\n        version: 5.4.2\n";
        let map = pnpm_lock(lock, &mut local);
        assert_eq!(map["react"], "18.3.1");
        assert_eq!(map["@vitejs/plugin-react"], "4.3.1");
        assert_eq!(map["vite"], "5.4.2");
    }

    #[test]
    fn yarn_classic_and_berry() {
        let declared = vec![Declared {
            name: "react".into(),
            ecosystem: Ecosystem::Npm,
            kind: DepKind::Prod,
            constraint: "^18.2.0".into(),
            locked: None,
            installed: None,
        }];
        let classic = "# yarn lockfile v1\n\n\"react@^18.0.0\", react@^18.2.0:\n  version \"18.3.1\"\n  resolved \"https://x\"\n";
        assert_eq!(yarn_lock(classic, &declared)["react"], "18.3.1");
        let berry = "__metadata:\n  version: 8\n\n\"react@npm:^18.2.0\":\n  version: 18.3.1\n  resolution: \"react@npm:18.3.1\"\n";
        assert_eq!(yarn_lock(berry, &declared)["react"], "18.3.1");
    }

    #[test]
    fn bun_text_lock_with_trailing_commas() {
        let text = r#"{
          "lockfileVersion": 1,
          "workspaces": { "": { "dependencies": { "react": "^18.2.0", }, }, },
          "packages": {
            "react": ["react@18.3.1", "", {}, "sha512-x"],
            "react/loose-envify": ["loose-envify@1.4.0", "", {}, "sha512-y"],
          },
        }"#;
        let lock: Value = serde_json::from_str(&strip_trailing_commas(text)).unwrap();
        let map = bun_lock(&lock);
        assert_eq!(map["react"], "18.3.1");
        assert!(!map.contains_key("react/loose-envify"));
        // Commas inside strings are kept.
        assert_eq!(strip_trailing_commas(r#"{"a": "x,}",}"#), r#"{"a": "x,}"}"#);
    }

    #[test]
    fn composer_lock_and_platform_requirements() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            &[
                (
                    "composer.json",
                    r#"{"require":{"php":"^8.2","ext-json":"*","laravel/framework":"^11.0"},"require-dev":{"phpunit/phpunit":"^11.0"}}"#,
                ),
                (
                    "composer.lock",
                    r#"{"packages":[{"name":"laravel/framework","version":"v11.9.2"}],"packages-dev":[{"name":"phpunit/phpunit","version":"11.2.1"}]}"#,
                ),
                (
                    "vendor/composer/installed.json",
                    r#"{"packages":[{"name":"laravel/framework","version":"v11.9.2"}]}"#,
                ),
            ],
        );
        let local = read(dir.path());
        assert_eq!(local.manager, Some(RuntimeKind::Composer));
        assert_eq!(local.packages.len(), 2, "php and ext-* are not packages");
        let laravel = &local.packages[0];
        assert_eq!(
            (laravel.locked.as_deref(), laravel.installed.as_deref()),
            (Some("11.9.2"), Some("11.9.2"))
        );
    }

    #[test]
    fn malformed_files_are_reported_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            &[
                ("package.json", "{ nope"),
                ("composer.json", r#"{"require":{"a/b":"^1"}}"#),
            ],
        );
        let local = read(dir.path());
        assert!(local.errors.iter().any(|e| e.contains("package.json")));
        assert_eq!(local.packages.len(), 1);
    }

    #[test]
    fn odd_names_never_leave_node_modules() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(installed_npm(dir.path(), "../../etc"), None);
        assert_eq!(installed_npm(dir.path(), "/etc/passwd"), None);
    }
}
