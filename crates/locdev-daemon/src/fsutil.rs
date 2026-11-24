use std::io::Write;
use std::path::{Path, PathBuf};

use locdev_core::AppError;

/// Writes through a temporary file and renames it into place, so a crash
/// never leaves a half-written config behind.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), AppError> {
    let tmp = path.with_extension("tmp");

    let mut file = std::fs::File::create(&tmp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);

    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Finds `program` on `path` the way a shell would (with PATHEXT-like
/// extensions on Windows, where package managers are `.cmd` files).
pub fn find_program(program: &str, path: &str) -> Option<PathBuf> {
    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };
    std::env::split_paths(path).find_map(|dir| {
        extensions
            .iter()
            .map(|ext| dir.join(format!("{program}{ext}")))
            .find(|candidate| candidate.is_file())
    })
}
