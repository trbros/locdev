//! Request metrics: what the proxy saw for a site since the daemon started
//! (or since the last reset). Kept in memory only; nothing leaves the machine.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RequestMetrics {
    pub site: String,
    /// When counting started (ms since the epoch); `None` = nothing seen yet.
    #[serde(default)]
    pub since: Option<u64>,
    pub total: u64,
    pub status: StatusCounts,
    /// Requests the site's server didn't answer at all (502 from LocDev).
    pub upstream_down: u64,
    /// Time to the response headers, over the last requests kept.
    pub latency: Latency,
    /// The last 60 minutes, oldest first; minutes without requests left out.
    pub per_minute: Vec<MinuteBucket>,
    /// Busiest paths (method + path without the query), most requests first.
    pub top_paths: Vec<PathMetric>,
    /// The last requests, newest first.
    pub recent: Vec<RequestSample>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusCounts {
    pub informational: u64,
    pub success: u64,
    pub redirect: u64,
    pub client_error: u64,
    pub server_error: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Latency {
    /// How many requests these figures come from.
    pub samples: u32,
    pub avg_ms: u32,
    pub p50_ms: u32,
    pub p95_ms: u32,
    pub p99_ms: u32,
    pub max_ms: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinuteBucket {
    /// Start of the minute, ms since the epoch.
    pub minute: u64,
    pub requests: u32,
    /// 5xx answers and unreachable server.
    pub errors: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathMetric {
    pub method: String,
    pub path: String,
    pub requests: u64,
    pub errors: u64,
    pub avg_ms: u32,
    pub max_ms: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSample {
    /// ms since the epoch.
    pub at: u64,
    pub method: String,
    /// Path and query, as requested.
    pub path: String,
    pub status: u16,
    pub ms: u32,
    /// The subdomain it came in on, when not the site itself.
    #[serde(default)]
    pub host: String,
}
