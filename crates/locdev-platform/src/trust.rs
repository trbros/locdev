//! Adding the LocDev CA to (and removing it from) the places browsers and
//! tools look for trusted roots:
//!
//! - Linux: the system bundle (`update-ca-certificates` or `update-ca-trust`)
//!   for curl, Python, Node…, plus the per-user NSS databases that Chrome and
//!   Firefox use (no root needed for those).
//! - macOS: the System keychain, marked trusted for SSL.
//! - Windows: `Cert:\LocalMachine\Root`.
//!
//! Removal matches certificates by the CA's common name, so it also cleans
//! up CAs from earlier LocDev installs.

use std::path::{Path, PathBuf};

use crate::{Plan, Script};

const DEBIAN_DIR: &str = "/usr/local/share/ca-certificates";
const DEBIAN_FILE: &str = "/usr/local/share/ca-certificates/locdev-ca.crt";
const PKI_ANCHORS: [&str; 2] = [
    "/etc/pki/ca-trust/source/anchors",
    "/etc/ca-certificates/trust-source/anchors",
];
const MAC_KEYCHAIN: &str = "/Library/Keychains/System.keychain";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Store {
    /// Debian, Ubuntu and derivatives.
    CaCertificates,
    /// Fedora, RHEL, Arch: p11-kit anchors directory.
    CaTrust(PathBuf),
    MacKeychain,
    WindowsRoot,
    Unsupported,
}

fn detect() -> Store {
    if cfg!(target_os = "macos") {
        return Store::MacKeychain;
    }
    if cfg!(windows) {
        return Store::WindowsRoot;
    }
    if on_path("update-ca-certificates") && Path::new(DEBIAN_DIR).is_dir() {
        return Store::CaCertificates;
    }
    if on_path("update-ca-trust")
        && let Some(dir) = PKI_ANCHORS.iter().map(Path::new).find(|d| d.is_dir())
    {
        return Store::CaTrust(dir.to_path_buf());
    }
    Store::Unsupported
}

/// `ca_pem`/`ca_der` are the CA certificate files; `ca_name` its common name.
pub fn install(ca_pem: &Path, ca_der: &Path, ca_name: &str) -> Result<Plan, String> {
    let pem = quote(&ca_pem.display().to_string());
    let name = quote(ca_name);

    let (summary, script) = match detect() {
        Store::CaCertificates => (
            "Trust the LocDev CA system-wide (update-ca-certificates)".to_string(),
            Script::Sh(format!(
                "set -e\ninstall -m 0644 {pem} {DEBIAN_FILE}\nupdate-ca-certificates\n"
            )),
        ),
        Store::CaTrust(dir) => (
            "Trust the LocDev CA system-wide (update-ca-trust)".to_string(),
            Script::Sh(format!(
                "set -e\ninstall -m 0644 {pem} {}/locdev-ca.pem\nupdate-ca-trust\n",
                dir.display()
            )),
        ),
        Store::MacKeychain => (
            "Trust the LocDev CA in the System keychain".to_string(),
            Script::Sh(format!(
                "set -e\n\
                 while security delete-certificate -c {name} {MAC_KEYCHAIN} >/dev/null 2>&1; do :; done\n\
                 security add-trusted-cert -d -r trustRoot -k {MAC_KEYCHAIN} {pem}\n"
            )),
        ),
        Store::WindowsRoot => (
            "Trust the LocDev CA in the Windows root store".to_string(),
            Script::PowerShell(format!(
                "$ErrorActionPreference = 'Stop'\n\
                 Get-ChildItem Cert:\\LocalMachine\\Root | Where-Object Subject -like '*CN={ca_name}*' | Remove-Item\n\
                 Import-Certificate -FilePath '{}' -CertStoreLocation Cert:\\LocalMachine\\Root | Out-Null\n",
                ca_der.display().to_string().replace('\'', "''")
            )),
        ),
        Store::Unsupported => {
            return Err(format!(
                "LocDev does not know this system's trust store; add {} to it manually",
                ca_pem.display()
            ));
        }
    };

    let (summary, user_script) = with_browsers(summary, |db| {
        format!(
            "certutil -d \"sql:{db}\" -D -n {name} >/dev/null 2>&1 || true\n\
             certutil -d \"sql:{db}\" -A -t 'C,,' -n {name} -i {pem}\n"
        )
    });

    Ok(Plan {
        summary,
        script,
        user_script,
    })
}

pub fn uninstall(ca_name: &str) -> Result<Plan, String> {
    let name = quote(ca_name);

    let (summary, script) = match detect() {
        Store::CaCertificates => (
            "Stop trusting the LocDev CA system-wide".to_string(),
            Script::Sh(format!(
                "set -e\nrm -f {DEBIAN_FILE}\nupdate-ca-certificates\n"
            )),
        ),
        Store::CaTrust(dir) => (
            "Stop trusting the LocDev CA system-wide".to_string(),
            Script::Sh(format!(
                "set -e\nrm -f {}/locdev-ca.pem\nupdate-ca-trust\n",
                dir.display()
            )),
        ),
        Store::MacKeychain => (
            "Remove the LocDev CA from the System keychain".to_string(),
            Script::Sh(format!(
                "while security delete-certificate -c {name} {MAC_KEYCHAIN} >/dev/null 2>&1; do :; done\n"
            )),
        ),
        Store::WindowsRoot => (
            "Remove the LocDev CA from the Windows root store".to_string(),
            Script::PowerShell(format!(
                "Get-ChildItem Cert:\\LocalMachine\\Root | Where-Object Subject -like '*CN={ca_name}*' | Remove-Item\n"
            )),
        ),
        Store::Unsupported => {
            return Err("nothing to remove: LocDev cannot manage this system's trust store".into());
        }
    };

    let (summary, user_script) = with_browsers(summary, |db| {
        format!("certutil -d \"sql:{db}\" -D -n {name} >/dev/null 2>&1 || true\n")
    });

    Ok(Plan {
        summary,
        script,
        user_script,
    })
}

/// Adds the per-user NSS step (Chrome/Firefox on Linux) when there are
/// browser databases and `certutil` to edit them.
fn with_browsers(summary: String, step: impl Fn(&str) -> String) -> (String, Option<Script>) {
    let dbs = nss_databases();
    if dbs.is_empty() {
        return (format!("{summary}."), None);
    }
    if !on_path("certutil") {
        return (
            format!(
                "{summary}. Chrome/Firefox keep their own store: install `certutil` (libnss3-tools) and re-run to cover them."
            ),
            None,
        );
    }

    let mut script = String::new();
    for db in &dbs {
        script.push_str(&step(&db.display().to_string().replace('"', "\\\"")));
    }
    (
        format!(
            "{summary}, then update {} browser certificate store(s) as you.",
            dbs.len()
        ),
        Some(Script::Sh(script)),
    )
}

/// NSS databases of Chrome/Chromium and Firefox profiles (native, snap and
/// flatpak installs). Only relevant on Linux; elsewhere browsers use the OS
/// store.
pub fn nss_databases() -> Vec<PathBuf> {
    if !cfg!(target_os = "linux") {
        return Vec::new();
    }
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    nss_databases_in(&home)
}

fn nss_databases_in(home: &Path) -> Vec<PathBuf> {
    let mut dbs = Vec::new();

    for chrome in [".pki/nssdb", "snap/chromium/current/.pki/nssdb"] {
        let dir = home.join(chrome);
        if dir.join("cert9.db").is_file() {
            dbs.push(dir);
        }
    }

    for profiles in [
        ".mozilla/firefox",
        "snap/firefox/common/.mozilla/firefox",
        ".var/app/org.mozilla.firefox/.mozilla/firefox",
    ] {
        let Ok(entries) = std::fs::read_dir(home.join(profiles)) else {
            continue;
        };
        let mut found: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.join("cert9.db").is_file())
            .collect();
        found.sort();
        dbs.extend(found);
    }
    dbs
}

/// Whether every browser database trusts the CA. `None` when there are no
/// databases or no `certutil` to ask.
pub fn browsers_trust(ca_name: &str) -> Option<bool> {
    let dbs = nss_databases();
    if dbs.is_empty() || !on_path("certutil") {
        return None;
    }
    Some(dbs.iter().all(|db| {
        locdev_core::process::command("certutil")
            .arg("-d")
            .arg(format!("sql:{}", db.display()))
            .args(["-L", "-n", ca_name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }))
}

/// Single-quotes for sh.
fn quote(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "'\\''"))
}

fn on_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(bin).is_file()))
        || Path::new("/usr/sbin").join(bin).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_paths_for_sh() {
        assert_eq!(quote("/a b/c"), "'/a b/c'");
        assert_eq!(quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn finds_browser_databases() {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        for dir in [
            ".pki/nssdb",
            "snap/firefox/common/.mozilla/firefox/abc.default",
            ".mozilla/firefox/xyz.default-release",
        ] {
            std::fs::create_dir_all(h.join(dir)).unwrap();
            std::fs::write(h.join(dir).join("cert9.db"), b"").unwrap();
        }
        // A profile directory without a database is not a store.
        std::fs::create_dir_all(h.join(".mozilla/firefox/Crash Reports")).unwrap();

        let found = nss_databases_in(h);
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found[0].ends_with(".pki/nssdb"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_plan_installs_the_given_file() {
        if detect() == Store::Unsupported {
            return;
        }
        let plan = install(
            Path::new("/home/u/My CA/ca.pem"),
            Path::new("/x/ca.cer"),
            "LocDev Local CA",
        )
        .unwrap();
        assert!(plan.script.source().contains("'/home/u/My CA/ca.pem'"));
        let undo = uninstall("LocDev Local CA").unwrap();
        assert!(undo.script.source().contains("rm -f"));
    }
}
