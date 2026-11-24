pub mod actions;
pub mod config;
pub mod db;
pub mod deploy;
pub mod deps;
pub mod detect;
pub mod domain;
pub mod editor;
pub mod email;
pub mod env;
pub mod error;
pub mod frameworks;
pub mod git;
pub mod host;
pub mod mail;
pub mod metrics;
pub mod model;
pub mod observe;
pub mod park;
pub mod paths;
pub mod phpini;
pub mod process;
pub mod protocol;
pub mod remote;
pub mod scaffold;
pub mod semver;
pub mod services;
pub mod subdomains;
pub mod tunnel;
pub mod update;

pub use domain::{DomainError, project_domain, project_name, slugify};
pub use error::{AppError, ErrorCode};

/// Top-level domain used for every local site.
pub const TLD: &str = "test";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
