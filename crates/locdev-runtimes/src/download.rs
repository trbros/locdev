//! HTTPS downloads with SHA-256 verification computed while streaming.

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256, Sha512};

use crate::RuntimeError;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .user_agent(concat!("locdev/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

pub fn fetch_text(url: &str) -> Result<String, RuntimeError> {
    fetch_text_accept(url, "*/*")
}

/// Like [`fetch_text`], asking for a specific representation.
pub fn fetch_text_accept(url: &str, accept: &str) -> Result<String, RuntimeError> {
    let mut response = agent()
        .get(url)
        .header("Accept", accept)
        .call()
        .map_err(|e| RuntimeError::network(url, e))?;
    response
        .body_mut()
        .with_config()
        .limit(48 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| RuntimeError::network(url, e))
}

/// Hashes of a downloaded file.
pub struct Digests {
    /// Lowercase hex.
    pub sha256: String,
    pub sha512: Vec<u8>,
}

/// Downloads `url` into `dest`, reporting `(downloaded, total)` as it goes,
/// and returns the SHA-256 of what was written (lowercase hex).
pub fn download(
    url: &str,
    dest: &Path,
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<String, RuntimeError> {
    download_digests(url, dest, progress).map(|d| d.sha256)
}

/// Like [`download`], computing every digest a publisher might list.
pub fn download_digests(
    url: &str,
    dest: &Path,
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<Digests, RuntimeError> {
    let response = agent()
        .get(url)
        .call()
        .map_err(|e| RuntimeError::network(url, e))?;
    let (_, body) = response.into_parts();
    let total = body.content_length();
    let mut reader = body.into_reader();

    let mut file = BufWriter::new(File::create(dest)?);
    let mut hasher = Sha256::new();
    let mut hasher512 = Sha512::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut downloaded = 0u64;
    let mut last_report = 0u64;

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| RuntimeError::network(url, e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        hasher512.update(&buf[..n]);
        file.write_all(&buf[..n])?;
        downloaded += n as u64;
        // Report every ~256 KiB, not on every chunk.
        if downloaded - last_report >= 256 * 1024 {
            progress(downloaded, total);
            last_report = downloaded;
        }
    }
    file.flush()?;
    progress(downloaded, total);

    Ok(Digests {
        sha256: hex(&hasher.finalize()),
        sha512: hasher512.finalize().to_vec(),
    })
}

/// Checks a Subresource Integrity string (`sha512-<base64>`, as the npm
/// registry publishes) against the bytes' SHA-512. Other algorithms are
/// refused rather than skipped.
pub fn integrity_matches(integrity: &str, sha512: &[u8]) -> Result<bool, RuntimeError> {
    let encoded = integrity
        .split_whitespace()
        .find_map(|entry| entry.strip_prefix("sha512-"))
        .ok_or_else(|| RuntimeError::Catalog(format!("no sha512 integrity in '{integrity}'")))?;
    let expected = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
        .map_err(|e| RuntimeError::Catalog(format!("bad integrity: {e}")))?;
    Ok(expected == sha512)
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Finds `file`'s hash in a `SHASUMS256.txt`-style listing
/// (`<hex>  <name>` per line).
pub fn checksum_for(listing: &str, file: &str) -> Option<String> {
    listing.lines().find_map(|line| {
        let (hash, name) = line.split_once(char::is_whitespace)?;
        (name.trim().trim_start_matches('*') == file && hash.len() == 64)
            .then(|| hash.to_ascii_lowercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_integrity() {
        let digest = Sha512::digest(b"hello").to_vec();
        let sri = format!(
            "sha512-{}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &digest)
        );
        assert!(integrity_matches(&sri, &digest).unwrap());
        assert!(!integrity_matches(&sri, &Sha512::digest(b"other")).unwrap());
        assert!(integrity_matches("sha1-abc", &digest).is_err());
    }

    #[test]
    fn finds_checksums() {
        let listing = "\
b8492284c76a54baf58d61668dd0e41ee0fc98adabf9503d30f05d08f55b4406  node-v22.23.4-linux-x64.tar.gz
cbc74b5e0982c9dbb41525fd8fb5dcd32fbee237863f7829072f4639a1ce5f20  node-v22.23.4-win-x64.zip
";
        assert_eq!(
            checksum_for(listing, "node-v22.23.4-win-x64.zip").as_deref(),
            Some("cbc74b5e0982c9dbb41525fd8fb5dcd32fbee237863f7829072f4639a1ce5f20")
        );
        assert!(checksum_for(listing, "node-v22.23.4-linux-x64.tar.xz").is_none());
        // A name that merely contains the wanted one must not match.
        assert!(checksum_for(listing, "linux-x64.tar.gz").is_none());
    }
}
