//! Finding a project's config files and the package versions they depend on.
//! Only files that exist are listed; symlinks are skipped so a "config file"
//! can't point outside the project.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use locdev_core::config::ConfigFormat;
use locdev_core::semver::Version;

use crate::catalog::{self, Kind};

/// Larger files aren't configuration; they are refused rather than loaded.
pub const MAX_FILE: u64 = 1024 * 1024;
/// Where a CSS-first Tailwind config usually lives.
const CSS_DIRS: &[&str] = &["src", "app", "resources/css", "styles", "assets", "css"];
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "vendor",
    "dist",
    "build",
    ".git",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "public",
];
const CSS_SCAN_LIMIT: usize = 400;

pub struct Found {
    pub rel: String,
    pub kind: &'static Kind,
}

fn regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_file() && m.len() <= MAX_FILE)
}

/// Every known config file in the project, in catalog order.
pub fn discover(root: &Path) -> Vec<Found> {
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if let Some(kind) = catalog::kind_of(&name)
                && regular_file(&entry.path())
            {
                found.push(Found { rel: name, kind });
            }
        }
    }
    if let Some(kind) = catalog::kind("tailwind-css") {
        for rel in tailwind_css(root) {
            found.push(Found { rel, kind });
        }
    }
    let order = |k: &Kind| {
        catalog::KINDS
            .iter()
            .position(|x| x.id == k.id)
            .unwrap_or(usize::MAX)
    };
    found.sort_by(|a, b| order(a.kind).cmp(&order(b.kind)).then(a.rel.cmp(&b.rel)));
    found
}

/// CSS files that load Tailwind (`@import "tailwindcss"`, or v3's `@tailwind`).
pub fn tailwind_css(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = 0;
    for dir in CSS_DIRS {
        scan_css(root, &root.join(dir), 0, &mut seen, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn scan_css(root: &Path, dir: &Path, depth: usize, seen: &mut usize, out: &mut Vec<String>) {
    if depth > 3 || *seen > CSS_SCAN_LIMIT {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        *seen += 1;
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().to_string();
        if meta.is_dir() {
            if !SKIP_DIRS.contains(&name.as_str()) && !name.starts_with('.') {
                scan_css(root, &path, depth + 1, seen, out);
            }
        } else if meta.is_file()
            && name.ends_with(".css")
            && meta.len() <= MAX_FILE
            && let Ok(text) = std::fs::read_to_string(&path)
            && loads_tailwind(&text)
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

pub fn loads_tailwind(css: &str) -> bool {
    css.contains("@import \"tailwindcss\"")
        || css.contains("@import 'tailwindcss'")
        || css.contains("@tailwind base")
}

/// The file `rel` names, if it is one `discover` lists (anything else is refused).
pub fn resolve(root: &Path, rel: &str) -> Result<(PathBuf, &'static Kind), String> {
    let found = discover(root)
        .into_iter()
        .find(|f| f.rel == rel)
        .ok_or_else(|| format!("{rel} isn't a config file LocDev manages in this project"))?;
    let path = root.join(&found.rel);
    // Belt and braces: the real path must still be inside the project.
    let real_root = root.canonicalize().map_err(|e| e.to_string())?;
    let real = path.canonicalize().map_err(|e| format!("{rel}: {e}"))?;
    if !real.starts_with(&real_root) {
        return Err(format!("{rel} is outside the project"));
    }
    Ok((path, found.kind))
}

/// The format, telling JSON from YAML for extension-less rc files.
pub fn format(kind: &Kind, rel: &str, text: &str) -> ConfigFormat {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let format = catalog::format_of(kind, name);
    let bare = name.starts_with('.') && !name[1..].contains('.');
    if bare && crate::json::check(text, false).is_some() {
        return ConfigFormat::Yaml;
    }
    format
}

/// Package versions: installed, else locked, else the floor of the declared range.
#[derive(Default)]
pub struct Versions(HashMap<String, Version>);

impl Versions {
    pub fn read(root: &Path) -> Self {
        let mut out = HashMap::new();
        for package in locdev_deps::local::read(root).packages {
            let raw = package
                .installed
                .or(package.locked)
                .unwrap_or(package.constraint);
            let floor = raw
                .trim()
                .trim_start_matches(['^', '~', '>', '=', 'v', ' ']);
            let first = floor.split([' ', '|', ',']).next().unwrap_or("");
            if let Some(version) = Version::parse(first).or_else(|| Version::parse_composer(first))
            {
                out.entry(package.name).or_insert(version);
            }
        }
        Self(out)
    }

    pub fn get(&self, package: &str) -> Option<&Version> {
        self.0.get(package)
    }

    #[cfg(test)]
    pub fn with(entries: &[(&str, &str)]) -> Self {
        Self(
            entries
                .iter()
                .map(|(n, v)| (n.to_string(), Version::parse(v).unwrap()))
                .collect(),
        )
    }
}
