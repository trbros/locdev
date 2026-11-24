//! Wire types for the GitHub Actions builder.
//!
//! `Workflow` is the subset of the workflow syntax the visual editor edits.
//! Files using anything else stay editable as YAML (the parser reports why).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Workflow {
    pub name: String,
    pub on: Triggers,
    /// Top-level `permissions` of the GITHUB_TOKEN.
    pub permissions: Option<Permissions>,
    pub concurrency: Option<Concurrency>,
    pub env: BTreeMap<String, String>,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Triggers {
    pub push: Option<Filter>,
    pub pull_request: Option<Filter>,
    pub workflow_dispatch: bool,
    /// Cron expressions (UTC).
    pub schedule: Vec<String>,
    /// `release: types: [published]`.
    pub release: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filter {
    pub branches: Vec<String>,
    pub tags: Vec<String>,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "scopes", rename_all = "snake_case")]
pub enum Permissions {
    ReadAll,
    WriteAll,
    /// `contents: read`, `pages: write`… An empty map is `permissions: {}`.
    Scopes(BTreeMap<String, String>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Concurrency {
    pub group: String,
    pub cancel_in_progress: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Job {
    /// The key under `jobs:`.
    pub id: String,
    pub name: Option<String>,
    pub runs_on: String,
    pub needs: Vec<String>,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub permissions: Option<Permissions>,
    /// A deployment environment (`github-pages`, `production`…).
    pub environment: Option<String>,
    pub timeout_minutes: Option<u32>,
    /// `strategy.matrix`: each key with its values.
    pub matrix: BTreeMap<String, Vec<String>>,
    pub env: BTreeMap<String, String>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Step {
    pub id: Option<String>,
    pub name: Option<String>,
    /// `owner/repo@ref`; exclusive with `run`.
    pub uses: Option<String>,
    pub with: BTreeMap<String, String>,
    pub run: Option<String>,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub working_directory: Option<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    /// Suggested file name under `.github/workflows/`.
    pub file: String,
    /// Fits what was detected in the project.
    pub recommended: bool,
    /// Why it was (or wasn't) recommended.
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExistingWorkflow {
    pub file: String,
    /// The workflow's `name:`, when it parses.
    pub name: Option<String>,
}

/// An action LocDev knows, with the version templates use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionVersion {
    /// `actions/checkout`.
    pub repo: String,
    /// The ref templates use (`v7`).
    pub version: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub level: FindingLevel,
    /// Short rule id: `injection`, `unpinned-action`, `syntax`…
    pub rule: String,
    pub message: String,
    /// 1-based line in the YAML, when known.
    pub line: Option<u32>,
    /// `locdev` or `actionlint`.
    pub source: String,
    /// YAML to add or change, for suggestions with a known fix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Validation {
    pub findings: Vec<Finding>,
    /// actionlint's version when it ran; `None` when it isn't installed.
    pub actionlint: Option<String>,
    /// The YAML as a model, when the visual editor can show it.
    pub workflow: Option<Workflow>,
    /// Why the visual editor can't show it.
    pub unsupported: Option<String>,
}

impl Validation {
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.level == FindingLevel::Error)
    }
}

/// One workflow file of the site, checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowAudit {
    pub file: String,
    #[serde(default)]
    pub name: Option<String>,
    pub findings: Vec<Finding>,
}

/// A local copy of a workflow file, kept by LocDev when it reads a changed
/// file and around every save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowVersion {
    pub id: String,
    /// ms since the epoch.
    pub at: u64,
    /// `read` (changed outside LocDev), `before-save`, `saved`.
    pub reason: String,
    pub size: u64,
}

/// What saving would do, shown before it happens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavePlan {
    /// Relative to the project: `.github/workflows/ci.yml`.
    pub path: String,
    pub exists: bool,
    /// SHA-256 of the current file; pass it back to replace it.
    pub current_hash: Option<String>,
    /// Unified diff (everything added for a new file).
    pub diff: String,
    pub validation: Validation,
}
