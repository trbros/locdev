//! Framework adapters: declarative definitions (`definitions/frameworks.toml`,
//! updatable as a signed set) evaluated by a native engine that only reads
//! files. Detection, dev commands, versions, monorepo packages and each
//! framework's `.env` rules come from here.

pub mod catalog;
pub mod discover;
pub mod engine;
pub mod info;
mod project;
pub mod registry;
mod workspace;

pub use catalog::{Catalog, Definition, EnvSpec};
pub use engine::{Detection, Overrides, PackageManager, analyze, detect, env_spec};

#[cfg(test)]
mod tests;
