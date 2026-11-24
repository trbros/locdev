//! Deploy assistant: what a project needs from a host, which platforms fit
//! and why, readiness checks and, where the format is documented and stable,
//! a config file to start from. Recommendations and preparation only:
//! nothing is sent to any platform, and a written file is never overwritten.

pub mod checks;
pub mod fit;
pub mod generate;
pub mod profile;
pub mod providers;
pub mod remote;

use std::io::Write;

use locdev_core::deploy::{DeployConfig, DeployReport};
use sha2::{Digest, Sha256};

pub use profile::Inputs;

fn hash(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn framework_id(inputs: &Inputs) -> Option<String> {
    let stack = inputs.stack;
    stack
        .components
        .iter()
        .find(|c| c.primary)
        .or_else(|| {
            stack
                .components
                .iter()
                .find(|c| c.category == locdev_core::model::ComponentCategory::Framework)
        })
        .map(|c| c.id.clone())
}

pub fn analyze(inputs: &Inputs) -> DeployReport {
    let files = profile::Files::read(inputs.root);
    let profile = profile::profile(inputs, &files);
    let id = framework_id(inputs);
    let fits = providers::PROVIDERS
        .iter()
        .map(|p| fit::fit(p, &profile, id.as_deref()))
        .collect();
    DeployReport {
        checks: checks::run(inputs.root, &profile, &files),
        providers: fit::rank(fits),
        reviewed: providers::REVIEWED.into(),
        profile,
    }
}

/// The config file LocDev would write for `provider`.
pub fn config(inputs: &Inputs, provider: &str) -> Result<DeployConfig, String> {
    let files = profile::Files::read(inputs.root);
    let profile = profile::profile(inputs, &files);
    let generated = match provider {
        "netlify" => generate::netlify(&profile)?,
        "render" => generate::render(&profile, inputs.name)?,
        other => {
            let p = providers::PROVIDERS
                .iter()
                .find(|p| p.id == other)
                .ok_or_else(|| format!("no platform {other}"))?;
            return Err(match p.setup {
                Some(setup) => {
                    format!("LocDev doesn't write a config file for {}: {setup}", p.name)
                }
                None => format!("LocDev doesn't write a config file for {}", p.name),
            });
        }
    };
    Ok(DeployConfig {
        provider: provider.into(),
        file: generated.file.into(),
        hash: hash(generated.content.as_bytes()),
        exists: inputs.root.join(generated.file).exists(),
        content: generated.content,
        notes: generated.notes,
    })
}

/// Writes the config the user reviewed (`expected` is its hash). The daemon
/// generates it again; an existing file is never replaced.
pub fn write(inputs: &Inputs, provider: &str, expected: &str) -> Result<String, String> {
    let config = config(inputs, provider)?;
    if config.hash != expected {
        return Err("the project changed since you reviewed the file; review it again".into());
    }
    let path = inputs.root.join(&config.file);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => format!(
                "{} already exists; LocDev doesn't overwrite it",
                config.file
            ),
            _ => format!("{}: {e}", config.file),
        })?;
    file.write_all(config.content.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("{}: {e}", config.file))?;
    Ok(config.file)
}

#[cfg(test)]
mod tests;
