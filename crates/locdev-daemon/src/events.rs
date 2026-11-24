//! The Debug Center's event store: the newest events in memory (bounded),
//! every event appended to `data/observe/<site>/<YYYY-MM-DD>.jsonl`, and
//! retention by age and by disk space. Nothing is kept without a limit.

use std::collections::{BTreeMap, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use locdev_core::observe::{Event, EventFilter, EventGroup};
use tracing::warn;

/// Events kept in memory, all sites together.
const MEMORY: usize = 5_000;
/// Folder for events with no site (LocDev's own jobs).
const NO_SITE: &str = "_locdev";
const HOUR_MS: u64 = 3_600_000;

pub struct EventStore {
    dir: PathBuf,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    next_id: u64,
    events: VecDeque<Event>,
}

/// `YYYY-MM-DD` (UTC) for a time in milliseconds.
pub fn day(ms: u64) -> String {
    // Days since 1970-01-01 → civil date (Howard Hinnant's algorithm).
    let z = (ms / 86_400_000) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn folder(site: Option<&str>) -> String {
    // Site names are validated slugs; anything else stays out of paths.
    match site {
        Some(s)
            if !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
        {
            s.to_string()
        }
        _ => NO_SITE.to_string(),
    }
}

fn matches(e: &Event, f: &EventFilter) -> bool {
    if f.site.as_ref().is_some_and(|s| e.site.as_ref() != Some(s)) {
        return false;
    }
    if !f.kinds.is_empty() && !f.kinds.contains(&e.kind) {
        return false;
    }
    if f.min_severity.is_some_and(|m| e.severity < m) {
        return false;
    }
    if f.after.is_some_and(|a| e.id <= a) || f.before.is_some_and(|b| e.id >= b) {
        return false;
    }
    if f.fingerprint
        .as_ref()
        .is_some_and(|fp| &e.fingerprint != fp)
    {
        return false;
    }
    if let Some(q) = f
        .query
        .as_deref()
        .map(str::to_lowercase)
        .filter(|q| !q.is_empty())
    {
        let hay = [
            Some(e.message.as_str()),
            e.code.as_deref(),
            e.file.as_deref(),
            Some(e.raw.as_str()),
            e.request.as_ref().and_then(|r| r.url.as_deref()),
        ];
        if !hay.iter().flatten().any(|h| h.to_lowercase().contains(&q)) {
            return false;
        }
    }
    true
}

impl EventStore {
    /// Loads the newest events from disk (up to the memory limit).
    pub fn load(dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(dir);
        let mut files: Vec<PathBuf> = Vec::new();
        for site in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            for file in std::fs::read_dir(site.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                if file.path().extension().is_some_and(|e| e == "jsonl") {
                    files.push(file.path());
                }
            }
        }
        // Newest days first.
        files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
        let mut events: Vec<Event> = Vec::new();
        for file in files {
            if events.len() >= MEMORY {
                break;
            }
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            events.extend(
                text.lines()
                    .filter_map(|l| serde_json::from_str::<Event>(l).ok()),
            );
        }
        events.sort_by_key(|e| e.id);
        let skip = events.len().saturating_sub(MEMORY);
        let events: VecDeque<Event> = events.into_iter().skip(skip).collect();
        let next_id = events.back().map_or(0, |e| e.id);
        Self {
            dir: dir.to_path_buf(),
            inner: Mutex::new(Inner { next_id, events }),
        }
    }

    /// Gives the event its id, keeps it and appends it to its day's file.
    pub fn push(&self, mut event: Event) -> Event {
        let mut inner = self.inner.lock().unwrap();
        inner.next_id += 1;
        event.id = inner.next_id;
        let folder = self.dir.join(folder(event.site.as_deref()));
        let _ = std::fs::create_dir_all(&folder);
        let path = folder.join(format!("{}.jsonl", day(event.at)));
        match serde_json::to_string(&event) {
            Ok(line) => {
                let written = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .and_then(|mut f| writeln!(f, "{line}"));
                if let Err(error) = written {
                    warn!(%error, path = %path.display(), "could not save an event");
                }
            }
            Err(error) => warn!(%error, "could not serialize an event"),
        }
        if inner.events.len() >= MEMORY {
            inner.events.pop_front();
        }
        inner.events.push_back(event.clone());
        event
    }

    /// Newest first.
    pub fn query(&self, filter: &EventFilter, limit: usize) -> Vec<Event> {
        let inner = self.inner.lock().unwrap();
        inner
            .events
            .iter()
            .rev()
            .filter(|e| matches(e, filter))
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn get(&self, id: u64) -> Option<Event> {
        self.inner
            .lock()
            .unwrap()
            .events
            .iter()
            .find(|e| e.id == id)
            .cloned()
    }

    pub fn newest(&self) -> u64 {
        self.inner.lock().unwrap().next_id
    }

    pub fn len(&self) -> u64 {
        self.inner.lock().unwrap().events.len() as u64
    }

    /// Repeats grouped by fingerprint, most recent first, with hourly
    /// counts over the last day.
    pub fn groups(&self, filter: &EventFilter, now: u64) -> Vec<EventGroup> {
        let inner = self.inner.lock().unwrap();
        let mut groups: BTreeMap<String, EventGroup> = BTreeMap::new();
        for e in inner.events.iter().filter(|e| matches(e, filter)) {
            let g = groups
                .entry(e.fingerprint.clone())
                .or_insert_with(|| EventGroup {
                    fingerprint: e.fingerprint.clone(),
                    kind: e.kind,
                    severity: e.severity,
                    site: e.site.clone(),
                    message: e.message.clone(),
                    code: e.code.clone(),
                    count: 0,
                    first: e.at,
                    last: e.at,
                    last_id: e.id,
                    hourly: vec![0; 24],
                });
            g.count += 1;
            g.first = g.first.min(e.at);
            if e.id >= g.last_id {
                g.last = e.at;
                g.last_id = e.id;
                g.message = e.message.clone();
                g.severity = e.severity;
            }
            let age = now.saturating_sub(e.at) / HOUR_MS;
            if age < 24 {
                g.hourly[23 - age as usize] += 1;
            }
        }
        let mut out: Vec<EventGroup> = groups.into_values().collect();
        out.sort_by_key(|g| std::cmp::Reverse(g.last_id));
        out
    }

    /// Forgets a site's events (or all of them), in memory and on disk.
    pub fn clear(&self, site: Option<&str>) {
        let mut inner = self.inner.lock().unwrap();
        match site {
            Some(site) => {
                inner.events.retain(|e| e.site.as_deref() != Some(site));
                let _ = std::fs::remove_dir_all(self.dir.join(folder(Some(site))));
            }
            None => {
                inner.events.clear();
                for entry in std::fs::read_dir(&self.dir).into_iter().flatten().flatten() {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
        }
    }

    fn files(&self) -> Vec<(PathBuf, String, u64)> {
        let mut out = Vec::new();
        for site in std::fs::read_dir(&self.dir).into_iter().flatten().flatten() {
            for file in std::fs::read_dir(site.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                let path = file.path();
                let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
                    continue;
                };
                let size = file.metadata().map(|m| m.len()).unwrap_or(0);
                out.push((path, stem, size));
            }
        }
        out
    }

    pub fn disk_bytes(&self) -> u64 {
        self.files().iter().map(|(_, _, s)| s).sum()
    }

    /// Drops days older than `days`, then the oldest days until the files
    /// fit in `max_bytes`. Memory follows: events of a dropped day go too.
    pub fn prune(&self, days: u32, max_bytes: u64, now: u64) {
        let oldest = day(now.saturating_sub(days as u64 * 86_400_000));
        let site_of = |path: &Path| {
            path.parent()
                .and_then(Path::file_name)
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        let mut files = self.files();
        files.sort_by(|a, b| a.1.cmp(&b.1));
        let mut total: u64 = files.iter().map(|(_, _, s)| s).sum();
        let mut dropped: std::collections::HashSet<(String, String)> = Default::default();
        for (path, stem, size) in &files {
            if stem.as_str() >= oldest.as_str() && total <= max_bytes {
                continue;
            }
            if std::fs::remove_file(path).is_ok() {
                total -= size;
                dropped.insert((site_of(path), stem.clone()));
            }
        }
        if !dropped.is_empty() {
            let mut inner = self.inner.lock().unwrap();
            inner
                .events
                .retain(|e| !dropped.contains(&(folder(e.site.as_deref()), day(e.at))));
        }
        // Empty site folders go too (`remove_dir` refuses non-empty ones).
        for site in std::fs::read_dir(&self.dir).into_iter().flatten().flatten() {
            let _ = std::fs::remove_dir(site.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use locdev_core::observe::{EventKind, EventSeverity};
    use locdev_observe::parse::blank;

    fn event(site: &str, at: u64, message: &str) -> Event {
        let mut e = blank(EventKind::Exception, EventSeverity::Error, message, at);
        e.site = Some(site.into());
        e.fingerprint = message.into();
        e
    }

    #[test]
    fn days() {
        assert_eq!(day(0), "1970-01-01");
        assert_eq!(day(1_793_836_800_000), "2026-11-05");
        assert_eq!(day(951_782_400_000), "2000-02-29");
    }

    #[test]
    fn stores_queries_groups_and_reloads() {
        let dir = tempfile::tempdir().unwrap();
        let now = 1_793_836_800_000 + 3 * HOUR_MS;
        let store = EventStore::load(dir.path());
        store.push(event("shop", now - HOUR_MS, "boom"));
        store.push(event("shop", now, "boom"));
        store.push(event("blog", now, "other"));
        let shop = EventFilter {
            site: Some("shop".into()),
            ..Default::default()
        };
        assert_eq!(store.query(&shop, 10).len(), 2);
        let groups = store.groups(&EventFilter::default(), now);
        let boom = groups.iter().find(|g| g.fingerprint == "boom").unwrap();
        assert_eq!(boom.count, 2);
        assert_eq!(boom.hourly[23] + boom.hourly[22], 2);
        assert_eq!(
            store
                .query(
                    &EventFilter {
                        after: Some(2),
                        ..Default::default()
                    },
                    10
                )
                .len(),
            1
        );

        let again = EventStore::load(dir.path());
        assert_eq!(again.len(), 3);
        assert_eq!(again.newest(), 3);
        assert_eq!(
            again.push(event("shop", now, "x")).id,
            4,
            "ids continue after a restart"
        );

        again.clear(Some("shop"));
        assert_eq!(again.query(&EventFilter::default(), 10).len(), 1);
        assert!(!dir.path().join("shop").exists());
    }

    #[test]
    fn retention_by_age_and_size() {
        let dir = tempfile::tempdir().unwrap();
        let now = 1_793_836_800_000;
        let store = EventStore::load(dir.path());
        store.push(event("shop", now - 10 * 86_400_000, "old"));
        store.push(event("shop", now - 86_400_000, "yesterday"));
        store.push(event("shop", now, "today"));
        store.prune(7, u64::MAX, now);
        let left: Vec<_> = store
            .query(&EventFilter::default(), 10)
            .into_iter()
            .map(|e| e.message)
            .collect();
        assert_eq!(left, ["today", "yesterday"]);
        store.prune(7, 1, now);
        assert_eq!(
            store.disk_bytes(),
            0,
            "over the size limit, the oldest days go first"
        );
    }
}
