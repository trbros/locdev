//! The programs starters need, where to find them and how to get them.
//! LocDev installs runtimes itself (verified downloads, see Runtimes); other
//! tools have their publisher's documented install command for this OS
//! (checked 2026-11-23 on each tool's install page), run in the app's
//! terminal after the user sees it, or a link when there is none. The
//! commands are constants: nothing in them comes from the user.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use locdev_core::process::command;
use locdev_core::scaffold::{Starter, Step, Tool, ToolInstall};

struct Def {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    docs: &'static str,
    programs: &'static [&'static str],
    install: Install,
}

enum Install {
    None,
    Runtime(&'static str),
    /// The first command for this OS; run in the user's home.
    Commands(&'static [Cmd]),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Os {
    Any,
    Windows,
    /// Linux and macOS.
    Unix,
    Mac,
}

struct Cmd {
    os: Os,
    program: &'static str,
    args: &'static [&'static str],
}

const fn cmd(os: Os, program: &'static str, args: &'static [&'static str]) -> Cmd {
    Cmd { os, program, args }
}

impl Os {
    fn here(self) -> bool {
        match self {
            Os::Any => true,
            Os::Windows => cfg!(windows),
            Os::Unix => cfg!(unix),
            Os::Mac => cfg!(target_os = "macos"),
        }
    }
}

/// The command for this OS, if the tool has one.
fn command_here(install: &Install) -> Option<&'static Cmd> {
    match install {
        Install::Commands(cmds) => cmds.iter().find(|c| c.os.here()),
        _ => None,
    }
}

const DEFS: &[Def] = &[
    Def {
        id: "php",
        name: "PHP",
        description: "Runs Laravel, Symfony and WordPress.",
        docs: "https://www.php.net/downloads",
        programs: &["php"],
        install: Install::Runtime("php"),
    },
    Def {
        id: "composer",
        name: "Composer",
        description: "PHP's package manager; creates Laravel, Symfony and Bedrock projects.",
        docs: "https://getcomposer.org/download/",
        programs: &["composer"],
        install: Install::Runtime("composer"),
    },
    Def {
        id: "laravel",
        name: "Laravel installer",
        description: "`laravel new`, with starter kits, testing and database choices.",
        docs: "https://laravel.com/docs/installation#installing-php",
        programs: &["laravel"],
        install: Install::Commands(&[cmd(
            Os::Any,
            "composer",
            &["global", "require", "laravel/installer"],
        )]),
    },
    Def {
        id: "symfony",
        name: "Symfony CLI",
        description: "`symfony new`; Composer works without it.",
        docs: "https://symfony.com/download",
        programs: &["symfony"],
        install: Install::Commands(&[
            cmd(Os::Windows, "scoop", &["install", "symfony-cli"]),
            cmd(Os::Mac, "brew", &["install", "symfony-cli/tap/symfony-cli"]),
            cmd(
                Os::Unix,
                "bash",
                &[
                    "-c",
                    "curl -sS https://get.symfony.com/cli/installer | bash",
                ],
            ),
        ]),
    },
    Def {
        id: "node",
        name: "Node.js",
        description: "Runs the JavaScript starters and front-end builds.",
        docs: "https://nodejs.org/en/download",
        programs: &["node"],
        install: Install::Runtime("node"),
    },
    Def {
        id: "npm",
        name: "npm",
        description: "Comes with Node.js.",
        docs: "https://docs.npmjs.com/downloading-and-installing-node-js-and-npm",
        programs: &["npm"],
        install: Install::Runtime("node"),
    },
    Def {
        id: "pnpm",
        name: "pnpm",
        description: "Fast, disk-efficient package manager.",
        docs: "https://pnpm.io/installation",
        programs: &["pnpm"],
        install: Install::Runtime("pnpm"),
    },
    Def {
        id: "bun",
        name: "Bun",
        description: "JavaScript runtime and package manager.",
        docs: "https://bun.com/docs/installation",
        programs: &["bun"],
        // Bun's docs list it ("the last `npm` command you'll ever need").
        install: Install::Commands(&[cmd(Os::Any, "npm", &["install", "-g", "bun"])]),
    },
    Def {
        id: "python",
        name: "Python",
        description: "Runs Django, Flask and FastAPI; each project gets its own virtual environment.",
        docs: "https://www.python.org/downloads/",
        programs: &["python3", "python"],
        install: Install::None,
    },
    Def {
        id: "uv",
        name: "uv",
        description: "Fast Python package and project manager; optional.",
        docs: "https://docs.astral.sh/uv/getting-started/installation/",
        programs: &["uv"],
        install: Install::Commands(&[
            cmd(
                Os::Windows,
                "winget",
                &["install", "--id=astral-sh.uv", "-e"],
            ),
            cmd(
                Os::Unix,
                "sh",
                &["-c", "curl -LsSf https://astral.sh/uv/install.sh | sh"],
            ),
        ]),
    },
    Def {
        id: "pipx",
        name: "pipx",
        description: "Installs Python command-line tools in their own environments; optional.",
        docs: "https://pipx.pypa.io/stable/installation/",
        programs: &["pipx"],
        install: Install::None,
    },
    Def {
        id: "poetry",
        name: "Poetry",
        description: "Python dependency management and packaging; optional.",
        docs: "https://python-poetry.org/docs/#installation",
        programs: &["poetry"],
        // The official installer (pipx, listed first, isn't always there).
        install: Install::Commands(&[
            cmd(
                Os::Windows,
                "powershell",
                &[
                    "-NoProfile",
                    "-Command",
                    "(Invoke-WebRequest -Uri https://install.python-poetry.org -UseBasicParsing).Content | py -",
                ],
            ),
            cmd(
                Os::Unix,
                "sh",
                &[
                    "-c",
                    "curl -sSL https://install.python-poetry.org | python3 -",
                ],
            ),
        ]),
    },
    Def {
        id: "cargo",
        name: "Cargo",
        description: "Rust's build tool and package manager; creates the Axum and Actix Web starters.",
        docs: "https://www.rust-lang.org/tools/install",
        programs: &["cargo"],
        // Cargo comes with rustup (Windows: rustup-init.exe from the link).
        install: Install::Commands(&[cmd(
            Os::Unix,
            "sh",
            &[
                "-c",
                "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh",
            ],
        )]),
    },
    Def {
        id: "rustup",
        name: "rustup",
        description: "Installs and updates Rust toolchains (and Cargo with them).",
        docs: "https://rustup.rs/",
        programs: &["rustup"],
        install: Install::Commands(&[cmd(
            Os::Unix,
            "sh",
            &[
                "-c",
                "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh",
            ],
        )]),
    },
    Def {
        id: "wp",
        name: "WP-CLI",
        description: "WordPress's command line; downloads WordPress for new projects.",
        docs: "https://wp-cli.org/#installing",
        programs: &["wp"],
        // WP-CLI's handbook, "Installing via Composer", also its Windows route.
        install: Install::Commands(&[cmd(
            Os::Any,
            "composer",
            &["global", "require", "wp-cli/wp-cli-bundle"],
        )]),
    },
    Def {
        id: "git",
        name: "Git",
        description: "Version control; starters can initialize a repository.",
        docs: "https://git-scm.com/downloads",
        programs: &["git"],
        install: Install::Commands(&[cmd(
            Os::Windows,
            "winget",
            &["install", "--id", "Git.Git", "-e", "--source", "winget"],
        )]),
    },
    Def {
        id: "rails",
        name: "Rails",
        description: "`rails new`; needs Ruby.",
        docs: "https://guides.rubyonrails.org/install_ruby_on_rails.html",
        programs: &["rails"],
        install: Install::Commands(&[cmd(Os::Any, "gem", &["install", "rails"])]),
    },
    Def {
        id: "dotnet",
        name: ".NET SDK",
        description: "`dotnet new` for ASP.NET Core.",
        docs: "https://dotnet.microsoft.com/download",
        programs: &["dotnet"],
        // The current LTS (10).
        install: Install::Commands(&[
            cmd(
                Os::Windows,
                "winget",
                &["install", "Microsoft.DotNet.SDK.10"],
            ),
            cmd(
                Os::Unix,
                "bash",
                &[
                    "-c",
                    "curl -sSL https://dot.net/v1/dotnet-install.sh | bash /dev/stdin --channel LTS",
                ],
            ),
        ]),
    },
];

/// Folders besides PATH where installers land: Composer's global `bin`
/// (the Laravel installer), Bun's, rustup's and the user's local bin.
pub fn extra_dirs(home: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![
        home.join(".config/composer/vendor/bin"),
        home.join(".composer/vendor/bin"),
        home.join(".bun/bin"),
        home.join(".cargo/bin"),
        home.join(".local/bin"),
        // Symfony CLI's and dotnet-install's folders.
        home.join(".symfony5/bin"),
        home.join(".dotnet"),
        home.join("scoop/shims"),
    ];
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        // winget's links for portable packages (uv).
        dirs.push(PathBuf::from(local).join("Microsoft/WinGet/Links"));
    }
    // winget's machine-wide installs land here; the daemon's PATH predates them.
    if cfg!(windows) {
        dirs.push(PathBuf::from("C:/Program Files/Git/cmd"));
        dirs.push(PathBuf::from("C:/Program Files/dotnet"));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("Composer/vendor/bin"));
    }
    dirs
}

/// Finds `program` on `path` (`.exe`/`.cmd`/`.bat` on Windows).
pub fn find(program: &str, path: &str) -> Option<PathBuf> {
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

/// The first line of `program --version`, or nothing after 5 seconds.
fn version(program: &Path, path: &str) -> Option<String> {
    let mut child = command(program)
        .arg("--version")
        .env("PATH", path)
        .env("NO_COLOR", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < Duration::from_secs(5) => {
                std::thread::sleep(Duration::from_millis(40));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    let line = out.lines().map(str::trim).find(|l| !l.is_empty())?;
    Some(strip_ansi(line).chars().take(80).collect())
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Every tool, found on `path` (already with LocDev's runtimes first and
/// [`extra_dirs`]). Runs `--version` for the ones found: blocking.
pub fn tools(path: &str, home: &Path, starters: &[Starter]) -> Vec<Tool> {
    DEFS.iter()
        .map(|def| {
            let found = def.programs.iter().find_map(|p| find(p, path));
            let used_by = starters
                .iter()
                .filter(|s| {
                    s.methods.iter().any(|m| m.tool == def.id)
                        || (def.id == "php" && s.family == "php")
                })
                .map(|s| s.name.clone())
                .collect();
            Tool {
                id: def.id.into(),
                name: def.name.into(),
                description: def.description.into(),
                docs: def.docs.into(),
                installed: found.is_some(),
                version: found.as_deref().and_then(|p| version(p, path)),
                path: found,
                install: match def.install {
                    Install::Runtime(runtime) => Some(ToolInstall::Runtime {
                        runtime: runtime.into(),
                    }),
                    _ => install_steps(def.id, home).map(|steps| ToolInstall::Command { steps }),
                },
                used_by,
            }
        })
        .collect()
}

/// The install steps of a tool LocDev can install by command.
pub fn install_steps(id: &str, home: &Path) -> Option<Vec<Step>> {
    let def = DEFS.iter().find(|d| d.id == id)?;
    let cmd = command_here(&def.install)?;
    Some(vec![Step {
        label: format!("Install {}", def.name),
        program: cmd.program.into(),
        args: cmd.args.iter().map(|a| (*a).to_string()).collect(),
        cwd: home.to_path_buf(),
        env: Vec::new(),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_are_fixed_official_commands() {
        let home = Path::new("/home/u");
        for def in DEFS {
            if let Install::Commands(cmds) = &def.install {
                assert!(!cmds.is_empty(), "{}", def.id);
                for c in *cmds {
                    // Only installers and package managers, never free text.
                    assert!(
                        [
                            "composer",
                            "npm",
                            "gem",
                            "winget",
                            "scoop",
                            "brew",
                            "sh",
                            "bash",
                            "powershell"
                        ]
                        .contains(&c.program),
                        "{}: {}",
                        def.id,
                        c.program
                    );
                    for a in c.args {
                        assert!(a.contains("https://") || !a.contains("://"), "{a}");
                    }
                }
            }
        }
        // Everything that has a command for every OS gets steps here.
        for id in ["laravel", "bun", "wp", "rails"] {
            assert!(install_steps(id, home).is_some(), "{id}");
        }
        assert_eq!(
            install_steps("bun", home).unwrap()[0].display(),
            "npm install -g bun"
        );
        assert!(
            install_steps("php", home).is_none(),
            "runtimes install as runtimes"
        );
    }
}
