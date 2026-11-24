//! New projects: the catalog of starters (each framework's own installer or
//! package manager, with the flags it documents), the tools they need, and
//! the plan shown before anything runs.
//!
//! Every command is a program from [`PROGRAMS`] with separate arguments: no
//! shell, no metacharacters, nothing built from free text except the
//! validated project name and package names.

pub mod catalog;
pub mod plan;
pub mod search;
pub mod tools;

pub use catalog::starters;
pub use plan::plan;
pub use tools::tools;

/// The only programs a plan may run (a project-local Python venv is the one
/// exception, checked to stay inside the new folder).
pub const PROGRAMS: &[&str] = &[
    "laravel", "composer", "php", "symfony", "npm", "npx", "pnpm", "bun", "bunx", "rails", "gem",
    "python3", "python", "dotnet", "cargo", "wp", "git",
];

#[cfg(test)]
mod tests;
