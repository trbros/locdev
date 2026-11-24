//! Managed services (databases and caches) as clients see them.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::ProcessInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    Postgres,
    Mariadb,
    Mysql,
    Mongodb,
    Valkey,
    Redis,
    /// A database file; nothing to run.
    Sqlite,
}

impl ServiceKind {
    pub const ALL: [ServiceKind; 7] = [
        Self::Postgres,
        Self::Mariadb,
        Self::Mysql,
        Self::Mongodb,
        Self::Valkey,
        Self::Redis,
        Self::Sqlite,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Postgres => "PostgreSQL",
            Self::Mariadb => "MariaDB",
            Self::Mysql => "MySQL",
            Self::Mongodb => "MongoDB",
            Self::Valkey => "Valkey",
            Self::Redis => "Redis",
            Self::Sqlite => "SQLite",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::Mariadb => "mariadb",
            Self::Mysql => "mysql",
            Self::Mongodb => "mongodb",
            Self::Valkey => "valkey",
            Self::Redis => "redis",
            Self::Sqlite => "sqlite",
        }
    }

    pub fn default_port(self) -> Option<u16> {
        match self {
            Self::Postgres => Some(5432),
            Self::Mariadb | Self::Mysql => Some(3306),
            Self::Mongodb => Some(27017),
            Self::Valkey | Self::Redis => Some(6379),
            Self::Sqlite => None,
        }
    }

    pub fn is_cache(self) -> bool {
        matches!(self, Self::Valkey | Self::Redis)
    }
}

impl std::str::FromStr for ServiceKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|k| k.id() == s || (s == "postgresql" && *k == Self::Postgres) || (s == "mongo" && *k == Self::Mongodb))
            .ok_or_else(|| format!("unknown service `{s}` (postgres, mariadb, mysql, mongodb, valkey, redis, sqlite)"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallSource {
    /// Downloaded and verified by LocDev.
    Managed,
    /// Found on this machine (never modified or removed by LocDev).
    System,
}

/// A version of a service that can run instances.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInstall {
    pub kind: ServiceKind,
    pub version: String,
    pub source: InstallSource,
    /// Where its programs are.
    pub bin_dir: PathBuf,
}

/// A version that can be downloaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceRelease {
    pub kind: ServiceKind,
    pub version: String,
    /// Who publishes the build ("PostgreSQL source built by theseus-rs", "MongoDB, Inc."…).
    pub publisher: String,
    pub url: String,
}

/// One service and what this machine has of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub kind: ServiceKind,
    pub name: String,
    #[serde(default)]
    pub default_port: Option<u16>,
    pub installs: Vec<ServiceInstall>,
    /// Why LocDev can't download it here, when it can't.
    #[serde(default)]
    pub no_download: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    /// Versions the system's package manager offers (installed from the
    /// app's terminal; system installs afterwards).
    #[serde(default)]
    pub packages: Vec<ServicePackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServicePackage {
    pub id: String,
    /// `8.4`, `latest`, `the distribution's`.
    pub version: String,
    /// Homebrew, apt, winget.
    pub manager: String,
    /// Run one after another, each a program and its arguments.
    pub commands: Vec<Vec<String>>,
}

/// A running copy of a service with its own data, port, config and logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub kind: ServiceKind,
    pub version: String,
    pub source: InstallSource,
    pub port: u16,
    #[serde(default)]
    pub autostart: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceView {
    #[serde(flatten)]
    pub instance: Instance,
    pub info: ProcessInfo,
    pub data_dir: PathBuf,
    /// Accepts connections (checked, not assumed from the PID).
    pub ready: bool,
    /// How to connect: a URL and the usual variables per framework.
    pub connection: Vec<(String, String)>,
    #[serde(default)]
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceBackup {
    pub id: String,
    pub instance: String,
    pub at: u64,
    pub size: u64,
    pub path: PathBuf,
}
