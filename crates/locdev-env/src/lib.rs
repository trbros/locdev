//! Shared environment variables: the `.env` reader, each framework's
//! loading rules, what a project's process gets (and why), `env sync`, and
//! the encrypted vault for secret values. No network, no shell.

pub mod dotenv;
pub mod resolve;
pub mod rules;
pub mod store;
pub mod sync;
pub mod vault;

use std::path::Path;

pub use store::{EnvStore, StoredVar, VarChange};

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn from_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Writes through a temporary file only the user can read, then renames it
/// into place: a crash never leaves a half-written file, and the contents
/// are never readable by others, even for a moment.
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn hex_round_trip() {
        let bytes = [0u8, 1, 0xab, 0xff];
        assert_eq!(super::from_hex(&super::hex(&bytes)).unwrap(), bytes);
        assert!(super::from_hex("abc").is_none());
        assert!(super::from_hex("zz").is_none());
    }
}
