//! Debug Center: the normalized events LocDev collects from the sites it
//! runs (exceptions, dumps, logs, HTTP errors, process events) and what the
//! clients ask about them. Parsing lives in `locdev-observe`; the daemon
//! owns the pipeline (association, redaction, storage, retention).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// An exception or error with a message (and often a stack trace).
    Exception,
    /// `dump()`/`dd()` from Symfony VarDumper (Laravel, Symfony).
    Dump,
    /// A warning or error line from a process or a log file.
    Log,
    /// A request that ended in a server error (5xx), seen by the proxy or
    /// the PHP built-in server.
    Http,
    /// A process that failed to start, crashed or was restarted.
    Process,
    /// A build/compile error from a dev server (Vite, Next.js, tsc…).
    Build,
    /// A LocDev background job that failed.
    Job,
}

impl EventKind {
    pub const ALL: [Self; 7] = [
        Self::Exception,
        Self::Dump,
        Self::Log,
        Self::Http,
        Self::Process,
        Self::Build,
        Self::Job,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Where an event came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    /// A process's stdout/stderr (site, helper, worker or service).
    Output,
    /// A project's log file (Laravel's `storage/logs`).
    LogFile,
    /// LocDev's dump server (VarDumper's `server` format).
    Dumper,
    /// The reverse proxy.
    Proxy,
    /// The supervisor (starts, exits, restarts).
    Supervisor,
    /// The job queue.
    Jobs,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    /// Inside dependencies (vendor, node_modules, site-packages, the Rust
    /// registry or std): collapsed in the UI.
    #[serde(default)]
    pub vendor: bool,
}

/// The request an event belongs to, when known.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller: Option<String>,
}

/// A dumped value, rebuilt from VarDumper's data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum DumpNode {
    Null,
    Bool {
        v: bool,
    },
    Int {
        v: i64,
    },
    Float {
        v: f64,
    },
    Str {
        v: String,
        /// Characters VarDumper left out.
        #[serde(default, skip_serializing_if = "is_zero")]
        cut: i64,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        binary: bool,
    },
    /// Constants, closures' signatures and other text VarDumper shows as is.
    Text {
        v: String,
    },
    Array {
        count: i64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        items: Vec<DumpEntry>,
        #[serde(default, skip_serializing_if = "is_zero")]
        cut: i64,
    },
    Object {
        class: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        handle: Option<i64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        items: Vec<DumpEntry>,
        #[serde(default, skip_serializing_if = "is_zero")]
        cut: i64,
    },
    Resource {
        kind: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        items: Vec<DumpEntry>,
    },
    /// Already shown above (a recursion or a repeated object).
    Seen {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        handle: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        class: Option<String>,
    },
    /// Hidden by LocDev: it looked like a secret.
    Redacted,
    /// Deeper than LocDev keeps.
    Truncated,
}

fn is_zero(n: &i64) -> bool {
    *n == 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyKind {
    /// Array index or key.
    Key,
    Public,
    Protected,
    Private,
    /// Added by VarDumper (`~`), not a real property.
    Meta,
    /// Set at runtime on the object.
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DumpEntry {
    pub key: String,
    pub kind: KeyKind,
    pub value: DumpNode,
}

/// One normalized event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    /// Milliseconds since the epoch.
    pub at: u64,
    pub kind: EventKind,
    pub severity: EventSeverity,
    pub source: EventSource,
    /// The site (project name) it belongs to; `None` for LocDev itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    /// Supervisor id of the process (`shop`, `shop:dev`, `@svc:pg`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,
    /// One line: what happened.
    pub message: String,
    /// Exception class, error code or HTTP status, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    /// `php`, `node`, `python`, `rust`…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stack: Vec<Frame>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<RequestInfo>,
    /// The dumped value (dumps only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dump: Option<DumpNode>,
    /// The raw text it was read from (redacted), for copying.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub raw: String,
    /// Anything else worth showing (label, command line, exit code…).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,
    /// Same fingerprint, same problem: what groups repeats.
    pub fingerprint: String,
    /// What usually causes this, when LocDev recognizes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Repeats of one problem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventGroup {
    pub fingerprint: String,
    pub kind: EventKind,
    pub severity: EventSeverity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub count: u64,
    pub first: u64,
    pub last: u64,
    /// The newest occurrence, to open.
    pub last_id: u64,
    /// Occurrences per hour over the last 24 hours, oldest first.
    pub hourly: Vec<u32>,
}

/// What the event queries filter on (all optional).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EventFilter {
    pub site: Option<String>,
    /// Empty: every kind.
    pub kinds: Vec<EventKind>,
    pub min_severity: Option<EventSeverity>,
    /// Text in the message, code, file or raw text (case-insensitive).
    pub query: Option<String>,
    /// Only events newer than this id (live updates).
    pub after: Option<u64>,
    /// Only events older than this id (paging back).
    pub before: Option<u64>,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ObserveSettings {
    /// Collect events at all.
    pub enabled: bool,
    /// Days events are kept on disk.
    pub retention_days: u32,
    /// Disk space for events, all sites together.
    pub max_mb: u32,
    /// Plain log lines from this level up become events (`warn` or `error`).
    pub logs_from: EventSeverity,
}

impl Default for ObserveSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: 7,
            max_mb: 100,
            logs_from: EventSeverity::Warn,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserveStatus {
    pub settings: ObserveSettings,
    /// Events kept in memory now and bytes on disk.
    pub events: u64,
    pub disk_bytes: u64,
    /// The dump server's address (`127.0.0.1:port`), when listening.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dump_server: Option<String>,
    /// Sites sending their dumps to LocDev.
    #[serde(default)]
    pub dump_sites: Vec<String>,
}
