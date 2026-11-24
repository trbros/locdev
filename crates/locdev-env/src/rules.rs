//! Which `.env*` files each framework's dev server reads, in what order,
//! and what it inlines into browser code. Every one of these loaders lets a
//! variable already in the process environment win over its files, which is
//! why LocDev stays out of keys a project defines itself: injecting one
//! would silently replace the project's own value.
//!
//! Sources (2026-11): Next.js "Environment Variables" (load order),
//! Vite "Env Variables and Modes", Nuxt ".env", Laravel "Configuration"
//! (phpdotenv, immutable repository), dotenv/phpdotenv READMEs.

use std::collections::BTreeMap;
use std::path::Path;

use locdev_core::env::{EnvFileInfo, EnvFileRole};
use locdev_core::model::ProjectKind;

use crate::dotenv::{self, Document};

/// The mode LocDev runs dev servers in.
pub const MODE: &str = "development";

/// Files scanned at most, so a folder full of `.env.*` stays cheap.
const MAX_FILES: usize = 40;
/// Larger files are listed but not read.
const MAX_FILE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    pub framework: String,
    /// Files the dev server reads, highest precedence first.
    pub active: Vec<String>,
    /// Prefixes this framework inlines into client code.
    pub public_prefixes: Vec<String>,
    pub notes: Vec<String>,
    /// Why `active` is what it is (shown next to inactive files).
    pub loader: String,
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// The rules for a project. `command` is the dev server command, when known
/// (`node --env-file=…` names its own file).
pub fn rules(kind: ProjectKind, root: &Path, command: Option<&str>) -> Rules {
    let vite_order = names(&[
        ".env.development.local",
        ".env.development",
        ".env.local",
        ".env",
    ]);
    let restart = "Changes reach a running process only after it restarts.".to_string();
    match kind {
        ProjectKind::Next => Rules {
            framework: "Next.js".into(),
            active: names(&[".env.development.local", ".env.local", ".env.development", ".env"]),
            public_prefixes: names(&["NEXT_PUBLIC_"]),
            notes: vec![
                "NEXT_PUBLIC_ variables are inlined when the code is compiled: restart the dev server after changing one, and rebuild for production.".into(),
                restart,
            ],
            loader: "@next/env".into(),
        },
        ProjectKind::Vite => Rules {
            framework: "Vite".into(),
            active: vite_order,
            public_prefixes: names(&["VITE_"]),
            notes: vec![
                "VITE_ variables are inlined into client code; restart the dev server after changing one.".into(),
                restart,
            ],
            loader: "Vite".into(),
        },
        ProjectKind::Astro => Rules {
            framework: "Astro".into(),
            active: vite_order,
            public_prefixes: names(&["PUBLIC_"]),
            notes: vec![
                "PUBLIC_ variables reach client code; the rest stay on the server.".into(),
                restart,
            ],
            loader: "Vite (Astro)".into(),
        },
        ProjectKind::Sveltekit => Rules {
            framework: "SvelteKit".into(),
            active: vite_order,
            public_prefixes: names(&["PUBLIC_", "VITE_"]),
            notes: vec![
                "PUBLIC_ variables are available to client code ($env/static/public).".into(),
                restart,
            ],
            loader: "Vite (SvelteKit)".into(),
        },
        ProjectKind::Nuxt => Rules {
            framework: "Nuxt".into(),
            active: names(&[".env"]),
            public_prefixes: names(&["NUXT_PUBLIC_"]),
            notes: vec![
                "Nuxt reads .env in development only; NUXT_PUBLIC_ variables override runtimeConfig.public and reach the browser.".into(),
                restart,
            ],
            loader: "Nuxt (c12)".into(),
        },
        ProjectKind::Laravel => {
            let mut notes = vec![
                "Laravel reads .env through phpdotenv; a variable already in the process environment wins over the file.".into(),
                "VITE_ variables in the environment are inlined into assets by the Vite helper.".into(),
            ];
            if root.join("bootstrap/cache/config.php").is_file() {
                notes.insert(
                    0,
                    "Configuration is cached (bootstrap/cache/config.php): Laravel ignores environment changes until `php artisan config:clear`.".into(),
                );
            }
            notes.push(restart);
            Rules {
                framework: "Laravel".into(),
                active: names(&[".env"]),
                public_prefixes: names(&["VITE_"]),
                notes,
                loader: "phpdotenv".into(),
            }
        }
        ProjectKind::Php | ProjectKind::Wordpress => {
            let dotenv = composer_requires(root, "vlucas/phpdotenv")
                || composer_requires(root, "symfony/dotenv")
                || composer_requires(root, "roots/wp-config");
            let framework = if kind == ProjectKind::Php {
                "PHP"
            } else {
                "WordPress"
            };
            if dotenv {
                Rules {
                    framework: framework.into(),
                    active: names(&[".env"]),
                    public_prefixes: Vec::new(),
                    notes: vec![restart],
                    loader: "phpdotenv".into(),
                }
            } else {
                Rules {
                    framework: framework.into(),
                    active: Vec::new(),
                    public_prefixes: Vec::new(),
                    notes: vec![
                        format!("{framework} doesn't read .env files by itself (no phpdotenv in composer.json); shared variables reach it through getenv()."),
                        restart,
                    ],
                    loader: "none".into(),
                }
            }
        }
        ProjectKind::Node => {
            if let Some(file) = command.and_then(env_file_flag) {
                return Rules {
                    framework: "Node.js".into(),
                    active: vec![file],
                    public_prefixes: Vec::new(),
                    notes: vec![restart],
                    loader: "node --env-file".into(),
                };
            }
            if package_depends(root, "dotenv") || package_depends(root, "@dotenvx/dotenvx") {
                Rules {
                    framework: "Node.js".into(),
                    active: names(&[".env"]),
                    public_prefixes: Vec::new(),
                    notes: vec![restart],
                    loader: "dotenv".into(),
                }
            } else {
                Rules {
                    framework: "Node.js".into(),
                    active: Vec::new(),
                    public_prefixes: Vec::new(),
                    notes: vec![
                        "Node.js doesn't read .env files by itself (no dotenv dependency, no --env-file).".into(),
                        restart,
                    ],
                    loader: "none".into(),
                }
            }
        }
        ProjectKind::Static => Rules {
            framework: "Static".into(),
            active: Vec::new(),
            public_prefixes: Vec::new(),
            notes: vec!["A static site has no server process to give variables to.".into()],
            loader: "none".into(),
        },
        ProjectKind::Unknown => Rules {
            framework: "Unknown".into(),
            // Without knowing the loader, assume the common pair is read.
            active: names(&[".env.local", ".env"]),
            public_prefixes: Vec::new(),
            notes: vec![
                "The framework wasn't recognized; .env and .env.local are treated as read.".into(),
                restart,
            ],
            loader: "unknown".into(),
        },
    }
}

impl Rules {
    /// Rules a framework definition states (`env` in frameworks.toml).
    pub fn from_spec(
        framework: &str,
        files: Vec<String>,
        loader: &str,
        public: Vec<String>,
        mut notes: Vec<String>,
    ) -> Self {
        notes.push("Changes reach a running process only after it restarts.".into());
        Rules {
            framework: framework.into(),
            active: files,
            public_prefixes: public,
            notes,
            loader: loader.into(),
        }
    }
}

/// `--env-file=path` / `--env-file path` in a Node.js command.
fn env_file_flag(command: &str) -> Option<String> {
    let mut words = command.split_whitespace();
    while let Some(word) = words.next() {
        if let Some(file) = word.strip_prefix("--env-file=") {
            return Some(
                file.trim_matches(['"', '\''])
                    .trim_start_matches("./")
                    .to_string(),
            );
        }
        if word == "--env-file" {
            return words.next().map(|f| {
                f.trim_matches(['"', '\''])
                    .trim_start_matches("./")
                    .to_string()
            });
        }
    }
    None
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

fn package_depends(root: &Path, name: &str) -> bool {
    read_json(&root.join("package.json")).is_some_and(|json| {
        ["dependencies", "devDependencies"]
            .iter()
            .any(|section| json[section].get(name).is_some())
    })
}

fn composer_requires(root: &Path, name: &str) -> bool {
    read_json(&root.join("composer.json")).is_some_and(|json| {
        ["require", "require-dev"]
            .iter()
            .any(|section| json[section].get(name).is_some())
    })
}

/// What a `.env*` file name says about it; `None` for files that aren't one.
pub fn classify(name: &str) -> Option<(EnvFileRole, Option<String>)> {
    if name == ".env" {
        return Some((EnvFileRole::Base, None));
    }
    let rest = name.strip_prefix(".env.")?;
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Some(match rest {
        "local" => (EnvFileRole::Local, None),
        "example" | "sample" | "dist" | "template" => (EnvFileRole::Template, None),
        _ => match rest.strip_suffix(".local") {
            Some(mode) => (EnvFileRole::ModeLocal, Some(mode.to_string())),
            None => (EnvFileRole::Mode, Some(rest.to_string())),
        },
    })
}

/// A scanned project: each file's facts, and the parsed ones by name.
#[derive(Debug, Default)]
pub struct Scan {
    pub files: Vec<EnvFileInfo>,
    pub docs: BTreeMap<String, Document>,
}

impl Scan {
    /// Files that define `key`, with whether each is read.
    pub fn defining(&self, key: &str) -> Vec<(&str, bool)> {
        self.files
            .iter()
            .filter(|f| self.docs.get(&f.file).is_some_and(|d| d.has(key)))
            .map(|f| (f.file.as_str(), f.active))
            .collect()
    }
}

/// Reads the project's `.env*` files (top level only; symlinks are read, never written through).
pub fn scan(root: &Path, rules: &Rules) -> Scan {
    let mut scan = Scan::default();
    let Ok(dir) = std::fs::read_dir(root) else {
        return scan;
    };
    let mut names: Vec<String> = dir
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file() || t.is_symlink()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| classify(n).is_some())
        .collect();
    // Files named by the loader (`--env-file`) may not look like `.env*`.
    for file in &rules.active {
        if !names.contains(file) && root.join(file).is_file() {
            names.push(file.clone());
        }
    }
    names.sort();
    names.truncate(MAX_FILES);

    for name in names {
        let (role, mode) = classify(&name).unwrap_or((EnvFileRole::Base, None));
        let rank = rules
            .active
            .iter()
            .position(|f| *f == name)
            .map(|i| i as u8 + 1);
        let note = match (rank, role) {
            (Some(1), _) if rules.active.len() > 1 => {
                format!("read by {}; wins over the other files", rules.loader)
            }
            (Some(_), _) => format!("read by {}", rules.loader),
            (None, EnvFileRole::Template) => "a template; no loader reads it".into(),
            (None, _) if rules.loader == "none" => {
                format!("{} doesn't read .env files", rules.framework)
            }
            (None, _) => match &mode {
                Some(mode) if mode != MODE => {
                    format!("only read in {mode} mode, not by the dev server")
                }
                _ => format!("not read by {}", rules.loader),
            },
        };
        let path = root.join(&name);
        let mut info = EnvFileInfo {
            file: name.clone(),
            role,
            mode,
            active: rank.is_some(),
            rank,
            note,
            keys: 0,
            duplicates: Vec::new(),
            error: None,
        };
        match std::fs::metadata(&path) {
            Ok(meta) if meta.len() > MAX_FILE_BYTES => {
                info.error = Some("larger than 1 MiB; not read".into());
            }
            Ok(_) => match std::fs::read(&path).map(|b| dotenv::parse(&b)) {
                Ok(Ok(doc)) => {
                    info.keys = doc.keys().len();
                    info.duplicates = doc.duplicates();
                    scan.docs.insert(name, doc);
                }
                Ok(Err(error)) => info.error = Some(error.to_string()),
                Err(error) => info.error = Some(error.to_string()),
            },
            Err(error) => info.error = Some(error.to_string()),
        }
        scan.files.push(info);
    }
    scan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_names() {
        assert_eq!(classify(".env"), Some((EnvFileRole::Base, None)));
        assert_eq!(classify(".env.local"), Some((EnvFileRole::Local, None)));
        assert_eq!(
            classify(".env.example"),
            Some((EnvFileRole::Template, None))
        );
        assert_eq!(
            classify(".env.production.local"),
            Some((EnvFileRole::ModeLocal, Some("production".into())))
        );
        assert_eq!(
            classify(".env.staging"),
            Some((EnvFileRole::Mode, Some("staging".into())))
        );
        assert_eq!(classify(".envrc"), None);
        assert_eq!(classify("env"), None);
    }

    #[test]
    fn next_reads_four_files_in_order() {
        let dir = tempfile::tempdir().unwrap();
        for f in [
            ".env",
            ".env.local",
            ".env.production",
            ".env.example",
            ".env.development.local",
        ] {
            std::fs::write(dir.path().join(f), "A=1\n").unwrap();
        }
        let rules = rules(ProjectKind::Next, dir.path(), None);
        let scan = scan(dir.path(), &rules);
        let by_name: BTreeMap<_, _> = scan.files.iter().map(|f| (f.file.as_str(), f)).collect();
        assert_eq!(by_name[".env.development.local"].rank, Some(1));
        assert_eq!(by_name[".env.local"].rank, Some(2));
        assert_eq!(by_name[".env"].rank, Some(4));
        assert!(!by_name[".env.production"].active);
        assert!(by_name[".env.production"].note.contains("production mode"));
        assert!(!by_name[".env.example"].active);
        assert_eq!(scan.defining("A").len(), 5);
    }

    #[test]
    fn laravel_cached_config_is_noted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("bootstrap/cache")).unwrap();
        std::fs::write(dir.path().join("bootstrap/cache/config.php"), "<?php").unwrap();
        let rules = rules(ProjectKind::Laravel, dir.path(), None);
        assert_eq!(rules.active, vec![".env".to_string()]);
        assert!(rules.notes[0].contains("config:clear"));
    }

    #[test]
    fn node_needs_a_loader() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert!(rules(ProjectKind::Node, dir.path(), None).active.is_empty());
        assert_eq!(
            rules(
                ProjectKind::Node,
                dir.path(),
                Some("node --env-file=.env.dev server.js")
            )
            .active,
            vec![".env.dev".to_string()]
        );
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"dotenv":"^17"}}"#,
        )
        .unwrap();
        assert_eq!(
            rules(ProjectKind::Node, dir.path(), None).active,
            vec![".env".to_string()]
        );
    }

    #[test]
    fn unreadable_files_are_listed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env"), [0xff, 0xfe, 0x00]).unwrap();
        let scan = scan(dir.path(), &rules(ProjectKind::Laravel, dir.path(), None));
        assert!(scan.files[0].error.is_some());
        assert!(scan.docs.is_empty());
    }
}
