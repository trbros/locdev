//! Every helper the daemon or the app starts must go through
//! `locdev_core::process::command` (no console window on Windows); a plain
//! `Command::new` there makes a terminal flash on screen.

use std::path::{Path, PathBuf};

/// Files that set the creation flags themselves.
const OWN_FLAGS: &[&str] = &[
    "locdev-core/src/process.rs",
    "locdev-daemon/src/supervisor.rs",
    "locdev-ipc/src/launcher.rs",
];
/// Unix-only programs: no console windows there.
const UNIX_ONLY: &[&str] = &[
    "\"/bin/sh\"",
    "\"sh\"",
    "\"sudo\"",
    "\"osascript\"",
    "\"pkexec\"",
    "\"id\"",
    "\"launchctl\"",
    "\"getconf\"",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            // Tests and build output don't ship in the daemon or the app.
            if name != "tests" && name != "target" && name != "node_modules" {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") && name != "tests.rs" {
            out.push(path);
        }
    }
}

#[test]
fn helpers_start_without_a_console_window() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    rust_files(&root.join("crates"), &mut files);
    rust_files(&root.join("apps/desktop/src-tauri/src"), &mut files);
    assert!(files.len() > 50, "found only {} files", files.len());
    let mut offenders = Vec::new();
    for file in files {
        let shown = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string()
            // Windows paths use `\`; OWN_FLAGS is written with `/`.
            .replace('\\', "/");
        if OWN_FLAGS.iter().any(|f| shown.ends_with(f)) {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap();
        let mut in_tests = false;
        for (n, line) in text.lines().enumerate() {
            if line.trim() == "#[cfg(test)]" {
                in_tests = true;
            }
            if !in_tests
                && line.contains("Command::new(")
                && !UNIX_ONLY.iter().any(|p| line.contains(p))
            {
                offenders.push(format!("{shown}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "use locdev_core::process::command:\n{}",
        offenders.join("\n")
    );
}
