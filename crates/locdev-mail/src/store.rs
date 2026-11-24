//! Captured messages on disk: `<id>.eml` (exactly as received) and
//! `<id>.json` (envelope, site, read flag). The index lives in memory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use locdev_core::mail::{MailDetail, MailEnvelope, MailSummary};
use serde::{Deserialize, Serialize};

use crate::parse;

/// Oldest messages go first beyond this.
pub const KEEP: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Meta {
    envelope: MailEnvelope,
    #[serde(default)]
    site: Option<String>,
    #[serde(default)]
    read: bool,
    summary: MailSummary,
}

pub struct Store {
    dir: PathBuf,
    index: Mutex<BTreeMap<String, Meta>>,
}

#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub site: Option<String>,
    /// Matched against subject, from, to and preview, case-insensitively.
    pub query: Option<String>,
    pub unread: bool,
    pub with_attachments: bool,
    pub limit: Option<usize>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 40 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

impl Store {
    pub fn open(dir: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        }
        let mut index = BTreeMap::new();
        for entry in std::fs::read_dir(dir)?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id) = name.strip_suffix(".json")
                && let Ok(bytes) = std::fs::read(entry.path())
                && let Ok(meta) = serde_json::from_slice::<Meta>(&bytes)
                && dir.join(format!("{id}.eml")).is_file()
            {
                index.insert(id.to_string(), meta);
            }
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            index: Mutex::new(index),
        })
    }

    /// Stores a message; returns its summary. `site` is the sending site, when known.
    pub fn add(
        &self,
        envelope: MailEnvelope,
        site: Option<String>,
        raw: &[u8],
    ) -> std::io::Result<MailSummary> {
        // Time-ordered ids: sorting the index sorts by arrival.
        let mut index = self.index.lock().unwrap();
        let mut n = envelope.received_at;
        let mut id = format!("{n:013}");
        while index.contains_key(&id) {
            n += 1;
            id = format!("{n:013}");
        }
        let parsed = parse::parse(raw);
        let summary = MailSummary {
            id: id.clone(),
            site: site.clone(),
            from: if parsed.from.is_empty() {
                envelope.from.clone()
            } else {
                parsed.from.clone()
            },
            to: if parsed.to.is_empty() {
                envelope.to.clone()
            } else {
                parsed.to.clone()
            },
            subject: parsed.subject.clone(),
            date: parsed.date,
            received_at: envelope.received_at,
            size: raw.len() as u64,
            attachments: parsed.attachments.iter().filter(|a| !a.inline).count(),
            has_html: parsed.html.is_some(),
            preview: parsed.preview.clone(),
            read: false,
        };
        std::fs::write(self.dir.join(format!("{id}.eml")), raw)?;
        let meta = Meta {
            envelope,
            site,
            read: false,
            summary: summary.clone(),
        };
        std::fs::write(
            self.dir.join(format!("{id}.json")),
            serde_json::to_vec(&meta)?,
        )?;
        index.insert(id, meta);
        while index.len() > KEEP {
            let Some(oldest) = index.keys().next().cloned() else {
                break;
            };
            index.remove(&oldest);
            self.remove_files(&oldest);
        }
        Ok(summary)
    }

    fn remove_files(&self, id: &str) {
        let _ = std::fs::remove_file(self.dir.join(format!("{id}.eml")));
        let _ = std::fs::remove_file(self.dir.join(format!("{id}.json")));
    }

    /// Newest first.
    pub fn list(&self, filter: &Filter) -> Vec<MailSummary> {
        let index = self.index.lock().unwrap();
        let query = filter
            .query
            .as_deref()
            .map(str::to_lowercase)
            .filter(|q| !q.is_empty());
        let mut out: Vec<MailSummary> = index
            .values()
            .rev()
            .filter(|m| {
                filter
                    .site
                    .as_ref()
                    .is_none_or(|s| m.site.as_ref() == Some(s))
            })
            .filter(|m| !filter.unread || !m.read)
            .filter(|m| !filter.with_attachments || m.summary.attachments > 0)
            .filter(|m| {
                query.as_ref().is_none_or(|q| {
                    let s = &m.summary;
                    [s.subject.as_str(), s.from.as_str(), s.preview.as_str()]
                        .iter()
                        .any(|f| f.to_lowercase().contains(q))
                        || s.to.iter().any(|t| t.to_lowercase().contains(q))
                })
            })
            .map(|m| MailSummary {
                read: m.read,
                ..m.summary.clone()
            })
            .collect();
        if let Some(limit) = filter.limit {
            out.truncate(limit);
        }
        out
    }

    /// Ids of the messages not opened yet.
    pub fn unread_ids(&self) -> std::collections::HashSet<String> {
        let index = self.index.lock().unwrap();
        index
            .iter()
            .filter(|(_, m)| !m.read)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn counts(&self) -> (usize, usize) {
        let index = self.index.lock().unwrap();
        (index.len(), index.values().filter(|m| !m.read).count())
    }

    pub fn raw(&self, id: &str) -> Option<Vec<u8>> {
        if !valid_id(id) || !self.index.lock().unwrap().contains_key(id) {
            return None;
        }
        std::fs::read(self.dir.join(format!("{id}.eml"))).ok()
    }

    /// The full message; marks it read.
    pub fn get(&self, id: &str) -> Option<MailDetail> {
        let raw = self.raw(id)?;
        let meta = {
            let mut index = self.index.lock().unwrap();
            let meta = index.get_mut(id)?;
            if !meta.read {
                meta.read = true;
                let _ = std::fs::write(
                    self.dir.join(format!("{id}.json")),
                    serde_json::to_vec(meta).unwrap_or_default(),
                );
            }
            meta.clone()
        };
        let parsed = parse::parse(&raw);
        let html = parsed.html.as_deref().map(|h| parse::inline_cids(&raw, h));
        Some(MailDetail {
            summary: MailSummary {
                read: true,
                ..meta.summary
            },
            envelope: meta.envelope,
            headers: parsed.headers,
            cc: parsed.cc,
            reply_to: parsed.reply_to,
            html,
            text: parsed.text,
            text_generated: parsed.text_generated,
            attachments: parsed.attachments,
            links: parsed.links,
            problems: parsed.problems,
            framework: None,
        })
    }

    pub fn delete(&self, ids: &[String]) -> usize {
        let mut index = self.index.lock().unwrap();
        let mut removed = 0;
        for id in ids {
            if index.remove(id).is_some() {
                self.remove_files(id);
                removed += 1;
            }
        }
        removed
    }

    /// Everything, or one site's box.
    pub fn clear(&self, site: Option<&str>) -> usize {
        let ids: Vec<String> = self
            .index
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, m)| site.is_none_or(|s| m.site.as_deref() == Some(s)))
            .map(|(id, _)| id.clone())
            .collect();
        self.delete(&ids)
    }
}
