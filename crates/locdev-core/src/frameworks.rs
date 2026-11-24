//! Framework adapters as clients see them: the definitions in use, where
//! they came from, and one project's stack with what it still needs.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::{ComponentCategory, Integration, Stack};

/// One framework definition, summarized for lists and docs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionInfo {
    pub id: String,
    pub name: String,
    pub category: ComponentCategory,
    /// What LocDev does with it at best (some projects may still need a command).
    pub integration: Integration,
    pub docs: String,
    #[serde(default)]
    pub runtimes: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    /// The dev commands it knows, as templates (`{port}`), in order.
    #[serde(default)]
    pub dev: Vec<String>,
    #[serde(default)]
    pub install: Option<String>,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub test: Option<String>,
    #[serde(default)]
    pub supported: Option<String>,
    #[serde(default)]
    pub config_files: Vec<String>,
    /// Service ids commonly used with it (managed services come in a later version).
    #[serde(default)]
    pub services: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionsOrigin {
    /// Compiled into this LocDev.
    #[default]
    Bundled,
    /// Downloaded, signature verified.
    Downloaded,
}

/// Where the definitions in use come from and how the last update went.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RegistryStatus {
    pub origin: DefinitionsOrigin,
    pub revision: u64,
    pub bundled_revision: u64,
    pub format: u32,
    pub count: usize,
    pub url: String,
    /// Unix ms of the last check, successful or not.
    #[serde(default)]
    pub checked_at: Option<u64>,
    #[serde(default)]
    pub error: Option<String>,
    /// A previous downloaded set to roll back to.
    #[serde(default)]
    pub previous: Option<u64>,
    /// A revision the user rolled back from; automatic updates skip it.
    #[serde(default)]
    pub skipped: Option<u64>,
}

/// A program a project's command needs, and whether it's on PATH.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prerequisite {
    pub program: String,
    #[serde(default)]
    pub found: Option<PathBuf>,
}

/// One project's stack, explained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackReport {
    /// The site, or the folder when it isn't registered.
    pub target: String,
    pub path: PathBuf,
    pub stack: Stack,
    /// The command the site runs with now (registered sites).
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub prerequisites: Vec<Prerequisite>,
    /// The primary adapter's definition.
    #[serde(default)]
    pub definition: Option<DefinitionInfo>,
    /// `install`/`build`/`test` with the project's package manager filled in.
    #[serde(default)]
    pub install: Option<String>,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub test: Option<String>,
    /// Components the user added or hid, and the adapter they chose.
    #[serde(default)]
    pub adapter_override: Option<String>,
    #[serde(default)]
    pub added: Vec<String>,
    #[serde(default)]
    pub hidden: Vec<String>,
}
