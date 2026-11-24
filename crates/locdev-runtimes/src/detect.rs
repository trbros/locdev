//! Runtimes already on the machine: package managers, nvm/fnm, Homebrew,
//! other tools' folders and whatever is on PATH. Each candidate is run once
//! to read its real version.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use locdev_core::model::{RuntimeKind, RuntimeSource, RuntimeVersion};

use crate::version;

const EXE: &str = if cfg!(windows) { ".exe" } else { "" };

/// Detected versions of `kind`, one per version (the first source wins),
/// newest first.
pub fn system(kind: RuntimeKind) -> Vec<RuntimeVersion> {
    let mut seen_paths = HashSet::new();
    let mut seen_versions = HashSet::new();
    let mut found = Vec::new();

    for (executable, origin) in candidates(kind) {
        // Multi-call binaries (`php8.3` and `php` both linking to one tool
        // that dispatches on its name) are distinct: key on the name too.
        let Ok(real) = executable.canonicalize() else {
            continue;
        };
        if !seen_paths.insert((real, executable.file_name().map(|n| n.to_os_string()))) {
            continue;
        }
        let Some(version) = read_version(kind, &executable) else {
            continue;
        };
        if !seen_versions.insert(version.clone()) {
            continue;
        }
        found.push(RuntimeVersion {
            version,
            source: RuntimeSource::System,
            executable,
            origin,
            support: None,
        });
    }

    found.sort_by(|a, b| version::compare(&b.version, &a.version));
    found
}

/// Candidate executables in preference order: dedicated installs before the
/// generic PATH entries, so each version is attributed to where it lives.
fn candidates(kind: RuntimeKind) -> Vec<(PathBuf, String)> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut out = Vec::new();

    match kind {
        RuntimeKind::Node => {
            if let Some(home) = &home {
                let nvm = std::env::var_os("NVM_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".nvm"));
                push_glob(&mut out, &nvm.join("versions/node"), "nvm", &|d| {
                    Some(d.join("bin/node"))
                });
                push_glob(
                    &mut out,
                    &home.join(".local/share/fnm/node-versions"),
                    "fnm",
                    &|d| Some(d.join("installation/bin/node")),
                );
            }
            for prefix in ["/opt/homebrew/opt", "/usr/local/opt"] {
                push_glob(&mut out, Path::new(prefix), "homebrew", &|d| {
                    let name = d.file_name()?.to_str()?;
                    (name == "node" || name.starts_with("node@")).then(|| d.join("bin/node"))
                });
            }
        }
        RuntimeKind::Php => {
            // Debian/Ubuntu (ondrej PPA) and similar: /usr/bin/php8.3 …
            for dir in ["/usr/bin", "/usr/local/bin"] {
                push_glob(&mut out, Path::new(dir), "system", &|p| {
                    versioned_php(p).then(|| p.to_path_buf())
                });
            }
            for prefix in ["/opt/homebrew/opt", "/usr/local/opt"] {
                push_glob(&mut out, Path::new(prefix), "homebrew", &|d| {
                    let name = d.file_name()?.to_str()?;
                    (name == "php" || name.starts_with("php@")).then(|| d.join("bin/php"))
                });
            }
        }
        // Package managers' own installers put them here.
        tool => {
            if let Some(home) = &home {
                let own: &[&str] = match tool {
                    RuntimeKind::Pnpm => &[".local/share/pnpm/pnpm", "Library/pnpm/pnpm"],
                    RuntimeKind::Bun => &[".bun/bin/bun"],
                    RuntimeKind::Yarn => &[".yarn/bin/yarn"],
                    RuntimeKind::Composer => &[".config/composer/composer", ".composer/composer"],
                    _ => &[],
                };
                for rel in own {
                    let path = home.join(rel);
                    if path.is_file() {
                        out.push((path, tool.executable().to_string()));
                    }
                }
            }
        }
    }

    // Everything on PATH, including other tools' folders (e.g. yerd/bin).
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            // LocDev's terminal shims only point at versions found elsewhere.
            if dir.join(crate::terminal::MARKER_FILE).is_file() {
                continue;
            }
            let origin = origin_of(&dir);
            for name in names(kind) {
                let plain = dir.join(name);
                if plain.is_file() {
                    out.push((plain, origin.clone()));
                    break;
                }
            }
            if kind == RuntimeKind::Php {
                push_glob(&mut out, &dir, &origin, &|p| {
                    versioned_php(p).then(|| p.to_path_buf())
                });
            }
        }
    }
    out
}

/// File names `kind` goes by on PATH. On Windows, Node.js package managers
/// are batch shims (`pnpm.cmd`) and Composer's installer writes `composer.bat`.
fn names(kind: RuntimeKind) -> Vec<String> {
    let name = kind.executable();
    if cfg!(windows) && kind.category() == locdev_core::model::RuntimeCategory::PackageManager {
        ["cmd", "bat", "exe"]
            .iter()
            .map(|ext| format!("{name}.{ext}"))
            .collect()
    } else {
        vec![format!("{name}{EXE}")]
    }
}

/// Adds `pick(entry)` for every entry of `dir`, in name order.
fn push_glob(
    out: &mut Vec<(PathBuf, String)>,
    dir: &Path,
    origin: &str,
    pick: &dyn Fn(&Path) -> Option<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if let Some(exe) = pick(&path).filter(|p| p.is_file()) {
            out.push((exe, origin.to_string()));
        }
    }
}

/// `php8.3`, `php8.4` … (not php-config8.3, phpize8.3).
fn versioned_php(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(EXE).unwrap_or(n).strip_prefix("php"))
        .is_some_and(|rest| {
            !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.')
        })
}

/// A readable name for where a PATH folder belongs.
fn origin_of(dir: &Path) -> String {
    let text = dir.to_string_lossy();
    for (needle, name) in [
        ("/yerd/", "yerd"),
        ("/.nvm/", "nvm"),
        ("/fnm", "fnm"),
        ("/.volta/", "volta"),
        ("/homebrew/", "homebrew"),
        ("/.asdf/", "asdf"),
        ("/mise/", "mise"),
        ("/Herd/", "herd"),
    ] {
        if text.contains(needle) {
            return name.into();
        }
    }
    "PATH".into()
}

fn read_version(kind: RuntimeKind, executable: &Path) -> Option<String> {
    let args: &[&str] = match kind {
        // -n: skip php.ini so a broken config can't hang or fail detection.
        RuntimeKind::Php => &["-n", "-r", "echo PHP_VERSION;"],
        RuntimeKind::Composer => &["--version", "--no-ansi", "--no-interaction"],
        _ => &["--version"],
    };
    // Version managers' shims can be slow on their first run.
    let output = run_with_timeout(executable, args, Duration::from_secs(10))?;
    find_version(&output)
}

/// The first `x.y.z` in a tool's output ("Composer version 2.8.4 2024-…").
fn find_version(output: &str) -> Option<String> {
    output
        .split(|c: char| c.is_whitespace() || c == ',')
        .map(|word| word.trim_start_matches('v'))
        .find(|word| {
            version::parts(word).len() >= 3
                || (version::parts(word).len() >= 2 && output.trim() == *word)
        })
        .map(|word| word.split(['-', '+']).next().unwrap_or(word).to_string())
}

fn run_with_timeout(program: &Path, args: &[&str], timeout: Duration) -> Option<String> {
    let mut child = locdev_core::process::command(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let start = Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(status) if status.success() => {
                let mut out = String::new();
                std::io::Read::read_to_string(child.stdout.as_mut()?, &mut out).ok()?;
                return Some(out);
            }
            Some(_) => return None,
            None if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_versioned_php() {
        assert!(versioned_php(Path::new("/usr/bin/php8.3")));
        assert!(versioned_php(Path::new("/usr/bin/php84")));
        assert!(!versioned_php(Path::new("/usr/bin/php")));
        assert!(!versioned_php(Path::new("/usr/bin/php-config8.3")));
        assert!(!versioned_php(Path::new("/usr/bin/phpize8.3")));
        assert!(!versioned_php(Path::new("/usr/bin/php8.3cover")));
    }

    #[test]
    fn reads_versions_from_tool_output() {
        assert_eq!(find_version("v22.11.0\n").as_deref(), Some("22.11.0"));
        assert_eq!(
            find_version("Composer version 2.8.4 2024-12-11 11:57:47\nPHP version 8.3.6")
                .as_deref(),
            Some("2.8.4")
        );
        assert_eq!(find_version("1.22.22").as_deref(), Some("1.22.22"));
        assert_eq!(find_version("8.3").as_deref(), Some("8.3"));
        assert_eq!(find_version("no version here"), None);
    }

    #[test]
    fn names_origins() {
        assert_eq!(
            origin_of(Path::new("/home/u/.local/share/yerd/bin")),
            "yerd"
        );
        assert_eq!(origin_of(Path::new("/usr/bin")), "PATH");
    }
}
