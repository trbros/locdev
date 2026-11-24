//! The `locdev` command for the terminal, when the CLI ships inside the app.
//! Per user, no administrator rights:
//!
//! - Linux and macOS: a two-line script at `~/.local/bin/locdev` that runs
//!   the CLI bundled with the app (a script, not a symlink, so moving the app
//!   only needs the link made again, and the marker fits in it).
//! - Windows: `locdev.cmd` in `%LOCALAPPDATA%\LocDev\bin`, and that folder on
//!   the user's `Path`.
//!
//! A `.deb`/`.rpm` install already puts `locdev` in `/usr/bin`, and the
//! standalone archive is unpacked wherever the user wants: then there is
//! nothing to do. An existing `locdev` that isn't ours is never replaced.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;

use crate::MARKER;

const BIN: &str = if cfg!(windows) {
    "locdev.exe"
} else {
    "locdev"
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CliStatus {
    /// The CLI next to the app, when there is one.
    pub bundled: Option<PathBuf>,
    /// What `locdev --version` of that CLI says.
    pub version: Option<String>,
    /// The `locdev` a new terminal would run (first on `PATH`).
    pub on_path: Option<PathBuf>,
    /// LocDev's own link, when it exists.
    pub link: Option<PathBuf>,
    /// Where the link goes (shown before creating it).
    pub link_path: Option<PathBuf>,
    /// Whether the link's folder is on `PATH` (otherwise the shell needs it).
    pub link_dir_on_path: bool,
    /// Something else already sits where the link would go.
    pub conflict: bool,
}

/// The CLI shipped next to the running executable (the app's sidecar).
pub fn bundled() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|dir| dir.join(BIN))
        .filter(|p| p.is_file())
}

pub fn status(bundled: Option<&Path>) -> CliStatus {
    let link_path = link_path();
    let ours = link_path.as_deref().filter(|p| is_ours(p));
    let link_dir_on_path = link_path
        .as_deref()
        .and_then(Path::parent)
        .is_some_and(|dir| path_dirs().iter().any(|d| same(d, dir)));
    CliStatus {
        bundled: bundled.map(Path::to_path_buf),
        version: bundled.and_then(version_of),
        on_path: which(),
        link: ours.map(Path::to_path_buf),
        link_path: link_path.clone(),
        link_dir_on_path,
        conflict: link_path.is_some_and(|p| p.exists() && !is_ours(&p)),
    }
}

/// Creates (or points again) LocDev's link to `cli`.
pub fn install(cli: &Path) -> io::Result<()> {
    let path = link_path().ok_or_else(|| io::Error::other("no home folder to put the link in"))?;
    if path.exists() && !is_ours(&path) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "{} exists and wasn't made by LocDev; it is left as it is",
                path.display()
            ),
        ));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, script(cli))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(windows)]
    if let Some(dir) = path.parent() {
        windows_path(dir, true, false)?;
    }
    Ok(())
}

/// Removes LocDev's link (and, on Windows, its folder from `Path`).
pub fn uninstall() -> io::Result<()> {
    let Some(path) = link_path() else {
        return Ok(());
    };
    if is_ours(&path) {
        std::fs::remove_file(&path)?;
    }
    #[cfg(windows)]
    if let Some(dir) = path.parent() {
        windows_path(dir, false, false)?;
    }
    Ok(())
}

fn link_path() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| {
            PathBuf::from(d)
                .join("LocDev")
                .join("bin")
                .join("locdev.cmd")
        })
    } else {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/bin/locdev"))
    }
}

fn is_ours(path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|s| s.contains(MARKER))
}

fn script(cli: &Path) -> String {
    let cli = cli.display().to_string();
    if cfg!(windows) {
        format!(
            "@echo off\r\nrem {MARKER}. Remove it from LocDev: Settings > About.\r\n\"{cli}\" %*\r\n"
        )
    } else {
        // Single quotes keep the path literal; a quote inside is closed,
        // escaped and reopened.
        let quoted = cli.replace('\'', r"'\''");
        format!(
            "#!/bin/sh\n# {MARKER}. Remove it from LocDev: Settings > About.\nexec '{quoted}' \"$@\"\n"
        )
    }
}

fn path_dirs() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default()
}

fn same(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

/// The `locdev` a terminal would find first.
fn which() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["locdev.exe", "locdev.cmd"]
    } else {
        &["locdev"]
    };
    path_dirs()
        .into_iter()
        .flat_map(|dir| names.iter().map(move |n| dir.join(n)))
        .find(|p| p.is_file())
}

/// `locdev --version` prints `locdev 0.7.0`.
fn version_of(cli: &Path) -> Option<String> {
    let output = locdev_core::process::command(cli)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace().nth(1).map(str::to_string)
}

/// Adds or removes `dir` in the user's `Path` (HKCU), at the end or, with
/// `first`, at the front. PowerShell's `SetEnvironmentVariable` also tells
/// running programs it changed.
#[cfg(windows)]
pub(crate) fn windows_path(dir: &Path, add: bool, first: bool) -> io::Result<()> {
    let dir = dir.display().to_string().replace('\'', "''");
    let script = if add && first {
        format!(
            "$p=[Environment]::GetEnvironmentVariable('Path','User'); \
             $d='{dir}'; \
             $rest=((\"$p\" -split ';') | Where-Object {{ $_ -and $_ -ne $d }}); \
             $n=(@($d) + @($rest)) -join ';'; \
             if ($n -ne $p) {{ [Environment]::SetEnvironmentVariable('Path', $n, 'User') }}"
        )
    } else if add {
        format!(
            "$p=[Environment]::GetEnvironmentVariable('Path','User'); \
             $d='{dir}'; \
             if (-not (($p -split ';') -contains $d)) {{ \
               [Environment]::SetEnvironmentVariable('Path', ((\"$p\".TrimEnd(';')), $d -join ';').TrimStart(';'), 'User') }}"
        )
    } else {
        format!(
            "$p=[Environment]::GetEnvironmentVariable('Path','User'); \
             $d='{dir}'; \
             $n=((\"$p\" -split ';') | Where-Object {{ $_ -and $_ -ne $d }}) -join ';'; \
             if ($n -ne $p) {{ [Environment]::SetEnvironmentVariable('Path', $n, 'User') }}"
        )
    };
    let status = locdev_core::process::command("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("could not change the user's Path"))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn the_script_runs_the_bundled_cli_literally() {
        let s = script(Path::new("/opt/Loc Dev/it's/locdev"));
        assert!(s.starts_with("#!/bin/sh\n"));
        assert!(s.contains(MARKER));
        assert!(s.contains(r#"exec '/opt/Loc Dev/it'\''s/locdev' "$@""#));
    }

    #[test]
    fn only_marked_files_are_ours() {
        let dir = tempfile::tempdir().unwrap();
        let mine = dir.path().join("mine");
        let theirs = dir.path().join("theirs");
        std::fs::write(&mine, script(Path::new("/x/locdev"))).unwrap();
        std::fs::write(&theirs, "#!/bin/sh\nexec something\n").unwrap();
        assert!(is_ours(&mine));
        assert!(!is_ours(&theirs));
        assert!(!is_ours(&dir.path().join("missing")));
    }
}
