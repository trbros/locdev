//! Versions a project's own files ask for (`.nvmrc`, `engines`,
//! `composer.json`…). Stack detection lives in `locdev-frameworks`.

use std::path::Path;

use serde_json::Value;

use crate::model::{Requirement, RuntimeKind};

/// Versions the project's files ask for, most specific first per kind:
/// version files (followed like pins) before ranges. Only reads files.
pub fn requirements(path: &Path) -> Vec<Requirement> {
    let mut out = Vec::new();
    let mut push = |kind, constraint: &str, file: &str, exact| {
        let constraint = constraint.trim().trim_start_matches(['v', 'V']).trim();
        if !constraint.is_empty() && constraint.len() <= 128 {
            out.push(Requirement {
                kind,
                constraint: constraint.to_string(),
                file: file.to_string(),
                exact,
            });
        }
    };
    let read = |file: &str| -> Option<String> {
        let full = path.join(file);
        // Version files are tiny; refuse anything that isn't.
        (std::fs::metadata(&full).ok()?.len() <= 64 * 1024)
            .then(|| std::fs::read_to_string(full).ok())
            .flatten()
    };
    let first_line = |text: String| {
        text.lines()
            .map(|l| l.split('#').next().unwrap_or_default().trim().to_string())
            .find(|l| !l.is_empty())
    };

    for file in [".nvmrc", ".node-version"] {
        if let Some(line) = read(file).and_then(first_line) {
            push(RuntimeKind::Node, &line, file, true);
        }
    }
    if let Some(line) = read(".php-version").and_then(first_line) {
        push(RuntimeKind::Php, &line, ".php-version", true);
    }
    if let Some(text) = read(".tool-versions") {
        for line in text.lines() {
            let mut words = line
                .split('#')
                .next()
                .unwrap_or_default()
                .split_whitespace();
            let kind = match words.next() {
                Some("nodejs" | "node") => RuntimeKind::Node,
                Some("php") => RuntimeKind::Php,
                Some("pnpm") => RuntimeKind::Pnpm,
                Some("yarn") => RuntimeKind::Yarn,
                Some("bun") => RuntimeKind::Bun,
                _ => continue,
            };
            if let Some(version) = words.next() {
                push(kind, version, ".tool-versions", true);
            }
        }
    }

    if let Some(package) = read_package_json(path) {
        // `"packageManager": "pnpm@9.12.0+sha512.…"` (Corepack's field).
        if let Some((name, version)) = package["packageManager"]
            .as_str()
            .and_then(|v| v.split_once('@'))
        {
            let version = version.split('+').next().unwrap_or_default();
            if let Ok(kind) = name.parse::<RuntimeKind>() {
                push(kind, version, "package.json (packageManager)", true);
            }
        }
        if let Some(volta) = package["volta"].as_object() {
            for (name, version) in volta {
                if let (Ok(kind), Some(version)) = (name.parse::<RuntimeKind>(), version.as_str()) {
                    push(kind, version, "package.json (volta)", true);
                }
            }
        }
        if let Some(engines) = package["engines"].as_object() {
            for (name, range) in engines {
                if let (Ok(kind), Some(range)) = (name.parse::<RuntimeKind>(), range.as_str()) {
                    push(kind, range, "package.json (engines)", false);
                }
            }
        }
    }

    if let Some(composer) =
        read("composer.json").and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    {
        if let Some(php) = composer["config"]["platform"]["php"].as_str() {
            push(
                RuntimeKind::Php,
                php,
                "composer.json (config.platform)",
                true,
            );
        }
        if let Some(php) = composer["require"]["php"].as_str() {
            push(RuntimeKind::Php, php, "composer.json (require)", false);
        }
    }

    // Exact entries first; within each group the order above is preference.
    out.sort_by_key(|r| !r.exact);
    out
}

fn read_package_json(path: &Path) -> Option<Value> {
    let raw = std::fs::read_to_string(path.join("package.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (file, contents) in files {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        dir
    }

    #[test]
    fn reads_version_requirements() {
        let dir = project(&[
            (".nvmrc", "# comment\nv20.11\n"),
            (
                "package.json",
                r#"{"engines":{"node":">=18","pnpm":"^9"},"packageManager":"pnpm@9.12.0+sha512.abc","volta":{"node":"20.11.1"}}"#,
            ),
            (
                "composer.json",
                r#"{"require":{"php":"^8.2","laravel/framework":"^11"}}"#,
            ),
            (".tool-versions", "php 8.3.14\nruby 3.3.0\n"),
        ]);
        let found: Vec<_> = requirements(dir.path())
            .into_iter()
            .map(|r| (r.kind, r.constraint, r.exact))
            .collect();

        assert_eq!(found[0], (RuntimeKind::Node, "20.11".into(), true));
        assert!(found.contains(&(RuntimeKind::Php, "8.3.14".into(), true)));
        assert!(found.contains(&(RuntimeKind::Pnpm, "9.12.0".into(), true)));
        assert!(found.contains(&(RuntimeKind::Node, "20.11.1".into(), true)));
        assert!(found.contains(&(RuntimeKind::Node, ">=18".into(), false)));
        assert!(found.contains(&(RuntimeKind::Pnpm, "^9".into(), false)));
        assert!(found.contains(&(RuntimeKind::Php, "^8.2".into(), false)));
        // Exact entries come before ranges.
        let first_range = found.iter().position(|r| !r.2).unwrap();
        assert!(found[first_range..].iter().all(|r| !r.2));
    }

    #[test]
    fn ignores_malformed_files() {
        let dir = project(&[("package.json", "{ not json"), (".nvmrc", "\n\n")]);
        assert!(requirements(dir.path()).is_empty());
    }
}
