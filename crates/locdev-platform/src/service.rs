//! Starting the daemon at login, per user, without administrator rights:
//!
//! - Linux: a `systemd --user` unit (`~/.config/systemd/user/locdevd.service`).
//! - macOS: a LaunchAgent (`~/Library/LaunchAgents/dev.locdev.locdevd.plist`).
//! - Windows: a value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//!
//! The daemon refuses to start twice, so a login item and an app-launched
//! daemon never conflict.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

use crate::MARKER;

const UNIT: &str = "locdevd.service";
const AGENT_LABEL: &str = "dev.locdev.locdevd";
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "LocDev";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceStatus {
    /// `systemd --user`, `launchd` or `Windows Run key`; empty if unsupported.
    pub manager: String,
    pub installed: bool,
    /// Where the login item lives (file or registry key).
    pub location: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Manager {
    SystemdUser,
    LaunchAgent,
    WindowsRun,
    Unsupported,
}

fn manager() -> Manager {
    if cfg!(target_os = "macos") {
        Manager::LaunchAgent
    } else if cfg!(windows) {
        Manager::WindowsRun
    } else if cfg!(target_os = "linux") && Path::new("/run/systemd/system").is_dir() {
        Manager::SystemdUser
    } else {
        Manager::Unsupported
    }
}

fn home() -> io::Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))
}

fn unit_path() -> io::Result<PathBuf> {
    let config = match std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => home()?.join(".config"),
    };
    Ok(config.join("systemd/user").join(UNIT))
}

fn agent_path() -> io::Result<PathBuf> {
    Ok(home()?
        .join("Library/LaunchAgents")
        .join(format!("{AGENT_LABEL}.plist")))
}

pub fn status() -> ServiceStatus {
    let (manager_name, location) = match manager() {
        Manager::SystemdUser => ("systemd --user", unit_path().ok()),
        Manager::LaunchAgent => ("launchd", agent_path().ok()),
        Manager::WindowsRun => {
            let installed = locdev_core::process::command("reg")
                .args(["query", RUN_KEY, "/v", RUN_VALUE])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success());
            return ServiceStatus {
                manager: "Windows Run key".into(),
                installed,
                location: Some(format!(r"{RUN_KEY}\{RUN_VALUE}")),
            };
        }
        Manager::Unsupported => {
            return ServiceStatus {
                manager: String::new(),
                installed: false,
                location: None,
            };
        }
    };

    let installed = location
        .as_ref()
        .is_some_and(|p| std::fs::read_to_string(p).is_ok_and(|s| s.contains(MARKER)));
    ServiceStatus {
        manager: manager_name.into(),
        installed,
        location: location.map(|p| p.display().to_string()),
    }
}

/// Registers `daemon` to start at login. `home_override` carries a custom
/// `LOCDEV_HOME` into the service environment.
pub fn install(daemon: &Path, home_override: Option<&Path>) -> io::Result<()> {
    let daemon = daemon.canonicalize()?;

    match manager() {
        Manager::SystemdUser => {
            let path = unit_path()?;
            std::fs::create_dir_all(path.parent().expect("unit path has a parent"))?;
            std::fs::write(&path, systemd_unit(&daemon, home_override))?;
            run("systemctl", &["--user", "daemon-reload"])?;
            run("systemctl", &["--user", "enable", UNIT])
        }
        Manager::LaunchAgent => {
            let path = agent_path()?;
            std::fs::create_dir_all(path.parent().expect("agent path has a parent"))?;
            std::fs::write(&path, launch_agent(&daemon, home_override))
        }
        Manager::WindowsRun => {
            // The Run key has no environment of its own; a custom home is
            // passed through `cmd /C set … &&`.
            let command = match home_override {
                Some(home) => format!(
                    "cmd /C \"set LOCDEV_HOME={}&& \"{}\"\"",
                    home.display(),
                    daemon.display()
                ),
                None => format!("\"{}\"", daemon.display()),
            };
            run(
                "reg",
                &[
                    "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &command, "/f",
                ],
            )
        }
        Manager::Unsupported => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no supported service manager (systemd, launchd or Windows) found",
        )),
    }
}

pub fn uninstall() -> io::Result<()> {
    match manager() {
        Manager::SystemdUser => {
            let _ = run("systemctl", &["--user", "disable", UNIT]);
            remove_if_ours(&unit_path()?)?;
            let _ = run("systemctl", &["--user", "daemon-reload"]);
            Ok(())
        }
        Manager::LaunchAgent => {
            if let Ok(uid) = Command::new("id").arg("-u").output() {
                let uid = String::from_utf8_lossy(&uid.stdout).trim().to_string();
                let _ = Command::new("launchctl")
                    .args(["bootout", &format!("gui/{uid}/{AGENT_LABEL}")])
                    .stderr(Stdio::null())
                    .status();
            }
            remove_if_ours(&agent_path()?)
        }
        Manager::WindowsRun => {
            let _ = run("reg", &["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]);
            Ok(())
        }
        Manager::Unsupported => Ok(()),
    }
}

fn remove_if_ours(path: &Path) -> io::Result<()> {
    match std::fs::read_to_string(path) {
        Ok(contents) if contents.contains(MARKER) => std::fs::remove_file(path),
        Ok(_) => Err(io::Error::other(format!(
            "{} was not created by LocDev; leaving it alone",
            path.display()
        ))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

fn run(program: &str, args: &[&str]) -> io::Result<()> {
    let status = locdev_core::process::command(program)
        .args(args)
        .stdout(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "`{program} {}` failed ({status})",
            args.join(" ")
        )))
    }
}

fn systemd_unit(daemon: &Path, home: Option<&Path>) -> String {
    let env = home
        .map(|h| format!("Environment=\"LOCDEV_HOME={}\"\n", h.display()))
        .unwrap_or_default();
    format!(
        "# {MARKER}. Remove with `locdev service uninstall`.\n\
         [Unit]\n\
         Description=LocDev local development daemon\n\
         \n\
         [Service]\n\
         ExecStart=\"{}\"\n\
         {env}\
         Restart=on-failure\n\
         RestartSec=2\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        daemon.display()
    )
}

fn launch_agent(daemon: &Path, home: Option<&Path>) -> String {
    let env = home
        .map(|h| {
            format!(
                "  <key>EnvironmentVariables</key>\n  <dict><key>LOCDEV_HOME</key><string>{}</string></dict>\n",
                xml_escape(&h.display().to_string())
            )
        })
        .unwrap_or_default();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!-- {MARKER}. Remove with `locdev service uninstall`. -->\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \x20 <key>Label</key><string>{AGENT_LABEL}</string>\n\
         \x20 <key>ProgramArguments</key><array><string>{}</string></array>\n\
         {env}\
         \x20 <key>RunAtLoad</key><true/>\n\
         \x20 <key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>\n\
         \x20 <key>ProcessType</key><string>Background</string>\n\
         </dict>\n\
         </plist>\n",
        xml_escape(&daemon.display().to_string())
    )
}

fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systemd_unit_quotes_the_path() {
        let unit = systemd_unit(Path::new("/opt/Loc Dev/locdevd"), Some(Path::new("/tmp/h")));
        assert!(unit.contains("ExecStart=\"/opt/Loc Dev/locdevd\""));
        assert!(unit.contains("Environment=\"LOCDEV_HOME=/tmp/h\""));
        assert!(unit.contains(MARKER));
        assert!(unit.contains("Restart=on-failure"));
    }

    #[test]
    fn launch_agent_escapes_xml() {
        let plist = launch_agent(Path::new("/Applications/A&B.app/locdevd"), None);
        assert!(plist.contains("<string>/Applications/A&amp;B.app/locdevd</string>"));
        assert!(plist.contains(AGENT_LABEL));
        assert!(!plist.contains("EnvironmentVariables"));
    }

    #[test]
    fn refuses_to_remove_foreign_files() {
        let dir = tempfile::tempdir().unwrap();
        let foreign = dir.path().join("locdevd.service");
        std::fs::write(&foreign, "[Unit]\n").unwrap();
        assert!(remove_if_ours(&foreign).is_err());
        assert!(foreign.exists());

        let ours = dir.path().join("ours.service");
        std::fs::write(&ours, format!("# {MARKER}\n")).unwrap();
        remove_if_ours(&ours).unwrap();
        assert!(!ours.exists());
        remove_if_ours(&ours).unwrap();
    }
}
