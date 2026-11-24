//! Environment health checks, shown by `locdev doctor` and the desktop app.

use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::PathBuf;
use std::time::Duration;

use locdev_core::model::{
    Check, CheckLevel, DnsStatus, Project, ProjectKind, ProxyStatus, ResolvedRuntime, Routing,
    RuntimeCategory, SupportState, TlsStatus,
};
use locdev_core::paths::Paths;
use tokio::net::TcpStream;

const PROBE_TIMEOUT: Duration = Duration::from_millis(500);

pub async fn run(
    paths: &Paths,
    projects: &[Project],
    proxy: &ProxyStatus,
    dns: &DnsStatus,
    tls: &TlsStatus,
) -> Vec<Check> {
    let mut checks = vec![
        config_writable(paths),
        project_paths(projects),
        proxy_check(proxy),
    ];

    for (port, label) in [(80, "HTTP port 80"), (443, "HTTPS port 443")] {
        if proxy.port == Some(port) || tls.port == Some(port) {
            checks.push(Check::new(
                &format!("port-{port}"),
                label,
                CheckLevel::Ok,
                "used by the LocDev proxy",
            ));
        } else {
            checks.push(privileged_port(port, label).await);
        }
    }

    checks.extend(tls_checks(tls));
    checks.push(dns_server_check(dns));
    checks.push(resolver_check(dns, proxy));

    for tool in TOOLS {
        checks.push(tool_check(tool, projects));
    }

    checks
}

fn config_writable(paths: &Paths) -> Check {
    let probe = paths.config_dir.join(".doctor-probe");
    let result = std::fs::write(&probe, b"ok").and_then(|()| std::fs::remove_file(&probe));

    match result {
        Ok(()) => Check::new(
            "config",
            "Configuration folder",
            CheckLevel::Ok,
            paths.config_dir.display().to_string(),
        ),
        Err(e) => Check::new(
            "config",
            "Configuration folder",
            CheckLevel::Error,
            format!("{} is not writable: {e}", paths.config_dir.display()),
        )
        .with_remedy(format!(
            "check the permissions of {}",
            paths.config_dir.display()
        )),
    }
}

fn project_paths(projects: &[Project]) -> Check {
    let missing: Vec<&str> = projects
        .iter()
        .filter(|p| !p.path.is_dir())
        .map(|p| p.name.as_str())
        .collect();

    if missing.is_empty() {
        Check::new(
            "projects",
            "Project folders",
            CheckLevel::Ok,
            format!("{} registered, all folders present", projects.len()),
        )
    } else {
        Check::new(
            "projects",
            "Project folders",
            CheckLevel::Warn,
            format!("missing folder: {}", missing.join(", ")),
        )
        .with_remedy("locdev remove <name>   # or restore the folder")
    }
}

async fn privileged_port(port: u16, label: &str) -> Check {
    let id = format!("port-{port}");
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));

    let in_use = tokio::time::timeout(PROBE_TIMEOUT, TcpStream::connect(addr))
        .await
        .is_ok_and(|r| r.is_ok());
    if in_use {
        return Check::new(
            &id,
            label,
            CheckLevel::Warn,
            "in use by another program; the LocDev proxy will need it",
        )
        .with_remedy(if cfg!(windows) {
            format!("netstat -ano | findstr :{port}")
        } else {
            format!("sudo lsof -iTCP:{port} -sTCP:LISTEN")
        });
    }

    match TcpListener::bind(addr) {
        Ok(_) => Check::new(&id, label, CheckLevel::Ok, "available"),
        Err(e) if e.kind() == ErrorKind::PermissionDenied => Check::new(
            &id,
            label,
            CheckLevel::Info,
            "free, but binding it needs a one-time privilege grant (asked when the proxy is enabled)",
        ),
        Err(e) => Check::new(&id, label, CheckLevel::Warn, e.to_string()),
    }
}

fn tls_checks(tls: &TlsStatus) -> Vec<Check> {
    if !tls.enabled {
        return vec![Check::new(
            "https",
            "HTTPS",
            CheckLevel::Info,
            "disabled in config.toml",
        )];
    }

    let https = match (tls.port, &tls.error) {
        (Some(port), None) => Check::new(
            "https",
            "HTTPS proxy",
            CheckLevel::Ok,
            format!("listening on 127.0.0.1:{port}"),
        ),
        (Some(port), Some(reason)) => Check::new(
            "https",
            "HTTPS proxy",
            CheckLevel::Warn,
            format!("{reason}; serving on port {port} instead"),
        )
        .with_remedy(format!(
            "free port {} or set tls.https_port in config.toml",
            tls.requested_port
        )),
        (None, reason) => Check::new(
            "https",
            "HTTPS proxy",
            CheckLevel::Error,
            reason.clone().unwrap_or_else(|| "not listening".into()),
        )
        .with_remedy("change tls.https_port / tls.fallback_port in config.toml"),
    };

    let Some(fingerprint) = &tls.fingerprint else {
        return vec![https];
    };
    let short = fingerprint.get(..23).unwrap_or(fingerprint);

    let system = if tls.trusted {
        Check::new(
            "ca",
            "Local CA",
            CheckLevel::Ok,
            format!("trusted by the system ({short}…)"),
        )
    } else {
        Check::new(
            "ca",
            "Local CA",
            CheckLevel::Info,
            format!("not trusted yet, so browsers warn on https:// ({short}…)"),
        )
        .with_remedy("locdev tls trust")
    };

    let mut checks = vec![https, system];
    if tls.trusted && tls.browsers == Some(false) {
        checks.push(
            Check::new(
                "ca-browsers",
                "Browser certificate stores",
                CheckLevel::Warn,
                "Chrome/Firefox keep their own store and don't trust the CA yet",
            )
            .with_remedy("locdev tls trust"),
        );
    }
    checks
}

fn dns_server_check(dns: &DnsStatus) -> Check {
    const LABEL: &str = "DNS server";

    match (dns.enabled, dns.port, &dns.error) {
        (false, ..) => Check::new("dns", LABEL, CheckLevel::Info, "disabled in config.toml"),
        (_, Some(port), _) => Check::new(
            "dns",
            LABEL,
            CheckLevel::Ok,
            format!("answering *.test on 127.0.0.1:{port}"),
        ),
        (_, None, error) => Check::new(
            "dns",
            LABEL,
            CheckLevel::Error,
            error.clone().unwrap_or_else(|| "not listening".into()),
        )
        .with_remedy(if cfg!(windows) {
            // NRPT can't name a port: 53 must be freed, not changed.
            "stop the program holding port 53 (e.g. another local DNS like Acrylic) and run `locdev restart`"
        } else {
            "set dns.port in config.toml to a free port and run `locdev restart`"
        }),
    }
}

fn resolver_check(dns: &DnsStatus, proxy: &ProxyStatus) -> Check {
    const LABEL: &str = ".test resolver";
    let others = dns.conflicts.join(", ");

    // Installed, but for an address or port the server isn't on (the
    // Windows fallback address, a changed dns.port): reinstall rewrites it.
    if let (Some(points), Some(listen)) = (dns.points_to.as_deref(), dns.listen())
        && points != listen.to_string()
    {
        return Check::new(
            "resolver",
            LABEL,
            CheckLevel::Error,
            format!("the resolver config sends *.test to {points}, but LocDev's DNS server is on {listen}"),
        )
        .with_remedy("locdev dns install");
    }

    match dns.routing {
        Routing::Locdev if dns.conflicts.is_empty() => Check::new(
            "resolver",
            LABEL,
            CheckLevel::Ok,
            format!("*.test is routed to LocDev via {}", dns.backend),
        ),
        Routing::Locdev => Check::new(
            "resolver",
            LABEL,
            CheckLevel::Warn,
            format!("*.test reaches LocDev, but {others} also claims it; lookups may be slow or flip between tools"),
        )
        .with_remedy(format!("remove the other tool's resolver config (e.g. sudo rm {})", dns.conflicts[0])),

        Routing::Other => {
            let detail = if others.is_empty() {
                "*.test resolves to this machine through another tool, not LocDev".to_string()
            } else {
                format!("*.test resolves through another tool ({others}), not LocDev")
            };
            let check = Check::new("resolver", LABEL, CheckLevel::Warn, detail);
            if dns.installed == Some(true) {
                check.with_remedy(format!("remove the other tool's config (e.g. sudo rm {})", dns.conflicts.first().map_or("…", String::as_str)))
            } else {
                check.with_remedy("locdev dns install")
            }
        }

        Routing::None => {
            let fallback = match proxy.port {
                Some(80) => "http://<site>.localhost works meanwhile".to_string(),
                Some(port) => format!("http://<site>.localhost:{port} works meanwhile"),
                None => "sites are reachable on 127.0.0.1:<port>".to_string(),
            };
            match dns.installed {
                Some(true) => Check::new(
                    "resolver",
                    LABEL,
                    CheckLevel::Error,
                    format!("LocDev's resolver config is installed but *.test does not resolve ({fallback})"),
                )
                .with_remedy(if cfg!(target_os = "linux") {
                    "sudo systemctl restart systemd-resolved"
                } else {
                    "locdev dns uninstall && locdev dns install"
                }),
                _ if dns.backend == "unsupported" => Check::new(
                    "resolver",
                    LABEL,
                    CheckLevel::Info,
                    format!("this system's resolver can't be configured automatically; {fallback}"),
                ),
                _ => Check::new(
                    "resolver",
                    LABEL,
                    CheckLevel::Info,
                    format!("not set up yet; {fallback}"),
                )
                .with_remedy("locdev dns install"),
            }
        }
    }
}

fn proxy_check(proxy: &ProxyStatus) -> Check {
    const LABEL: &str = "HTTP proxy";

    if !proxy.enabled {
        return Check::new("proxy", LABEL, CheckLevel::Info, "disabled in config.toml")
            .with_remedy("set proxy.enabled = true in config.toml and run `locdev restart`");
    }

    let Some(port) = proxy.port else {
        return Check::new(
            "proxy",
            LABEL,
            CheckLevel::Error,
            proxy
                .error
                .clone()
                .unwrap_or_else(|| "not listening".into()),
        )
        .with_remedy("change proxy.http_port / proxy.fallback_port in config.toml");
    };

    // Port 0 asks the system for any free port: nothing fell back.
    if port == proxy.requested_port || proxy.requested_port == 0 {
        return Check::new(
            "proxy",
            LABEL,
            CheckLevel::Ok,
            format!("listening on 127.0.0.1:{port}"),
        );
    }

    let reason = proxy.error.clone().unwrap_or_default();
    let check = Check::new(
        "proxy",
        LABEL,
        CheckLevel::Warn,
        format!("{reason}; serving on port {port} instead"),
    );

    if reason.contains("privileges") && cfg!(target_os = "linux") {
        let exe = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "locdevd".into());
        check.with_remedy(format!(
            "sudo setcap 'cap_net_bind_service=+ep' {exe} && locdev restart"
        ))
    } else {
        check.with_remedy(format!(
            "free port {} or set proxy.http_port in config.toml",
            proxy.requested_port
        ))
    }
}

struct Tool {
    bin: &'static str,
    label: &'static str,
    needed_by: &'static [ProjectKind],
}

const PHP_KINDS: &[ProjectKind] = &[
    ProjectKind::Laravel,
    ProjectKind::Wordpress,
    ProjectKind::Php,
];
const NODE_KINDS: &[ProjectKind] = &[ProjectKind::Next, ProjectKind::Node];

const TOOLS: &[Tool] = &[
    Tool {
        bin: "php",
        label: "PHP",
        needed_by: PHP_KINDS,
    },
    Tool {
        bin: "composer",
        label: "Composer",
        needed_by: &[ProjectKind::Laravel],
    },
    Tool {
        bin: "node",
        label: "Node.js",
        needed_by: NODE_KINDS,
    },
    Tool {
        bin: "npm",
        label: "npm",
        needed_by: NODE_KINDS,
    },
];

fn tool_check(tool: &Tool, projects: &[Project]) -> Check {
    let id = format!("tool-{}", tool.bin);

    if let Some(path) = which(tool.bin) {
        return Check::new(&id, tool.label, CheckLevel::Ok, path.display().to_string());
    }

    let needed = projects.iter().any(|p| tool.needed_by.contains(&p.kind));
    if needed {
        Check::new(
            &id,
            tool.label,
            CheckLevel::Warn,
            format!(
                "not found on PATH, but registered projects need {}",
                tool.label
            ),
        )
        .with_remedy(format!(
            "install {} (managed runtimes arrive in v0.2)",
            tool.label
        ))
    } else {
        Check::new(&id, tool.label, CheckLevel::Info, "not found on PATH")
    }
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };

    std::env::split_paths(&path).find_map(|dir| {
        extensions
            .iter()
            .map(|ext| dir.join(format!("{bin}{ext}")))
            .find(|candidate| candidate.is_file())
    })
}

/// Warnings for runtimes that don't resolve: one per site whose own pin is
/// missing (it won't start), and one per runtime whose default is missing
/// (sites fall back to the system PATH).
pub fn runtime_checks(
    projects: &[Project],
    resolve: impl Fn(&Project) -> Vec<ResolvedRuntime>,
    tool_conflicts: impl Fn(&[ResolvedRuntime]) -> Vec<String>,
) -> Vec<Check> {
    let mut checks = Vec::new();
    let mut missing_defaults = std::collections::BTreeMap::new();

    for project in projects {
        let resolved = resolve(project);
        for r in &resolved {
            let id = format!("runtime-{}-{}", project.name, r.kind.executable());
            let label = format!("{} for {}", r.kind.label(), project.domain);
            let Some(version) = &r.version else {
                if !r.pinned {
                    missing_defaults.insert(r.kind, r.requested.clone());
                    continue;
                }
                let blocks = r.kind.category() == RuntimeCategory::Runtime;
                checks.push(
                    Check::new(
                        &id,
                        &label,
                        CheckLevel::Warn,
                        format!(
                            "{} asks for {}, which is not installed{}",
                            r.from,
                            r.requested,
                            if blocks {
                                ", so the site won't start"
                            } else {
                                "; the one on PATH is used"
                            }
                        ),
                    )
                    .with_remedy(format!(
                        "locdev runtime install {} {}",
                        r.kind.executable(),
                        r.requested
                    )),
                );
                continue;
            };
            if let Some(unmet) = &r.unmet {
                checks.push(
                    Check::new(
                        &format!("{id}-range"),
                        &label,
                        CheckLevel::Warn,
                        format!(
                            "{} wants {}, but the site runs {} ({})",
                            unmet.file, unmet.constraint, version.version, r.from
                        ),
                    )
                    .with_remedy(format!(
                        "install a version in that range, or change the pin with `locdev edit {} --{} <version>`",
                        project.name,
                        r.kind.executable()
                    )),
                );
            }
            if let Some(support) = &version.support
                && support.state == SupportState::Eol
            {
                checks.push(
                    Check::new(
                        &format!("{id}-eol"),
                        &label,
                        CheckLevel::Warn,
                        format!(
                            "{} {} is end-of-life{}: no more security fixes",
                            r.kind.label(),
                            version.version,
                            support
                                .until
                                .as_deref()
                                .map(|d| format!(" since {d}"))
                                .unwrap_or_default()
                        ),
                    )
                    .with_remedy(format!(
                        "move the site to a supported {} version",
                        r.kind.label()
                    )),
                );
            }
        }
        for (i, conflict) in tool_conflicts(&resolved).into_iter().enumerate() {
            checks.push(Check::new(
                &format!("runtime-{}-tool-{i}", project.name),
                &format!("Package manager for {}", project.domain),
                CheckLevel::Warn,
                conflict,
            ));
        }
    }

    for (kind, requested) in missing_defaults {
        checks.push(
            Check::new(
                &format!("runtime-default-{}", kind.executable()),
                &format!("Default {}", kind.label()),
                CheckLevel::Warn,
                format!(
                    "default {requested} is not installed; sites without a pin use the system PATH"
                ),
            )
            .with_remedy(format!(
                "locdev runtime install {} {requested}",
                kind.executable()
            )),
        );
    }
    checks
}
