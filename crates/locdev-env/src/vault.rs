//! Secret values, encrypted at rest. Each value is sealed with
//! ChaCha20-Poly1305 under a random 256-bit key and bound to its id (the
//! associated data), so a sealed value can't be moved to another key.
//!
//! The key lives in the system keyring (Secret Service, Keychain, Windows
//! Credential Manager). A key file (`0600`) is used only when the user picks
//! it in `config.toml` (`[env] key_store = "file"`), for machines without a
//! keyring; then the file is as sensitive as the secrets themselves.
//!
//! A missing key with sealed values present is an error, never a reason to
//! make a new key: that would orphan every secret.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ring::aead::{Aad, CHACHA20_POLY1305, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

use crate::{from_hex, hex, write_private};

const FORMAT: u32 = 1;
const KEYRING_SERVICE: &str = "dev.locdev.locdev";
const KEYRING_USER: &str = "env-vault";

/// Where the vault key is kept.
pub trait KeySource: Send + Sync {
    fn load(&self) -> Result<Option<[u8; 32]>, String>;
    fn store(&self, key: &[u8; 32]) -> Result<(), String>;
    fn label(&self) -> &'static str;
}

pub struct KeyringKey {
    user: String,
}

impl KeyringKey {
    /// The vault's own entry.
    pub fn vault() -> Self {
        Self {
            user: KEYRING_USER.into(),
        }
    }

    fn entry(&self) -> Result<keyring::Entry, String> {
        keyring::Entry::new(KEYRING_SERVICE, &self.user).map_err(|e| e.to_string())
    }
}

impl KeySource for KeyringKey {
    fn load(&self) -> Result<Option<[u8; 32]>, String> {
        let entry = self.entry()?;
        match entry.get_password() {
            Ok(raw) => decode_key(&raw).map(Some),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn store(&self, key: &[u8; 32]) -> Result<(), String> {
        self.entry()?
            .set_password(&hex(key))
            .map_err(|e| e.to_string())
    }

    fn label(&self) -> &'static str {
        "system keyring"
    }
}

pub struct FileKey(pub PathBuf);

impl KeySource for FileKey {
    fn load(&self) -> Result<Option<[u8; 32]>, String> {
        match std::fs::read_to_string(&self.0) {
            Ok(raw) => {
                check_private(&self.0)?;
                decode_key(raw.trim()).map(Some)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn store(&self, key: &[u8; 32]) -> Result<(), String> {
        write_private(&self.0, hex(key).as_bytes()).map_err(|e| e.to_string())
    }

    fn label(&self) -> &'static str {
        "key file"
    }
}

/// For tests: a key that lives only in memory.
#[derive(Default)]
pub struct MemoryKey(pub Mutex<Option<[u8; 32]>>);

impl KeySource for MemoryKey {
    fn load(&self) -> Result<Option<[u8; 32]>, String> {
        Ok(*self.0.lock().unwrap())
    }
    fn store(&self, key: &[u8; 32]) -> Result<(), String> {
        *self.0.lock().unwrap() = Some(*key);
        Ok(())
    }
    fn label(&self) -> &'static str {
        "memory"
    }
}

fn decode_key(raw: &str) -> Result<[u8; 32], String> {
    from_hex(raw)
        .and_then(|b| <[u8; 32]>::try_from(b).ok())
        .ok_or_else(|| "the stored vault key is malformed".to_string())
}

#[cfg(unix)]
fn check_private(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.mode() & 0o077 != 0 {
        return Err(format!(
            "{} must only be readable by you (chmod 600)",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_private(_: &Path) -> Result<(), String> {
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sealed {
    /// Nonce, hex.
    n: String,
    /// Ciphertext with its tag, hex.
    c: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct VaultFile {
    format: u32,
    entries: BTreeMap<String, Sealed>,
}

pub struct Vault {
    path: PathBuf,
    source: Box<dyn KeySource>,
    key: Mutex<Option<LessSafeKey>>,
    entries: Mutex<BTreeMap<String, Sealed>>,
    rng: SystemRandom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultError(pub String);

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Vault {
    pub fn load(path: &Path, source: Box<dyn KeySource>) -> Result<Self, VaultError> {
        let entries = match std::fs::read(path) {
            Ok(bytes) => {
                let file: VaultFile = serde_json::from_slice(&bytes)
                    .map_err(|e| VaultError(format!("{} is unreadable: {e}", path.display())))?;
                if file.format > FORMAT {
                    return Err(VaultError(format!(
                        "{} was written by a newer LocDev",
                        path.display()
                    )));
                }
                file.entries
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => return Err(VaultError(e.to_string())),
        };
        Ok(Self {
            path: path.to_path_buf(),
            source,
            key: Mutex::new(None),
            entries: Mutex::new(entries),
            rng: SystemRandom::new(),
        })
    }

    pub fn backend(&self) -> &'static str {
        self.source.label()
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the key can be had right now (loads it once).
    pub fn unlock(&self) -> Result<(), VaultError> {
        self.with_key(|_| Ok(()))
    }

    fn with_key<T>(
        &self,
        f: impl FnOnce(&LessSafeKey) -> Result<T, VaultError>,
    ) -> Result<T, VaultError> {
        let mut slot = self.key.lock().unwrap();
        if slot.is_none() {
            let raw = match self
                .source
                .load()
                .map_err(|e| VaultError(format!("{}: {e}", self.source.label())))?
            {
                Some(raw) => raw,
                None if self.is_empty() => {
                    let mut raw = [0u8; 32];
                    self.rng
                        .fill(&mut raw)
                        .map_err(|_| VaultError("no system randomness".into()))?;
                    self.source.store(&raw).map_err(|e| {
                        VaultError(format!(
                            "couldn't store the vault key in the {}: {e}",
                            self.source.label()
                        ))
                    })?;
                    raw
                }
                None => {
                    return Err(VaultError(format!(
                        "the vault key is missing from the {}; its {} secret(s) can't be read",
                        self.source.label(),
                        self.len()
                    )));
                }
            };
            let key = UnboundKey::new(&CHACHA20_POLY1305, &raw)
                .map_err(|_| VaultError("bad key".into()))?;
            *slot = Some(LessSafeKey::new(key));
        }
        f(slot.as_ref().expect("set above"))
    }

    /// `plain` sealed and bound to `aad`: nonce followed by ciphertext.
    pub fn seal(&self, aad: &str, plain: &[u8]) -> Result<Vec<u8>, VaultError> {
        self.with_key(|key| {
            let mut nonce = [0u8; NONCE_LEN];
            self.rng
                .fill(&mut nonce)
                .map_err(|_| VaultError("no system randomness".into()))?;
            let mut data = plain.to_vec();
            key.seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(aad.as_bytes()),
                &mut data,
            )
            .map_err(|_| VaultError("encryption failed".into()))?;
            let mut out = nonce.to_vec();
            out.extend(data);
            Ok(out)
        })
    }

    pub fn open(&self, aad: &str, sealed: &[u8]) -> Result<Vec<u8>, VaultError> {
        if sealed.len() < NONCE_LEN {
            return Err(VaultError("sealed data is truncated".into()));
        }
        self.with_key(|key| {
            let (nonce, data) = sealed.split_at(NONCE_LEN);
            let nonce = Nonce::try_assume_unique_for_key(nonce)
                .map_err(|_| VaultError("bad nonce".into()))?;
            let mut data = data.to_vec();
            let plain = key
                .open_in_place(nonce, Aad::from(aad.as_bytes()), &mut data)
                .map_err(|_| VaultError("a sealed value failed its integrity check".into()))?;
            Ok(plain.to_vec())
        })
    }

    pub fn get(&self, id: &str) -> Result<Option<String>, VaultError> {
        let Some(sealed) = self.entries.lock().unwrap().get(id).cloned() else {
            return Ok(None);
        };
        let mut bytes = from_hex(&sealed.n).ok_or_else(|| VaultError("bad entry".into()))?;
        bytes.extend(from_hex(&sealed.c).ok_or_else(|| VaultError("bad entry".into()))?);
        let plain = self.open(id, &bytes)?;
        String::from_utf8(plain)
            .map(Some)
            .map_err(|_| VaultError("bad entry".into()))
    }

    pub fn set(&self, id: &str, value: &str) -> Result<(), VaultError> {
        let sealed = self.seal(id, value.as_bytes())?;
        let (n, c) = sealed.split_at(NONCE_LEN);
        self.entries.lock().unwrap().insert(
            id.to_string(),
            Sealed {
                n: hex(n),
                c: hex(c),
            },
        );
        self.save()
    }

    pub fn remove(&self, id: &str) -> Result<(), VaultError> {
        if self.entries.lock().unwrap().remove(id).is_some() {
            self.save()?;
        }
        Ok(())
    }

    pub fn ids(&self) -> Vec<String> {
        self.entries.lock().unwrap().keys().cloned().collect()
    }

    fn save(&self) -> Result<(), VaultError> {
        let file = VaultFile {
            format: FORMAT,
            entries: self.entries.lock().unwrap().clone(),
        };
        let json = serde_json::to_vec_pretty(&file).map_err(|e| VaultError(e.to_string()))?;
        write_private(&self.path, &json).map_err(|e| VaultError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(dir: &Path, key: std::sync::Arc<MemoryKey>) -> Vault {
        struct Shared(std::sync::Arc<MemoryKey>);
        impl KeySource for Shared {
            fn load(&self) -> Result<Option<[u8; 32]>, String> {
                self.0.load()
            }
            fn store(&self, key: &[u8; 32]) -> Result<(), String> {
                self.0.store(key)
            }
            fn label(&self) -> &'static str {
                "memory"
            }
        }
        Vault::load(&dir.join("vault.json"), Box::new(Shared(key))).unwrap()
    }

    #[test]
    fn round_trip_and_nothing_in_clear() {
        let dir = tempfile::tempdir().unwrap();
        let key = std::sync::Arc::new(MemoryKey::default());
        let v = vault(dir.path(), key.clone());
        v.set("global/STRIPE_SECRET_KEY", "sk_test_123456").unwrap();
        assert_eq!(
            v.get("global/STRIPE_SECRET_KEY").unwrap().as_deref(),
            Some("sk_test_123456")
        );
        let on_disk = std::fs::read_to_string(dir.path().join("vault.json")).unwrap();
        assert!(!on_disk.contains("sk_test"));

        // Reloaded with the same key: readable.
        let again = vault(dir.path(), key);
        assert_eq!(
            again.get("global/STRIPE_SECRET_KEY").unwrap().as_deref(),
            Some("sk_test_123456")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.path().join("vault.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn missing_key_never_regenerates() {
        let dir = tempfile::tempdir().unwrap();
        let v = vault(dir.path(), std::sync::Arc::new(MemoryKey::default()));
        v.set("global/A", "1").unwrap();
        // Another key store, empty: the vault must refuse, not re-key.
        let other = vault(dir.path(), std::sync::Arc::new(MemoryKey::default()));
        let error = other.get("global/A").unwrap_err();
        assert!(error.0.contains("missing"), "{error}");
        assert!(other.set("global/B", "2").is_err());
    }

    #[test]
    fn tampering_and_moving_are_detected() {
        let dir = tempfile::tempdir().unwrap();
        let key = std::sync::Arc::new(MemoryKey::default());
        let v = vault(dir.path(), key.clone());
        v.set("global/A", "secret-a").unwrap();
        let path = dir.path().join("vault.json");
        let mut file: VaultFile = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();

        // Moved under another id: the associated data no longer matches.
        let sealed = file.entries.remove("global/A").unwrap();
        file.entries.insert("global/B".into(), sealed.clone());
        std::fs::write(&path, serde_json::to_vec(&file).unwrap()).unwrap();
        assert!(vault(dir.path(), key.clone()).get("global/B").is_err());

        // A flipped byte.
        let mut c = from_hex(&sealed.c).unwrap();
        c[0] ^= 1;
        file.entries.insert(
            "global/A".into(),
            Sealed {
                n: sealed.n,
                c: hex(&c),
            },
        );
        std::fs::write(&path, serde_json::to_vec(&file).unwrap()).unwrap();
        assert!(vault(dir.path(), key).get("global/A").is_err());
    }

    /// Uses the real system keyring with a throwaway entry, from inside a
    /// tokio runtime like the daemon. `cargo test -p locdev-env -- --ignored`
    #[test]
    #[ignore = "touches the system keyring"]
    fn real_keyring_round_trip() {
        let rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();
        rt.block_on(async {
            let source = KeyringKey {
                user: "env-vault-selftest".into(),
            };
            source.store(&[9u8; 32]).expect("store");
            assert_eq!(source.load().expect("load"), Some([9u8; 32]));
            source.entry().unwrap().delete_credential().expect("delete");
            assert_eq!(source.load().unwrap(), None);
        });
    }

    #[test]
    fn file_key_must_be_private() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.key");
        let source = FileKey(path.clone());
        source.store(&[7u8; 32]).unwrap();
        assert_eq!(source.load().unwrap(), Some([7u8; 32]));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(source.load().is_err());
        }
    }
}
