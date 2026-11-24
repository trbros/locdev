//! Config Files as clients see them: the configuration files a project
//! already has, the options LocDev can edit visually, and findings.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFormat {
    /// Strict JSON (`package.json`, `composer.json`).
    Json,
    /// JSON with comments and trailing commas (`tsconfig.json`, `biome.jsonc`).
    Jsonc,
    /// JavaScript or TypeScript module; read statically, never executed.
    Js,
    Xml,
    Toml,
    Yaml,
    Css,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigFile {
    /// Relative to the project root, `/`-separated.
    pub path: String,
    /// Catalog id: `vite`, `next`, `tsconfig`…
    pub kind: String,
    pub label: String,
    pub format: ConfigFormat,
    /// The package whose installed version decides which options apply.
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    pub docs: String,
    /// Has options the form can show.
    pub visual: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionKind {
    Bool,
    Number,
    String,
    /// One of `choices`.
    Choice,
}

/// What a file says for an option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum OptionState {
    /// A literal value.
    Set {
        value: Value,
    },
    /// Computed in code (a variable, a call, an expression): shown, not edited.
    Code {
        code: String,
    },
    Missing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigOption {
    /// Dotted path: `server.port`, `compilerOptions.strict`.
    pub key: String,
    pub section: String,
    pub label: String,
    pub description: String,
    pub kind: OptionKind,
    #[serde(default)]
    pub choices: Vec<Value>,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub example: Option<String>,
    #[serde(default)]
    pub docs: Option<String>,
    /// Package versions the option exists in (`>= 15.5`).
    #[serde(default)]
    pub versions: Option<String>,
    pub state: OptionState,
    pub editable: bool,
    /// Why it can't be edited here (version, code, file shape).
    #[serde(default)]
    pub blocked: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigFileView {
    pub file: ConfigFile,
    pub content: String,
    /// SHA-256 of the content; a save must name it.
    pub hash: String,
    pub options: Vec<ConfigOption>,
    /// Read-only facts worth showing (`plugins: react(), tailwindcss()`).
    #[serde(default)]
    pub notes: Vec<String>,
    /// Why the form is unavailable (dynamic config, parse error…).
    #[serde(default)]
    pub visual_blocked: Option<String>,
    #[serde(default)]
    pub syntax_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConfigChange {
    /// One option, through the parser (comments and formatting kept).
    Set { key: String, value: Value },
    /// The whole text, from the code editor.
    Text { content: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ConfigPreview {
    pub content: String,
    /// Unified diff against the file as it is.
    pub diff: String,
    #[serde(default)]
    pub syntax_error: Option<String>,
    /// Lines with comments that the change removes.
    #[serde(default)]
    pub removed_comments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigHistoryEntry {
    pub id: String,
    /// Unix milliseconds of the save that replaced this content.
    pub at: u64,
    pub size: u64,
    /// What replaced it: `set server.port`, `text edit`, `revert`.
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    Incompatible,
    Deprecated,
    Security,
    Performance,
    Development,
    Available,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigFinding {
    /// Stable rule id (`next.images-domains`).
    pub rule: String,
    pub kind: FindingKind,
    pub file: String,
    #[serde(default)]
    pub key: Option<String>,
    pub title: String,
    /// Why, in a sentence or two.
    pub detail: String,
    /// The versions the rule is about (`next >= 14`), with what's installed.
    pub applies: String,
    pub source: String,
    /// An option change that addresses it; never applied without asking.
    #[serde(default)]
    pub fix: Option<(String, Value)>,
}
