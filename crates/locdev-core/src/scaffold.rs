//! New projects: the starters LocDev can create (each framework's own
//! installer or package manager, with the options it documents), the tools
//! they need, the plan shown before anything runs, and the terminal that
//! runs it. The catalog and planning live in `locdev-scaffold`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// One way to create a project (Laravel, Next.js, Django…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Starter {
    pub id: String,
    pub name: String,
    /// Language family, for grouping: php, javascript, python, ruby, dotnet.
    pub family: String,
    /// The framework definition id, for its icon and detection afterwards.
    pub framework: String,
    pub description: String,
    /// The framework's official guide for creating a project.
    pub docs: String,
    /// Installer or package manager that creates it; at least one.
    pub methods: Vec<StarterMethod>,
    pub options: Vec<StarterOption>,
    /// Packages added after creation come from this registry.
    #[serde(default)]
    pub packages: Option<PackageRegistry>,
    /// The project can be pointed at a LocDev database instance.
    #[serde(default)]
    pub database: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarterMethod {
    pub id: String,
    pub label: String,
    /// The tool (see [`Tool`]) that must be installed.
    pub tool: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarterOption {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub help: Option<String>,
    /// A choice when there are values; a switch ("true"/"false") otherwise.
    #[serde(default)]
    pub choices: Vec<OptionChoice>,
    pub default: String,
    /// Only offered with these methods (all when empty).
    #[serde(default)]
    pub methods: Vec<String>,
    /// Only asked when every condition holds (the installer's own branching,
    /// e.g. Laravel asks for the auth provider only with a starter kit).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<OptionWhen>,
}

/// `option` (its chosen value, or its default) is one of `values`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionWhen {
    pub option: String,
    pub values: Vec<String>,
}

impl Starter {
    /// The value of `id`: the chosen one or the option's default.
    pub fn value<'a>(&'a self, chosen: &'a BTreeMap<String, String>, id: &str) -> &'a str {
        chosen.get(id).map(String::as_str).unwrap_or_else(|| {
            self.options
                .iter()
                .find(|o| o.id == id)
                .map_or("", |o| o.default.as_str())
        })
    }

    /// Whether `option` is asked with `method` and the values chosen so far.
    pub fn asks(
        &self,
        option: &StarterOption,
        method: &str,
        chosen: &BTreeMap<String, String>,
    ) -> bool {
        (option.methods.is_empty() || option.methods.iter().any(|m| m == method))
            && option.when.iter().all(|w| {
                let value = self.value(chosen, &w.option);
                w.values.iter().any(|v| v == value)
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionChoice {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageRegistry {
    Npm,
    Packagist,
}

/// A program a starter needs (an installer, a runtime, a package manager).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Where it comes from, and how to install it by hand.
    pub docs: String,
    /// Found on PATH (or in a known folder), with its version when it says.
    pub installed: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// How LocDev can install it: a command run in the terminal, after the
    /// user sees it, or the Runtimes page for the ones LocDev manages.
    #[serde(default)]
    pub install: Option<ToolInstall>,
    /// Starters that need it.
    #[serde(default)]
    pub used_by: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolInstall {
    /// Run these steps (shown before running).
    Command { steps: Vec<Step> },
    /// Installed from Runtimes (verified downloads).
    Runtime { runtime: String },
}

/// What the user picked in the wizard (or on the CLI).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NewProject {
    pub starter: String,
    pub method: String,
    /// The folder name, also the site's name.
    pub name: String,
    /// The folder it's created in.
    pub parent: PathBuf,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
    /// Packages to add after creating it (from the starter's registry).
    #[serde(default)]
    pub packages: Vec<String>,
    /// A LocDev database instance to create the project's database in.
    #[serde(default)]
    pub database: Option<DatabaseChoice>,
    /// Register the folder as a LocDev site at the end.
    #[serde(default = "yes")]
    pub register: bool,
    /// Node.js version to create it with and pin the site to ("22").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    /// PHP version to create it with and pin the site to ("8.4").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub php: Option<String>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseChoice {
    /// postgres, mysql or mariadb.
    pub service: String,
    /// An existing instance, or a new one with this name.
    pub instance: String,
}

/// One command, run without a shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    /// Where it runs; the project folder unless said otherwise.
    pub cwd: PathBuf,
    /// Variables set for this command only (shown with it).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<(String, String)>,
}

impl Step {
    /// The command as it would be typed, for display only (never run this).
    pub fn display(&self) -> String {
        let quote = |a: &String| {
            if a.is_empty()
                || a.contains(|c: char| c.is_whitespace() || "\"'$&|;<>*?(){}".contains(c))
            {
                format!("'{}'", a.replace('\'', "'\\''"))
            } else {
                a.clone()
            }
        };
        self.env
            .iter()
            .map(|(k, v)| format!("{k}={}", quote(v)))
            .chain(std::iter::once(&self.program).chain(&self.args).map(quote))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A file LocDev writes into the new folder before the steps run (a
/// starter's own files when the framework has no generator). Never over an
/// existing file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedFile {
    /// Relative to the new folder, with `/`.
    pub path: String,
    pub contents: String,
}

/// Everything that will happen, shown before it runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPlan {
    /// The folder that will be created.
    pub dir: PathBuf,
    /// LocDev creates the folder itself before the steps (they run inside it).
    #[serde(default)]
    pub make_dir: bool,
    /// Written into the new folder before the steps.
    #[serde(default)]
    pub files: Vec<PlannedFile>,
    pub steps: Vec<Step>,
    /// What LocDev itself does after `steps` (instance, database, .env).
    #[serde(default)]
    pub actions: Vec<String>,
    /// Run after the actions (migrations against the new database).
    #[serde(default)]
    pub finish: Vec<Step>,
    #[serde(default)]
    pub notes: Vec<String>,
    /// Tools this plan needs that aren't installed.
    #[serde(default)]
    pub missing: Vec<String>,
}

/// A terminal LocDev runs (project creation, a tool install).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalInfo {
    pub id: u64,
    pub title: String,
    pub running: bool,
    /// Exit code of the whole run; `None` while running or when killed.
    #[serde(default)]
    pub code: Option<i32>,
    /// The step running (or that failed), 1-based.
    #[serde(default)]
    pub step: usize,
    pub steps: usize,
    /// The site registered at the end, if any.
    #[serde(default)]
    pub site: Option<String>,
}

/// New output since an offset; the client keeps reading from `next`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalOutput {
    pub info: TerminalInfo,
    /// Raw bytes as UTF-8 (lossy), with the program's ANSI sequences.
    pub data: String,
    pub next: u64,
    /// Older output was dropped (the buffer keeps the last part).
    #[serde(default)]
    pub truncated: bool,
}

/// A package found in a registry search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageHit {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub downloads: Option<u64>,
    #[serde(default)]
    pub url: Option<String>,
}
