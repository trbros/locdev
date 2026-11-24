//! Monorepo packages, from the workspace configuration only: package.json
//! `workspaces`, pnpm-workspace.yaml, Cargo `[workspace]` and go.work.
//! Patterns are expanded one level (`apps/*`); each package is its own site.

use crate::project::ProjectFiles;

/// Packages listed at most, so a huge monorepo stays cheap to analyze.
const MAX_MEMBERS: usize = 24;

/// Relative paths of the workspace's packages that exist.
pub fn members(files: &ProjectFiles) -> Vec<String> {
    let mut patterns: Vec<String> = Vec::new();
    if let Some(package) = files.package() {
        let list = package["workspaces"]
            .as_array()
            .or_else(|| package["workspaces"]["packages"].as_array());
        patterns.extend(
            list.into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_string)),
        );
    }
    if let Some(text) = files.text("pnpm-workspace.yaml") {
        let mut inside = false;
        for line in text.lines() {
            if line.trim_start().starts_with("packages:") {
                inside = true;
                continue;
            }
            if inside {
                match line.trim().strip_prefix('-') {
                    Some(item) => patterns.push(item.trim().trim_matches(['"', '\'']).to_string()),
                    None if line.starts_with(char::is_whitespace) || line.trim().is_empty() => {}
                    None => inside = false,
                }
            }
        }
    }
    if let Some(table) = files
        .text("Cargo.toml")
        .and_then(|t| t.parse::<toml::Table>().ok())
        && let Some(list) = table
            .get("workspace")
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
    {
        patterns.extend(list.iter().filter_map(|v| v.as_str().map(str::to_string)));
    }
    if let Some(text) = files.text("go.work") {
        let mut block = false;
        for line in text.lines().map(str::trim) {
            if line.starts_with("use (") {
                block = true;
            } else if block && line == ")" {
                block = false;
            } else if block && !line.is_empty() {
                patterns.push(line.to_string());
            } else if let Some(one) = line.strip_prefix("use ") {
                patterns.push(one.trim().to_string());
            }
        }
    }

    let mut out: Vec<String> = Vec::new();
    for pattern in patterns {
        let pattern = pattern.trim_start_matches("./").trim_end_matches('/');
        // Excludes (`!apps/legacy`) and anything leaving the project are ignored.
        if pattern.starts_with('!') || pattern.contains("..") || pattern.starts_with('/') {
            continue;
        }
        let expanded: Vec<String> = match pattern
            .strip_suffix("/*")
            .or_else(|| pattern.strip_suffix("/**"))
        {
            Some(parent) if !parent.contains('*') => {
                let mut children: Vec<String> = std::fs::read_dir(files.root.join(parent))
                    .map(|dir| {
                        dir.filter_map(Result::ok)
                            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                            .filter_map(|e| e.file_name().into_string().ok())
                            .filter(|n| !n.starts_with('.') && n != "node_modules")
                            .map(|n| format!("{parent}/{n}"))
                            .collect()
                    })
                    .unwrap_or_default();
                children.sort();
                children
            }
            None if !pattern.contains('*') && pattern != "." => vec![pattern.to_string()],
            _ => Vec::new(),
        };
        for path in expanded {
            if files.root.join(&path).is_dir() && !out.contains(&path) {
                out.push(path);
            }
        }
    }
    out.truncate(MAX_MEMBERS);
    out
}
