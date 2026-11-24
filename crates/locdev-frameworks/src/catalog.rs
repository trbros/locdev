//! The framework definitions: pure data (detection rules, version sources,
//! dev commands, env rules, links) read by the native engine. A definition
//! can't add behavior the engine doesn't have: unknown fields are refused,
//! and a newer `format` needs a newer LocDev.

use std::collections::BTreeSet;

use locdev_core::model::ComponentCategory;
use serde::{Deserialize, Serialize};

/// The definitions format this LocDev understands. Bump when the engine
/// learns a new field or rule; older apps then keep their last good set.
pub const FORMAT: u32 = 2;

/// The definitions shipped inside this build.
pub const BUNDLED: &str = include_str!("../definitions/frameworks.toml");

/// Programs a dev command may start. A definition that names anything else
/// is refused, so even a signed catalog can't run arbitrary binaries.
const PROGRAMS: &[&str] = &[
    // LocDev's own static server (`locdevd static`).
    "locdevd",
    "npm",
    "npx",
    "pnpm",
    "yarn",
    "bun",
    "bunx",
    "deno",
    "node",
    "php",
    "composer",
    "python",
    "python3",
    "flask",
    "fastapi",
    "uvicorn",
    "bundle",
    "bin/rails",
    "ruby",
    "cargo",
    "go",
    "hugo",
    "dotnet",
    "./mvnw",
    "mvnw.cmd",
    "mvn",
    "./gradlew",
    "gradlew.bat",
    "gradle",
    "celery",
    "bin/jobs",
];

/// Placeholders a command may hold: the port LocDev assigns, and the
/// Django project package read from manage.py.
const PLACEHOLDERS: &[&str] = &["{port}", "{django_module}"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub format: u32,
    /// Grows with every published change (`yyyymmddNN`).
    pub revision: u64,
    /// Oldest LocDev that understands this set.
    pub min_locdev: String,
    #[serde(rename = "framework")]
    pub frameworks: Vec<Definition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: String,
    pub name: String,
    pub category: ComponentCategory,
    /// The `ProjectKind` it maps to (`laravel`, `next`, `node`…), for the
    /// parts of LocDev that still reason by kind.
    #[serde(default)]
    pub family: Option<String>,
    /// Higher wins when several frameworks match the same folder.
    #[serde(default)]
    pub priority: i32,
    pub docs: String,
    /// Runtimes LocDev manages or detects (`node`, `php`, `python`…).
    #[serde(default)]
    pub runtimes: Vec<String>,
    /// Programs the dev command needs on PATH.
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub detect: Detect,
    /// Where the version comes from, tried in order.
    #[serde(default)]
    pub version: Vec<Rule>,
    /// Versions the adapter was written for (npm range dialect).
    #[serde(default)]
    pub supported: Option<String>,
    /// Ways to start the dev server; the first whose `when` holds is used.
    #[serde(default)]
    pub dev: Vec<DevVariant>,
    /// The tool behind the dev server (`vite`, `next`, `nuxt`, `astro`).
    #[serde(default)]
    pub dev_tool: Option<String>,
    /// A helper dev server it may run next to the main one (`vite`).
    #[serde(default)]
    pub helper: Option<String>,
    /// Shown to the user; never run as a side effect.
    #[serde(default)]
    pub install: Option<String>,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub test: Option<String>,
    #[serde(default)]
    pub config_files: Vec<String>,
    #[serde(default)]
    pub env: Option<EnvSpec>,
    /// Service ids (from the site's catalog) this stack commonly uses.
    #[serde(default)]
    pub services: Vec<String>,
    /// Components it brings along (`laravel` implies `php`).
    #[serde(default)]
    pub implies: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    /// Runs inside the project's Python virtual environment, when it has one.
    #[serde(default)]
    pub venv: bool,
    /// Auxiliary processes (queue worker, scheduler…); offered, never
    /// started without the user's decision.
    #[serde(default)]
    pub processes: Vec<ProcessDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessDef {
    /// Unique within the project (`queue`, `scheduler`…); not `main`/`dev`.
    pub id: String,
    pub name: String,
    pub kind: locdev_core::model::ProcessKind,
    /// Offered only when one of these holds (a package is installed…).
    #[serde(default)]
    pub requires_any: Vec<Rule>,
    /// Ways to start it; the first whose `when` holds is used.
    pub run: Vec<DevVariant>,
    /// Gets a port of its own (`{port}` in the command).
    #[serde(default)]
    pub port: bool,
    /// Alternatives that must not run at the same time.
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Detect {
    /// At least one must hold (when not empty).
    #[serde(default)]
    pub any: Vec<Rule>,
    /// Every one must hold.
    #[serde(default)]
    pub all: Vec<Rule>,
    /// None may hold.
    #[serde(default)]
    pub none: Vec<Rule>,
}

impl Detect {
    pub fn is_empty(&self) -> bool {
        self.any.is_empty() && self.all.is_empty()
    }
}

/// One piece of evidence. Exactly one field is set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// A file at the project root (or a relative path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    /// `*.ext` at the project root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<String>,
    /// A package.json dependency (dependencies or devDependencies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    /// A package.json script whose body contains this text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gem: Option<String>,
    /// requirements*.txt, pyproject.toml or Pipfile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<String>,
    /// A module path in go.mod.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub go: Option<String>,
    /// An artifactId in pom.xml (substring).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maven: Option<String>,
    /// Text in build.gradle(.kts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradle: Option<String>,
    /// Text in a *.csproj; `TargetFramework` as a version source reads the TFM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nuget: Option<String>,
    /// `[file, text]`: the file contains the text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<[String; 2]>,
}

impl Rule {
    fn set(&self) -> usize {
        [
            self.file.is_some(),
            self.dir.is_some(),
            self.glob.is_some(),
            self.npm.is_some(),
            self.script.is_some(),
            self.composer.is_some(),
            self.gem.is_some(),
            self.python.is_some(),
            self.cargo.is_some(),
            self.go.is_some(),
            self.maven.is_some(),
            self.gradle.is_some(),
            self.nuget.is_some(),
            self.contains.is_some(),
        ]
        .into_iter()
        .filter(|s| *s)
        .count()
    }

    fn paths(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = [&self.file, &self.dir, &self.glob]
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect();
        if let Some([file, _]) = &self.contains {
            paths.push(file);
        }
        paths
    }
}

/// How the port reaches the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortMode {
    /// On the command line (`{port}` in `args`/`command`).
    #[default]
    Args,
    /// Through `PORT` (always set) and `port_env`.
    Env,
    /// The framework takes it from its own config; the note says where.
    Config,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevVariant {
    /// Every rule must hold for this variant to be used.
    #[serde(default)]
    pub when: Vec<Rule>,
    /// Run the first of these package.json scripts that exists…
    #[serde(default)]
    pub script: Vec<String>,
    /// …and whose body contains this text, when set.
    #[serde(default)]
    pub script_contains: Option<String>,
    /// Appended to the script (after `--` for npm).
    #[serde(default)]
    pub args: Option<String>,
    /// Or a fixed command.
    #[serde(default)]
    pub command: Option<String>,
    /// The command on Windows, when it differs (`mvnw.cmd`).
    #[serde(default)]
    pub command_windows: Option<String>,
    #[serde(default)]
    pub port: PortMode,
    /// More variables for the port: `SERVER_PORT` (gets the number) or
    /// `ASPNETCORE_URLS=http://127.0.0.1:{port}` (a template).
    #[serde(default)]
    pub port_env: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// How the framework loads `.env` files (used by shared variables).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvSpec {
    /// Only when one of these holds (a dotenv library is installed…);
    /// otherwise the framework reads no `.env` file.
    #[serde(default)]
    pub if_any: Vec<Rule>,
    /// Files read in development, highest precedence first.
    #[serde(default)]
    pub files: Vec<String>,
    pub loader: String,
    /// Prefixes inlined into browser code.
    #[serde(default)]
    pub public: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl Catalog {
    /// The bundled set, parsed once.
    pub fn bundled_ref() -> &'static Catalog {
        static BUNDLED_SET: std::sync::OnceLock<Catalog> = std::sync::OnceLock::new();
        BUNDLED_SET.get_or_init(Catalog::bundled)
    }

    pub fn bundled() -> Self {
        let catalog: Catalog = toml::from_str(BUNDLED).expect("bundled definitions parse");
        catalog.validate().expect("bundled definitions are valid");
        catalog
    }

    pub fn get(&self, id: &str) -> Option<&Definition> {
        self.frameworks.iter().find(|d| d.id == id)
    }

    /// Everything a definition set must satisfy before it is used.
    pub fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT {
            return Err(format!(
                "definitions format {} (this LocDev reads format {FORMAT})",
                self.format
            ));
        }
        if !version_at_least(env!("CARGO_PKG_VERSION"), &self.min_locdev) {
            return Err(format!(
                "these definitions need LocDev {} or newer",
                self.min_locdev
            ));
        }
        let mut ids = BTreeSet::new();
        for def in &self.frameworks {
            let ctx = |m: String| format!("{}: {m}", def.id);
            if def.id.is_empty()
                || !def
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                return Err(format!("bad id `{}`", def.id));
            }
            if !ids.insert(def.id.as_str()) {
                return Err(format!("duplicate id `{}`", def.id));
            }
            if !def.docs.starts_with("https://") {
                return Err(ctx("docs must be an https link".into()));
            }
            let rules = def
                .detect
                .any
                .iter()
                .chain(&def.detect.all)
                .chain(&def.detect.none)
                .chain(&def.version)
                .chain(def.env.iter().flat_map(|e| &e.if_any))
                .chain(def.dev.iter().flat_map(|v| &v.when))
                .chain(def.processes.iter().flat_map(|p| &p.requires_any))
                .chain(
                    def.processes
                        .iter()
                        .flat_map(|p| p.run.iter().flat_map(|v| &v.when)),
                );
            for rule in rules {
                if rule.set() != 1 {
                    return Err(ctx("a rule must set exactly one field".into()));
                }
                for path in rule.paths() {
                    if path.is_empty()
                        || path.starts_with('/')
                        || path.contains("..")
                        || path.contains('\\')
                    {
                        return Err(ctx(format!(
                            "`{path}` must be a relative path inside the project"
                        )));
                    }
                }
            }
            for process in &def.processes {
                let ok = !process.id.is_empty()
                    && !matches!(process.id.as_str(), "main" | "dev")
                    && process
                        .id
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
                if !ok {
                    return Err(ctx(format!("bad process id `{}`", process.id)));
                }
                if process.run.is_empty() {
                    return Err(ctx(format!("process `{}` has no way to run", process.id)));
                }
            }
            for variant in def
                .dev
                .iter()
                .chain(def.processes.iter().flat_map(|p| &p.run))
            {
                for text in [&variant.args, &variant.command, &variant.command_windows]
                    .into_iter()
                    .flatten()
                {
                    check_command_text(text).map_err(ctx)?;
                }
                for program in [&variant.command, &variant.command_windows]
                    .into_iter()
                    .flatten()
                    .filter_map(|c| c.split_whitespace().next())
                {
                    if !PROGRAMS.contains(&program) {
                        return Err(ctx(format!("`{program}` is not an allowed program")));
                    }
                }
                if variant.command.is_none() && variant.script.is_empty() {
                    return Err(ctx("a dev variant needs a script or a command".into()));
                }
                for entry in &variant.port_env {
                    let (name, value) = entry.split_once('=').unwrap_or((entry, "{port}"));
                    if name.is_empty()
                        || !name
                            .chars()
                            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                    {
                        return Err(ctx(format!("`{name}` is not a variable name")));
                    }
                    check_command_text(value).map_err(ctx)?;
                }
            }
            for implied in &def.implies {
                if implied == &def.id {
                    return Err(ctx("implies itself".into()));
                }
            }
        }
        for def in &self.frameworks {
            for implied in &def.implies {
                if !ids.contains(implied.as_str()) {
                    return Err(format!("{}: implies unknown `{implied}`", def.id));
                }
            }
        }
        Ok(())
    }
}

/// Commands are split on whitespace and run by the supervisor's shell, so
/// they may only hold plain words: no quoting, expansion or chaining.
fn check_command_text(text: &str) -> Result<(), String> {
    let ok = text.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(
                c,
                ' ' | '-' | '_' | '.' | '/' | ':' | '=' | '{' | '}' | '@' | ',' | '+'
            )
    });
    let placeholders_ok = text
        .match_indices('{')
        .all(|(i, _)| PLACEHOLDERS.iter().any(|p| text[i..].starts_with(p)));
    if ok && placeholders_ok {
        Ok(())
    } else {
        Err(format!(
            "`{text}` may only contain plain words and the {{port}} or {{django_module}} placeholders"
        ))
    }
}

/// `a >= b` for dotted versions (`0.3.0` against `0.2`).
fn version_at_least(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.split('.')
            .map(|p| p.trim().parse().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parts(a), parts(b));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x > y;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_definitions_are_valid() {
        let catalog = Catalog::bundled();
        assert!(catalog.frameworks.len() >= 41);
    }

    #[test]
    fn commands_cannot_escape() {
        for bad in [
            "npm run dev; rm -rf ~",
            "npm run dev && curl x",
            "npm run $(whoami)",
            "npm run dev `id`",
            "npm run dev | sh",
            "npm run dev > /etc/x",
            "npm run {other}",
            "php -r \"system('id')\"",
        ] {
            assert!(check_command_text(bad).is_err(), "{bad}");
        }
        assert!(check_command_text("php artisan serve --host=127.0.0.1 --port={port}").is_ok());
    }

    #[test]
    fn refuses_unknown_programs_and_fields() {
        let mut catalog = Catalog::bundled();
        catalog.frameworks[0].dev = vec![DevVariant {
            command: Some("curl https://x".into()),
            ..Default::default()
        }];
        assert!(
            catalog
                .validate()
                .unwrap_err()
                .contains("not an allowed program")
        );

        let text = BUNDLED.replacen("[[framework]]", "[[framework]]\nshell = \"x\"", 1);
        assert!(toml::from_str::<Catalog>(&text).is_err());
    }

    #[test]
    fn refuses_paths_outside_and_newer_formats() {
        let mut catalog = Catalog::bundled();
        catalog.frameworks[0].detect.any = vec![Rule {
            file: Some("../../etc/passwd".into()),
            ..Default::default()
        }];
        assert!(catalog.validate().is_err());

        let mut catalog = Catalog::bundled();
        catalog.format = FORMAT + 1;
        assert!(catalog.validate().unwrap_err().contains("format"));
        let mut catalog = Catalog::bundled();
        catalog.min_locdev = "99.0".into();
        assert!(catalog.validate().unwrap_err().contains("newer"));
    }

    #[test]
    fn versions_compare() {
        assert!(version_at_least("0.1.0", "0.1"));
        assert!(version_at_least("0.10.0", "0.9.9"));
        assert!(!version_at_least("0.1.0", "0.2"));
    }
}
