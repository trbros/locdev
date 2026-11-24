//! Per-site request counters, filled by the proxy as requests pass through.
//! Memory is bounded: a fixed window of latencies, minutes and samples, and
//! at most `MAX_PATHS` distinct paths per site.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use locdev_core::metrics::{
    Latency, MinuteBucket, PathMetric, RequestMetrics, RequestSample, StatusCounts,
};

const LATENCIES: usize = 1000;
const MINUTES: u64 = 60;
const RECENT: usize = 100;
// ponytail: paths past this are not tracked individually (still counted in
// the totals); an LRU would keep newer ones if that ever matters.
const MAX_PATHS: usize = 300;
const TOP: usize = 20;

#[derive(Debug, Default)]
struct Site {
    since: u64,
    total: u64,
    status: StatusCounts,
    upstream_down: u64,
    latencies: VecDeque<u32>,
    minutes: VecDeque<MinuteBucket>,
    paths: HashMap<(String, String), (u64, u64, u64, u32)>, // requests, errors, total ms, max ms
    recent: VecDeque<RequestSample>,
}

/// One request as the proxy saw it.
pub struct Seen<'a> {
    pub site: &'a str,
    pub host: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub status: u16,
    pub ms: u32,
    pub upstream_down: bool,
    pub at: u64,
}

#[derive(Debug, Default)]
pub struct Metrics {
    sites: Mutex<HashMap<String, Site>>,
}

impl Metrics {
    pub fn record(&self, seen: Seen<'_>) {
        let mut sites = self.sites.lock().unwrap();
        let site = sites.entry(seen.site.to_string()).or_insert_with(|| Site {
            since: seen.at,
            ..Default::default()
        });
        site.total += 1;
        let error = seen.upstream_down || seen.status >= 500;
        match seen.status {
            100..=199 => site.status.informational += 1,
            200..=299 => site.status.success += 1,
            300..=399 => site.status.redirect += 1,
            400..=499 => site.status.client_error += 1,
            _ => site.status.server_error += 1,
        }
        site.upstream_down += u64::from(seen.upstream_down);

        if site.latencies.len() == LATENCIES {
            site.latencies.pop_front();
        }
        site.latencies.push_back(seen.ms);

        let minute = seen.at - seen.at % 60_000;
        match site.minutes.back_mut() {
            Some(last) if last.minute == minute => {
                last.requests += 1;
                last.errors += u32::from(error);
            }
            _ => site.minutes.push_back(MinuteBucket {
                minute,
                requests: 1,
                errors: u32::from(error),
            }),
        }
        while site
            .minutes
            .front()
            .is_some_and(|m| m.minute + MINUTES * 60_000 <= minute)
        {
            site.minutes.pop_front();
        }

        let path = seen.path.split('?').next().unwrap_or("/").to_string();
        let key = (seen.method.to_string(), path);
        if site.paths.len() < MAX_PATHS || site.paths.contains_key(&key) {
            let entry = site.paths.entry(key).or_default();
            entry.0 += 1;
            entry.1 += u64::from(error);
            entry.2 += u64::from(seen.ms);
            entry.3 = entry.3.max(seen.ms);
        }

        if site.recent.len() == RECENT {
            site.recent.pop_back();
        }
        site.recent.push_front(RequestSample {
            at: seen.at,
            method: seen.method.into(),
            path: seen.path.into(),
            status: seen.status,
            ms: seen.ms,
            host: seen.host.into(),
        });
    }

    pub fn snapshot(&self, name: &str) -> RequestMetrics {
        let sites = self.sites.lock().unwrap();
        let Some(site) = sites.get(name) else {
            return RequestMetrics {
                site: name.into(),
                ..Default::default()
            };
        };
        let mut paths: Vec<PathMetric> = site
            .paths
            .iter()
            .map(
                |((method, path), (requests, errors, total, max))| PathMetric {
                    method: method.clone(),
                    path: path.clone(),
                    requests: *requests,
                    errors: *errors,
                    avg_ms: (total / requests.max(&1)) as u32,
                    max_ms: *max,
                },
            )
            .collect();
        paths.sort_by(|a, b| b.requests.cmp(&a.requests).then(a.path.cmp(&b.path)));
        paths.truncate(TOP);
        RequestMetrics {
            site: name.into(),
            since: Some(site.since),
            total: site.total,
            status: site.status,
            upstream_down: site.upstream_down,
            latency: latency(&site.latencies),
            per_minute: site.minutes.iter().copied().collect(),
            top_paths: paths,
            recent: site.recent.iter().cloned().collect(),
        }
    }

    pub fn reset(&self, name: &str) {
        self.sites.lock().unwrap().remove(name);
    }
}

fn latency(values: &VecDeque<u32>) -> Latency {
    if values.is_empty() {
        return Latency::default();
    }
    let mut sorted: Vec<u32> = values.iter().copied().collect();
    sorted.sort_unstable();
    // Nearest rank.
    let pick = |p: usize| sorted[(sorted.len() * p).div_ceil(100).saturating_sub(1)];
    Latency {
        samples: sorted.len() as u32,
        avg_ms: (sorted.iter().map(|v| u64::from(*v)).sum::<u64>() / sorted.len() as u64) as u32,
        p50_ms: pick(50),
        p95_ms: pick(95),
        p99_ms: pick(99),
        max_ms: *sorted.last().unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(path: &str, status: u16, ms: u32, at: u64) -> Seen<'_> {
        Seen {
            site: "shop",
            host: "shop.test",
            method: "GET",
            path,
            status,
            ms,
            upstream_down: status == 502,
            at,
        }
    }

    #[test]
    fn counts_percentiles_minutes_and_paths() {
        let m = Metrics::default();
        let base = 1_700_000_000_000 - 1_700_000_000_000 % 60_000;
        for i in 1..=100u32 {
            m.record(seen("/a?x=1", 200, i, base + u64::from(i)));
        }
        m.record(seen("/b", 500, 5, base + 61_000));
        m.record(seen("/b", 502, 1, base + 62_000));
        let s = m.snapshot("shop");
        assert_eq!(s.total, 102);
        assert_eq!(s.status.success, 100);
        assert_eq!(s.status.server_error, 2);
        assert_eq!(s.upstream_down, 1);
        // 102 samples (1..=100, 5, 1): the 51st smallest is 49.
        assert_eq!(s.latency.p50_ms, 49);
        assert_eq!(s.latency.max_ms, 100);
        assert_eq!(s.per_minute.len(), 2);
        assert_eq!(s.per_minute[1].errors, 2);
        assert_eq!(s.top_paths[0].path, "/a");
        assert_eq!(s.top_paths[0].requests, 100);
        assert_eq!(s.recent[0].status, 502);
        // An hour later the old minutes are gone.
        m.record(seen("/a", 200, 1, base + 3_700_000));
        assert_eq!(m.snapshot("shop").per_minute.len(), 1);
        m.reset("shop");
        assert_eq!(m.snapshot("shop").total, 0);
        assert_eq!(m.snapshot("other").since, None);
    }
}
