//! The commands that update selected packages with the project's own package
//! manager, as program + arguments (never a shell string). Updates within
//! the declared constraint use the manager's "update"; moving past it (a new
//! major, only when asked) rewrites the constraint with the same style.
//! Install scripts are skipped unless the user asks for them.

use std::path::Path;

use locdev_core::deps::{DepKind, Ecosystem};
use locdev_core::model::RuntimeKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pick {
    pub name: String,
    pub ecosystem: Ecosystem,
    pub kind: DepKind,
    /// The manifest's current constraint.
    pub declared: String,
    pub target: String,
    /// Whether `target` satisfies `declared`.
    pub within_range: bool,
}

impl Pick {
    /// The constraint written when leaving the declared range: the same
    /// operator (`^`, `~`, exact) with the new version.
    pub fn new_constraint(&self) -> String {
        let declared = self.declared.trim();
        let prefix = if declared.starts_with('~') {
            "~"
        } else if declared.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            ""
        } else {
            "^"
        };
        format!("{prefix}{}", self.target)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub argv: Vec<String>,
    /// Extra environment for this command only.
    pub env: Vec<(String, String)>,
}

/// Yarn 2+ (berry) is told apart from Yarn 1 by its lockfile or config.
pub fn yarn_berry(root: &Path) -> bool {
    root.join(".yarnrc.yml").is_file()
        || std::fs::read_to_string(root.join("yarn.lock")).is_ok_and(|t| t.contains("__metadata:"))
}

/// Files the update is expected to touch.
pub fn files(manager: RuntimeKind, picks: &[Pick]) -> Vec<String> {
    let mut out = Vec::new();
    if picks.iter().any(|p| p.ecosystem == Ecosystem::Npm) {
        out.push("package.json".into());
        out.push(
            match manager {
                RuntimeKind::Pnpm => "pnpm-lock.yaml",
                RuntimeKind::Yarn => "yarn.lock",
                RuntimeKind::Bun => "bun.lock",
                _ => "package-lock.json",
            }
            .into(),
        );
    }
    if picks.iter().any(|p| p.ecosystem == Ecosystem::Composer) {
        out.push("composer.json".into());
        out.push("composer.lock".into());
    }
    out
}

pub fn commands(
    manager: RuntimeKind,
    berry: bool,
    picks: &[Pick],
    run_scripts: bool,
) -> Vec<Command> {
    let mut out = Vec::new();
    let npm_picks: Vec<&Pick> = picks
        .iter()
        .filter(|p| p.ecosystem == Ecosystem::Npm)
        .collect();
    let composer_picks: Vec<&Pick> = picks
        .iter()
        .filter(|p| p.ecosystem == Ecosystem::Composer)
        .collect();

    if !npm_picks.is_empty() {
        out.extend(node_commands(manager, berry, &npm_picks, run_scripts));
    }
    if !composer_picks.is_empty() {
        out.extend(composer_commands(&composer_picks, run_scripts));
    }
    out
}

fn cmd(argv: &[&str], rest: impl IntoIterator<Item = String>) -> Command {
    Command {
        argv: argv.iter().map(|s| s.to_string()).chain(rest).collect(),
        env: Vec::new(),
    }
}

fn node_commands(
    manager: RuntimeKind,
    berry: bool,
    picks: &[&Pick],
    run_scripts: bool,
) -> Vec<Command> {
    let mut out = Vec::new();
    let in_range: Vec<String> = picks
        .iter()
        .filter(|p| p.within_range)
        .map(|p| p.name.clone())
        .collect();
    let past: Vec<&&Pick> = picks.iter().filter(|p| !p.within_range).collect();
    let ignore = (!run_scripts).then(|| "--ignore-scripts".to_string());

    if manager == RuntimeKind::Yarn && berry {
        // `yarn up` handles both; berry turns scripts off through its env.
        let specs = picks.iter().map(|p| {
            if p.within_range {
                format!("{}@{}", p.name, p.declared)
            } else {
                format!("{}@{}", p.name, p.new_constraint())
            }
        });
        let mut command = cmd(&["yarn", "up"], specs);
        if !run_scripts {
            command.env.push(("YARN_ENABLE_SCRIPTS".into(), "0".into()));
        }
        return vec![command];
    }

    if !in_range.is_empty() {
        let base: &[&str] = match manager {
            RuntimeKind::Pnpm => &["pnpm", "update"],
            RuntimeKind::Yarn => &["yarn", "upgrade"],
            RuntimeKind::Bun => &["bun", "update"],
            _ => &["npm", "update"],
        };
        out.push(cmd(base, in_range.into_iter().chain(ignore.clone())));
    }
    for dev in [false, true] {
        let group: Vec<String> = past
            .iter()
            .filter(|p| (p.kind == DepKind::Dev) == dev)
            .map(|p| format!("{}@{}", p.name, p.new_constraint()))
            .collect();
        if group.is_empty() {
            continue;
        }
        let (base, dev_flag): (&[&str], &str) = match manager {
            RuntimeKind::Pnpm => (&["pnpm", "add"], "-D"),
            RuntimeKind::Yarn => (&["yarn", "add"], "--dev"),
            RuntimeKind::Bun => (&["bun", "add"], "--dev"),
            _ => (&["npm", "install"], "--save-dev"),
        };
        let flags = dev.then(|| dev_flag.to_string()).into_iter();
        out.push(cmd(base, flags.chain(group).chain(ignore.clone())));
    }
    out
}

fn composer_commands(picks: &[&Pick], run_scripts: bool) -> Vec<Command> {
    let mut out = Vec::new();
    let common = || {
        let mut flags = vec!["--no-interaction".to_string()];
        if !run_scripts {
            flags.push("--no-scripts".into());
        }
        flags
    };
    let in_range: Vec<String> = picks
        .iter()
        .filter(|p| p.within_range)
        .map(|p| p.name.clone())
        .collect();
    if !in_range.is_empty() {
        out.push(cmd(
            &["composer", "update"],
            in_range
                .into_iter()
                .chain(["--with-dependencies".to_string()])
                .chain(common()),
        ));
    }
    for dev in [false, true] {
        let group: Vec<String> = picks
            .iter()
            .filter(|p| !p.within_range && (p.kind == DepKind::Dev) == dev)
            .map(|p| format!("{}:{}", p.name, p.new_constraint()))
            .collect();
        if group.is_empty() {
            continue;
        }
        let dev_flag = dev.then(|| "--dev".to_string());
        out.push(cmd(
            &["composer", "require"],
            dev_flag
                .into_iter()
                .chain(group)
                .chain(["--with-all-dependencies".to_string()])
                .chain(common()),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pick(name: &str, kind: DepKind, declared: &str, target: &str, within: bool) -> Pick {
        Pick {
            name: name.into(),
            ecosystem: if name.contains('/') && !name.starts_with('@') {
                Ecosystem::Composer
            } else {
                Ecosystem::Npm
            },
            kind,
            declared: declared.into(),
            target: target.into(),
            within_range: within,
        }
    }

    fn argv(commands: &[Command]) -> Vec<String> {
        commands.iter().map(|c| c.argv.join(" ")).collect()
    }

    #[test]
    fn npm_updates_in_range_and_installs_majors() {
        let picks = [
            pick("react", DepKind::Prod, "^18.2.0", "18.3.1", true),
            pick("vite", DepKind::Dev, "^5.0.0", "7.1.0", false),
            pick("lodash", DepKind::Prod, "~4.17.0", "5.0.0", false),
        ];
        assert_eq!(
            argv(&commands(RuntimeKind::Npm, false, &picks, false)),
            vec![
                "npm update react --ignore-scripts",
                "npm install lodash@~5.0.0 --ignore-scripts",
                "npm install --save-dev vite@^7.1.0 --ignore-scripts",
            ]
        );
    }

    #[test]
    fn pnpm_and_bun_keep_their_own_verbs() {
        let picks = [pick("react", DepKind::Prod, "18.2.0", "19.0.0", false)];
        assert_eq!(
            argv(&commands(RuntimeKind::Pnpm, false, &picks, true)),
            vec!["pnpm add react@19.0.0"]
        );
        assert_eq!(
            argv(&commands(RuntimeKind::Bun, false, &picks, false)),
            vec!["bun add react@19.0.0 --ignore-scripts"]
        );
    }

    #[test]
    fn yarn_berry_turns_scripts_off_through_env() {
        let picks = [pick("react", DepKind::Prod, "^18.2.0", "18.3.1", true)];
        let commands = commands(RuntimeKind::Yarn, true, &picks, false);
        assert_eq!(argv(&commands), vec!["yarn up react@^18.2.0"]);
        assert_eq!(
            commands[0].env,
            vec![("YARN_ENABLE_SCRIPTS".to_string(), "0".to_string())]
        );
    }

    #[test]
    fn composer_update_and_require() {
        let picks = [
            pick("laravel/framework", DepKind::Prod, "^11.0", "11.31.0", true),
            pick("phpunit/phpunit", DepKind::Dev, "^10.5", "11.4.0", false),
        ];
        assert_eq!(
            argv(&commands(RuntimeKind::Composer, false, &picks, false)),
            vec![
                "composer update laravel/framework --with-dependencies --no-interaction --no-scripts",
                "composer require --dev phpunit/phpunit:^11.4.0 --with-all-dependencies --no-interaction --no-scripts",
            ]
        );
    }

    #[test]
    fn lists_the_files_it_touches() {
        let picks = [pick("react", DepKind::Prod, "^18", "18.3.1", true)];
        assert_eq!(
            files(RuntimeKind::Pnpm, &picks),
            vec!["package.json", "pnpm-lock.yaml"]
        );
    }
}
