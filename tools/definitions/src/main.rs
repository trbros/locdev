//! Publishing LocDev's framework definitions (used by the definitions CI
//! workflow, and once by maintainers to create the signing key): build the
//! JSON set from the TOML, sign it, verify it, and watch upstream releases.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use locdev_frameworks::Catalog;
use locdev_frameworks::registry::{Source, verify};

const SOURCE_FILE: &str = "crates/locdev-frameworks/definitions-source.toml";
const DEFINITIONS: &str = "crates/locdev-frameworks/definitions/frameworks.toml";
const UPSTREAM: &str = "crates/locdev-frameworks/definitions/upstream.json";

#[derive(Parser)]
#[command(about = "Build, sign and verify LocDev's framework definitions")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create the signing key pair (once, by a maintainer).
    Keygen {
        /// Public GitHub repository whose `definitions` release holds the set, as owner/name.
        #[arg(long)]
        repo: String,
        #[arg(long, default_value = ".definitions-keys")]
        out: PathBuf,
    },
    /// Validate frameworks.toml and write the JSON set that gets published.
    Build {
        #[arg(long, default_value = "dist/frameworks.json")]
        out: PathBuf,
    },
    /// Sign a built set with the secret key from an environment variable.
    Sign {
        file: PathBuf,
        #[arg(long, default_value = "DEFINITIONS_SIGNING_KEY")]
        key_env: String,
    },
    /// Check a set and its .minisig against the public key LocDev ships.
    Verify { file: PathBuf },
    /// Ask the package registries for each framework's latest release and
    /// report new major versions since upstream.json (exit code 2).
    Upstream {
        /// Rewrite upstream.json with what was found.
        #[arg(long)]
        update: bool,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Keygen { repo, out } => keygen(&repo, &out),
        Command::Build { out } => {
            let text =
                std::fs::read_to_string(DEFINITIONS).context("run from the repository root")?;
            let catalog: Catalog = toml::from_str(&text)?;
            catalog.validate().map_err(anyhow::Error::msg)?;
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&out, serde_json::to_vec(&catalog)?)?;
            println!(
                "{} definitions, revision {} → {}",
                catalog.frameworks.len(),
                catalog.revision,
                out.display()
            );
            Ok(())
        }
        Command::Sign { file, key_env } => {
            let secret =
                std::env::var(&key_env).with_context(|| format!("{key_env} is not set"))?;
            let key = minisign::SecretKey::from_unencrypted_box(
                minisign::SecretKeyBox::from_string(&secret)?,
            )?;
            let bytes = std::fs::read(&file)?;
            let signature = minisign::sign(None, &key, std::io::Cursor::new(&bytes), None, None)?;
            let mut sig = file.as_os_str().to_owned();
            sig.push(".minisig");
            std::fs::write(PathBuf::from(sig), signature.to_string())?;
            println!("signed {}", file.display());
            Ok(())
        }
        Command::Verify { file } => {
            let source = Source::bundled();
            let mut sig = file.as_os_str().to_owned();
            sig.push(".minisig");
            let signature = std::fs::read_to_string(PathBuf::from(sig))
                .context("no .minisig next to the file")?;
            let catalog = verify(&std::fs::read(&file)?, &signature, &source.public_key)
                .map_err(anyhow::Error::msg)?;
            println!(
                "ok: revision {}, {} definitions",
                catalog.revision,
                catalog.frameworks.len()
            );
            Ok(())
        }
        Command::Upstream { update } => upstream(update),
    }
}

fn keygen(repo: &str, out: &Path) -> anyhow::Result<()> {
    if !repo.contains('/') {
        bail!("--repo must look like owner/name");
    }
    let secret_path = out.join("definitions.key");
    if secret_path.exists() {
        bail!(
            "{} already exists; refusing to replace a signing key",
            secret_path.display()
        );
    }
    let keys = minisign::KeyPair::generate_unencrypted_keypair()?;
    let secret = keys
        .sk
        .to_box(Some("LocDev framework definitions signing key"))?
        .into_string();
    std::fs::create_dir_all(out)?;
    write_private(&secret_path, secret.as_bytes())?;
    let public_key = keys.pk.to_base64();
    let url = format!("https://github.com/{repo}/releases/download/definitions/frameworks.json");
    std::fs::write(
        SOURCE_FILE,
        format!(
            "# Where LocDev's framework definitions are published, and the minisign\n\
             # public key that signs them. Generated by `cargo run -p definitions -- keygen`.\n\
             url = \"{url}\"\n\
             public_key = \"{public_key}\"\n"
        ),
    )
    .with_context(|| format!("run this from the repository root (writing {SOURCE_FILE})"))?;
    println!("public key {public_key}\nurl        {url}\nupdated    {SOURCE_FILE}\n");
    println!(
        "Secret key written to {} (mode 0600). Next:",
        secret_path.display()
    );
    println!(
        "  1. gh secret set DEFINITIONS_SIGNING_KEY < {}",
        secret_path.display()
    );
    println!(
        "  2. keep an offline backup; losing it means re-keying, which needs a new LocDev release."
    );
    Ok(())
}

fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(contents)
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, contents)
    }
}

/// Latest stable version of a package, by registry.
fn latest(agent: &ureq::Agent, registry: &str, name: &str) -> anyhow::Result<String> {
    let get = |url: String| -> anyhow::Result<serde_json::Value> {
        Ok(agent
            .get(&url)
            .header(
                "User-Agent",
                "locdev-definitions (https://github.com/trbros/locdev)",
            )
            .call()?
            .body_mut()
            .read_json()?)
    };
    let value = match registry {
        "npm" => get(format!("https://registry.npmjs.org/{name}/latest"))?["version"].clone(),
        "composer" => {
            let json = get(format!("https://repo.packagist.org/p2/{name}.json"))?;
            json["packages"][name]
                .as_array()
                .and_then(|versions| {
                    versions
                        .iter()
                        .filter_map(|v| v["version_normalized"].as_str())
                        .find(|v| !v.contains('-'))
                        .map(|v| serde_json::Value::String(v.to_string()))
                })
                .unwrap_or_default()
        }
        "python" => get(format!("https://pypi.org/pypi/{name}/json"))?["info"]["version"].clone(),
        "gem" => get(format!(
            "https://rubygems.org/api/v1/versions/{name}/latest.json"
        ))?["version"]
            .clone(),
        _ => bail!("no registry for {registry}"),
    };
    value
        .as_str()
        .map(str::to_string)
        .context("no version in the response")
}

fn upstream(update: bool) -> anyhow::Result<()> {
    let catalog = Catalog::bundled_ref();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build()
        .into();
    let known: BTreeMap<String, String> = std::fs::read(UPSTREAM)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let mut found = BTreeMap::new();
    let mut new_majors = Vec::new();
    for def in &catalog.frameworks {
        let Some(rule) = def.version.first() else {
            continue;
        };
        let (registry, name) = match (&rule.npm, &rule.composer, &rule.python, &rule.gem) {
            (Some(n), ..) => ("npm", n),
            (_, Some(n), ..) => ("composer", n),
            (_, _, Some(n), _) => ("python", n),
            (.., Some(n)) => ("gem", n),
            _ => continue,
        };
        match latest(&agent, registry, name) {
            Ok(version) => {
                let major = |v: &str| v.split('.').next().unwrap_or_default().to_string();
                let previous = known.get(&def.id);
                let flag = match previous {
                    Some(p) if major(p) != major(&version) => {
                        new_majors.push(format!("{} {} → {version}", def.id, p));
                        "NEW MAJOR"
                    }
                    None => "new",
                    _ => "",
                };
                println!(
                    "{:<12} {registry:<9} {name:<28} {version:<14} {flag}",
                    def.id
                );
                found.insert(def.id.clone(), version);
            }
            Err(error) => println!("{:<12} {registry:<9} {name:<28} error: {error}", def.id),
        }
    }
    if update {
        std::fs::write(UPSTREAM, serde_json::to_string_pretty(&found)? + "\n")?;
        println!("updated {UPSTREAM}");
    }
    if !new_majors.is_empty() && !update {
        println!(
            "\nNew major versions; review their definitions:\n  {}",
            new_majors.join("\n  ")
        );
        std::process::exit(2);
    }
    Ok(())
}
