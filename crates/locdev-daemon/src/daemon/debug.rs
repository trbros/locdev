//! The Debug Center pipeline: events come from the processes' output (the
//! supervisor's tap), Laravel's log files, LocDev's dump server (Symfony
//! VarDumper's `server` format), the proxy (5xx answers) and the job queue.
//! Each one is tied to its site, has secrets hidden, gets a fingerprint and
//! a likely cause, and is stored with limits (`events.rs`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use locdev_core::model::{
    LogLevel, LogLine, ProcessRole, ProcessState, Project, ProjectKind, now_ms,
};
use locdev_core::observe::{
    Event, EventKind, EventSeverity, EventSource, ObserveStatus, RequestInfo,
};
use locdev_core::protocol::{Request, Response};
use locdev_core::{AppError, ErrorCode};
use locdev_observe::laravel::LaravelLog;
use locdev_observe::parse::{Assembler, Line, blank};
use locdev_observe::redact::{Redactor, is_secret_key};
use locdev_observe::{dump, finish};
use locdev_proxy::{Failures, HttpFailure};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::Daemon;
use crate::events::EventStore;

/// The prepend shim every PHP site runs (see `ensure_php_shim`).
const PHP_SHIM: &str = r#"<?php
// Managed by LocDev. Runs before every request of the PHP sites LocDev runs.
//
// HTTPS: LocDev's proxy ends TLS and reaches PHP's built-in server over
// plain HTTP, naming the original scheme and port in X-Forwarded-Proto and
// X-Forwarded-Port. Frameworks read those headers only from proxies they
// were told to trust (Laravel's TrustProxies, Symfony's trusted_proxies),
// so a page opened over https:// got http:// links to its own assets and was
// blocked as mixed content. A request from the proxy (loopback, where it
// always sets those headers) gets the variables a TLS server would set, in
// every project, without touching its code.
//
// Dumps: copies VAR_DUMPER_FORMAT and VAR_DUMPER_SERVER from the environment
// into $_SERVER (PHP's built-in server leaves them out), so Symfony VarDumper
// sends dump()/dd() to LocDev's Debug Center when the site has dumps on.
//
// Then runs the auto_prepend_file that was set before this one, if any.
if (isset($_SERVER['HTTP_X_FORWARDED_PROTO'])
    && in_array($_SERVER['REMOTE_ADDR'] ?? '', ['127.0.0.1', '::1'], true)
) {
    if ('https' === strtolower($_SERVER['HTTP_X_FORWARDED_PROTO'])) {
        $_SERVER['HTTPS'] = 'on';
        $_SERVER['REQUEST_SCHEME'] = 'https';
    } else {
        unset($_SERVER['HTTPS']);
        $_SERVER['REQUEST_SCHEME'] = 'http';
    }
    if (isset($_SERVER['HTTP_X_FORWARDED_PORT']) && ctype_digit($_SERVER['HTTP_X_FORWARDED_PORT'])) {
        $_SERVER['SERVER_PORT'] = $_SERVER['HTTP_X_FORWARDED_PORT'];
    }
}
foreach (['VAR_DUMPER_FORMAT', 'VAR_DUMPER_SERVER'] as $locdevKey) {
    if (!isset($_SERVER[$locdevKey]) && false !== ($locdevValue = getenv($locdevKey))) {
        $_SERVER[$locdevKey] = $locdevValue;
    }
}
unset($locdevKey, $locdevValue);
if (false !== ($locdevNext = getenv('LOCDEV_PREPEND_NEXT')) && '' !== $locdevNext && is_file($locdevNext)) {
    unset($locdevNext);
    require getenv('LOCDEV_PREPEND_NEXT');
} else {
    unset($locdevNext);
}
"#;

/// File names of the shim, now and when it only handled dumps, so neither is
/// taken for a prepend file the user set.
const PHP_SHIM_FILE: &str = "locdev-prepend.php";
const OLD_SHIM_FILE: &str = "locdev-dumps.php";

/// Output stays quiet this long → whatever was being read is complete.
const IDLE: Duration = Duration::from_millis(400);
const REDACTOR_TTL: Duration = Duration::from_secs(30);
/// Bytes read from a log file per check.
const LOG_CHUNK: u64 = 1 << 20;
/// One dump payload, at most (base64 of `php::MAX_BYTES`).
const MAX_DUMP_LINE: u64 = 12 << 20;

pub(super) struct DebugState {
    pub(super) store: EventStore,
    assemblers: Mutex<HashMap<String, (Assembler, Instant)>>,
    redactors: Mutex<HashMap<String, (Instant, Arc<Redactor>)>>,
    dump_server: Mutex<Option<SocketAddr>>,
    logs: Mutex<HashMap<PathBuf, (u64, LaravelLog, bool)>>,
}

impl DebugState {
    pub(super) fn load(dir: &Path) -> Self {
        Self {
            store: EventStore::load(dir),
            assemblers: Mutex::default(),
            redactors: Mutex::default(),
            dump_server: Mutex::default(),
            logs: Mutex::default(),
        }
    }
}

fn severity_of(level: Option<LogLevel>) -> Option<EventSeverity> {
    level.map(|l| match l {
        LogLevel::Error => EventSeverity::Error,
        LogLevel::Warn => EventSeverity::Warn,
        LogLevel::Info => EventSeverity::Info,
        LogLevel::Debug => EventSeverity::Debug,
    })
}

/// The runtime a project runs on, for labels and for which parser applies.
fn runtime_of(kind: ProjectKind, adapter: Option<&str>, command: Option<&str>) -> Option<String> {
    let by_kind = match kind {
        ProjectKind::Laravel | ProjectKind::Wordpress | ProjectKind::Php => Some("php"),
        ProjectKind::Next
        | ProjectKind::Nuxt
        | ProjectKind::Astro
        | ProjectKind::Sveltekit
        | ProjectKind::Vite
        | ProjectKind::Node => Some("node"),
        _ => None,
    };
    let by_adapter = adapter.and_then(|a| match a {
        "django" | "flask" | "fastapi" | "python" => Some("python"),
        "axum" | "actix" | "rust" | "cargo" => Some("rust"),
        "rails" | "sinatra" => Some("ruby"),
        _ => None,
    });
    let by_command = command.and_then(|c| {
        let first = c.split_whitespace().next()?;
        let first = first.rsplit('/').next()?;
        Some(match first {
            p if p.starts_with("php") => "php",
            "python" | "python3" | "uvicorn" | "gunicorn" | "flask" | "fastapi" => "python",
            "cargo" => "rust",
            "node" | "npm" | "pnpm" | "yarn" | "bun" | "npx" => "node",
            _ => return None,
        })
    });
    by_kind.or(by_adapter).or(by_command).map(str::to_string)
}

/// `.env` values that aren't plain settings (hosts, ports, drivers, flags…).
fn env_secrets(project: &Project) -> Vec<String> {
    const PLAIN_SUFFIXES: &[&str] = &[
        "_HOST",
        "_PORT",
        "_DRIVER",
        "_CONNECTION",
        "_CHANNEL",
        "_ENV",
        "_DEBUG",
        "_LEVEL",
        "_LOCALE",
        "_TIMEZONE",
        "_NAME",
        "_STACK",
        "_SCHEME",
        "_MAILER",
        "_STORE",
        "_QUEUE",
        "_LIFETIME",
        "_ENCRYPT",
        "_PATH",
    ];
    let mut out = Vec::new();
    for file in [".env", ".env.local"] {
        let Ok(bytes) = std::fs::read(project.path.join(file)) else {
            continue;
        };
        let Ok(doc) = locdev_env::dotenv::parse(&bytes) else {
            continue;
        };
        for entry in doc.entries {
            let value = entry.value.trim();
            let plain = !is_secret_key(&entry.key)
                && (PLAIN_SUFFIXES.iter().any(|s| entry.key.ends_with(s))
                    || matches!(value, "true" | "false" | "null")
                    || value.parse::<f64>().is_ok());
            if !plain {
                out.push(value.to_string());
            }
        }
    }
    out
}

impl Daemon {
    /// Starts every source. Returns the channel the proxy reports 5xx to.
    pub(crate) fn start_debug(self: &Arc<Self>) -> Failures {
        // Process output.
        let (tap, mut lines) = mpsc::unbounded_channel::<(String, LogLine)>();
        self.supervisor.set_tap(tap);
        let this = self.clone();
        tokio::spawn(async move {
            while let Some((id, line)) = lines.recv().await {
                this.on_line(&id, &line).await;
            }
        });

        // Quiet output, log files, retention.
        let this = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(500));
            let mut n: u64 = 0;
            loop {
                tick.tick().await;
                n += 1;
                this.flush_idle().await;
                if n.is_multiple_of(2) {
                    this.tail_logs().await;
                }
                if n % 1200 == 1 {
                    this.prune_events();
                }
            }
        });

        // Process failures.
        let this = self.clone();
        tokio::spawn(async move {
            let mut ends = this.supervisor.subscribe();
            loop {
                match ends.recv().await {
                    Ok((id, info)) if info.state == ProcessState::Failed => {
                        let mut e = blank(
                            EventKind::Process,
                            EventSeverity::Error,
                            info.message
                                .clone()
                                .unwrap_or_else(|| "the process stopped with an error".into()),
                            now_ms(),
                        );
                        e.source = EventSource::Supervisor;
                        e.code = info.exit_code.map(|c| format!("exit {c}"));
                        e.process = Some(id.clone());
                        let tail: Vec<String> = this
                            .supervisor
                            .logs(&id, None, 30)
                            .into_iter()
                            .map(|l| l.text)
                            .collect();
                        e.raw = tail.join("\n");
                        this.record(e).await;
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(_) => return,
                }
            }
        });

        // Dumps.
        let this = self.clone();
        tokio::spawn(async move {
            match TcpListener::bind("127.0.0.1:0").await {
                Ok(listener) => {
                    let addr = listener.local_addr().ok();
                    *this.debug.dump_server.lock().unwrap() = addr;
                    info!(?addr, "dump server listening");
                    loop {
                        let Ok((stream, peer)) = listener.accept().await else {
                            continue;
                        };
                        if !peer.ip().is_loopback() {
                            continue;
                        }
                        let this = this.clone();
                        tokio::spawn(async move { this.read_dumps(stream).await });
                    }
                }
                Err(error) => warn!(%error, "dump server could not listen"),
            }
        });

        // Server errors seen by the proxy.
        let (failures, mut rx) = mpsc::unbounded_channel::<HttpFailure>();
        let this = self.clone();
        tokio::spawn(async move {
            while let Some(f) = rx.recv().await {
                this.on_http_failure(f).await;
            }
        });
        failures
    }

    pub(super) fn dump_server(&self) -> Option<SocketAddr> {
        *self.debug.dump_server.lock().unwrap()
    }

    fn php_shim_dir(&self) -> PathBuf {
        self.paths.data_dir.join("php-prepend")
    }

    /// LocDev's prepend shim and the ini that loads it (rewritten only when
    /// they differ). Under PHP's built-in server (`php -S`, `artisan serve`)
    /// the shim gives proxied HTTPS requests the server variables PHP's
    /// frameworks look for, and copies VarDumper's variables to `$_SERVER`.
    fn ensure_php_shim(&self) -> std::io::Result<PathBuf> {
        let dir = self.php_shim_dir();
        std::fs::create_dir_all(&dir)?;
        let shim = dir.join(PHP_SHIM_FILE);
        let ini = dir.join("95-locdev-prepend.ini");
        let write = |path: &Path, text: &str| -> std::io::Result<()> {
            if std::fs::read_to_string(path).ok().as_deref() != Some(text) {
                std::fs::write(path, text)?;
            }
            Ok(())
        };
        write(&shim, PHP_SHIM)?;
        write(
            &ini,
            &format!(
                "; {}: loads the shim that tells PHP a request came over HTTPS and lets dump()/dd() reach LocDev's Debug Center.\nauto_prepend_file=\"{}\"\n",
                locdev_platform::MARKER,
                shim.display()
            ),
        )?;
        Ok(dir)
    }

    /// The `auto_prepend_file` PHP already had, so the shim can run it too.
    fn previous_prepend(&self, project: &Project, scan: Option<&str>) -> Option<String> {
        let php = self
            .runtimes
            .for_project(project)
            .into_iter()
            .find(|r| r.kind == locdev_core::model::RuntimeKind::Php)
            .and_then(|r| r.version)
            .map(|v| v.executable)
            .unwrap_or_else(|| PathBuf::from("php"));
        let mut command = locdev_core::process::command(&php);
        command.args(["-r", "echo ini_get('auto_prepend_file');"]);
        match scan {
            Some(scan) => command.env("PHP_INI_SCAN_DIR", scan),
            None => command.env_remove("PHP_INI_SCAN_DIR"),
        };
        let out = command.stdin(std::process::Stdio::null()).output().ok()?;
        let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (out.status.success()
            && !value.is_empty()
            && !value.ends_with(PHP_SHIM_FILE)
            && !value.ends_with(OLD_SHIM_FILE))
        .then_some(value)
    }

    /// What every PHP site gets: LocDev's ini folder added to
    /// PHP_INI_SCAN_DIR (the shim, chaining any prepend file already set),
    /// and VarDumper's two variables when its dumps go to LocDev. Nothing in
    /// the project.
    pub(super) fn php_shim_env(&self, project: &Project, env: &mut Vec<(String, String)>) {
        let stack = self.stack_of(project);
        let php = runtime_of(
            stack.kind,
            stack.adapter.as_deref(),
            project.command.as_deref(),
        )
        .as_deref()
            == Some("php");
        if !php {
            return;
        }
        if let Some(addr) = self.dump_server().filter(|_| project.dumps) {
            env.push(("VAR_DUMPER_FORMAT".into(), "server".into()));
            env.push(("VAR_DUMPER_SERVER".into(), addr.to_string()));
        }
        let dir = match self.ensure_php_shim() {
            Ok(dir) => dir,
            Err(error) => {
                warn!(%error, "could not write the PHP shim; HTTPS pages may link to http:// assets and dumps work only where $_SERVER has the environment");
                return;
            }
        };
        let existing = env
            .iter()
            .rev()
            .find(|(k, _)| k == "PHP_INI_SCAN_DIR")
            .map(|(_, v)| v.clone())
            .or_else(|| {
                std::env::var("PHP_INI_SCAN_DIR")
                    .ok()
                    .filter(|v| !v.is_empty())
            });
        if let Some(next) = self.previous_prepend(project, existing.as_deref()) {
            env.push(("LOCDEV_PREPEND_NEXT".into(), next));
        }
        env.retain(|(k, _)| k != "PHP_INI_SCAN_DIR");
        env.push((
            "PHP_INI_SCAN_DIR".into(),
            locdev_runtimes::php_ini::scan_dir_with(existing.as_deref(), &dir),
        ));
    }

    fn redactor(&self, project: Option<&Project>) -> Arc<Redactor> {
        let key = project.map(|p| p.name.clone()).unwrap_or_default();
        if let Some((at, r)) = self.debug.redactors.lock().unwrap().get(&key)
            && at.elapsed() < REDACTOR_TTL
        {
            return r.clone();
        }
        let mut values = Vec::new();
        if let Some(project) = project {
            values.extend(env_secrets(project));
            // Shared variables and secrets LocDev injects into its processes.
            let (injected, _) = self.shared_env(project, ProcessRole::Main);
            values.extend(injected.into_iter().map(|(_, v)| v));
        }
        let r = Arc::new(Redactor::new(values));
        self.debug
            .redactors
            .lock()
            .unwrap()
            .insert(key, (Instant::now(), r.clone()));
        r
    }

    /// Ties the event to its site, hides secrets, fingerprints and stores it.
    pub(super) async fn record(&self, mut e: Event) -> Option<Event> {
        if !self.settings.get().observe.enabled {
            return None;
        }
        let projects = self.registry.lock().await.all().to_vec();
        if e.site.is_none()
            && let Some(id) = e.process.as_deref()
            && !id.starts_with('@')
        {
            e.site = Some(id.split(':').next().unwrap_or(id).to_string());
        }
        if e.site.is_none()
            && let Some(file) = e.file.as_deref()
        {
            e.site = projects
                .iter()
                .filter(|p| Path::new(file).starts_with(&p.path))
                .max_by_key(|p| p.path.as_os_str().len())
                .map(|p| p.name.clone());
        }
        if let Some(id) = e.process.as_deref()
            && let Some(service) = id.strip_prefix("@svc:")
        {
            e.context.insert("service".into(), service.to_string());
        }
        let project = e
            .site
            .as_ref()
            .and_then(|s| projects.iter().find(|p| &p.name == s));
        if let Some(project) = project {
            let stack = self.stack_of(project);
            if e.framework.is_none() && stack.kind != ProjectKind::Unknown {
                e.framework = Some(stack.framework.clone());
            }
            if e.runtime.as_deref().is_none_or(str::is_empty) {
                e.runtime = runtime_of(
                    stack.kind,
                    stack.adapter.as_deref(),
                    project.command.as_deref(),
                );
            }
        }
        if e.runtime.as_deref() == Some("") {
            e.runtime = None;
        }
        self.redactor(project).event(&mut e);
        finish::finish(&mut e);
        Some(self.debug.store.push(e))
    }

    async fn on_line(&self, id: &str, line: &LogLine) {
        let settings = self.settings.get().observe;
        if !settings.enabled {
            return;
        }
        let events = {
            let fresh = !self.debug.assemblers.lock().unwrap().contains_key(id);
            if fresh {
                let site = id.split(':').next().unwrap_or(id);
                let project = if id.starts_with('@') {
                    None
                } else {
                    self.registry.lock().await.get(site).ok().cloned()
                };
                let (runtime, framework) = match &project {
                    Some(p) => {
                        let stack = self.stack_of(p);
                        let command = if id.ends_with(":dev") {
                            p.dev_server.as_ref().map(|d| d.command.clone())
                        } else {
                            p.command.clone()
                        };
                        (
                            runtime_of(stack.kind, stack.adapter.as_deref(), command.as_deref()),
                            (stack.kind != ProjectKind::Unknown).then(|| stack.framework.clone()),
                        )
                    }
                    None => (None, None),
                };
                self.debug.assemblers.lock().unwrap().insert(
                    id.to_string(),
                    (
                        Assembler::new(runtime, framework, settings.logs_from),
                        Instant::now(),
                    ),
                );
            }
            let mut map = self.debug.assemblers.lock().unwrap();
            let Some((assembler, last)) = map.get_mut(id) else {
                return;
            };
            *last = Instant::now();
            assembler.push(Line {
                text: &line.text,
                at: line.at,
                level: severity_of(line.level),
            })
        };
        for mut e in events {
            e.process = Some(id.to_string());
            if e.at == 0 {
                e.at = line.at;
            }
            self.record(e).await;
        }
    }

    async fn flush_idle(&self) {
        let events: Vec<(String, Event)> = {
            let mut map = self.debug.assemblers.lock().unwrap();
            map.iter_mut()
                .filter(|(_, (_, last))| last.elapsed() >= IDLE)
                .flat_map(|(id, (a, _))| {
                    a.flush()
                        .into_iter()
                        .map(|e| (id.clone(), e))
                        .collect::<Vec<_>>()
                })
                .collect()
        };
        for (id, mut e) in events {
            e.process = Some(id);
            if e.at == 0 {
                e.at = now_ms();
            }
            self.record(e).await;
        }
    }

    /// New entries in the Laravel logs of running sites (read only; history
    /// from before LocDev looked is skipped).
    async fn tail_logs(&self) {
        let settings = self.settings.get().observe;
        if !settings.enabled {
            return;
        }
        let projects = self.registry.lock().await.all().to_vec();
        let today = crate::events::day(now_ms());
        for project in projects {
            if self.stack_of(&project).kind != ProjectKind::Laravel
                || self
                    .supervisor
                    .info(&ProcessRole::Main.id(&project.name))
                    .state
                    != ProcessState::Running
            {
                continue;
            }
            let dir = project.path.join("storage/logs");
            for name in ["laravel.log".to_string(), format!("laravel-{today}.log")] {
                let path = dir.join(&name);
                let Ok(meta) = std::fs::metadata(&path) else {
                    continue;
                };
                let size = meta.len();
                let (start, fresh) = {
                    let mut logs = self.debug.logs.lock().unwrap();
                    let entry =
                        logs.entry(path.clone())
                            .or_insert((size, LaravelLog::default(), true));
                    if size < entry.0 {
                        entry.0 = 0; // rotated or truncated
                    }
                    let fresh = entry.2;
                    entry.2 = false;
                    (entry.0, fresh)
                };
                let mut events = Vec::new();
                if size > start && !fresh {
                    let read = tokio::task::spawn_blocking({
                        let path = path.clone();
                        move || -> std::io::Result<String> {
                            use std::io::{Read, Seek, SeekFrom};
                            let mut f = std::fs::File::open(path)?;
                            f.seek(SeekFrom::Start(start))?;
                            let mut buf = Vec::new();
                            f.take(LOG_CHUNK).read_to_end(&mut buf)?;
                            Ok(String::from_utf8_lossy(&buf).into_owned())
                        }
                    })
                    .await;
                    if let Ok(Ok(text)) = read {
                        let mut logs = self.debug.logs.lock().unwrap();
                        if let Some(entry) = logs.get_mut(&path) {
                            entry.0 = start + text.len() as u64;
                            events = entry.1.push(&text, now_ms(), settings.logs_from);
                        }
                    }
                } else if let Some(entry) = self.debug.logs.lock().unwrap().get_mut(&path) {
                    events = entry.1.flush(now_ms(), settings.logs_from);
                }
                for mut e in events {
                    e.site = Some(project.name.clone());
                    e.context
                        .insert("log".into(), format!("storage/logs/{name}"));
                    self.record(e).await;
                }
            }
        }
    }

    async fn read_dumps(&self, stream: tokio::net::TcpStream) {
        let mut reader = BufReader::new(stream);
        loop {
            let mut buf = Vec::new();
            let read = (&mut reader)
                .take(MAX_DUMP_LINE)
                .read_until(b'\n', &mut buf)
                .await;
            match read {
                Ok(0) | Err(_) => return,
                Ok(_) if !buf.ends_with(b"\n") && buf.len() as u64 >= MAX_DUMP_LINE => {
                    warn!("a dump was too large; closing that connection");
                    return;
                }
                Ok(_) => {}
            }
            let line = String::from_utf8_lossy(&buf);
            match dump::decode(&line) {
                Ok(d) => self.on_dump(d).await,
                Err(error) => debug!(%error, "unreadable dump"),
            }
        }
    }

    async fn on_dump(&self, d: dump::Dump) {
        let summary = dump::summary(&d.value);
        let message = match &d.label {
            Some(label) => format!("{label}: {summary}"),
            None => summary,
        };
        let at = d
            .timestamp
            .map(|t| (t * 1000.0) as u64)
            .filter(|t| *t > 0)
            .unwrap_or_else(now_ms);
        let mut e = blank(EventKind::Dump, EventSeverity::Info, message, at);
        e.source = EventSource::Dumper;
        e.runtime = Some("php".into());
        e.file = d.file;
        e.line = d.line;
        e.request = d.request;
        if let Some(command) = d.command {
            e.context.insert("command".into(), command);
        }
        e.dump = Some(d.value);
        // No file inside a site: try the request's host.
        if let Some(host) = e
            .request
            .as_ref()
            .and_then(|r| r.url.as_deref())
            .and_then(|u| u.split("://").nth(1))
            .and_then(|r| r.split(['/', ':']).next())
        {
            let projects = self.registry.lock().await.all().to_vec();
            if let Some(p) = projects
                .iter()
                .find(|p| p.domain == host || host.ends_with(&format!(".{}", p.domain)))
            {
                e.site = Some(p.name.clone());
            }
        }
        self.record(e).await;
    }

    async fn on_http_failure(&self, f: HttpFailure) {
        let (severity, message) = if f.upstream_down {
            (
                EventSeverity::Warn,
                format!("{} {} — the site's server didn't answer", f.method, f.path),
            )
        } else {
            (
                EventSeverity::Error,
                format!("{} {} {}", f.status, f.method, f.path),
            )
        };
        let mut e = blank(EventKind::Http, severity, message, now_ms());
        e.source = EventSource::Proxy;
        e.site = Some(f.site);
        e.code = Some(f.status.to_string());
        e.request = Some(RequestInfo {
            method: Some(f.method),
            url: Some(format!("{}{}", f.host, f.path)),
            status: Some(f.status),
            duration_ms: Some(f.duration_ms),
            controller: None,
        });
        self.record(e).await;
    }

    /// A failed background job (from the queue).
    pub(super) async fn record_job_failure(&self, label: &str, project: Option<&str>, error: &str) {
        let mut e = blank(EventKind::Job, EventSeverity::Error, error, now_ms());
        e.source = EventSource::Jobs;
        e.code = Some(label.to_string());
        e.site = project.map(str::to_string);
        self.record(e).await;
    }

    fn prune_events(&self) {
        let s = self.settings.get().observe;
        self.debug.store.prune(
            s.retention_days.max(1),
            s.max_mb.max(1) as u64 * 1024 * 1024,
            now_ms(),
        );
    }

    async fn observe_status(&self) -> ObserveStatus {
        let projects = self.registry.lock().await.all().to_vec();
        ObserveStatus {
            settings: self.settings.get().observe,
            events: self.debug.store.len(),
            disk_bytes: self.debug.store.disk_bytes(),
            dump_server: self.dump_server().map(|a| a.to_string()),
            dump_sites: projects
                .iter()
                .filter(|p| p.dumps)
                .map(|p| p.name.clone())
                .collect(),
        }
    }

    pub(super) async fn debug_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::ObserveEvents { filter, limit } => Ok(Response::ObserveEvents {
                events: self
                    .debug
                    .store
                    .query(&filter, limit.unwrap_or(200).min(2000)),
                newest: self.debug.store.newest(),
            }),
            Request::ObserveEvent { id } => self
                .debug
                .store
                .get(id)
                .map(|event| Response::ObserveEvent { event })
                .ok_or_else(|| {
                    AppError::new(
                        ErrorCode::NotFound,
                        format!("no event {id} (it may have been cleaned up)"),
                    )
                }),
            Request::ObserveGroups { filter } => Ok(Response::ObserveGroups {
                groups: self.debug.store.groups(&filter, now_ms()),
            }),
            Request::ObserveStatus => Ok(Response::ObserveStatus(self.observe_status().await)),
            Request::ObserveClear { site } => {
                if let Some(site) = &site {
                    self.project(site).await?;
                }
                self.debug.store.clear(site.as_deref());
                Ok(Response::Ok)
            }
            Request::ObserveDumps { site, enabled } => {
                let project = {
                    let mut registry = self.registry.lock().await;
                    let entry = registry.get_mut(&site)?;
                    entry.dumps = enabled;
                    let project = entry.clone();
                    self.commit(&registry)?;
                    project
                };
                let running = self
                    .supervisor
                    .is_active(&ProcessRole::Main.id(&project.name));
                if running {
                    self.restart(&project).await?;
                }
                Ok(Response::ObserveDumps {
                    site,
                    enabled,
                    restarted: running,
                })
            }
            _ => unreachable!("debug_request only gets Debug Center requests"),
        }
    }

    /// Settings changed: apply the new limits now.
    pub(super) fn debug_settings_changed(&self) {
        self.prune_events();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtimes() {
        assert_eq!(
            runtime_of(ProjectKind::Laravel, None, None).as_deref(),
            Some("php")
        );
        assert_eq!(
            runtime_of(ProjectKind::Unknown, Some("django"), None).as_deref(),
            Some("python")
        );
        assert_eq!(
            runtime_of(ProjectKind::Unknown, None, Some("cargo run")).as_deref(),
            Some("rust")
        );
        assert_eq!(
            runtime_of(
                ProjectKind::Unknown,
                None,
                Some("/x/.venv/bin/uvicorn main:app")
            )
            .as_deref(),
            Some("python")
        );
        assert_eq!(runtime_of(ProjectKind::Unknown, None, None), None);
    }

    #[test]
    fn env_values_worth_hiding() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "APP_NAME=Shop\nAPP_KEY=base64:abcdefghijkl\nDB_HOST=127.0.0.1\nDB_PORT=3306\nDB_PASSWORD=s3cret-pw\nAPP_DEBUG=true\nSTRIPE_SECRET=sk_test_123456\n",
        )
        .unwrap();
        let project = Project {
            path: dir.path().to_path_buf(),
            ..Default::default()
        };
        let values = env_secrets(&project);
        assert!(values.contains(&"base64:abcdefghijkl".to_string()));
        assert!(values.contains(&"s3cret-pw".to_string()));
        assert!(values.contains(&"sk_test_123456".to_string()));
        assert!(
            !values
                .iter()
                .any(|v| v == "127.0.0.1" || v == "3306" || v == "true" || v == "Shop")
        );
    }

    /// The shim under PHP's built-in server: a proxied HTTPS request looks
    /// like HTTPS to PHP, a plain one doesn't. Skipped without `php`.
    #[test]
    fn php_shim_marks_proxied_https() {
        let probe = std::process::Command::new("php").arg("-v").output();
        if !probe.is_ok_and(|o| o.status.success()) {
            eprintln!("php not found; skipped");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let shim = dir.path().join(PHP_SHIM_FILE);
        std::fs::write(&shim, PHP_SHIM).unwrap();
        let root = dir.path().join("public");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("index.php"),
            r#"<?php echo ($_SERVER['HTTPS'] ?? '-'), ' ', $_SERVER['REQUEST_SCHEME'] ?? '-', ' ', $_SERVER['SERVER_PORT'];"#,
        )
        .unwrap();
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let mut server = std::process::Command::new("php")
            .arg("-d")
            .arg(format!("auto_prepend_file={}", shim.display()))
            .args(["-S", &format!("127.0.0.1:{port}"), "-t"])
            .arg(&root)
            .env_remove("LOCDEV_PREPEND_NEXT")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let get = |headers: &str| -> String {
            use std::io::{Read, Write};
            for _ in 0..50 {
                if let Ok(mut s) = std::net::TcpStream::connect(("127.0.0.1", port)) {
                    write!(
                        s,
                        "GET / HTTP/1.1\r\nHost: shop.test\r\n{headers}Connection: close\r\n\r\n"
                    )
                    .unwrap();
                    let mut out = String::new();
                    s.read_to_string(&mut out).unwrap();
                    return out.rsplit("\r\n\r\n").next().unwrap_or("").to_string();
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            panic!("php -S did not answer");
        };
        let https = get("X-Forwarded-Proto: https\r\nX-Forwarded-Port: 443\r\n");
        let http = get("X-Forwarded-Proto: http\r\nX-Forwarded-Port: 80\r\n");
        let direct = get("");
        let _ = server.kill();
        let _ = server.wait();
        assert_eq!(https, "on https 443");
        assert_eq!(http, "- http 80");
        assert_eq!(direct, format!("- - {port}"));
    }
}
