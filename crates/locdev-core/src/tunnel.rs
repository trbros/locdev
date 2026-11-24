//! Sharing a site on the internet through a tunnel provider
//! (`locdev-tunnel`, `daemon/tunnels.rs`, `locdev tunnel`). LocDev runs
//! the provider's own program; it never relays traffic itself.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A provider's program found on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelProgram {
    pub version: String,
    pub path: PathBuf,
    /// Downloaded and verified by LocDev (else found on PATH).
    pub managed: bool,
    /// Too old for something LocDev uses (`--allowed-mail`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outdated: Option<String>,
}

/// A tunnel provider and what it takes to use it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelProvider {
    pub id: String,
    pub name: String,
    pub description: String,
    pub docs: String,
    pub license: String,
    /// Needs an account with the provider.
    pub account: bool,
    /// Facts to know before sharing (limits, terms), each from the docs.
    pub limits: Vec<String>,
    #[serde(default)]
    pub program: Option<TunnelProgram>,
    /// LocDev can download and verify it here.
    pub installable: bool,
    /// Why it can't, when it can't (no build for this system, checksums
    /// that don't agree).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TunnelState {
    /// Started, waiting for the provider to hand out the address.
    #[default]
    Starting,
    Online,
    Failed,
    Stopped,
}

/// A site shared on the internet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelView {
    pub site: String,
    pub provider: String,
    pub state: TunnelState,
    /// The public address, once the provider gave it.
    #[serde(default)]
    pub url: Option<String>,
    /// Unix milliseconds.
    #[serde(default)]
    pub started_at: Option<u64>,
    /// Only these e-mail addresses (or `*@domain`) may open it; empty =
    /// anyone with the address.
    #[serde(default)]
    pub allowed: Vec<String>,
    /// The local address it forwards to (`http://127.0.0.1:8100`).
    pub target: String,
    #[serde(default)]
    pub error: Option<String>,
}
