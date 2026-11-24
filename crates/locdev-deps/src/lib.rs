//! Project dependencies: what is declared, locked and installed (read from
//! disk), what registries publish (npm, Packagist), known advisories (OSV),
//! the compatibility of an update, and the commands that perform one.

pub mod analysis;
pub mod local;
pub mod registry;
pub mod update;
