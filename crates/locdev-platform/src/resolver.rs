//! Routing `*.<tld>` lookups to the LocDev DNS server, per OS:
//!
//! - Linux: a systemd-resolved drop-in with a routing-only domain (`~test`),
//!   so only `.test` queries reach LocDev.
//! - macOS: `/etc/resolver/<tld>`, the native per-domain resolver.
//! - Windows: an NRPT rule for the namespace `.<tld>`, which covers every
//!   name under it at any depth. NRPT cannot carry a port, so the DNS server
//!   listens on port 53 there, on 127.0.0.1 or, when that's taken, LocDev's
//!   own loopback address; the rule names whichever is bound. Chromium
//!   browsers see NRPT rules and use the system resolver because of them
//!   (`net/dns/public/win_dns_system_settings.cc`).
//!
//! A plain hosts file is not an option: it has no wildcards, so every site
//! would need its own privileged edit.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{MARKER, Plan, Script};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    SystemdResolved,
    MacResolver,
    WindowsNrpt,
    Unsupported,
}

impl Backend {
    pub fn label(self) -> &'static str {
        match self {
            Self::SystemdResolved => "systemd-resolved",
            Self::MacResolver => "/etc/resolver",
            Self::WindowsNrpt => "Windows NRPT",
            Self::Unsupported => "unsupported",
        }
    }
}

const RESOLVED_DIR: &str = "/etc/systemd/resolved.conf.d";
const MAC_RESOLVER_DIR: &str = "/etc/resolver";

pub fn detect() -> Backend {
    if cfg!(target_os = "macos") {
        Backend::MacResolver
    } else if cfg!(windows) {
        Backend::WindowsNrpt
    } else if cfg!(target_os = "linux") && uses_systemd_resolved(Path::new("/")) {
        Backend::SystemdResolved
    } else {
        Backend::Unsupported
    }
}

/// resolved must be running *and* be what applications ask: either through
/// the 127.0.0.53 stub in resolv.conf or through nss-resolve.
fn uses_systemd_resolved(root: &Path) -> bool {
    if !root.join("run/systemd/resolve").is_dir() {
        return false;
    }
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap_or_default();
    let stub = read("etc/resolv.conf")
        .lines()
        .any(|l| l.trim_start().starts_with("nameserver 127.0.0.53"));
    let nss = read("etc/nsswitch.conf")
        .lines()
        .any(|l| l.trim_start().starts_with("hosts:") && l.contains("resolve"));
    stub || nss
}

fn dropin_path(tld: &str) -> PathBuf {
    Path::new(RESOLVED_DIR).join(format!("locdev-{tld}.conf"))
}

fn mac_path(tld: &str) -> PathBuf {
    Path::new(MAC_RESOLVER_DIR).join(tld)
}

/// Routes `*.<tld>` to the DNS server at `addr` (what the daemon bound).
pub fn install(tld: &str, addr: SocketAddr) -> Result<Plan, String> {
    match detect() {
        Backend::SystemdResolved => Ok(resolved_plan(tld, addr)),
        Backend::MacResolver => Ok(mac_plan(tld, addr)),
        Backend::WindowsNrpt => windows_plan(tld, addr),
        Backend::Unsupported => Err(unsupported_help(tld, addr)),
    }
}

fn resolved_plan(tld: &str, addr: SocketAddr) -> Plan {
    let path = dropin_path(tld);
    Plan {
        user_script: None,
        summary: format!(
            "Route *.{tld} to LocDev ({addr}) by adding {} and restarting systemd-resolved.",
            path.display()
        ),
        script: Script::Sh(format!(
            "set -e\n\
             mkdir -p {RESOLVED_DIR}\n\
             cat > {path} <<'LOCDEV'\n\
             # {MARKER}: send *.{tld} lookups to the LocDev daemon.\n\
             # Remove with `locdev dns uninstall`.\n\
             [Resolve]\n\
             DNS={addr}\n\
             Domains=~{tld}\n\
             LOCDEV\n\
             systemctl restart systemd-resolved\n",
            path = path.display()
        )),
    }
}

fn mac_plan(tld: &str, addr: SocketAddr) -> Plan {
    let path = mac_path(tld);
    Plan {
        user_script: None,
        summary: format!(
            "Route *.{tld} to LocDev ({addr}) by adding {}.",
            path.display()
        ),
        script: Script::Sh(format!(
            "set -e\n\
             mkdir -p {MAC_RESOLVER_DIR}\n\
             cat > {path} <<'LOCDEV'\n\
             # {MARKER}. Remove with `locdev dns uninstall`.\n\
             nameserver {ip}\n\
             port {port}\n\
             LOCDEV\n\
             dscacheutil -flushcache\n\
             killall -HUP mDNSResponder || true\n",
            path = path.display(),
            ip = addr.ip(),
            port = addr.port(),
        )),
    }
}

fn windows_plan(tld: &str, addr: SocketAddr) -> Result<Plan, String> {
    if addr.port() != 53 {
        return Err(format!(
            "Windows NRPT rules cannot name a port, so the DNS server must listen on port 53; it's on {addr} (dns.port in config.toml)"
        ));
    }
    let ip = addr.ip();
    Ok(Plan {
        user_script: None,
        summary: format!(
            "Route *.{tld} (every subdomain, at any depth) to LocDev's DNS server on {ip} with a Name Resolution Policy Table rule. \
             Only .{tld} names are affected; the hosts file isn't touched."
        ),
        script: Script::PowerShell(format!(
            "$ErrorActionPreference = 'Stop'\n\
             Get-DnsClientNrptRule | Where-Object Comment -eq '{MARKER}' | Remove-DnsClientNrptRule -Force\n\
             Add-DnsClientNrptRule -Namespace '.{tld}' -NameServers '{ip}' -Comment '{MARKER}' | Out-Null\n\
             Clear-DnsClientCache\n"
        )),
    })
}

pub fn uninstall(tld: &str) -> Result<Plan, String> {
    match detect() {
        Backend::SystemdResolved => {
            let path = dropin_path(tld);
            Ok(Plan {
                user_script: None,
                summary: format!(
                    "Stop routing *.{tld} to LocDev: remove {} and restart systemd-resolved.",
                    path.display()
                ),
                script: Script::Sh(format!(
                    "set -e\nrm -f {}\nsystemctl restart systemd-resolved\n",
                    path.display()
                )),
            })
        }
        Backend::MacResolver => {
            let path = mac_path(tld);
            Ok(Plan {
                user_script: None,
                summary: format!("Stop routing *.{tld} to LocDev: remove {}.", path.display()),
                // Only delete the file if LocDev wrote it.
                script: Script::Sh(format!(
                    "set -e\n\
                     if grep -q '{MARKER}' {path} 2>/dev/null; then rm -f {path}; fi\n\
                     dscacheutil -flushcache\n\
                     killall -HUP mDNSResponder || true\n",
                    path = path.display()
                )),
            })
        }
        Backend::WindowsNrpt => Ok(Plan {
            user_script: None,
            summary: format!("Stop routing *.{tld} to LocDev: remove LocDev's NRPT rule."),
            script: Script::PowerShell(format!(
                "$ErrorActionPreference = 'Stop'\n\
                 Get-DnsClientNrptRule | Where-Object Comment -eq '{MARKER}' | Remove-DnsClientNrptRule -Force\n\
                 Clear-DnsClientCache\n"
            )),
        }),
        Backend::Unsupported => {
            Err("nothing to remove: LocDev cannot manage this system's resolver".into())
        }
    }
}

fn unsupported_help(tld: &str, addr: SocketAddr) -> String {
    format!(
        "this system does not use systemd-resolved, so LocDev cannot configure it automatically. \
         Point *.{tld} at {addr} in your resolver (dnsmasq: `server=/{tld}/{ip}#{port}`), \
         or use http://<site>.localhost, which needs no DNS",
        ip = addr.ip(),
        port = addr.port(),
    )
}

/// Whether LocDev's resolver configuration is present; `None` when the
/// system is unsupported.
pub fn installed(tld: &str) -> Option<bool> {
    match detect() {
        Backend::Unsupported => None,
        _ => Some(configured(tld).is_some()),
    }
}

/// Where LocDev's installed configuration sends `*.<tld>` (`127.0.0.1:10053`),
/// so a stale one (another port, another address) can be noticed.
pub fn configured(tld: &str) -> Option<SocketAddr> {
    match detect() {
        Backend::SystemdResolved => {
            parse_resolved(&std::fs::read_to_string(dropin_path(tld)).ok()?)
        }
        Backend::MacResolver => parse_mac(&std::fs::read_to_string(mac_path(tld)).ok()?),
        Backend::WindowsNrpt => windows_rules(tld).ours,
        Backend::Unsupported => None,
    }
}

fn parse_resolved(text: &str) -> Option<SocketAddr> {
    if !text.contains(MARKER) {
        return None;
    }
    text.lines()
        .find_map(|l| l.trim().strip_prefix("DNS=")?.trim().parse().ok())
}

fn parse_mac(text: &str) -> Option<SocketAddr> {
    if !text.contains(MARKER) {
        return None;
    }
    let value = |key: &str| {
        text.lines()
            .find_map(|l| l.trim().strip_prefix(key).map(|v| v.trim().to_string()))
    };
    let ip: std::net::IpAddr = value("nameserver ")?.parse().ok()?;
    let port = value("port ").and_then(|p| p.parse().ok()).unwrap_or(53);
    Some(SocketAddr::new(ip, port))
}

#[derive(Debug, Default, PartialEq)]
struct NrptRules {
    ours: Option<SocketAddr>,
    /// Other rules for the same namespace (another tool's, or a policy's).
    others: Vec<String>,
}

/// Reads the NRPT rules for `.<tld>` (one PowerShell call, no window).
fn windows_rules(tld: &str) -> NrptRules {
    if !cfg!(windows) {
        return NrptRules::default();
    }
    let script = format!(
        "Get-DnsClientNrptRule | Where-Object {{ $_.Namespace -contains '.{tld}' }} | \
         ForEach-Object {{ '{{0}}|{{1}}' -f $_.Comment, ($_.NameServers -join ',') }}"
    );
    let output = locdev_core::process::command("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    match output {
        Ok(out) if out.status.success() => parse_nrpt(&String::from_utf8_lossy(&out.stdout)),
        _ => NrptRules::default(),
    }
}

/// Lines of `comment|ns1,ns2`.
fn parse_nrpt(text: &str) -> NrptRules {
    let mut rules = NrptRules::default();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (comment, servers) = line.rsplit_once('|').unwrap_or((line, ""));
        let first = servers
            .split(',')
            .next()
            .and_then(|ip| ip.trim().parse::<std::net::IpAddr>().ok());
        if comment == MARKER {
            rules.ours = first.map(|ip| SocketAddr::new(ip, 53));
        } else {
            rules
                .others
                .push(format!("NRPT rule \"{comment}\" → {servers}"));
        }
    }
    rules
}

/// Other tools' configuration that also claims the TLD. Their files are
/// never touched; they are only reported.
pub fn conflicts(tld: &str) -> Vec<String> {
    match detect() {
        Backend::SystemdResolved => resolved_conflicts(Path::new(RESOLVED_DIR), tld),
        Backend::MacResolver => {
            let path = mac_path(tld);
            match std::fs::read_to_string(&path) {
                Ok(contents) if !contents.contains(MARKER) => vec![path.display().to_string()],
                _ => Vec::new(),
            }
        }
        Backend::WindowsNrpt => windows_rules(tld).others,
        Backend::Unsupported => Vec::new(),
    }
}

fn resolved_conflicts(dir: &Path, tld: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let routing = format!("~{tld}");

    let mut found: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "conf"))
        .filter(|p| {
            std::fs::read_to_string(p).is_ok_and(|s| {
                !s.contains(MARKER)
                    && s.lines()
                        .filter(|l| l.trim_start().starts_with("Domains="))
                        .any(|l| l.split(['=', ' ']).any(|d| d == routing))
            })
        })
        .map(|p| p.display().to_string())
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_resolved_via_stub_or_nss() {
        let root = tempfile::tempdir().unwrap();
        let r = root.path();
        assert!(!uses_systemd_resolved(r));

        std::fs::create_dir_all(r.join("run/systemd/resolve")).unwrap();
        std::fs::create_dir_all(r.join("etc")).unwrap();
        std::fs::write(r.join("etc/resolv.conf"), "nameserver 1.1.1.1\n").unwrap();
        assert!(
            !uses_systemd_resolved(r),
            "resolved runs but nobody asks it"
        );

        std::fs::write(
            r.join("etc/nsswitch.conf"),
            "hosts: files resolve [!UNAVAIL=return] dns\n",
        )
        .unwrap();
        assert!(uses_systemd_resolved(r));

        std::fs::remove_file(r.join("etc/nsswitch.conf")).unwrap();
        std::fs::write(
            r.join("etc/resolv.conf"),
            "nameserver 127.0.0.53\noptions edns0\n",
        )
        .unwrap();
        assert!(uses_systemd_resolved(r));
    }

    #[test]
    fn finds_other_tools_claiming_the_tld() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        std::fs::write(
            d.join("yerd-test.conf"),
            "[Resolve]\nDNS=127.0.0.1:1053\nDomains=~test\n",
        )
        .unwrap();
        std::fs::write(d.join("other.conf"), "[Resolve]\nDomains=~corp ~testing\n").unwrap();
        std::fs::write(
            d.join("locdev-test.conf"),
            format!("# {MARKER}\n[Resolve]\nDomains=~test\n"),
        )
        .unwrap();
        std::fs::write(d.join("notes.txt"), "Domains=~test\n").unwrap();

        let found = resolved_conflicts(d, "test");
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("yerd-test.conf"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_plan_is_scoped_to_the_tld() {
        if detect() != Backend::SystemdResolved {
            return;
        }
        let plan = install("test", "127.0.0.1:10053".parse().unwrap()).unwrap();
        let script = plan.script.source();
        assert!(script.contains("DNS=127.0.0.1:10053"));
        assert!(script.contains("Domains=~test"));
        assert!(script.contains(MARKER));

        let undo = uninstall("test").unwrap();
        assert!(
            undo.script
                .source()
                .contains("rm -f /etc/systemd/resolved.conf.d/locdev-test.conf")
        );
    }

    #[test]
    fn windows_plan_names_the_bound_address_and_needs_port_53() {
        let plan = windows_plan("test", "127.53.0.1:53".parse().unwrap()).unwrap();
        let script = plan.script.source();
        assert!(script.contains("-Namespace '.test' -NameServers '127.53.0.1'"));
        assert!(
            script.contains("Remove-DnsClientNrptRule"),
            "replaces its own rule only"
        );
        assert!(!script.to_lowercase().contains("hosts"));
        assert!(
            windows_plan("test", "127.0.0.1:10053".parse().unwrap())
                .unwrap_err()
                .contains("port 53")
        );
    }

    #[test]
    fn reads_back_where_the_configuration_points() {
        let linux = format!("# {MARKER}: x\n[Resolve]\nDNS=127.0.0.1:10053\nDomains=~test\n");
        assert_eq!(
            parse_resolved(&linux),
            Some("127.0.0.1:10053".parse().unwrap())
        );
        assert_eq!(parse_resolved("[Resolve]\nDNS=1.1.1.1\n"), None, "not ours");
        let mac = format!("# {MARKER}.\nnameserver 127.0.0.1\nport 10053\n");
        assert_eq!(parse_mac(&mac), Some("127.0.0.1:10053".parse().unwrap()));
        let nrpt = parse_nrpt(&format!("{MARKER}|127.53.0.1\nHerd|127.0.0.1\n"));
        assert_eq!(nrpt.ours, Some("127.53.0.1:53".parse().unwrap()));
        assert_eq!(
            nrpt.others,
            vec!["NRPT rule \"Herd\" → 127.0.0.1".to_string()]
        );
    }
}
