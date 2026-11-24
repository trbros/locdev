//! Readiness checks: things to fix before any platform sees the project.
//! Read-only; nothing here touches the repository.

use std::path::Path;

use locdev_core::deploy::{CheckLevel, DeployProfile, ReadinessCheck};

use crate::profile::Files;

fn check(
    id: &str,
    level: CheckLevel,
    title: impl Into<String>,
    detail: impl Into<String>,
) -> ReadinessCheck {
    ReadinessCheck {
        id: id.into(),
        level,
        title: title.into(),
        detail: detail.into(),
    }
}

pub fn run(root: &Path, profile: &DeployProfile, files: &Files) -> Vec<ReadinessCheck> {
    let mut out = Vec::new();
    let repo = locdev_git::Repo::open(root).ok();
    let has_env = root.join(".env").is_file();

    match &repo {
        None => out.push(check(
            "git",
            CheckLevel::Warn,
            "Not a Git repository",
            "Most platforms deploy from a Git repository; `git init` and push it first.",
        )),
        Some(repo) => {
            if let Ok(status) = repo.status() {
                let changed = status.changes.len();
                out.push(if changed == 0 {
                    check(
                        "git",
                        CheckLevel::Ok,
                        "Working tree clean",
                        "What's committed is what a Git-based deploy builds.",
                    )
                } else {
                    check(
                        "git",
                        CheckLevel::Warn,
                        format!("{changed} uncommitted changes"),
                        "A deploy from Git builds the last commit, without these.",
                    )
                });
            }
            if has_env {
                if repo.tracked(".env").unwrap_or(false) {
                    out.push(check(
                        "env-committed",
                        CheckLevel::Fail,
                        ".env is committed",
                        "Its secrets are in the repository history. Remove it from Git (`git rm --cached .env`), add it to .gitignore and rotate the secrets it held.",
                    ));
                } else if !repo.ignored(".env").unwrap_or(true) {
                    out.push(check(
                        "env-ignored",
                        CheckLevel::Warn,
                        ".env isn't in .gitignore",
                        "One `git add .` away from committing secrets.",
                    ));
                } else {
                    out.push(check(
                        "env-ignored",
                        CheckLevel::Ok,
                        ".env stays out of Git",
                        "It's ignored; set its values on the platform instead.",
                    ));
                }
            }
        }
    }

    if has_env {
        match &files.example {
            None => out.push(check(
                "env-example",
                CheckLevel::Warn,
                "No .env.example",
                "A template with the variable names (no values) tells the platform, and other people, what to set.",
            )),
            Some(example) => {
                let missing: Vec<&String> = files.env.keys().filter(|k| !example.contains(k)).collect();
                out.push(if missing.is_empty() {
                    check("env-example", CheckLevel::Ok, ".env.example lists every variable", "Names only; values stay on your machine.")
                } else {
                    let names: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
                    check(
                        "env-example",
                        CheckLevel::Warn,
                        format!("{} variables aren't in .env.example", missing.len()),
                        format!("{} (add the names if production needs them; `locdev env sync` can append them).", names.join(", ")),
                    )
                });
            }
        }
    }

    match profile.runtime.as_str() {
        "node" => {
            let lock = [
                "package-lock.json",
                "pnpm-lock.yaml",
                "yarn.lock",
                "bun.lock",
                "bun.lockb",
            ]
            .iter()
            .any(|f| root.join(f).is_file());
            out.push(if lock {
                check(
                    "lockfile",
                    CheckLevel::Ok,
                    "Lockfile present",
                    "Builds install the versions you tested.",
                )
            } else {
                check(
                    "lockfile",
                    CheckLevel::Warn,
                    "No lockfile",
                    "Without one the build resolves versions again and may install different ones.",
                )
            });
            if !files.scripts.contains_key("build")
                && !matches!(profile.rendering, locdev_core::deploy::Rendering::Server)
            {
                out.push(check(
                    "build",
                    CheckLevel::Warn,
                    "No build script",
                    "package.json has no `build` script for the platform to run.",
                ));
            }
            out.push(match &profile.runtime_version {
                Some(v) => check("runtime", CheckLevel::Ok, format!("Node.js version declared ({v})"), "Platforms read engines.node or .nvmrc to pick the version."),
                None => check(
                    "runtime",
                    CheckLevel::Warn,
                    "Node.js version not declared",
                    "Add engines.node to package.json (or .nvmrc) so production runs the version you use here.",
                ),
            });
        }
        "php" => {
            out.push(if root.join("composer.lock").is_file() {
                check(
                    "lockfile",
                    CheckLevel::Ok,
                    "composer.lock present",
                    "Builds install the versions you tested.",
                )
            } else {
                check(
                    "lockfile",
                    CheckLevel::Warn,
                    "No composer.lock",
                    "Commit it so production installs the versions you tested.",
                )
            });
            out.push(match &profile.runtime_version {
                Some(v) => check(
                    "runtime",
                    CheckLevel::Ok,
                    format!("PHP version declared ({v})"),
                    "From composer.json's require.php.",
                ),
                None => check(
                    "runtime",
                    CheckLevel::Warn,
                    "PHP version not declared",
                    "Add `php` to composer.json's require.",
                ),
            });
            if files
                .env
                .get("APP_DEBUG")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
            {
                out.push(check(
                    "app-debug",
                    CheckLevel::Warn,
                    "APP_DEBUG is true locally",
                    "Fine here; make sure production sets APP_DEBUG=false, or error pages show code and configuration.",
                ));
            }
        }
        _ => {}
    }

    if profile.needs.iter().any(|n| n.id == "sqlite") {
        out.push(check(
            "sqlite",
            CheckLevel::Warn,
            "The database is a SQLite file",
            "On platforms without a persistent disk it's lost on every deploy; use a volume or a managed database.",
        ));
    }
    out
}
