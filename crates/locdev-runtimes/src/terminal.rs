//! Shims for the terminal: one small script per command (`php`, `node`,
//! `npm`, `composer`…) in a folder LocDev owns, each running the default
//! version's executable. The folder goes first on the user's PATH
//! (`locdev_platform::terminal`), so a new terminal uses the same versions
//! as sites without a pin.
//!
//! Scripts, not symlinks: multi-call tools pick the version from the name
//! they were invoked as, and a script carries the marker.

use std::io;
use std::path::{Path, PathBuf};

/// File in the shim folder that marks it as LocDev's (detection skips it).
pub const MARKER_FILE: &str = ".locdev-shims";
const MARKER: &str = "Managed by LocDev";

/// A command and the executable it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shim {
    pub name: String,
    pub target: PathBuf,
}

fn file_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.cmd")
    } else {
        name.to_string()
    }
}

fn script(target: &Path) -> String {
    let target = target.display().to_string();
    if cfg!(windows) {
        format!(
            "@echo off\r\nrem {MARKER}: the default version (Runtimes in LocDev).\r\n\"{target}\" %*\r\n"
        )
    } else {
        let quoted = target.replace('\'', r"'\''");
        format!(
            "#!/bin/sh\n# {MARKER}: the default version (Runtimes in LocDev).\nexec '{quoted}' \"$@\"\n"
        )
    }
}

fn ours(path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|s| s.contains(MARKER))
}

/// Makes `dir` hold exactly `shims`: writes new or changed ones, removes
/// LocDev's shims that aren't wanted anymore. Files that aren't ours are
/// left alone (and a shim is never written over one).
pub fn sync(dir: &Path, shims: &[Shim]) -> io::Result<Vec<String>> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join(MARKER_FILE),
        format!("{MARKER}: shims for the terminal\n"),
    )?;

    let wanted: Vec<String> = shims.iter().map(|s| file_name(&s.name)).collect();
    for entry in std::fs::read_dir(dir)?.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name != MARKER_FILE && !wanted.contains(&name) && ours(&entry.path()) {
            std::fs::remove_file(entry.path())?;
        }
    }

    let mut written = Vec::new();
    for shim in shims {
        let path = dir.join(file_name(&shim.name));
        if path.exists() && !ours(&path) {
            continue;
        }
        let contents = script(&shim.target);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(contents.as_str()) {
            std::fs::write(&path, &contents)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
            }
        }
        written.push(shim.name.clone());
    }
    written.sort();
    Ok(written)
}

/// Commands that come with a Node.js install, next to `node` (npm runs on
/// whatever `node` is first on PATH: the shim, so the same version).
pub fn node_companions(node: &Path) -> Vec<Shim> {
    let Some(bin) = node.parent() else {
        return Vec::new();
    };
    ["npm", "npx", "corepack"]
        .into_iter()
        .filter_map(|name| {
            let target = if cfg!(windows) {
                bin.join(format!("{name}.cmd"))
            } else {
                bin.join(name)
            };
            target.is_file().then(|| Shim {
                name: name.to_string(),
                target,
            })
        })
        .collect()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn keeps_exactly_the_wanted_shims_and_never_touches_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let shims = dir.path().join("bin");
        std::fs::create_dir_all(&shims).unwrap();
        std::fs::write(shims.join("mine"), "#!/bin/sh\necho user\n").unwrap();
        std::fs::write(shims.join("node"), "not ours").unwrap();

        let php = Shim {
            name: "php".into(),
            target: PathBuf::from("/opt/it's here/php"),
        };
        let node = Shim {
            name: "node".into(),
            target: PathBuf::from("/x/node"),
        };
        let written = sync(&shims, &[php.clone(), node]).unwrap();
        assert_eq!(written, ["php"], "a file that isn't ours is never replaced");
        let script = std::fs::read_to_string(shims.join("php")).unwrap();
        assert!(
            script.contains(r#"exec '/opt/it'\''s here/php' "$@""#),
            "{script}"
        );
        assert!(shims.join(MARKER_FILE).is_file());

        // Dropping the default removes only LocDev's shim.
        sync(&shims, &[]).unwrap();
        assert!(!shims.join("php").exists());
        assert_eq!(
            std::fs::read_to_string(shims.join("node")).unwrap(),
            "not ours"
        );
        assert!(shims.join("mine").exists());
    }

    #[test]
    fn the_shim_runs_its_target_with_the_arguments() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real");
        std::fs::write(&target, "#!/bin/sh\necho \"real $*\"\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
        let shims = dir.path().join("bin");
        sync(
            &shims,
            &[Shim {
                name: "php".into(),
                target,
            }],
        )
        .unwrap();
        let out = locdev_core::process::command(shims.join("php"))
            .args(["-v", "a b"])
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "real -v a b");
    }
}
