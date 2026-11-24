//! PHP settings per version line (`8.3`): what each PHP installation
//! reads, and LocDev's own ini file on top of it. LocDev never edits a
//! php.ini it didn't write; its file is added through PHP_INI_SCAN_DIR to
//! the processes it runs (`locdev-runtimes::php_ini`, `daemon/phpini.rs`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How a directive's value is written and checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectiveKind {
    /// Bytes with an optional K/M/G suffix; `-1` means no limit.
    Size,
    /// Whole number; `-1` allowed where PHP documents it.
    Int,
    /// On/Off.
    Bool,
    /// A timezone identifier (`America/Sao_Paulo`).
    Timezone,
    /// `E_ALL & ~E_DEPRECATED` or a number.
    ErrorLevel,
}

/// One setting LocDev offers in the form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhpDirective {
    pub key: String,
    pub label: String,
    pub kind: DirectiveKind,
    pub description: String,
    /// php.net's page for it.
    pub docs: String,
    /// What PHP reads without LocDev's file (php.ini, its scan folder, or
    /// PHP's built-in default). `None` when PHP doesn't know the directive.
    #[serde(default)]
    pub original: Option<String>,
    /// What PHP reads with LocDev's file.
    #[serde(default)]
    pub effective: Option<String>,
    /// LocDev's value, when it sets one.
    #[serde(default)]
    pub value: Option<String>,
}

/// A saved earlier version of LocDev's file for one line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhpIniBackup {
    pub id: String,
    /// Unix milliseconds.
    pub at: u64,
    pub lines: usize,
}

/// Everything about one PHP version line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhpConfigView {
    /// The version line (`8.3`).
    pub line: String,
    /// The installation probed (the one sites resolve to).
    pub version: String,
    pub executable: PathBuf,
    /// The SAPI LocDev's sites run on (PHP's built-in server is `cli`).
    pub sapi: String,
    /// The php.ini PHP loaded, if any.
    #[serde(default)]
    pub loaded_ini: Option<PathBuf>,
    /// Extra files from PHP's scan folder (`conf.d`), before LocDev's.
    #[serde(default)]
    pub scanned: Vec<PathBuf>,
    /// LocDev's own file for this line, and its contents.
    pub file: PathBuf,
    #[serde(default)]
    pub text: String,
    /// sha256 of `text` (saving the advanced editor needs it).
    pub hash: String,
    pub directives: Vec<PhpDirective>,
    /// Inconsistencies worth knowing (`post_max_size` under the upload size).
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Sites running with this line now.
    #[serde(default)]
    pub sites: Vec<String>,
    #[serde(default)]
    pub history: Vec<PhpIniBackup>,
}

/// A change to LocDev's file for one line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PhpIniChange {
    /// Set (`Some`) or drop (`None`) directives from the form.
    Set {
        values: BTreeMap<String, Option<String>>,
    },
    /// The whole file from the advanced editor; `hash` is the one it was
    /// opened with.
    Text { text: String, hash: String },
    /// Back to a saved version.
    Restore { id: String },
    /// No LocDev settings for this line (PHP's own values again).
    Reset,
}

/// What a change would do, before it's applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhpIniPreview {
    pub before: String,
    pub after: String,
    /// Running sites that would restart to pick it up.
    pub restart: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}
