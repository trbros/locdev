//! Runs project dev servers as child processes and tracks their state and output.
//!
//! Each process gets its own process group (Unix) or process tree (Windows) so
//! stopping a project also stops whatever its command spawned (npm → node,
//! artisan → php, ...). A project has up to two processes, keyed by
//! [`ProcessRole::id`]: its main server and an optional helper (Vite).

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, Write};
use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use locdev_core::model::{LogLevel, LogLine, LogStream, ProcessInfo, ProcessState, now_ms};
use locdev_core::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::fsutil::write_atomic;

const GRACE_PERIOD: Duration = Duration::from_secs(8);
const KILL_TIMEOUT: Duration = Duration::from_secs(5);
/// Automatic restarts allowed inside `RESTART_WINDOW` before giving up.
const MAX_RESTARTS: u32 = 5;
const RESTART_WINDOW: Duration = Duration::from_secs(120);

/// How much output is kept, in memory and on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogLimits {
    pub lines: usize,
    pub file_bytes: u64,
}

impl Default for LogLimits {
    fn default() -> Self {
        Self {
            lines: 2000,
            file_bytes: 5 * 1024 * 1024,
        }
    }
}

/// Everything needed to (re)start one process.
#[derive(Debug, Clone)]
pub struct Spec {
    pub id: String,
    pub command: String,
    pub cwd: PathBuf,
    pub port: Option<u16>,
    pub env: Vec<(String, String)>,
    pub restart_on_failure: bool,
    /// Lines for the process's log before it starts (shared variables that
    /// were skipped and why). Never values.
    pub notes: Vec<String>,
}

pub struct Supervisor {
    procs: Mutex<HashMap<String, Arc<Proc>>>,
    /// Every process end, for listeners (failure notifications).
    events: tokio::sync::broadcast::Sender<(String, ProcessInfo)>,
    /// Where each process's output is appended (`<id>.log`).
    log_dir: Option<PathBuf>,
    /// Running processes, so a restarted daemon can find what a crashed one
    /// left behind.
    state_file: Option<PathBuf>,
    limits: LogLimits,
    /// Where every output line is also sent (the Debug Center), once set.
    tap: std::sync::OnceLock<Tap>,
}

/// Output lines with the id of the process that wrote them.
pub type Tap = tokio::sync::mpsc::UnboundedSender<(String, LogLine)>;

struct Proc {
    info: watch::Sender<ProcessInfo>,
    logs: Mutex<LogBuffer>,
    /// Set while a stop is in flight, so the exit is reported as `Stopped`.
    stopping: AtomicBool,
    spec: Mutex<Option<Spec>>,
    /// Automatic restarts inside the current window.
    restarts: Mutex<VecDeque<Instant>>,
    id: String,
    tap: Option<Tap>,
}

#[derive(Default)]
struct LogBuffer {
    lines: VecDeque<LogLine>,
    next_seq: u64,
    file: Option<LogFile>,
    /// Lines from a previous daemon's file are loaded once, lazily.
    loaded: bool,
}

struct LogFile {
    path: PathBuf,
    written: u64,
    max: u64,
}

impl LogFile {
    fn append(&mut self, line: &LogLine) {
        // Rotate before growing past the limit: keep one previous file.
        if self.written > self.max {
            let _ = std::fs::rename(&self.path, self.path.with_extension("log.1"));
            self.written = 0;
        }
        let stream = match line.stream {
            LogStream::Stdout => "out",
            LogStream::Stderr => "err",
            LogStream::System => "sys",
        };
        let text = format!("{} {stream} {}\n", line.at, line.text);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            && file.write_all(text.as_bytes()).is_ok()
        {
            self.written += text.len() as u64;
        }
    }
}

impl Proc {
    fn new(id: &str, file: Option<LogFile>, tap: Option<Tap>) -> Self {
        Self {
            id: id.to_string(),
            tap,
            info: watch::Sender::new(ProcessInfo::default()),
            logs: Mutex::new(LogBuffer {
                file,
                ..Default::default()
            }),
            stopping: AtomicBool::new(false),
            spec: Mutex::new(None),
            restarts: Mutex::default(),
        }
    }

    fn log(&self, stream: LogStream, text: String, capacity: usize) {
        let mut logs = self.logs.lock().unwrap();
        logs.next_seq += 1;
        let level = match stream {
            LogStream::System => None,
            _ => detect_level(&text),
        };
        let line = LogLine {
            seq: logs.next_seq,
            at: now_ms(),
            stream,
            text,
            level,
        };
        if let Some(file) = logs.file.as_mut() {
            file.append(&line);
        }
        if let Some(tap) = &self.tap
            && stream != LogStream::System
        {
            let _ = tap.send((self.id.clone(), line.clone()));
        }
        if logs.lines.len() >= capacity {
            logs.lines.pop_front();
        }
        logs.lines.push_back(line);
    }

    fn is_active(&self) -> bool {
        matches!(
            self.info.borrow().state,
            ProcessState::Running | ProcessState::Stopping
        )
    }
}

/// A record of a running process, kept on disk for crash recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Running {
    id: String,
    pid: u32,
    command: String,
}

impl Supervisor {
    /// A supervisor that keeps output in `log_dir` and its running set in
    /// `state_file`.
    pub fn new(log_dir: PathBuf, state_file: PathBuf, limits: LogLimits) -> Self {
        let _ = std::fs::create_dir_all(&log_dir);
        Self {
            procs: Mutex::default(),
            events: tokio::sync::broadcast::Sender::new(64),
            log_dir: Some(log_dir),
            state_file: Some(state_file),
            limits,
            tap: std::sync::OnceLock::new(),
        }
    }

    /// Sends every output line to `tap` from now on (set once, at startup).
    pub fn set_tap(&self, tap: Tap) {
        let _ = self.tap.set(tap);
    }

    /// Process ends as they happen: `(id, final state)`.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<(String, ProcessInfo)> {
        self.events.subscribe()
    }

    fn get(&self, id: &str) -> Option<Arc<Proc>> {
        self.procs.lock().unwrap().get(id).cloned()
    }

    fn entry(&self, id: &str) -> Arc<Proc> {
        self.procs
            .lock()
            .unwrap()
            .entry(id.to_string())
            .or_insert_with(|| {
                let file = self.log_path(id).map(|path| LogFile {
                    written: std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                    path,
                    max: self.limits.file_bytes,
                });
                Arc::new(Proc::new(id, file, self.tap.get().cloned()))
            })
            .clone()
    }

    /// Where a process's output is kept (ids are project names, which are
    /// already safe file names, plus `:dev`).
    pub fn log_path(&self, id: &str) -> Option<PathBuf> {
        Some(
            self.log_dir
                .as_ref()?
                .join(format!("{}.log", id.replace(':', "-"))),
        )
    }

    pub fn info(&self, id: &str) -> ProcessInfo {
        self.get(id)
            .map(|proc| proc.info.borrow().clone())
            .unwrap_or_default()
    }

    pub fn is_active(&self, id: &str) -> bool {
        self.get(id).is_some_and(|proc| proc.is_active())
    }

    /// Number of projects with an active main process.
    pub fn running(&self) -> usize {
        self.procs
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, proc)| !id.contains(':') && proc.is_active())
            .count()
    }

    pub fn logs(&self, id: &str, after: Option<u64>, limit: usize) -> Vec<LogLine> {
        let proc = match self.get(id) {
            Some(proc) => proc,
            // Output from before this daemon started lives only on disk.
            None if self.log_path(id).is_some_and(|p| p.is_file()) => self.entry(id),
            None => return Vec::new(),
        };
        let mut logs = proc.logs.lock().unwrap();
        if !logs.loaded {
            logs.loaded = true;
            if logs.lines.is_empty()
                && let Some(path) = self.log_path(id)
            {
                for line in read_tail(&path, self.limits.lines) {
                    logs.next_seq += 1;
                    let seq = logs.next_seq;
                    logs.lines.push_back(LogLine { seq, ..line });
                }
            }
        }
        let after = after.unwrap_or(0);
        let fresh: Vec<_> = logs.lines.iter().filter(|l| l.seq > after).collect();
        let skip = fresh.len().saturating_sub(limit);
        fresh.into_iter().skip(skip).cloned().collect()
    }

    /// Starts `spec`. Starting a running process is a no-op.
    pub fn start(self: &Arc<Self>, spec: Spec) -> Result<ProcessInfo, AppError> {
        let proc = self.entry(&spec.id);
        if proc.is_active() {
            return Ok(proc.info.borrow().clone());
        }
        proc.restarts.lock().unwrap().clear();
        self.spawn(&proc, spec, 0)
    }

    fn spawn(
        self: &Arc<Self>,
        proc: &Arc<Proc>,
        spec: Spec,
        restarts: u32,
    ) -> Result<ProcessInfo, AppError> {
        let capacity = self.limits.lines;
        let fail = |message: String| {
            proc.log(LogStream::System, message.clone(), capacity);
            proc.info.send_replace(ProcessInfo {
                state: ProcessState::Failed,
                message: Some(message.clone()),
                restarts,
                ..Default::default()
            });
            AppError::new(ErrorCode::NotRunnable, message)
        };

        if restarts == 0 {
            for note in &spec.notes {
                proc.log(LogStream::System, note.clone(), capacity);
            }
        }
        if !spec.cwd.is_dir() {
            return Err(fail(format!(
                "project folder {} does not exist",
                spec.cwd.display()
            )));
        }
        // Something else on the port would make the dev server fail or, worse,
        // move to another port the proxy doesn't know about.
        if let Some(port) = spec.port
            && TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_err()
        {
            let owner = locdev_platform::ports::owner(port)
                .map(|o| format!(" by {} (pid {})", o.name, o.pid))
                .unwrap_or_default();
            return Err(AppError::new(
                ErrorCode::Conflict,
                fail(format!(
                    "port {port} is already in use{owner}; stop that process or give the site another port"
                ))
                .message,
            ));
        }

        let mut cmd = shell(&spec.command);
        cmd.current_dir(&spec.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("HOST", "127.0.0.1")
            .env("LOCDEV", "1")
            .env(
                "LOCDEV_PROJECT",
                spec.id.split(':').next().unwrap_or(&spec.id),
            )
            .kill_on_drop(false);
        if let Some(port) = spec.port {
            cmd.env("PORT", port.to_string());
        }
        cmd.envs(spec.env.iter().map(|(k, v)| (k, v)));

        proc.log(LogStream::System, format!("$ {}", spec.command), capacity);

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(error) => return Err(fail(format!("failed to start: {error}"))),
        };

        let pid = child.id();
        info!(id = %spec.id, ?pid, command = %spec.command, "process started");

        proc.stopping.store(false, Ordering::SeqCst);
        let info = ProcessInfo {
            state: ProcessState::Running,
            pid,
            started_at: Some(now_ms()),
            restarts,
            ..Default::default()
        };
        proc.info.send_replace(info.clone());
        *proc.spec.lock().unwrap() = Some(spec.clone());
        self.save_running();

        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(pump(proc.clone(), stdout, LogStream::Stdout, capacity));
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(pump(proc.clone(), stderr, LogStream::Stderr, capacity));
        }

        let this = self.clone();
        let proc = proc.clone();
        tokio::spawn(async move {
            let status = child.wait().await;
            let stopping = proc.stopping.load(Ordering::SeqCst);

            let (state, exit_code, message) = match status {
                Ok(status) if stopping => (ProcessState::Stopped, status.code(), None),
                Ok(status) if status.success() => (ProcessState::Exited, status.code(), None),
                Ok(status) => (
                    ProcessState::Failed,
                    status.code(),
                    Some(format!("process exited with {status}")),
                ),
                Err(error) => (ProcessState::Failed, None, Some(error.to_string())),
            };

            info!(id = %spec.id, ?state, ?exit_code, "process ended");
            proc.log(
                LogStream::System,
                match exit_code {
                    Some(code) => format!("process ended with code {code}"),
                    None => "process ended".to_string(),
                },
                capacity,
            );
            proc.info.send_modify(|info| {
                info.state = state;
                info.exit_code = exit_code;
                info.message = message;
            });
            let _ = this
                .events
                .send((spec.id.clone(), proc.info.borrow().clone()));
            this.save_running();

            if state == ProcessState::Failed && spec.restart_on_failure {
                this.restart_later(proc, spec, restarts).await;
            }
        });

        Ok(info)
    }

    /// Restarts a failed process with exponential backoff, at most
    /// `MAX_RESTARTS` times in `RESTART_WINDOW`; a manual start resets it.
    async fn restart_later(self: Arc<Self>, proc: Arc<Proc>, spec: Spec, restarts: u32) {
        let capacity = self.limits.lines;
        let recent = {
            let mut window = proc.restarts.lock().unwrap();
            window.retain(|at| at.elapsed() < RESTART_WINDOW);
            window.len() as u32
        };
        if recent >= MAX_RESTARTS {
            proc.log(
                LogStream::System,
                format!(
                    "failed {MAX_RESTARTS} times in {}s; not restarting again",
                    RESTART_WINDOW.as_secs()
                ),
                capacity,
            );
            proc.info.send_modify(|info| {
                info.message = Some(format!("gave up after {MAX_RESTARTS} automatic restarts"));
            });
            return;
        }
        let delay = Duration::from_secs(1 << recent.min(5));
        proc.log(
            LogStream::System,
            format!(
                "restarting in {}s (restart on failure is on)",
                delay.as_secs()
            ),
            capacity,
        );
        tokio::time::sleep(delay).await;
        // A manual start or stop in the meantime wins.
        if proc.is_active() || proc.info.borrow().state != ProcessState::Failed {
            return;
        }
        proc.restarts.lock().unwrap().push_back(Instant::now());
        if let Err(error) = self.spawn(&proc, spec.clone(), restarts + 1) {
            warn!(id = %spec.id, %error, "automatic restart failed");
        }
    }

    /// Stops a process gracefully, forcing it after a grace period.
    pub async fn stop(&self, id: &str) -> ProcessInfo {
        let Some(proc) = self.get(id) else {
            return ProcessInfo::default();
        };
        let capacity = self.limits.lines;

        let mut pid = None;
        proc.info.send_if_modified(|info| {
            if info.state != ProcessState::Running {
                return false;
            }
            pid = info.pid;
            info.state = ProcessState::Stopping;
            true
        });
        let Some(pid) = pid else {
            return proc.info.borrow().clone();
        };

        proc.stopping.store(true, Ordering::SeqCst);
        proc.log(LogStream::System, "stopping…".to_string(), capacity);

        let mut state = proc.info.subscribe();
        terminate(pid, false).await;

        let ended = |info: &ProcessInfo| info.state != ProcessState::Stopping;
        let stopped = tokio::time::timeout(GRACE_PERIOD, async {
            let _ = state.wait_for(ended).await;
            group_gone(pid).await;
        })
        .await;
        if stopped.is_err() {
            warn!(id, pid, "did not stop in time, killing");
            proc.log(
                LogStream::System,
                "did not stop in time, killing".to_string(),
                capacity,
            );
            terminate(pid, true).await;
            let _ = tokio::time::timeout(KILL_TIMEOUT, async {
                let _ = state.wait_for(ended).await;
                group_gone(pid).await;
            })
            .await;
        }

        proc.info.borrow().clone()
    }

    pub async fn stop_all(&self) {
        let ids: Vec<String> = self.procs.lock().unwrap().keys().cloned().collect();
        futures_util::future::join_all(ids.iter().map(|id| self.stop(id))).await;
    }

    /// Drops state and logs of a project that is no longer registered.
    /// Ids of a project's processes that are running or stopping.
    pub fn active_of(&self, project: &str) -> Vec<String> {
        self.procs
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, proc)| id.split(':').next() == Some(project) && proc.is_active())
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn forget(&self, project: &str) {
        self.procs
            .lock()
            .unwrap()
            .retain(|id, _| id.split(':').next() != Some(project));
    }

    fn save_running(&self) {
        let Some(path) = &self.state_file else {
            return;
        };
        let running: Vec<Running> = self
            .procs
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(id, proc)| {
                let info = proc.info.borrow();
                let spec = proc.spec.lock().unwrap();
                if info.state != ProcessState::Running {
                    return None;
                }
                Some(Running {
                    id: id.clone(),
                    pid: info.pid?,
                    command: spec.as_ref()?.command.clone(),
                })
            })
            .collect();
        if let Ok(raw) = serde_json::to_vec_pretty(&running) {
            let _ = write_atomic(path, &raw);
        }
    }

    /// Stops process groups a previous daemon left running (it crashed or
    /// was killed): their output can't be reattached, so they are stopped
    /// and, if marked to, started again by auto-start. Only processes whose
    /// command line still matches the record are touched.
    pub async fn reap_orphans(&self) -> Vec<String> {
        let Some(path) = &self.state_file else {
            return Vec::new();
        };
        let records: Vec<Running> = std::fs::read(path)
            .ok()
            .and_then(|raw| serde_json::from_slice(&raw).ok())
            .unwrap_or_default();
        let _ = std::fs::remove_file(path);

        let mut reaped = Vec::new();
        for record in records {
            if !locdev_platform::ports::process_matches(record.pid, &record.command) {
                continue;
            }
            warn!(id = %record.id, pid = record.pid, "stopping a dev server left by a previous locdevd");
            terminate(record.pid, false).await;
            tokio::time::sleep(Duration::from_millis(500)).await;
            if locdev_platform::ports::process_matches(record.pid, &record.command) {
                terminate(record.pid, true).await;
            }
            let proc = self.entry(&record.id);
            proc.log(
                LogStream::System,
                format!(
                    "stopped pid {} left running by a previous locdevd (its output can't be reattached)",
                    record.pid
                ),
                self.limits.lines,
            );
            reaped.push(record.id);
        }
        reaped
    }
}

/// A level only when the line states one: a leading or bracketed tag
/// (`ERROR`, `[warn]`, `WARNING:`), PHP's `PHP Fatal error:`, Laravel's
/// `local.ERROR:`, npm's `npm ERR!`/`npm warn`. Anything else has none.
pub fn detect_level(raw: &str) -> Option<LogLevel> {
    let text = strip_ansi(raw);
    let trimmed = text.trim_start();
    let upper: String = trimmed
        .chars()
        .take(48)
        .collect::<String>()
        .to_ascii_uppercase();

    for (prefix, level) in [
        ("NPM ERR!", LogLevel::Error),
        ("NPM ERROR", LogLevel::Error),
        ("NPM WARN", LogLevel::Warn),
        ("PHP FATAL ERROR", LogLevel::Error),
        ("PHP PARSE ERROR", LogLevel::Error),
        ("PHP WARNING", LogLevel::Warn),
        ("PHP NOTICE", LogLevel::Info),
        ("PHP DEPRECATED", LogLevel::Warn),
    ] {
        if upper.starts_with(prefix) {
            return Some(level);
        }
    }
    // Laravel/Monolog: `[2026-01-01 10:00:00] local.ERROR: …`
    for (tag, level) in [
        (".ERROR:", LogLevel::Error),
        (".CRITICAL:", LogLevel::Error),
        (".EMERGENCY:", LogLevel::Error),
        (".ALERT:", LogLevel::Error),
        (".WARNING:", LogLevel::Warn),
        (".NOTICE:", LogLevel::Info),
        (".INFO:", LogLevel::Info),
        (".DEBUG:", LogLevel::Debug),
    ] {
        if upper.contains(tag) {
            return Some(level);
        }
    }
    // A leading word, optionally bracketed, followed by `:`, `]` or a space.
    let word: String = upper
        .trim_start_matches(['[', '('])
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let after = upper.trim_start_matches(['[', '(']).chars().nth(word.len());
    let tagged = matches!(after, Some(':' | ']' | ')' | ' '));
    match word.as_str() {
        "ERROR" | "ERR" | "FATAL" | "PANIC" if tagged => Some(LogLevel::Error),
        "WARN" | "WARNING" if tagged => Some(LogLevel::Warn),
        "INFO" if tagged => Some(LogLevel::Info),
        "DEBUG" | "TRACE" if tagged => Some(LogLevel::Debug),
        _ => None,
    }
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                // Skip parameters up to the final byte (a letter).
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// The last `max` lines of a log file written by [`LogFile::append`].
fn read_tail(path: &Path, max: usize) -> Vec<LogLine> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut lines: VecDeque<LogLine> = VecDeque::new();
    for raw in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        let mut parts = raw.splitn(3, ' ');
        let (Some(at), Some(stream), text) =
            (parts.next(), parts.next(), parts.next().unwrap_or(""))
        else {
            continue;
        };
        let stream = match stream {
            "out" => LogStream::Stdout,
            "err" => LogStream::Stderr,
            "sys" => LogStream::System,
            _ => continue,
        };
        if lines.len() >= max {
            lines.pop_front();
        }
        lines.push_back(LogLine {
            seq: 0,
            at: at.parse().unwrap_or(0),
            stream,
            level: (stream != LogStream::System)
                .then(|| detect_level(text))
                .flatten(),
            text: text.to_string(),
        });
    }
    lines.into()
}

async fn pump(proc: Arc<Proc>, stream: impl AsyncRead + Unpin, kind: LogStream, capacity: usize) {
    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buf);
                proc.log(
                    kind,
                    text.trim_end_matches(['\r', '\n']).to_string(),
                    capacity,
                );
            }
        }
    }
}

/// Expands `{port}` in a command template.
pub fn resolve_command(
    name: &str,
    template: Option<&str>,
    port: Option<u16>,
) -> Result<String, AppError> {
    let template = template
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::NotRunnable,
                format!("project '{name}' has no command configured"),
            )
        })?;

    let command = match port {
        Some(port) => template.replace("{port}", &port.to_string()),
        None if template.contains("{port}") => {
            return Err(AppError::new(
                ErrorCode::NotRunnable,
                "the command uses {port} but the project has no port",
            ));
        }
        None => template.to_string(),
    };
    Ok(own_program(command))
}

/// `locdevd …` (the static server) runs this very executable, which isn't
/// necessarily on PATH (the app ships it as a sidecar).
fn own_program(command: String) -> String {
    let Some(rest) = command.strip_prefix("locdevd ") else {
        return command;
    };
    let Ok(exe) = std::env::current_exe() else {
        return command;
    };
    let exe = exe.display().to_string();
    if cfg!(windows) {
        // `cmd /C` with a quoted program: the outer quotes keep it intact.
        format!("\"\"{exe}\" {rest}\"")
    } else {
        format!("'{}' {rest}", exe.replace('\'', "'\\''"))
    }
}

#[cfg(unix)]
fn shell(command: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    // New process group: lets us signal the whole tree, and keeps the
    // daemon's own Ctrl+C away from the children.
    cmd.process_group(0);
    cmd
}

#[cfg(windows)]
fn shell(command: &str) -> Command {
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut cmd = Command::new("cmd");
    cmd.arg("/C").raw_arg(command);
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    cmd
}

#[cfg(unix)]
async fn terminate(pid: u32, force: bool) {
    use nix::sys::signal::{Signal, killpg};
    use nix::unistd::Pid;

    let signal = if force {
        Signal::SIGKILL
    } else {
        Signal::SIGTERM
    };
    if let Ok(pid) = i32::try_from(pid) {
        // ESRCH just means it is already gone.
        let _ = killpg(Pid::from_raw(pid), signal);
    }
}

/// Waits until no process is left in the group led by `pgid`. The leader
/// exiting isn't enough: dash (`/bin/sh` on Debian/Ubuntu) doesn't `exec`
/// the command, so on SIGTERM the shell dies at once while the server it
/// started (PostgreSQL, say) is still shutting down and writing its files.
#[cfg(unix)]
async fn group_gone(pgid: u32) {
    use nix::sys::signal::killpg;
    use nix::unistd::Pid;

    let Ok(pgid) = i32::try_from(pgid) else {
        return;
    };
    while killpg(Pid::from_raw(pgid), None).is_ok() {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// `taskkill /T /F` already waits for the whole tree.
#[cfg(windows)]
async fn group_gone(_pgid: u32) {}

#[cfg(windows)]
async fn terminate(pid: u32, _force: bool) {
    // Console processes ignore WM_CLOSE, so a "graceful" taskkill would only
    // delay the inevitable; kill the whole tree right away.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_only_when_stated() {
        let cases = [
            ("ERROR: database is down", Some(LogLevel::Error)),
            ("[error] something", Some(LogLevel::Error)),
            ("  WARN  deprecated option", Some(LogLevel::Warn)),
            ("npm ERR! code ENOENT", Some(LogLevel::Error)),
            ("npm warn deprecated glob@7", Some(LogLevel::Warn)),
            ("PHP Fatal error:  Uncaught Error", Some(LogLevel::Error)),
            (
                "[2026-11-02 10:00:00] local.ERROR: boom",
                Some(LogLevel::Error),
            ),
            ("\u{1b}[31mERROR\u{1b}[0m: colored", Some(LogLevel::Error)),
            (
                "   INFO  Server running on [http://127.0.0.1:8100].",
                Some(LogLevel::Info),
            ),
            // Plain output, even alarming-sounding, gets no level.
            ("Compiled with 0 errors", None),
            ("errors: none", None),
            ("GET /api 200 12ms", None),
            ("Errorless build", None),
        ];
        for (line, expected) in cases {
            assert_eq!(detect_level(line), expected, "{line}");
        }
    }

    #[test]
    fn port_placeholder_requires_port() {
        assert_eq!(
            resolve_command("demo", Some("serve --port {port}"), Some(8123)).unwrap(),
            "serve --port 8123"
        );
        assert_eq!(
            resolve_command("demo", Some("serve --port {port}"), None)
                .unwrap_err()
                .code,
            ErrorCode::NotRunnable
        );
        assert_eq!(
            resolve_command("demo", Some("  "), None).unwrap_err().code,
            ErrorCode::NotRunnable
        );
    }
}

#[cfg(all(test, unix))]
mod process_tests {
    use super::*;

    fn spec(dir: &Path, command: &str, port: Option<u16>) -> Spec {
        Spec {
            id: "demo".into(),
            command: command.into(),
            cwd: dir.to_path_buf(),
            port,
            env: Vec::new(),
            notes: Vec::new(),
            restart_on_failure: false,
        }
    }

    fn supervisor(dir: &Path) -> Arc<Supervisor> {
        Arc::new(Supervisor::new(
            dir.join("logs"),
            dir.join("running.json"),
            LogLimits::default(),
        ))
    }

    async fn wait_state(sup: &Supervisor, id: &str, state: ProcessState) {
        let proc = sup.get(id).unwrap();
        let mut rx = proc.info.subscribe();
        tokio::time::timeout(Duration::from_secs(10), rx.wait_for(|i| i.state == state))
            .await
            .expect("state not reached")
            .unwrap();
    }

    fn free_port() -> u16 {
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[tokio::test]
    async fn captures_output_and_exit() {
        let dir = tempfile::tempdir().unwrap();
        let sup = supervisor(dir.path());
        let port = free_port();
        sup.start(spec(
            dir.path(),
            "echo port=$PORT; echo 'ERROR: oops' >&2; exit 3",
            Some(port),
        ))
        .unwrap();
        wait_state(&sup, "demo", ProcessState::Failed).await;
        assert_eq!(sup.info("demo").exit_code, Some(3));

        // Output pumps may lag behind the exit notification, more so on a
        // busy machine: wait for both lines, up to a few seconds.
        let mut lines = Vec::new();
        for _ in 0..60 {
            lines = sup.logs("demo", None, 100);
            if lines.iter().any(|l| l.text == "ERROR: oops")
                && lines.iter().any(|l| l.text.starts_with("port="))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            lines.iter().any(|l| l.text == format!("port={port}")),
            "{lines:?}"
        );
        let oops = lines.iter().find(|l| l.text == "ERROR: oops").unwrap();
        assert_eq!(
            (oops.stream, oops.level),
            (LogStream::Stderr, Some(LogLevel::Error))
        );

        // The same output is on disk, and a new supervisor reads it back.
        let reborn = supervisor(dir.path());
        let restored = reborn.logs("demo", None, 100);
        assert!(
            restored.iter().any(|l| l.text == "ERROR: oops"),
            "{restored:?}"
        );
    }

    #[tokio::test]
    async fn stop_kills_the_whole_tree() {
        let dir = tempfile::tempdir().unwrap();
        let sup = supervisor(dir.path());
        // The shell spawns a grandchild; stopping must take both down.
        let pid = sup
            .start(spec(dir.path(), "sleep 30 & wait", None))
            .unwrap()
            .pid
            .unwrap();
        let info = sup.stop("demo").await;

        assert_eq!(info.state, ProcessState::Stopped);
        let group_alive =
            nix::sys::signal::killpg(nix::unistd::Pid::from_raw(pid as i32), None).is_ok();
        assert!(!group_alive, "process group still alive");
    }

    #[tokio::test]
    async fn stop_waits_for_the_server_behind_the_shell() {
        let dir = tempfile::tempdir().unwrap();
        let sup = supervisor(dir.path());
        // The outer shell (`; true` keeps it from exec'ing) dies on SIGTERM
        // right away; the inner one still has work to do, like a database
        // writing its shutdown checkpoint.
        sup.start(spec(
            dir.path(),
            "sh -c 'trap \"sleep 0.5; touch shut-down; exit 0\" TERM; while :; do sleep 0.1; done'; true",
            None,
        ))
        .unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        let info = sup.stop("demo").await;

        assert_eq!(info.state, ProcessState::Stopped);
        assert!(
            dir.path().join("shut-down").exists(),
            "stop returned before the server finished"
        );
    }

    #[tokio::test]
    async fn starting_twice_runs_one_instance() {
        let dir = tempfile::tempdir().unwrap();
        let sup = supervisor(dir.path());
        let first = sup.start(spec(dir.path(), "sleep 30", None)).unwrap();
        let second = sup.start(spec(dir.path(), "sleep 30", None)).unwrap();
        assert_eq!(first.pid, second.pid);
        sup.stop("demo").await;
    }

    #[tokio::test]
    async fn refuses_a_taken_port_and_names_it() {
        let dir = tempfile::tempdir().unwrap();
        let sup = supervisor(dir.path());
        let taken = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = taken.local_addr().unwrap().port();

        let error = sup.start(spec(dir.path(), "true", Some(port))).unwrap_err();
        assert_eq!(error.code, ErrorCode::Conflict);
        assert!(
            error
                .message
                .contains(&format!("port {port} is already in use")),
            "{}",
            error.message
        );
        assert_eq!(sup.info("demo").state, ProcessState::Failed);
    }

    #[tokio::test]
    async fn restarts_on_failure_with_a_limit() {
        let dir = tempfile::tempdir().unwrap();
        let sup = supervisor(dir.path());
        let mut failing = spec(dir.path(), "exit 1", None);
        failing.restart_on_failure = true;
        sup.start(failing).unwrap();

        // The first automatic restart comes after 1 s of backoff.
        tokio::time::timeout(Duration::from_secs(10), async {
            while sup.info("demo").restarts < 1 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("no automatic restart");
        assert!(
            sup.logs("demo", None, 100)
                .iter()
                .any(|l| l.text.starts_with("restarting in"))
        );
        // A manual stop ends the cycle.
        sup.stop("demo").await;
    }

    #[tokio::test]
    async fn logs_after_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let sup = supervisor(dir.path());
        sup.start(spec(dir.path(), "echo a; echo b; echo c", None))
            .unwrap();
        wait_state(&sup, "demo", ProcessState::Exited).await;
        tokio::time::sleep(Duration::from_millis(150)).await;

        let all = sup.logs("demo", None, 100);
        let cursor = all[1].seq;
        let rest = sup.logs("demo", Some(cursor), 100);
        assert_eq!(rest.len(), all.len() - 2);

        let last_two = sup.logs("demo", None, 2);
        assert_eq!(last_two, all[all.len() - 2..]);
    }

    #[tokio::test]
    async fn reaps_what_a_crashed_daemon_left() {
        let dir = tempfile::tempdir().unwrap();
        let sup = supervisor(dir.path());
        let pid = sup
            .start(spec(dir.path(), "sleep 31", None))
            .unwrap()
            .pid
            .unwrap();
        assert!(dir.path().join("running.json").is_file());
        // While `sh -c` execs `sleep`, /proc shows no command line for an
        // instant; a real orphan has been running for a while. Up to 10 s:
        // a busy CI runner (macOS) can take more than 3.
        for _ in 0..200 {
            if locdev_platform::ports::process_matches(pid, "sleep 31") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // A new daemon (the old one "crashed": nothing was stopped).
        let next = supervisor(dir.path());
        let reaped = next.reap_orphans().await;
        assert_eq!(reaped, vec!["demo".to_string()]);
        // The group takes a moment to go away, longer on a busy machine.
        let group = nix::unistd::Pid::from_raw(pid as i32);
        let mut alive = true;
        for _ in 0..60 {
            alive = nix::sys::signal::killpg(group, None).is_ok();
            if !alive {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(!alive, "orphan still alive");
        drop(sup);
    }
}
