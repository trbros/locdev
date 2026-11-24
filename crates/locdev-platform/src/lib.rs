//! Operating-system integration that needs administrator rights.
//!
//! Nothing here runs privileged code on its own: [`resolver`] produces a
//! [`Plan`] (a script the user can read), and [`elevate`] runs it only after
//! the client has shown it and the user agreed. The daemon itself never
//! elevates.

pub mod cli;
pub mod elevate;
pub mod ports;
pub mod resolver;
pub mod service;
pub mod terminal;
pub mod trust;

use serde::Serialize;

/// Marker written into every file LocDev manages, so uninstall only ever
/// touches LocDev's own changes.
pub const MARKER: &str = "Managed by LocDev";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "shell", content = "source", rename_all = "snake_case")]
pub enum Script {
    Sh(String),
    PowerShell(String),
}

impl Script {
    pub fn source(&self) -> &str {
        match self {
            Self::Sh(s) | Self::PowerShell(s) => s,
        }
    }
}

/// A privileged change, described before it is applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Plan {
    /// One-line description for confirmation prompts.
    pub summary: String,
    /// Runs with administrator rights.
    pub script: Script,
    /// Runs afterwards as the current user (e.g. per-user browser stores).
    pub user_script: Option<Script>,
}

impl Plan {
    /// Runs the privileged part through the OS prompt, then the user part.
    pub fn apply(&self, mode: elevate::Mode) -> std::io::Result<()> {
        elevate::run(&self.script, mode)?;
        if let Some(script) = &self.user_script {
            elevate::run_as_user(script)?;
        }
        Ok(())
    }
}
