//! Updating the definitions without a new LocDev: a signed JSON set,
//! published by `tools/definitions`, verified with the minisign key compiled
//! in (`definitions-source.toml`) and validated before use.
//!
//! - Only a newer revision replaces the active set; a revision older than
//!   the newest ever seen is refused (no downgrade by replaying old files).
//! - The previous downloaded set is kept for a manual rollback; the revision
//!   rolled back from is skipped by automatic checks.
//! - A missing, tampered or incompatible file falls back to the bundled set.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use locdev_core::frameworks::{DefinitionsOrigin, RegistryStatus};
use serde::{Deserialize, Serialize};

use crate::catalog::{Catalog, FORMAT};

/// Where definitions are published and the key that signs them.
pub const SOURCE: &str = include_str!("../definitions-source.toml");

/// Larger downloads are refused.
const MAX_DOWNLOAD: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Source {
    pub url: String,
    pub public_key: String,
}

impl Source {
    pub fn bundled() -> Self {
        toml::from_str(SOURCE).expect("definitions-source.toml parses")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Meta {
    checked_at: Option<u64>,
    error: Option<String>,
    /// The newest revision ever accepted.
    max_seen: u64,
    skip_revision: Option<u64>,
}

struct State {
    active: Arc<Catalog>,
    origin: DefinitionsOrigin,
    meta: Meta,
}

pub struct Registry {
    dir: PathBuf,
    source: Source,
    state: Mutex<State>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Updated { from: u64, to: u64 },
    UpToDate,
    Skipped(u64),
}

/// Checks `signature` over the exact bytes, then parses and validates them.
pub fn verify(bytes: &[u8], signature: &str, public_key: &str) -> Result<Catalog, String> {
    let key = minisign_verify::PublicKey::from_base64(public_key.trim())
        .map_err(|e| format!("bad public key: {e}"))?;
    let signature =
        minisign_verify::Signature::decode(signature).map_err(|e| format!("bad signature: {e}"))?;
    key.verify(bytes, &signature, false)
        .map_err(|_| "the signature doesn't match the definitions".to_string())?;
    let catalog: Catalog =
        serde_json::from_slice(bytes).map_err(|e| format!("unreadable definitions: {e}"))?;
    catalog.validate()?;
    Ok(catalog)
}

fn sig_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".minisig");
    PathBuf::from(name)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)
}

fn now_ms() -> u64 {
    locdev_core::model::now_ms()
}

impl Registry {
    /// The active set from `dir` (`current.json` + `.minisig`), or the
    /// bundled one when there is none or it doesn't verify.
    pub fn load(dir: &Path, source: Source) -> Self {
        let _ = std::fs::create_dir_all(dir);
        let mut meta: Meta = std::fs::read(dir.join("meta.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let bundled = Catalog::bundled_ref().clone();
        let (active, origin) = match Self::read_verified(&dir.join("current.json"), &source) {
            Some(Ok(set)) if set.revision >= bundled.revision => {
                (set, DefinitionsOrigin::Downloaded)
            }
            // This build already ships something at least as new.
            Some(Ok(_)) | None => (bundled, DefinitionsOrigin::Bundled),
            Some(Err(error)) => {
                meta.error = Some(format!("the downloaded definitions were not used: {error}"));
                (bundled, DefinitionsOrigin::Bundled)
            }
        };
        meta.max_seen = meta.max_seen.max(active.revision);
        Self {
            dir: dir.to_path_buf(),
            source,
            state: Mutex::new(State {
                active: Arc::new(active),
                origin,
                meta,
            }),
        }
    }

    fn read_verified(path: &Path, source: &Source) -> Option<Result<Catalog, String>> {
        let bytes = std::fs::read(path).ok()?;
        let signature = match std::fs::read_to_string(sig_path(path)) {
            Ok(s) => s,
            Err(_) => return Some(Err("its signature file is missing".into())),
        };
        Some(verify(&bytes, &signature, &source.public_key))
    }

    pub fn active(&self) -> Arc<Catalog> {
        self.state.lock().unwrap().active.clone()
    }

    pub fn status(&self) -> RegistryStatus {
        let state = self.state.lock().unwrap();
        let previous = Self::read_verified(&self.dir.join("previous.json"), &self.source)
            .and_then(Result::ok)
            .map(|c| c.revision);
        RegistryStatus {
            origin: state.origin,
            revision: state.active.revision,
            bundled_revision: Catalog::bundled_ref().revision,
            format: FORMAT,
            count: state.active.frameworks.len(),
            url: self.source.url.clone(),
            checked_at: state.meta.checked_at,
            error: state.meta.error.clone(),
            previous,
            skipped: state.meta.skip_revision,
        }
    }

    fn save_meta(&self, meta: &Meta) {
        if let Ok(json) = serde_json::to_vec_pretty(meta) {
            let _ = write_atomic(&self.dir.join("meta.json"), &json);
        }
    }

    /// Records the outcome of a check without changing the active set.
    fn note(&self, error: Option<String>) {
        let mut state = self.state.lock().unwrap();
        state.meta.checked_at = Some(now_ms());
        state.meta.error = error;
        let meta = state.meta.clone();
        drop(state);
        self.save_meta(&meta);
    }

    /// Downloads and applies a newer set. Blocking: call from a worker.
    /// `force` also takes a revision the user rolled back from.
    pub fn refresh(&self, force: bool) -> Result<Outcome, String> {
        let result = (|| {
            let bytes = download(&self.source.url)?;
            let signature = String::from_utf8(download(&format!("{}.minisig", self.source.url))?)
                .map_err(|_| "the signature isn't text".to_string())?;
            let set = verify(&bytes, &signature, &self.source.public_key)?;
            self.apply(bytes, signature, set, force)
        })();
        match result {
            Err(error) if error == NOT_PUBLISHED => {
                self.note(None);
                Ok(Outcome::UpToDate)
            }
            Ok(outcome) => {
                self.note(None);
                Ok(outcome)
            }
            Err(error) => {
                self.note(Some(error.clone()));
                Err(error)
            }
        }
    }

    /// Applies an already verified set (also used by tests).
    fn apply(
        &self,
        bytes: Vec<u8>,
        signature: String,
        set: Catalog,
        force: bool,
    ) -> Result<Outcome, String> {
        let mut state = self.state.lock().unwrap();
        let from = state.active.revision;
        if set.revision <= from {
            return Ok(Outcome::UpToDate);
        }
        if set.revision < state.meta.max_seen {
            return Err(format!(
                "revision {} is older than {} already seen; refusing a downgrade",
                set.revision, state.meta.max_seen
            ));
        }
        if !force && state.meta.skip_revision == Some(set.revision) {
            return Ok(Outcome::Skipped(set.revision));
        }
        let current = self.dir.join("current.json");
        if state.origin == DefinitionsOrigin::Downloaded {
            let _ = std::fs::rename(&current, self.dir.join("previous.json"));
            let _ = std::fs::rename(
                sig_path(&current),
                sig_path(&self.dir.join("previous.json")),
            );
        }
        write_atomic(&sig_path(&current), signature.as_bytes()).map_err(|e| e.to_string())?;
        write_atomic(&current, &bytes).map_err(|e| e.to_string())?;
        state.meta.max_seen = state.meta.max_seen.max(set.revision);
        state.meta.skip_revision = None;
        let to = set.revision;
        state.active = Arc::new(set);
        state.origin = DefinitionsOrigin::Downloaded;
        let meta = state.meta.clone();
        drop(state);
        self.save_meta(&meta);
        Ok(Outcome::Updated { from, to })
    }

    /// Goes back to the previous downloaded set, or to the bundled one.
    /// The revision left behind is skipped by automatic updates.
    pub fn rollback(&self) -> Result<RegistryStatus, String> {
        let mut state = self.state.lock().unwrap();
        if state.origin == DefinitionsOrigin::Bundled {
            return Err("the bundled definitions are in use; there is nothing to roll back".into());
        }
        let left = state.active.revision;
        let current = self.dir.join("current.json");
        let previous = self.dir.join("previous.json");
        let restored = match Self::read_verified(&previous, &self.source) {
            Some(Ok(set)) if set.revision >= Catalog::bundled_ref().revision => {
                std::fs::rename(&previous, &current).map_err(|e| e.to_string())?;
                std::fs::rename(sig_path(&previous), sig_path(&current))
                    .map_err(|e| e.to_string())?;
                Some(set)
            }
            _ => {
                let _ = std::fs::remove_file(&current);
                let _ = std::fs::remove_file(sig_path(&current));
                None
            }
        };
        match restored {
            Some(set) => {
                state.active = Arc::new(set);
                state.origin = DefinitionsOrigin::Downloaded;
            }
            None => {
                state.active = Arc::new(Catalog::bundled_ref().clone());
                state.origin = DefinitionsOrigin::Bundled;
            }
        }
        state.meta.skip_revision = Some(left);
        let meta = state.meta.clone();
        drop(state);
        self.save_meta(&meta);
        Ok(self.status())
    }
}

/// The update address answers 404: no set was published (yet). Not a
/// failure: the bundled definitions stay in use.
const NOT_PUBLISHED: &str = "no definitions published";

fn download(url: &str) -> Result<Vec<u8>, String> {
    if !url.starts_with("https://") {
        return Err("definitions are only fetched over HTTPS".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(|e| match e {
        ureq::Error::StatusCode(404) => NOT_PUBLISHED.to_string(),
        e => format!("{url}: {e}"),
    })?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .read_to_vec()
        .map_err(|e| format!("{url}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Signer {
        secret: minisign::SecretKey,
        public: String,
    }

    impl Signer {
        fn new() -> Self {
            let keys = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
            Self {
                public: keys.pk.to_base64(),
                secret: keys.sk,
            }
        }

        fn sign(&self, bytes: &[u8]) -> String {
            minisign::sign(None, &self.secret, std::io::Cursor::new(bytes), None, None)
                .unwrap()
                .to_string()
        }

        fn source(&self) -> Source {
            Source {
                url: "https://example.invalid/frameworks.json".into(),
                public_key: self.public.clone(),
            }
        }
    }

    fn set(revision: u64) -> (Vec<u8>, Catalog) {
        let mut catalog = Catalog::bundled_ref().clone();
        catalog.revision = revision;
        catalog.frameworks[0]
            .notes
            .push(format!("revision {revision}"));
        (serde_json::to_vec(&catalog).unwrap(), catalog)
    }

    #[test]
    fn verifies_and_rejects_tampering() {
        let signer = Signer::new();
        let (bytes, _) = set(Catalog::bundled_ref().revision + 1);
        let signature = signer.sign(&bytes);
        assert!(verify(&bytes, &signature, &signer.public).is_ok());

        let mut tampered = bytes.clone();
        let i = tampered.len() / 2;
        tampered[i] ^= 1;
        assert!(verify(&tampered, &signature, &signer.public).is_err());
        assert!(verify(&bytes, &signature, &Signer::new().public).is_err());
    }

    #[test]
    fn a_signed_set_that_breaks_the_rules_is_refused() {
        let signer = Signer::new();
        let mut catalog = Catalog::bundled_ref().clone();
        catalog.frameworks[0].dev[0].command = Some("sh -c id".into());
        let bytes = serde_json::to_vec(&catalog).unwrap();
        let error = verify(&bytes, &signer.sign(&bytes), &signer.public).unwrap_err();
        assert!(error.contains("not an allowed program"), "{error}");
    }

    #[test]
    fn update_downgrade_rollback_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let signer = Signer::new();
        let base = Catalog::bundled_ref().revision;
        let registry = Registry::load(dir.path(), signer.source());
        assert_eq!(registry.status().origin, DefinitionsOrigin::Bundled);

        let apply = |registry: &Registry, revision: u64, force: bool| {
            let (bytes, catalog) = set(revision);
            let signature = signer.sign(&bytes);
            registry.apply(bytes, signature, catalog, force)
        };
        assert_eq!(
            apply(&registry, base + 1, false),
            Ok(Outcome::Updated {
                from: base,
                to: base + 1
            })
        );
        assert_eq!(
            apply(&registry, base + 2, false),
            Ok(Outcome::Updated {
                from: base + 1,
                to: base + 2
            })
        );
        assert_eq!(apply(&registry, base + 2, false), Ok(Outcome::UpToDate));
        assert_eq!(registry.status().previous, Some(base + 1));

        // Survives a restart, signature re-checked.
        let reloaded = Registry::load(dir.path(), signer.source());
        assert_eq!(reloaded.active().revision, base + 2);
        // A different key: the files aren't trusted, the bundled set is used.
        let stranger = Registry::load(dir.path(), Signer::new().source());
        assert_eq!(stranger.status().origin, DefinitionsOrigin::Bundled);
        assert!(stranger.status().error.is_some());

        // Rolling back goes to the previous set and skips the bad revision…
        let status = registry.rollback().unwrap();
        assert_eq!(status.revision, base + 1);
        assert_eq!(status.skipped, Some(base + 2));
        assert_eq!(
            apply(&registry, base + 2, false),
            Ok(Outcome::Skipped(base + 2))
        );
        // …unless forced; an older replay is refused.
        assert_eq!(
            apply(&registry, base + 2, true),
            Ok(Outcome::Updated {
                from: base + 1,
                to: base + 2
            })
        );
        assert!(apply(&registry, base + 1, false).is_ok_and(|o| o == Outcome::UpToDate));

        // Back to bundled when there's no previous set.
        registry.rollback().unwrap();
        let status = registry.rollback().unwrap();
        assert_eq!(status.origin, DefinitionsOrigin::Bundled);
        assert!(registry.rollback().is_err());
    }

    #[test]
    fn only_https() {
        assert!(download("http://example.com/x").is_err());
    }
}
