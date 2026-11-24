//! Human and JSON rendering for CLI results.

use std::io::IsTerminal;

use locdev_core::actions::{
    ActionVersion, ExistingWorkflow, FindingLevel, TemplateInfo, Validation,
};
use locdev_core::deps::{
    Compatibility, DepKind, DepStatus, DepsReport, Job, JobStatus, Notification,
    NotificationCategory, PackageReport, Settings, Severity, UpdateKind, UpdatePlan,
};
use locdev_core::git::{
    Branch, ChangeKind, Commit, GitStatus, GithubAccount, GithubAuthMethod, RefLogEntry, RepoState,
    SecurityStatus, Stash,
};
use locdev_core::model::{
    Check, CheckLevel, DaemonStatus, DevServerMode, DnsStatus, LogLevel, LogLine, LogStream,
    ProcessRole, ProcessState, ProcessView, ProjectView, Release, Routing, RuntimeCategory,
    RuntimeInfo, RuntimeKind, RuntimeSource, RuntimeVersion, SupportState, TlsStatus, now_ms,
};
use locdev_core::protocol::PROTOCOL_VERSION;
use serde_json::json;

mod config;
mod db;
mod debug;
mod deploy;
mod editor;
mod email;
mod env;
mod git;
mod mail;
mod park;
mod php;
mod process;
mod remote;
mod service;
mod stack;
mod subdomain;
mod tunnel;
mod update;

pub struct Output {
    json: bool,
    color: bool,
}

#[derive(Clone, Copy)]
enum Style {
    Bold = 1,
    Dim = 2,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Cyan = 36,
}

impl Output {
    pub fn new(json: bool) -> Self {
        let color = !json
            && std::io::stdout().is_terminal()
            && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty());
        Self { json, color }
    }

    fn paint(&self, style: Style, text: &str) -> String {
        if self.color {
            format!("\x1b[{}m{text}\x1b[0m", style as u8)
        } else {
            text.to_string()
        }
    }

    pub(crate) fn print_json(&self, value: impl serde::Serialize) {
        println!(
            "{}",
            serde_json::to_string_pretty(&value).unwrap_or_default()
        );
    }

    pub fn error(&self, error: &anyhow::Error) {
        if self.json {
            self.print_json(json!({ "error": format!("{error:#}") }));
        } else {
            eprintln!("{} {error:#}", self.paint(Style::Red, "error:"));
        }
    }

    /// A side remark on stderr, so `--json` output stays parseable.
    pub fn note(&self, text: &str) {
        eprintln!("{}", self.paint(Style::Dim, text));
    }

    pub fn message(&self, text: &str) {
        if self.json {
            self.print_json(json!({ "message": text }));
        } else {
            println!("{text}");
        }
    }

    pub fn version(&self, daemon: Option<&DaemonStatus>) {
        if self.json {
            self.print_json(json!({
                "client": { "version": locdev_core::VERSION, "protocol": PROTOCOL_VERSION },
                "daemon": daemon.map(|d| json!({ "version": d.version, "protocol": d.protocol })),
            }));
            return;
        }

        println!(
            "locdev  {} (protocol {PROTOCOL_VERSION})",
            locdev_core::VERSION
        );
        match daemon {
            Some(d) => println!("locdevd {} (protocol {})", d.version, d.protocol),
            None => println!("locdevd {}", self.paint(Style::Dim, "not running")),
        }
    }

    pub fn status(&self, status: Option<&DaemonStatus>) {
        if self.json {
            self.print_json(json!({ "running": status.is_some(), "daemon": status }));
            return;
        }

        let Some(s) = status else {
            println!("{} daemon is not running", self.paint(Style::Red, "●"));
            println!("  start it with: locdev start");
            return;
        };

        println!(
            "{} daemon running  {}",
            self.paint(Style::Green, "●"),
            self.paint(
                Style::Dim,
                &format!("pid {} · up {}", s.pid, uptime(s.started_at))
            )
        );
        println!("  version   {} (protocol {})", s.version, s.protocol);
        println!(
            "  projects  {} registered, {} running",
            s.projects, s.running
        );
        println!("  home      {}", s.home.display());
        let proxy = &s.proxy;
        match proxy.port {
            Some(port) => {
                let note = proxy
                    .error
                    .as_deref()
                    .map(|e| self.paint(Style::Yellow, &format!("  ({e})")))
                    .unwrap_or_default();
                println!("  proxy     127.0.0.1:{port} · *.{}{note}", proxy.suffix);
            }
            None if !proxy.enabled => println!("  proxy     disabled"),
            None => println!(
                "  proxy     {}",
                self.paint(
                    Style::Red,
                    proxy.error.as_deref().unwrap_or("not listening")
                )
            ),
        }
        match s.dns.port {
            Some(port) => println!(
                "  dns       127.0.0.1:{port} · {}",
                self.routing(s.dns.routing)
            ),
            None => println!(
                "  dns       {}",
                self.paint(Style::Red, s.dns.error.as_deref().unwrap_or("off"))
            ),
        }
        let tls = &s.tls;
        match tls.port {
            Some(port) => println!(
                "  https     127.0.0.1:{port} · CA {}",
                if tls.trusted {
                    self.paint(Style::Green, "trusted")
                } else {
                    self.paint(Style::Yellow, "not trusted (locdev tls trust)")
                }
            ),
            None if !tls.enabled => println!("  https     disabled"),
            None => println!(
                "  https     {}",
                self.paint(Style::Red, tls.error.as_deref().unwrap_or("not listening"))
            ),
        }
    }

    fn routing(&self, routing: Routing) -> String {
        match routing {
            Routing::Locdev => self.paint(Style::Green, "*.test → LocDev"),
            Routing::Other => self.paint(Style::Yellow, "*.test → another tool"),
            Routing::None => self.paint(Style::Dim, "*.test not resolving"),
        }
    }

    pub fn service(&self, status: &locdev_platform::service::ServiceStatus) {
        if self.json {
            self.print_json(status);
            return;
        }
        if status.manager.is_empty() {
            println!("starting at login is not supported on this system");
            return;
        }
        let state = if status.installed {
            self.paint(Style::Green, "starts at login")
        } else {
            self.paint(Style::Dim, "does not start at login")
        };
        println!("locdevd     {state} ({})", status.manager);
        if let Some(location) = &status.location {
            println!("location    {location}");
        }
    }

    pub fn is_json(&self) -> bool {
        self.json
    }

    pub fn terminal_runtimes(&self, t: &locdev_core::model::TerminalRuntimes) {
        if self.json {
            self.print_json(t);
            return;
        }
        let state = if t.enabled {
            self.paint(Style::Green, "on")
        } else {
            self.paint(Style::Dim, "off")
        };
        println!("terminal    {state}");
        println!("folder      {}", t.dir.display());
        println!(
            "commands    {}",
            if t.commands.is_empty() {
                "none (set a default with `locdev runtime default <kind> <version>`)".to_string()
            } else {
                t.commands.join(" ")
            }
        );
        for file in &t.files {
            println!("in          {}", file.display());
        }
        if t.enabled && !t.active {
            self.note("open a new terminal to use them");
        } else if !t.enabled {
            self.note("turn it on with `locdev runtime terminal on`");
        }
    }

    pub fn runtimes(&self, runtimes: &[RuntimeInfo]) {
        if self.json {
            self.print_json(runtimes);
            return;
        }

        for (category, title) in [
            (RuntimeCategory::Runtime, "runtimes"),
            (RuntimeCategory::PackageManager, "package managers"),
        ] {
            let group: Vec<&RuntimeInfo> =
                runtimes.iter().filter(|r| r.category == category).collect();
            if group.is_empty() {
                continue;
            }
            println!("{}", self.paint(Style::Dim, &format!("── {title}")));
            for info in group {
                self.runtime(info);
            }
            println!();
        }
    }

    fn runtime(&self, info: &RuntimeInfo) {
        let default = match (&info.default, info.kind.pinnable()) {
            (Some(d), _) => format!("default {d}"),
            (None, false) => "follows Node.js".into(),
            (None, true) => "no default (system PATH)".into(),
        };
        println!(
            "{}  {}",
            self.paint(Style::Bold, info.kind.label()),
            self.paint(Style::Dim, &default)
        );

        // The version the default resolves to (versions are newest first).
        let current = info.default.as_deref().and_then(|pin| {
            info.versions
                .iter()
                .find(|v| RuntimeVersion::matches(pin, &v.version))
                .map(|v| v.version.as_str())
        });
        if info.versions.is_empty() {
            println!("  {}", self.paint(Style::Dim, "none found"));
        }
        for v in &info.versions {
            let mark = if Some(v.version.as_str()) == current {
                self.paint(Style::Green, "●")
            } else {
                " ".into()
            };
            let source = match v.source {
                RuntimeSource::Managed if v.origin != "locdev" => {
                    self.paint(Style::Cyan, &v.origin)
                }
                RuntimeSource::Managed => self.paint(Style::Cyan, "locdev"),
                RuntimeSource::System => self.paint(Style::Dim, &v.origin),
            };
            let support = match v.support.as_ref().map(|s| s.state) {
                Some(SupportState::Eol) => self.paint(Style::Red, " end-of-life"),
                Some(SupportState::Security) => self.paint(Style::Yellow, " security fixes only"),
                Some(SupportState::Maintenance) => self.paint(Style::Dim, " maintenance"),
                _ => String::new(),
            };
            let used = info
                .usage
                .get(&v.version)
                .map(|sites| self.paint(Style::Dim, &format!(" · used by {}", sites.join(", "))))
                .unwrap_or_default();
            println!(
                "  {mark} {:<10} {:<16} {}{support}{used}",
                v.version,
                source,
                self.paint(Style::Dim, &v.executable.display().to_string())
            );
        }
        for update in &info.updates {
            println!(
                "  {} {} {} → {}   locdev runtime update {} {}",
                self.paint(Style::Yellow, "↑"),
                update.line,
                update.installed,
                self.paint(Style::Bold, &update.latest),
                info.kind.executable(),
                update.line
            );
        }
        if !info.downloads {
            let note = match info.kind {
                RuntimeKind::Npm => "comes with each Node.js version",
                _ => "LocDev doesn't download this one; installed versions are detected",
            };
            println!("  {}", self.paint(Style::Dim, note));
        }
    }

    pub fn catalog(&self, kind: RuntimeKind, releases: &[Release]) {
        if self.json {
            self.print_json(releases);
            return;
        }
        for release in releases {
            let lts = release
                .lts
                .as_deref()
                .map(|name| self.paint(Style::Green, &format!("LTS {name}")))
                .unwrap_or_default();
            println!(
                "  {:<10} {}  {lts}",
                release.version,
                self.paint(Style::Dim, &release.date)
            );
        }
        println!(
            "\ninstall with: locdev runtime install {} <version>   (\"lts\" and partial versions work)",
            kind.executable()
        );
    }

    pub fn tls(&self, tls: &TlsStatus) {
        if self.json {
            self.print_json(tls);
            return;
        }

        match (tls.enabled, tls.port) {
            (false, _) => println!("https       disabled"),
            (_, Some(port)) => println!("https       127.0.0.1:{port}"),
            (_, None) => println!(
                "https       {}",
                self.paint(Style::Red, tls.error.as_deref().unwrap_or("not listening"))
            ),
        }
        if let Some(path) = &tls.ca_path {
            println!("ca          {}", path.display());
        }
        if let Some(fingerprint) = &tls.fingerprint {
            println!("sha-256     {fingerprint}");
        }
        let system = if tls.trusted {
            self.paint(Style::Green, "trusted")
        } else {
            self.paint(Style::Yellow, "not trusted")
        };
        println!("system      {system}");
        match tls.browsers {
            Some(true) => println!("browsers    {}", self.paint(Style::Green, "trusted")),
            Some(false) => println!("browsers    {}", self.paint(Style::Yellow, "not trusted")),
            None => {}
        }
        if !tls.trusted || tls.browsers == Some(false) {
            println!("\ntrust it with: locdev tls trust");
        }
    }

    pub fn dns(&self, dns: &DnsStatus) {
        if self.json {
            self.print_json(dns);
            return;
        }

        match (dns.enabled, dns.port) {
            (false, _) => println!("dns server  disabled"),
            (_, Some(port)) => println!("dns server  127.0.0.1:{port}"),
            (_, None) => println!(
                "dns server  {}",
                self.paint(Style::Red, dns.error.as_deref().unwrap_or("not listening"))
            ),
        }
        let installed = match dns.installed {
            Some(true) => "installed",
            Some(false) => "not installed",
            None => "unknown",
        };
        println!("resolver    {} ({installed})", dns.backend);
        println!("routing     {}", self.routing(dns.routing));
        for conflict in &dns.conflicts {
            println!(
                "conflict    {}",
                self.paint(Style::Yellow, &format!("{conflict} also claims *.test"))
            );
        }
        if dns.routing != Routing::Locdev && dns.installed != Some(true) {
            println!("\nset it up with: locdev dns install");
        }
    }

    /// Shows a privileged change before it is applied (on stderr, so `--json`
    /// output stays clean).
    pub fn plan(&self, plan: &locdev_platform::Plan) {
        eprintln!("{}", self.paint(Style::Bold, &plan.summary));
        eprintln!();
        eprintln!("{}", self.paint(Style::Dim, "as administrator:"));
        for line in plan.script.source().lines() {
            eprintln!("  {}", self.paint(Style::Cyan, line));
        }
        if let Some(user) = &plan.user_script {
            eprintln!("{}", self.paint(Style::Dim, "then as you:"));
            for line in user.source().lines() {
                eprintln!("  {}", self.paint(Style::Cyan, line));
            }
        }
        eprintln!();
    }

    pub fn checks(&self, checks: &[Check]) {
        if self.json {
            self.print_json(checks);
            return;
        }

        let width = checks.iter().map(|c| c.label.len()).max().unwrap_or(0);
        for check in checks {
            let mark = match check.level {
                CheckLevel::Ok => self.paint(Style::Green, "✓"),
                CheckLevel::Info => self.paint(Style::Blue, "i"),
                CheckLevel::Warn => self.paint(Style::Yellow, "!"),
                CheckLevel::Error => self.paint(Style::Red, "✗"),
            };
            println!("{mark} {:width$}  {}", check.label, check.detail);
            if let Some(remedy) = &check.remedy {
                println!(
                    "  {:width$}  {} {}",
                    "",
                    self.paint(Style::Dim, "→"),
                    self.paint(Style::Cyan, remedy)
                );
            }
        }
    }

    pub fn projects(&self, projects: &[ProjectView]) {
        if self.json {
            self.print_json(projects);
            return;
        }

        if projects.is_empty() {
            println!("no projects yet — register one with: locdev add <path>");
            return;
        }

        let name_w = projects
            .iter()
            .map(|p| p.project.domain.len())
            .max()
            .unwrap_or(0);
        // The stack's name ("Laravel + Vite", "Django"), else the kind.
        let label = |v: &ProjectView| {
            if v.stack.framework.is_empty() {
                v.project.kind.label().to_string()
            } else {
                v.stack.framework.clone()
            }
        };
        let kind_w = projects.iter().map(|p| label(p).len()).max().unwrap_or(0);

        for view in projects {
            let p = &view.project;
            println!(
                "{} {:name_w$}  {:kind_w$}  {:>5}  {}",
                self.state_dot(view.process.state),
                p.domain,
                label(view),
                p.port.map(|p| p.to_string()).unwrap_or_else(|| "-".into()),
                self.paint(Style::Dim, &p.path.display().to_string()),
            );
        }
    }

    pub fn project(&self, view: &ProjectView) {
        if self.json {
            self.print_json(view);
            return;
        }

        let p = &view.project;
        println!(
            "{} {}  {}",
            self.state_dot(view.process.state),
            self.paint(Style::Bold, &p.domain),
            self.paint(Style::Dim, state_label(view.process.state)),
        );
        println!(
            "  stack    {}",
            if view.stack.framework.is_empty() {
                p.kind.label()
            } else {
                &view.stack.framework
            }
        );
        println!("  path     {}", p.path.display());
        println!("  command  {}", p.command.as_deref().unwrap_or("-"));
        if let Some(url) = &view.url {
            println!("  url      {}", self.paint(Style::Cyan, url));
        }
        if let Some(url) = view
            .https_url
            .as_ref()
            .filter(|u| Some(*u) != view.url.as_ref())
        {
            println!("  https    {}", self.paint(Style::Dim, url));
        }
        for runtime in &view.runtimes {
            let line = match &runtime.version {
                Some(v) => format!("{} ({} {})", v.version, runtime.from, runtime.requested),
                None => self.paint(
                    Style::Red,
                    &format!("{} {} not installed", runtime.from, runtime.requested),
                ),
            };
            println!("  {:<8} {line}", runtime.kind.executable());
            if let Some(unmet) = &runtime.unmet {
                println!(
                    "  {:<8} {}",
                    "",
                    self.paint(
                        Style::Yellow,
                        &format!("{} wants {}", unmet.file, unmet.constraint)
                    )
                );
            }
        }
        if let Some(url) = &view.direct_url {
            println!("  direct   {}", self.paint(Style::Dim, url));
        }
        if p.auto_start {
            println!("  starts with the daemon");
        }
        if let Some(message) = &view.process.message {
            println!("  {}", self.paint(Style::Red, message));
        }
    }

    /// `locdev info`: everything about one site.
    pub fn project_info(&self, view: &ProjectView) {
        if self.json {
            self.print_json(view);
            return;
        }
        self.project(view);
        let stack = &view.stack;
        println!("  stack    {}", stack.framework);
        if let Some(pm) = stack.package_manager {
            println!("  packages {}", pm.label());
        }
        if view.project.secure {
            println!(
                "  https    {}",
                self.paint(Style::Green, "HTTPS only (HTTP redirects)")
            );
        }
        if view.project.restart_on_failure {
            println!("  restarts on failure");
        }
        for r in &view.requirements {
            println!(
                "  wants    {} {} ({})",
                r.kind.executable(),
                r.constraint,
                r.file
            );
        }
        if view.project.dev_server.is_some() || stack.dev_server.is_some() {
            println!();
            self.dev_server(view);
        }
    }

    /// The helper dev server: what it runs, the user's decision, its state.
    pub fn dev_server(&self, view: &ProjectView) {
        if self.json {
            self.print_json(json!({
                "dev_server": view.project.dev_server,
                "detected": view.stack.dev_server,
                "process": view.dev_process,
                "url": view.dev_url,
            }));
            return;
        }
        let Some(dev) = view
            .project
            .dev_server
            .as_ref()
            .or(view.stack.dev_server.as_ref())
        else {
            println!(
                "{} has no helper dev server (none detected)",
                view.project.domain
            );
            return;
        };
        let mode = match dev.mode {
            DevServerMode::Ask => self.paint(
                Style::Yellow,
                "waiting for your decision (locdev dev mode …)",
            ),
            DevServerMode::Manual => "started by hand".into(),
            DevServerMode::Auto => self.paint(Style::Green, "starts with the site"),
            DevServerMode::Never => self.paint(Style::Dim, "not managed"),
        };
        let state = view
            .dev_process
            .as_ref()
            .map(|p| p.state)
            .unwrap_or(ProcessState::Stopped);
        println!(
            "{} {}  {}",
            self.state_dot(state),
            self.paint(Style::Bold, dev.tool.label()),
            self.paint(Style::Dim, state_label(state))
        );
        println!("  command  {}", dev.command);
        println!("  mode     {mode}");
        if let Some(url) = &view.dev_url {
            println!("  direct   {}", self.paint(Style::Dim, url));
        }
        if let Some(message) = view.dev_process.as_ref().and_then(|p| p.message.as_deref()) {
            println!("  {}", self.paint(Style::Red, message));
        }
    }

    /// `locdev deps list|outdated`.
    pub fn deps(&self, report: &DepsReport, only_outdated: bool) {
        if self.json {
            self.print_json(report);
            return;
        }
        let manager = report.manager.map(|m| m.label()).unwrap_or("none");
        let checked = report
            .checked_at
            .map(|at| format!("checked {} ago", uptime(at)))
            .unwrap_or_else(|| "not checked yet (locdev deps check)".into());
        println!(
            "{}",
            self.paint(
                Style::Dim,
                &format!("{} · {manager} · {checked}", report.files.join(", "))
            )
        );
        for error in &report.errors {
            println!("{}", self.paint(Style::Yellow, &format!("! {error}")));
        }
        let shown: Vec<&PackageReport> = report
            .packages
            .iter()
            .filter(|p| {
                !only_outdated || matches!(p.status, DepStatus::Outdated | DepStatus::Vulnerable)
            })
            .collect();
        if shown.is_empty() {
            println!(
                "{}",
                if only_outdated {
                    "everything is up to date"
                } else {
                    "no dependencies declared"
                }
            );
            return;
        }
        let width = shown
            .iter()
            .map(|p| p.name.len())
            .max()
            .unwrap_or(0)
            .min(40);
        println!(
            "{}",
            self.paint(
                Style::Dim,
                &format!(
                    "  {:<width$}  {:<10} {:<10} {:<10} {:<10} {}",
                    "PACKAGE", "DECLARED", "INSTALLED", "WANTED", "LATEST", "STATUS"
                )
            )
        );
        for p in shown {
            let status = match p.status {
                DepStatus::UpToDate => self.paint(Style::Green, "up to date"),
                DepStatus::Outdated => self.paint(
                    if p.update == UpdateKind::Major {
                        Style::Yellow
                    } else {
                        Style::Cyan
                    },
                    &format!(
                        "{} · {:?}",
                        crate::work::update_label(p.update),
                        p.compatibility
                    )
                    .to_lowercase(),
                ),
                DepStatus::Vulnerable => {
                    self.paint(Style::Red, &format!("{} advisories", p.advisories.len()))
                }
                DepStatus::Unknown => self.paint(Style::Dim, "unknown"),
            };
            let kind = match p.kind {
                DepKind::Dev => self.paint(Style::Dim, " dev"),
                DepKind::Peer => self.paint(Style::Dim, " peer"),
                DepKind::Optional => self.paint(Style::Dim, " optional"),
                DepKind::Prod => String::new(),
            };
            println!(
                "  {:<width$}  {:<10} {:<10} {:<10} {:<10} {status}{kind}",
                truncate(&p.name, width),
                truncate(&p.declared, 10),
                p.installed
                    .as_deref()
                    .or(p.locked.as_deref())
                    .unwrap_or("-"),
                p.wanted.as_deref().unwrap_or("-"),
                p.latest.as_deref().unwrap_or("-"),
            );
        }
    }

    /// `locdev deps audit`.
    pub fn advisories(&self, report: &DepsReport) {
        if self.json {
            self.print_json(
                report
                    .packages
                    .iter()
                    .filter(|p| !p.advisories.is_empty())
                    .collect::<Vec<_>>(),
            );
            return;
        }
        let vulnerable: Vec<_> = report
            .packages
            .iter()
            .filter(|p| !p.advisories.is_empty())
            .collect();
        if vulnerable.is_empty() {
            println!(
                "{} no known vulnerabilities in the versions in use",
                self.paint(Style::Green, "✓")
            );
            return;
        }
        for p in vulnerable {
            println!(
                "{} {} {}",
                self.paint(Style::Red, "✗"),
                self.paint(Style::Bold, &p.name),
                p.installed.as_deref().or(p.locked.as_deref()).unwrap_or("")
            );
            for a in &p.advisories {
                println!(
                    "    {} {}{}",
                    self.paint(Style::Yellow, &a.id),
                    a.summary,
                    a.fixed
                        .as_deref()
                        .map(|f| format!(" (fixed in {f})"))
                        .unwrap_or_default()
                );
                println!("    {}", self.paint(Style::Dim, &a.url));
            }
        }
    }

    /// What `locdev deps update` will do, before asking.
    pub fn update_plan(&self, plan: &UpdatePlan) {
        if self.json {
            self.print_json(plan);
            return;
        }
        for change in &plan.changes {
            let verdict = match change.compatibility {
                Compatibility::Compatible => self.paint(Style::Green, "no conflicts found"),
                Compatibility::Possibly => self.paint(Style::Yellow, "possibly compatible"),
                Compatibility::Incompatible => self.paint(Style::Red, "incompatible"),
                Compatibility::Unknown => self.paint(Style::Dim, "unknown"),
            };
            println!(
                "{} {} → {}  {}  {verdict}",
                self.paint(Style::Bold, &change.name),
                change.from.as_deref().unwrap_or("?"),
                change.to,
                crate::work::update_label(change.update)
            );
            if let Some(constraint) = &change.constraint {
                println!("    constraint becomes {constraint}");
            }
            for reason in &change.reasons {
                println!("    {}", self.paint(Style::Dim, reason));
            }
            if let Some(url) = change
                .links
                .releases
                .as_ref()
                .or(change.links.repository.as_ref())
            {
                println!("    {}", self.paint(Style::Cyan, url));
            }
        }
        for warning in &plan.warnings {
            println!("{}", self.paint(Style::Yellow, &format!("! {warning}")));
        }
        if plan.commands.is_empty() {
            return;
        }
        println!();
        println!(
            "{}",
            self.paint(Style::Dim, "will run, in the site's folder:")
        );
        for command in &plan.commands {
            println!("  {} {}", self.paint(Style::Cyan, "$"), command.join(" "));
        }
        println!(
            "{}",
            self.paint(Style::Dim, &format!("files: {}", plan.files.join(", ")))
        );
        if !plan.run_scripts {
            println!(
                "{}",
                self.paint(
                    Style::Dim,
                    "install scripts are skipped (--scripts runs them)"
                )
            );
        }
    }

    pub fn job(&self, job: &Job) {
        if self.json {
            self.print_json(job);
            return;
        }
        let status = match job.status {
            JobStatus::Completed => self.paint(Style::Green, "completed"),
            JobStatus::Failed => self.paint(Style::Red, "failed"),
            JobStatus::Cancelled => self.paint(Style::Yellow, "cancelled"),
            JobStatus::Running => self.paint(Style::Cyan, "running"),
            JobStatus::Pending => "pending".into(),
            JobStatus::Paused => self.paint(Style::Dim, "paused"),
        };
        println!(
            "#{} {} {}{}  {status}",
            job.id,
            job.kind.label(),
            job.project.as_deref().unwrap_or(""),
            if job.automatic {
                self.paint(Style::Dim, " (automatic)")
            } else {
                String::new()
            }
        );
        if let Some(result) = &job.result {
            println!("  {result}");
        }
        if let Some(error) = &job.error {
            println!("  {}", self.paint(Style::Red, error));
        }
        for line in &job.log {
            println!("  {}", self.paint(Style::Dim, line));
        }
    }

    pub fn jobs(&self, jobs: &[Job]) {
        if self.json {
            self.print_json(jobs);
            return;
        }
        if jobs.is_empty() {
            println!("no jobs yet");
            return;
        }
        for job in jobs.iter().take(30) {
            let when = job.finished_at.or(job.started_at).unwrap_or(job.created_at);
            println!(
                "#{:<5} {:<18} {:<14} {:<10} {}",
                job.id,
                job.kind.label(),
                truncate(job.project.as_deref().unwrap_or("-"), 14),
                format!("{:?}", job.status).to_lowercase(),
                self.paint(Style::Dim, &format!("{} ago", uptime(when)))
            );
        }
    }

    pub fn notifications(&self, notifications: &[Notification], unread: usize) {
        if self.json {
            self.print_json(json!({ "unread": unread, "notifications": notifications }));
            return;
        }
        if notifications.is_empty() {
            println!("no notifications");
            return;
        }
        for n in notifications {
            let mark = match (n.read, n.severity) {
                (true, _) => self.paint(Style::Dim, "○"),
                (false, Severity::Error) => self.paint(Style::Red, "●"),
                (false, Severity::Warn) => self.paint(Style::Yellow, "●"),
                (false, Severity::Info) => self.paint(Style::Cyan, "●"),
            };
            println!(
                "{mark} #{} {}  {}",
                n.id,
                n.title,
                self.paint(Style::Dim, &format!("{} ago", uptime(n.created_at)))
            );
            for line in n.body.lines() {
                println!("    {}", self.paint(Style::Dim, line));
            }
        }
    }

    pub fn settings(&self, settings: &Settings) {
        if self.json {
            self.print_json(settings);
            return;
        }
        let a = &settings.automation;
        println!(
            "automation.audit                 {}",
            serde_json::to_value(a.audit)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        );
        println!("automation.concurrency           {}", a.concurrency);
        println!("automation.requests_per_second   {}", a.requests_per_second);
        println!("automation.definitions           {}", a.definitions);
        println!("automation.updates               {}", a.updates);
        let o = &settings.observe;
        println!("observe.enabled                  {}", o.enabled);
        println!("observe.retention_days           {}", o.retention_days);
        println!("observe.max_mb                   {}", o.max_mb);
        println!(
            "observe.logs_from                {}",
            serde_json::to_value(o.logs_from)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        );
        println!(
            "network.offline                  {}",
            settings.network.offline
        );
        println!(
            "env.conflicts                    {}",
            serde_json::to_value(settings.env.conflicts)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        );
        println!(
            "notifications.native             {}",
            settings.notifications.native
        );
        for category in NotificationCategory::ALL {
            let key = serde_json::to_value(category)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            println!(
                "notifications.{key:<19}{}",
                settings.notifications.enabled(category)
            );
        }
    }

    pub fn git_status(&self, status: &GitStatus) {
        if self.json {
            self.print_json(status);
            return;
        }
        let branch = status.branch.as_deref().unwrap_or("(detached HEAD)");
        let mut line = self.paint(Style::Bold, branch);
        if let Some(upstream) = &status.upstream {
            line.push_str(&format!(" → {upstream}"));
            if status.ahead > 0 || status.behind > 0 {
                line.push_str(&format!(" (↑{} ↓{})", status.ahead, status.behind));
            }
        }
        if status.state != RepoState::Clean {
            let state = format!("{:?}", status.state).to_lowercase();
            line.push_str(&format!("  {}", self.paint(Style::Yellow, &state)));
        }
        println!("{line}");
        if let Some(repo) = &status.github {
            println!("{}", self.paint(Style::Dim, &repo.url()));
        }
        if status.changes.is_empty() {
            println!("nothing to commit, working tree clean");
            return;
        }
        for change in &status.changes {
            let code = |kind: Option<ChangeKind>| match kind {
                Some(ChangeKind::Added) => 'A',
                Some(ChangeKind::Modified) => 'M',
                Some(ChangeKind::Deleted) => 'D',
                Some(ChangeKind::Renamed) => 'R',
                Some(ChangeKind::Copied) => 'C',
                Some(ChangeKind::TypeChanged) => 'T',
                None => ' ',
            };
            let mark = if change.conflicted {
                self.paint(Style::Red, "UU")
            } else if change.untracked {
                self.paint(Style::Dim, "??")
            } else {
                format!(
                    "{}{}",
                    self.paint(Style::Green, &code(change.staged).to_string()),
                    self.paint(Style::Red, &code(change.unstaged).to_string())
                )
            };
            let path = match &change.orig_path {
                Some(from) => format!("{from} → {}", change.path),
                None => change.path.clone(),
            };
            let lines = match (change.added, change.removed) {
                (Some(a), Some(r)) => self.paint(Style::Dim, &format!("  +{a} -{r}")),
                _ => String::new(),
            };
            println!("{mark} {path}{lines}");
        }
    }

    pub fn commits(&self, commits: &[Commit]) {
        if self.json {
            self.print_json(commits);
            return;
        }
        for c in commits {
            let refs = if c.refs.is_empty() {
                String::new()
            } else {
                format!(" ({})", c.refs.join(", "))
            };
            println!(
                "{} {}{}  {}",
                self.paint(Style::Yellow, &c.short),
                truncate(&c.subject, 72),
                self.paint(Style::Cyan, &refs),
                self.paint(
                    Style::Dim,
                    &format!("{} · {} ago", c.author, ago(c.time * 1000))
                )
            );
        }
    }

    pub fn branches(&self, branches: &[Branch]) {
        if self.json {
            self.print_json(branches);
            return;
        }
        for b in branches {
            let mark = if b.current {
                self.paint(Style::Green, "*")
            } else {
                " ".into()
            };
            let mut track = String::new();
            if let Some(upstream) = &b.upstream {
                track = format!(" → {upstream}");
                if b.ahead > 0 || b.behind > 0 {
                    track.push_str(&format!(" ↑{} ↓{}", b.ahead, b.behind));
                }
            }
            let name = if b.remote {
                self.paint(Style::Red, &b.name)
            } else {
                b.name.clone()
            };
            println!(
                "{mark} {name}{}  {} {}",
                self.paint(Style::Dim, &track),
                self.paint(Style::Yellow, &b.sha),
                truncate(&b.subject, 60)
            );
        }
    }

    pub fn stashes(&self, stashes: &[Stash]) {
        if self.json {
            self.print_json(stashes);
            return;
        }
        if stashes.is_empty() {
            println!("no stashes");
        }
        for s in stashes {
            println!(
                "stash@{{{}}} {}  {}",
                s.index,
                s.message,
                self.paint(Style::Dim, &format!("{} ago", ago(s.time * 1000)))
            );
        }
    }

    pub fn git_history(&self, reflog: &[RefLogEntry], operations: &[String]) {
        if self.json {
            self.print_json(json!({ "reflog": reflog, "operations": operations }));
            return;
        }
        println!("{}", self.paint(Style::Bold, "Run through LocDev"));
        if operations.is_empty() {
            println!("  {}", self.paint(Style::Dim, "nothing yet"));
        }
        for line in operations.iter().take(30) {
            println!("  {line}");
        }
        println!("{}", self.paint(Style::Bold, "HEAD history (reflog)"));
        for entry in reflog.iter().take(30) {
            println!(
                "  {} {}  {}",
                self.paint(Style::Yellow, &entry.sha[..entry.sha.len().min(7)]),
                entry.message,
                self.paint(Style::Dim, &format!("{} ago", ago(entry.time * 1000)))
            );
        }
    }

    pub fn github_account(&self, account: Option<&GithubAccount>) {
        if self.json {
            self.print_json(json!({ "account": account }));
            return;
        }
        let Some(account) = account else {
            println!("not connected to GitHub");
            return;
        };
        let name = account
            .name
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        println!(
            "connected as {}{name}",
            self.paint(Style::Bold, &account.login)
        );
        let method = match account.method {
            GithubAuthMethod::Device => "browser sign-in",
            GithubAuthMethod::Token => "personal access token",
            GithubAuthMethod::GhCli => "copied from the GitHub CLI",
        };
        println!("  method   {method}");
        println!("  token    in the {}", account.storage);
        if !account.scopes.is_empty() {
            println!("  scopes   {}", account.scopes.join(", "));
        }
    }

    pub fn security(&self, status: &SecurityStatus) {
        if self.json {
            self.print_json(status);
            return;
        }
        let on_off = |on: bool| if on { "on" } else { "off" };
        println!(
            "password           {}",
            if status.password_set {
                "set"
            } else {
                "not set"
            }
        );
        println!("require password   {}", on_off(status.require_password));
        println!("backups            {}", on_off(status.backups));
        if let Some(until) = status.locked_until {
            let wait = until.saturating_sub(now_ms()).div_ceil(1000);
            println!(
                "{}",
                self.paint(
                    Style::Red,
                    &format!("locked for {wait}s after wrong passwords")
                )
            );
        }
        println!(
            "{}",
            self.paint(
                Style::Dim,
                "Guards destructive operations run through LocDev; git in a terminal is not affected."
            )
        );
    }

    pub fn actions_overview(
        &self,
        templates: &[TemplateInfo],
        existing: &[ExistingWorkflow],
        actionlint: Option<&str>,
    ) {
        if self.json {
            self.print_json(
                json!({ "templates": templates, "existing": existing, "actionlint": actionlint }),
            );
            return;
        }
        println!("{}", self.paint(Style::Bold, "Templates"));
        for t in templates {
            let mark = if t.recommended {
                self.paint(Style::Green, "●")
            } else {
                self.paint(Style::Dim, "○")
            };
            println!(
                "{mark} {:<18} {}  {}",
                t.id,
                t.title,
                self.paint(Style::Dim, &t.reason)
            );
        }
        println!(
            "{}",
            self.paint(Style::Bold, "Workflow files (.github/workflows/)")
        );
        if existing.is_empty() {
            println!("  {}", self.paint(Style::Dim, "none"));
        }
        for w in existing {
            println!(
                "  {:<24} {}",
                w.file,
                self.paint(Style::Dim, w.name.as_deref().unwrap_or(""))
            );
        }
        match actionlint {
            Some(version) => self.note(&format!("actionlint {version} found: its checks run too")),
            None => self.note("actionlint not found on PATH: only LocDev's checks run"),
        }
    }

    pub fn action_versions(&self, actions: &[ActionVersion], checked_at: Option<u64>) {
        if self.json {
            self.print_json(json!({ "actions": actions, "checked_at": checked_at }));
            return;
        }
        for a in actions {
            println!(
                "{:<36} {:<5} {}",
                a.repo,
                a.version,
                self.paint(Style::Dim, &a.purpose)
            );
        }
        match checked_at {
            Some(at) => self.note(&format!("checked against GitHub {} ago", ago(at))),
            None => self.note("built-in list; `locdev actions versions --refresh` asks GitHub"),
        }
    }

    /// Findings for one file; nothing but a check mark when clean.
    pub fn validation(&self, file: &str, validation: &Validation) {
        if self.json {
            self.print_json(json!({ "file": file, "validation": validation }));
            return;
        }
        if validation.findings.is_empty() {
            println!("{} {file}", self.paint(Style::Green, "✓"));
            return;
        }
        println!("{}", self.paint(Style::Bold, file));
        for f in &validation.findings {
            let level = match f.level {
                FindingLevel::Error => self.paint(Style::Red, "error"),
                FindingLevel::Warning => self.paint(Style::Yellow, "warn "),
                FindingLevel::Info => self.paint(Style::Cyan, "info "),
            };
            let at = f.line.map(|l| format!(":{l}")).unwrap_or_default();
            let rule = self.paint(
                Style::Dim,
                &format!(
                    "[{}{}]",
                    f.rule,
                    if f.source == "actionlint" {
                        ", actionlint"
                    } else {
                        ""
                    }
                ),
            );
            println!("  {level} {file}{at} {} {rule}", f.message);
            if let Some(fix) = &f.fix {
                for line in fix.lines() {
                    println!("        {}", self.paint(Style::Green, line));
                }
            }
        }
    }

    pub fn workflow(&self, yaml: &str, file: &str, validation: &Validation) {
        if self.json {
            self.print_json(json!({ "file": file, "yaml": yaml, "validation": validation }));
            return;
        }
        print!("{yaml}");
        eprintln!();
        if validation.findings.is_empty() {
            self.note(&format!("{file}: no problems found"));
        } else {
            self.validation(file, validation);
        }
    }

    /// `locdev ps`.
    pub fn processes(&self, processes: &[ProcessView]) {
        if self.json {
            self.print_json(processes);
            return;
        }
        if processes.is_empty() {
            println!("no sites yet");
            return;
        }
        println!(
            "{}",
            self.paint(
                Style::Dim,
                &format!(
                    "  {:<22} {:<10} {:>7} {:>6} {:>6} {:>8}  {}",
                    "SITE", "PROCESS", "PID", "PORT", "CPU", "MEMORY", "UPTIME"
                )
            )
        );
        for p in processes {
            let site = match (&p.aux, p.role) {
                (Some(aux), _) => format!("{} ({aux})", p.project),
                (None, ProcessRole::Main) => p.project.clone(),
                (None, ProcessRole::Dev) => format!("{} (helper)", p.project),
            };
            let running = p.info.state == ProcessState::Running;
            println!(
                "{} {:<22} {:<10} {:>7} {:>6} {:>6} {:>8}  {}",
                self.state_dot(p.info.state),
                truncate(&site, 22),
                truncate(&p.label, 10),
                p.info
                    .pid
                    .filter(|_| running)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".into()),
                p.port.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
                p.cpu
                    .map(|v| format!("{v:.1}%"))
                    .unwrap_or_else(|| "-".into()),
                p.memory
                    .map(|v| format!("{:.0} MB", v as f64 / 1e6))
                    .unwrap_or_else(|| "-".into()),
                match (running, p.info.started_at) {
                    (true, Some(at)) => uptime(at),
                    _ => self.paint(Style::Dim, state_label(p.info.state)),
                }
            );
        }
    }

    pub fn project_added(&self, view: &ProjectView) {
        if !self.json {
            println!("registered {}", self.paint(Style::Bold, &view.project.name));
        }
        self.project(view);
        if !self.json
            && let Some(dev) = &view.project.dev_server
        {
            println!(
                "  {}",
                self.paint(
                    Style::Yellow,
                    &format!(
                        "{} detected: LocDev won't run it until you decide (locdev dev mode auto|manual|never)",
                        dev.tool.label()
                    )
                )
            );
        }
        if !self.json && view.project.command.is_none() {
            println!(
                "  {}",
                self.paint(Style::Yellow, "no dev server command detected; set one with `locdev edit <name> --command ...`")
            );
        }
    }

    pub fn logs(&self, lines: &[LogLine]) {
        if self.json {
            for line in lines {
                println!("{}", serde_json::to_string(line).unwrap_or_default());
            }
            return;
        }

        for line in lines {
            match (line.stream, line.level) {
                (LogStream::System, _) => {
                    println!("{}", self.paint(Style::Dim, &format!("── {}", line.text)))
                }
                (_, Some(LogLevel::Error)) => println!("{}", self.paint(Style::Red, &line.text)),
                (_, Some(LogLevel::Warn)) => println!("{}", self.paint(Style::Yellow, &line.text)),
                (LogStream::Stdout, _) => println!("{}", line.text),
                (LogStream::Stderr, _) => println!("{}", self.paint(Style::Yellow, &line.text)),
            }
        }
    }

    fn state_dot(&self, state: ProcessState) -> String {
        match state {
            ProcessState::Running => self.paint(Style::Green, "●"),
            ProcessState::Stopping => self.paint(Style::Yellow, "●"),
            ProcessState::Failed => self.paint(Style::Red, "●"),
            ProcessState::Stopped | ProcessState::Exited => self.paint(Style::Dim, "○"),
        }
    }
}

fn state_label(state: ProcessState) -> &'static str {
    match state {
        ProcessState::Stopped => "stopped",
        ProcessState::Running => "running",
        ProcessState::Stopping => "stopping",
        ProcessState::Exited => "exited",
        ProcessState::Failed => "failed",
    }
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(width - 1).collect::<String>())
    }
}

/// Coarse "how long ago" for timestamps that can be days old.
fn ago(at_ms: u64) -> String {
    let secs = now_ms().saturating_sub(at_ms) / 1000;
    match secs {
        s if s < 3600 => uptime(at_ms),
        s if s < 86_400 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86_400),
    }
}

fn uptime(started_at: u64) -> String {
    let secs = now_ms().saturating_sub(started_at) / 1000;
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m {}s", s / 60, s % 60),
        s => format!("{}h {}m", s / 3600, (s % 3600) / 60),
    }
}
