//! Code editors LocDev can open a project or a file in: which ones it
//! knows, the programs they install and the arguments their command-line
//! launchers document. Detection and launching happen in the daemon.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How an editor's launcher takes a file at a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// VS Code and its forks: `--goto <file>:<line>[:<column>]`
    /// (https://code.visualstudio.com/docs/configure/command-line).
    VsCode,
    /// `<file>:<line>[:<column>]`: Zed (https://zed.dev/docs/reference/cli)
    /// and Sublime Text (https://www.sublimetext.com/docs/command_line.html).
    Suffix,
    /// JetBrains IDEs: `--line <n> [--column <n>] <file>`
    /// (https://www.jetbrains.com/help/idea/opening-files-from-command-line.html).
    JetBrains,
}

pub struct Def {
    pub id: &'static str,
    pub name: &'static str,
    /// Program names to look for, in order (without `.exe`/`.cmd`).
    pub programs: &'static [&'static str],
    /// Install locations off PATH, relative to the home folder (`~/`) or
    /// absolute, per system.
    pub places: &'static [(&'static str, &'static str)],
    pub style: Style,
}

/// Known editors, in the order they're offered.
pub const EDITORS: &[Def] = &[
    Def {
        id: "vscode",
        name: "Visual Studio Code",
        programs: &["code"],
        places: &[
            (
                "macos",
                "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
            ),
            (
                "windows",
                "~/AppData/Local/Programs/Microsoft VS Code/bin/code.cmd",
            ),
            ("windows", "C:/Program Files/Microsoft VS Code/bin/code.cmd"),
        ],
        style: Style::VsCode,
    },
    Def {
        id: "cursor",
        name: "Cursor",
        programs: &["cursor"],
        places: &[],
        style: Style::VsCode,
    },
    Def {
        id: "windsurf",
        name: "Windsurf",
        programs: &["windsurf"],
        places: &[],
        style: Style::VsCode,
    },
    Def {
        id: "vscodium",
        name: "VSCodium",
        programs: &["codium"],
        places: &[],
        style: Style::VsCode,
    },
    Def {
        id: "zed",
        name: "Zed",
        // Some Linux packages name the launcher `zeditor`.
        programs: &["zed", "zeditor"],
        places: &[],
        style: Style::Suffix,
    },
    Def {
        id: "sublime",
        name: "Sublime Text",
        programs: &["subl"],
        places: &[
            (
                "macos",
                "/Applications/Sublime Text.app/Contents/SharedSupport/bin/subl",
            ),
            ("windows", "C:/Program Files/Sublime Text/subl.exe"),
        ],
        style: Style::Suffix,
    },
    Def {
        id: "phpstorm",
        name: "PhpStorm",
        programs: &["phpstorm", "phpstorm.sh", "phpstorm64"],
        places: &[],
        style: Style::JetBrains,
    },
    Def {
        id: "webstorm",
        name: "WebStorm",
        programs: &["webstorm", "webstorm.sh", "webstorm64"],
        places: &[],
        style: Style::JetBrains,
    },
    Def {
        id: "idea",
        name: "IntelliJ IDEA",
        programs: &["idea", "idea.sh", "idea64"],
        places: &[],
        style: Style::JetBrains,
    },
    Def {
        id: "pycharm",
        name: "PyCharm",
        programs: &["pycharm", "pycharm.sh", "pycharm64"],
        places: &[],
        style: Style::JetBrains,
    },
    Def {
        id: "rustrover",
        name: "RustRover",
        programs: &["rustrover", "rustrover.sh", "rustrover64"],
        places: &[],
        style: Style::JetBrains,
    },
];

/// Folders launchers commonly land in that a daemon's PATH may lack
/// (relative ones are under the home folder).
pub const EXTRA_DIRS: &[(&str, &str)] = &[
    ("unix", "/usr/local/bin"),
    ("unix", "/opt/homebrew/bin"),
    ("unix", "/snap/bin"),
    ("unix", "~/.local/bin"),
    ("linux", "~/.local/share/JetBrains/Toolbox/scripts"),
    (
        "macos",
        "~/Library/Application Support/JetBrains/Toolbox/scripts",
    ),
    ("windows", "~/AppData/Local/JetBrains/Toolbox/scripts"),
];

pub fn def(id: &str) -> Option<&'static Def> {
    EDITORS.iter().find(|d| d.id == id)
}

/// An editor found on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorInfo {
    pub id: String,
    pub name: String,
    /// The launcher LocDev runs.
    pub program: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorsView {
    pub editors: Vec<EditorInfo>,
    /// The chosen one (`[editor] preferred` in settings.toml), if any.
    #[serde(default)]
    pub preferred: Option<String>,
    /// What opens when nothing is chosen: the preferred one if it's here,
    /// else the first found.
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorSettings {
    /// An editor id from [`EDITORS`]; empty = the first one found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred: Option<String>,
}

/// The arguments that open `target` (a folder, or a file with an optional
/// line and column). Paths are absolute, so none starts with `-`.
pub fn args(style: Style, target: &Path, line: Option<u32>, column: Option<u32>) -> Vec<String> {
    let path = target.display().to_string();
    let Some(line) = line.filter(|l| *l > 0) else {
        return match style {
            Style::VsCode if target.is_file() => vec!["--goto".into(), path],
            _ => vec![path],
        };
    };
    let column = column.filter(|c| *c > 0);
    let at = match column {
        Some(column) => format!("{path}:{line}:{column}"),
        None => format!("{path}:{line}"),
    };
    match style {
        Style::VsCode => vec!["--goto".into(), at],
        Style::Suffix => vec![at],
        Style::JetBrains => {
            let mut out = vec!["--line".into(), line.to_string()];
            if let Some(column) = column {
                out.extend(["--column".into(), column.to_string()]);
            }
            out.push(path);
            out
        }
    }
}

/// A path inside a project, from what a client sent: relative, without
/// `..`, and still inside `root` once links are resolved.
pub fn inside(root: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative.components().any(|c| {
            !matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let target = root.join(relative).canonicalize().ok()?;
    target.starts_with(&root).then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_follow_each_launcher() {
        let file = Path::new("/p/app.php");
        assert_eq!(
            args(Style::VsCode, file, Some(12), None),
            ["--goto", "/p/app.php:12"]
        );
        assert_eq!(
            args(Style::VsCode, file, Some(12), Some(4)),
            ["--goto", "/p/app.php:12:4"]
        );
        assert_eq!(
            args(Style::Suffix, file, Some(12), Some(4)),
            ["/p/app.php:12:4"]
        );
        assert_eq!(
            args(Style::JetBrains, file, Some(12), Some(4)),
            ["--line", "12", "--column", "4", "/p/app.php"]
        );
        assert_eq!(args(Style::JetBrains, Path::new("/p"), None, None), ["/p"]);
        assert_eq!(args(Style::Suffix, file, Some(0), None), ["/p/app.php"]);
    }

    #[test]
    fn paths_stay_inside_the_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.ts"), "").unwrap();
        assert!(inside(dir.path(), "src/a.ts").is_some());
        assert!(inside(dir.path(), "./src/a.ts").is_some());
        assert!(inside(dir.path(), "../etc/passwd").is_none());
        assert!(inside(dir.path(), "/etc/passwd").is_none());
        assert!(inside(dir.path(), "src/missing.ts").is_none());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc", dir.path().join("out")).unwrap();
            assert!(
                inside(dir.path(), "out/hostname").is_none(),
                "links out are refused"
            );
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = EDITORS.iter().map(|d| d.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), EDITORS.len());
    }
}
