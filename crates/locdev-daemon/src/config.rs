//! Global daemon configuration (`config.toml`).

use std::net::Ipv4Addr;
use std::path::Path;

use locdev_core::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};

use crate::fsutil::write_atomic;

const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub ports: PortRange,
    pub proxy: ProxyConfig,
    pub dns: DnsConfig,
    pub tls: TlsConfig,
    pub runtimes: RuntimesConfig,
    pub logs: LogsConfig,
    pub env: EnvConfig,
    pub definitions: DefinitionsConfig,
    pub mail: MailConfig,
    pub deploy: DeployConfig,
    pub updates: UpdatesConfig,
}

/// Where LocDev's releases are read from (the update check).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdatesConfig {
    /// `owner/name` on GitHub.
    pub repo: String,
    /// Another API address (tests): https, or http on loopback only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
}

impl Default for UpdatesConfig {
    fn default() -> Self {
        Self {
            repo: "trbros/locdev".into(),
            api: None,
        }
    }
}

/// Deploy platforms LocDev calls (Vercel, Netlify).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeployConfig {
    /// Where platform tokens are kept (`file`: `deploy-tokens.toml`, `0600`,
    /// for machines without a keyring).
    pub token_store: KeyStore,
    /// Another address for an API (a company proxy, tests): https, or http
    /// on loopback only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vercel_api: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netlify_api: Option<String>,
}

/// LocDev's capture-only SMTP server (loopback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MailConfig {
    pub enabled: bool,
    /// Tried first; the next ones up to `port + 9` when it's taken (Mailpit uses 1025 too).
    pub port: u16,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 1025,
        }
    }
}

/// Where signed framework definitions come from. Both empty: the source
/// compiled into LocDev (`definitions-source.toml`); set both for a fork.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DefinitionsConfig {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub public_key: String,
}

/// Where the key of the secrets vault lives.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyStore {
    /// Secret Service, Keychain or Windows Credential Manager.
    #[default]
    Keyring,
    /// `env-vault.key` (`0600`) next to this file, for machines without a
    /// keyring. The file is then as sensitive as the secrets.
    File,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnvConfig {
    pub key_store: KeyStore,
}

/// How much process output is kept per process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LogsConfig {
    /// Lines kept in memory (and shown by clients).
    pub lines: usize,
    /// Size of each log file on disk before it rotates (one old file kept).
    pub file_mb: u64,
}

impl Default for LogsConfig {
    fn default() -> Self {
        Self {
            lines: 2000,
            file_mb: 5,
        }
    }
}

/// Where PHP builds come from. Empty: the publishers' own servers
/// (StaticPHP on Linux/macOS, windows.php.net on Windows); set the URLs to
/// use a mirror with the same layout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimesConfig {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub php_static_url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub php_windows_url: String,
    /// StaticPHP extension set: `bulk` (default) or `common`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub php_flavor: String,
}

impl RuntimesConfig {
    pub fn php_source(&self) -> locdev_runtimes::PhpSource {
        let mut source = locdev_runtimes::PhpSource::default();
        if !self.php_static_url.trim().is_empty() {
            source.static_url = self.php_static_url.trim().to_string();
        }
        if !self.php_windows_url.trim().is_empty() {
            source.windows_url = self.php_windows_url.trim().to_string();
        }
        if !self.php_flavor.trim().is_empty() {
            source.flavor = self.php_flavor.trim().to_string();
        }
        source
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TlsConfig {
    pub enabled: bool,
    pub https_port: u16,
    /// Used when `https_port` is taken or needs privileges we don't have.
    pub fallback_port: u16,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            https_port: 443,
            fallback_port: 7443,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DnsConfig {
    pub enabled: bool,
    /// Windows NRPT rules cannot name a port, so Windows needs 53.
    pub port: u16,
    /// Loopback addresses to try, in order; the resolver configuration names
    /// the one that bound. On Windows, LocDev's own loopback address is the
    /// fallback when something else already answers on 127.0.0.1:53.
    pub addresses: Vec<Ipv4Addr>,
}

/// LocDev's own loopback address (all of 127.0.0.0/8 is loopback), unlikely
/// to be taken by another DNS tool.
pub const OWN_LOOPBACK: Ipv4Addr = Ipv4Addr::new(127, 53, 0, 1);

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: if cfg!(windows) { 53 } else { 10053 },
            addresses: if cfg!(windows) {
                vec![Ipv4Addr::LOCALHOST, OWN_LOOPBACK]
            } else {
                vec![Ipv4Addr::LOCALHOST]
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxyConfig {
    pub enabled: bool,
    pub http_port: u16,
    /// Used when `http_port` is taken or needs privileges we don't have.
    pub fallback_port: u16,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            http_port: 80,
            fallback_port: 7080,
        }
    }
}

/// Ports handed out to project dev servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            ports: PortRange {
                start: 8100,
                end: 8999,
            },
            proxy: ProxyConfig::default(),
            dns: DnsConfig::default(),
            tls: TlsConfig::default(),
            runtimes: RuntimesConfig::default(),
            logs: LogsConfig::default(),
            env: EnvConfig::default(),
            definitions: DefinitionsConfig::default(),
            mail: MailConfig::default(),
            deploy: DeployConfig::default(),
            updates: UpdatesConfig::default(),
        }
    }
}

impl Config {
    /// Loads the config, creating it with defaults on first run.
    pub fn load_or_create(path: &Path) -> Result<Self, AppError> {
        let config = match std::fs::read_to_string(path) {
            Ok(raw) => toml::from_str::<Self>(&raw).map_err(|e| {
                AppError::new(ErrorCode::Config, format!("{}: {e}", path.display()))
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                let raw = toml::to_string_pretty(&config)
                    .map_err(|e| AppError::internal(e.to_string()))?;
                write_atomic(path, raw.as_bytes())?;
                config
            }
            Err(e) => return Err(e.into()),
        };

        config.validate().map_err(|message| {
            AppError::new(ErrorCode::Config, format!("{}: {message}", path.display()))
        })?;

        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if self.version > CONFIG_VERSION {
            return Err(format!(
                "written by a newer LocDev (format {}, this build reads up to {CONFIG_VERSION})",
                self.version
            ));
        }

        if !(100..=100_000).contains(&self.logs.lines) || !(1..=500).contains(&self.logs.file_mb) {
            return Err("logs.lines must be 100..=100000 and logs.file_mb 1..=500".into());
        }

        let PortRange { start, end } = self.ports;
        if start < 1024 || start > end {
            return Err(format!(
                "ports.start must be >= 1024 and <= ports.end (got {start}..{end})"
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");

        assert_eq!(Config::load_or_create(&file).unwrap(), Config::default());
        assert!(file.exists());
        assert_eq!(Config::load_or_create(&file).unwrap(), Config::default());
    }

    #[test]
    fn rejects_privileged_port_range() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, "[ports]\nstart = 80\nend = 90\n").unwrap();

        assert_eq!(
            Config::load_or_create(&file).unwrap_err().code,
            ErrorCode::Config
        );
    }
}
