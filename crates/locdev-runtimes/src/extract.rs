//! Unpacking release archives, dropping their single top-level folder
//! (`node-v22.11.0-linux-x64/bin/node` → `bin/node`). Entries that would land
//! outside the destination (absolute paths, `..`) are rejected.

use std::fs::File;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::RuntimeError;

pub fn tar_gz(archive: &Path, dest: &Path) -> Result<(), RuntimeError> {
    tar_gz_with(archive, dest, true)
}

/// Like [`tar_gz`], keeping every path as is: StaticPHP's archives hold a
/// lone `php` at the root.
pub fn tar_gz_flat(archive: &Path, dest: &Path) -> Result<(), RuntimeError> {
    tar_gz_with(archive, dest, false)
}

fn tar_gz_with(archive: &Path, dest: &Path, strip: bool) -> Result<(), RuntimeError> {
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(File::open(archive)?));
    tar.set_preserve_permissions(true);

    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let relative = if strip {
            match strip_top(&path)? {
                Some(relative) => relative,
                None => continue,
            }
        } else {
            match contained(&path)? {
                Some(relative) => relative,
                None => continue,
            }
        };

        match entry.header().entry_type() {
            tar::EntryType::Link => {
                return Err(RuntimeError::Archive(format!(
                    "hard link in archive: {}",
                    path.display()
                )));
            }
            // A link that points outside `dest` would let later entries
            // write through it, so only relative, contained targets pass.
            tar::EntryType::Symlink => {
                let link = entry.link_name()?.unwrap_or_default().into_owned();
                if !stays_inside(&relative, &link) {
                    return Err(RuntimeError::Archive(format!(
                        "symlink escapes the archive: {} -> {}",
                        path.display(),
                        link.display()
                    )));
                }
            }
            _ => {}
        }

        let target = dest.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        entry.unpack(&target)?;
    }
    Ok(())
}

/// `strip` drops the single top-level folder, as for tarballs; Windows PHP
/// zips have none.
pub fn zip(archive: &Path, dest: &Path, strip: bool) -> Result<(), RuntimeError> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?)
        .map_err(|e| RuntimeError::Archive(e.to_string()))?;

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| RuntimeError::Archive(e.to_string()))?;
        let Some(path) = entry.enclosed_name() else {
            return Err(RuntimeError::Archive(format!(
                "unsafe path in archive: {}",
                entry.name().map(|n| n.into_owned()).unwrap_or_default()
            )));
        };
        let relative = if strip {
            match strip_top(&path)? {
                Some(relative) => relative,
                None => continue,
            }
        } else {
            path
        };
        let target = dest.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        io::copy(&mut entry, &mut File::create(&target)?)?;
    }
    Ok(())
}

/// Whether a symlink at `at` (relative to the destination) pointing to
/// `link` resolves inside the destination.
fn stays_inside(at: &Path, link: &Path) -> bool {
    if link.is_absolute() {
        return false;
    }
    let mut depth = at.components().count() as i64 - 1;
    for component in link.components() {
        match component {
            Component::ParentDir => depth -= 1,
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            _ => return false,
        }
        if depth < 0 {
            return false;
        }
    }
    true
}

/// `path` without `./`; refuses anything but plain names. `None` for the
/// archive root itself.
fn contained(path: &Path) -> Result<Option<PathBuf>, RuntimeError> {
    let mut rest = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(name) => rest.push(name),
            _ => {
                return Err(RuntimeError::Archive(format!(
                    "unsafe path in archive: {}",
                    path.display()
                )));
            }
        }
    }
    Ok((!rest.as_os_str().is_empty()).then_some(rest))
}

/// Drops the first component; `None` for the top-level folder itself.
fn strip_top(path: &Path) -> Result<Option<PathBuf>, RuntimeError> {
    let mut components = path.components();
    components.next();
    let rest: PathBuf = components.collect();

    if rest
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(RuntimeError::Archive(format!(
            "unsafe path in archive: {}",
            path.display()
        )));
    }
    Ok((!rest.as_os_str().is_empty()).then_some(rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_the_top_folder() {
        assert_eq!(
            strip_top(Path::new("node-v22-linux-x64/bin/node")).unwrap(),
            Some(PathBuf::from("bin/node"))
        );
        assert_eq!(strip_top(Path::new("node-v22-linux-x64/")).unwrap(), None);
        assert!(strip_top(Path::new("top/../../etc/passwd")).is_err());
    }

    #[test]
    fn flat_paths_stay_contained() {
        assert_eq!(
            contained(Path::new("./php")).unwrap(),
            Some(PathBuf::from("php"))
        );
        assert_eq!(contained(Path::new("./")).unwrap(), None);
        assert!(contained(Path::new("../php")).is_err());
        assert!(contained(Path::new("/usr/bin/php")).is_err());
    }

    #[test]
    fn symlinks_must_stay_inside() {
        // npm's real layout.
        assert!(stays_inside(
            Path::new("bin/npm"),
            Path::new("../lib/node_modules/npm/bin/npm-cli.js")
        ));
        assert!(!stays_inside(
            Path::new("bin/npm"),
            Path::new("../../etc/passwd")
        ));
        assert!(!stays_inside(Path::new("lib"), Path::new("/etc")));
    }

    #[test]
    fn unpacks_tar_gz() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.tar.gz");
        {
            let gz = flate2::write::GzEncoder::new(
                File::create(&archive).unwrap(),
                flate2::Compression::fast(),
            );
            let mut builder = tar::Builder::new(gz);
            let mut header = tar::Header::new_gnu();
            header.set_size(5);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, "node-v1-linux-x64/bin/node", &b"hello"[..])
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }

        let out = dir.path().join("out");
        tar_gz(&archive, &out).unwrap();
        assert_eq!(std::fs::read(out.join("bin/node")).unwrap(), b"hello");
    }
}
