//! Previous versions of a config file, kept by the daemon before each save
//! so any change made through LocDev can be compared and reverted.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use locdev_core::config::ConfigHistoryEntry;
use serde::{Deserialize, Serialize};

/// Versions kept per file; the oldest go first.
const KEEP: usize = 50;

pub struct History {
    dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct Meta {
    at: u64,
    reason: String,
    size: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// A file name for any relative path (`src/app.css` → `src%2Fapp.css`).
fn slot(rel: &str) -> String {
    rel.chars()
        .map(|c| match c {
            '/' | '\\' => "%2F".to_string(),
            '%' => "%25".to_string(),
            c => c.to_string(),
        })
        .collect()
}

impl History {
    /// History of `rel` in `site`, under `base` (the daemon's state dir).
    pub fn new(base: &Path, site: &str, rel: &str) -> Self {
        Self {
            dir: base.join(slot(site)).join(slot(rel)),
        }
    }

    fn ensure(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.dir, std::fs::Permissions::from_mode(0o700))?;
        }
        Ok(())
    }

    /// Keeps `content` (what is about to be replaced); returns its id.
    pub fn record(&self, content: &str, reason: &str) -> std::io::Result<String> {
        self.ensure()?;
        let mut at = now_ms();
        while self.dir.join(format!("{at}.txt")).exists() {
            at += 1;
        }
        let id = at.to_string();
        std::fs::write(self.dir.join(format!("{id}.txt")), content)?;
        let meta = Meta {
            at,
            reason: reason.into(),
            size: content.len() as u64,
        };
        std::fs::write(
            self.dir.join(format!("{id}.json")),
            serde_json::to_vec(&meta)?,
        )?;
        let entries = self.list();
        for old in entries.iter().skip(KEEP) {
            let _ = std::fs::remove_file(self.dir.join(format!("{}.txt", old.id)));
            let _ = std::fs::remove_file(self.dir.join(format!("{}.json", old.id)));
        }
        Ok(id)
    }

    /// Newest first.
    pub fn list(&self) -> Vec<ConfigHistoryEntry> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return out;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(id) = name.strip_suffix(".json") else {
                continue;
            };
            if let Ok(bytes) = std::fs::read(entry.path())
                && let Ok(meta) = serde_json::from_slice::<Meta>(&bytes)
            {
                out.push(ConfigHistoryEntry {
                    id: id.to_string(),
                    at: meta.at,
                    size: meta.size,
                    reason: meta.reason,
                });
            }
        }
        out.sort_by_key(|e| std::cmp::Reverse(e.at));
        out
    }

    pub fn load(&self, id: &str) -> Result<String, String> {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("no saved version {id}"));
        }
        std::fs::read_to_string(self.dir.join(format!("{id}.txt")))
            .map_err(|_| format!("no saved version {id}"))
    }
}
