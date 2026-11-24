//! `env sync`: copying shared variables into one of a project's files, on
//! request. Only lines are appended; nothing already in the file changes.
//! The plan is computed again when it is applied and must match the token
//! the user saw.

use std::path::{Path, PathBuf};

use locdev_core::env::{EntryStatus, EnvBackup, EnvFileRole, SyncLine, SyncPlan, SyncSkip};
use serde::{Deserialize, Serialize};

use crate::dotenv::{self, Document, format_value};
use crate::resolve::Resolution;
use crate::rules::{Scan, classify};
use crate::vault::Vault;
use crate::{hex, write_private};

pub const DEFAULT_FILE: &str = ".env";
const HEADER: &str = "# Shared variables added by LocDev (locdev env sync)";
const BACKUPS_KEPT: usize = 20;

/// Modes whose files never get development secrets.
fn deployed(mode: Option<&str>) -> bool {
    matches!(mode, Some("production" | "prod" | "staging" | "stage"))
}

/// Checks the target file name: a `.env*` at the project's top level.
pub fn check_target(file: &str) -> Result<(EnvFileRole, Option<String>), String> {
    if file.contains(['/', '\\']) || file.contains("..") {
        return Err(format!(
            "{file}: only files at the project's top level can be synced"
        ));
    }
    classify(file)
        .ok_or_else(|| format!("{file} is not a .env file (.env, .env.local, .env.<mode>…)"))
}

/// The plan, plus the bytes to write if it is confirmed.
pub fn plan(
    project: &str,
    root: &Path,
    file: &str,
    resolution: &Resolution,
    values: &std::collections::BTreeMap<(String, String), String>,
    scan: &Scan,
) -> (SyncPlan, Option<Vec<u8>>) {
    let mut out = SyncPlan {
        project: project.into(),
        file: file.into(),
        exists: false,
        add: Vec::new(),
        skip: Vec::new(),
        preview: String::new(),
        token: String::new(),
        refused: None,
    };
    let refuse = |mut out: SyncPlan, why: String| {
        out.refused = Some(why);
        (out, None)
    };
    let (role, mode) = match check_target(file) {
        Ok(v) => v,
        Err(why) => return refuse(out, why),
    };
    let path = root.join(file);
    let (current, doc) = match std::fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return refuse(
                out,
                format!("{file} is a symbolic link; LocDev doesn't write through links"),
            );
        }
        Ok(meta) if !meta.is_file() => return refuse(out, format!("{file} is not a regular file")),
        Ok(_) => match std::fs::read(&path) {
            Ok(bytes) => match dotenv::parse(&bytes) {
                Ok(doc) => (Some(bytes), doc),
                Err(e) => return refuse(out, format!("{file}: {e}")),
            },
            Err(e) => return refuse(out, format!("{file}: {e}")),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (None, Document::default()),
        Err(e) => return refuse(out, format!("{file}: {e}")),
    };
    out.exists = current.is_some();

    let mut lines = vec![HEADER.to_string()];
    let mut preview = vec![HEADER.to_string()];
    for entry in resolution.effective() {
        let key = entry.key.as_str();
        let skip = |reason: String| SyncSkip {
            key: key.into(),
            reason,
        };
        if doc.has(key) {
            out.skip.push(skip(format!("already in {file}")));
            continue;
        }
        if let Some((other, _)) = scan
            .defining(key)
            .into_iter()
            .find(|(f, active)| *active && *f != file)
        {
            out.skip.push(skip(format!(
                "defined in {other}, which the dev server reads"
            )));
            continue;
        }
        // Templates are usually committed: no real value goes in, secret or not.
        let placeholder = role == EnvFileRole::Template;
        if entry.secret && deployed(mode.as_deref()) {
            out.skip.push(skip(format!(
                "secret; development secrets aren't copied into {file}"
            )));
            continue;
        }
        let value = if placeholder {
            String::new()
        } else {
            match values.get(&(entry.profile.clone(), entry.key.clone())) {
                Some(v) => v.clone(),
                None => {
                    out.skip
                        .push(skip("secret, and the vault is locked".into()));
                    continue;
                }
            }
        };
        lines.push(format!("{key}={}", format_value(&value)));
        preview.push(if entry.secret && !placeholder {
            format!("{key}=********")
        } else {
            format!("{key}={}", format_value(&value))
        });
        out.add.push(SyncLine {
            key: key.into(),
            profile: entry.profile.clone(),
            secret: entry.secret,
            placeholder,
        });
    }
    // Keys the process doesn't get at all are worth naming too.
    for entry in resolution
        .entries
        .iter()
        .filter(|e| matches!(e.status, EntryStatus::Disabled | EntryStatus::Blocked))
    {
        out.skip.push(SyncSkip {
            key: entry.key.clone(),
            reason: entry
                .reason
                .clone()
                .unwrap_or_else(|| "not used by this project".into()),
        });
    }

    if out.add.is_empty() {
        out.token = token(current.as_deref(), "");
        return (out, None);
    }
    let bytes = doc.appended(&lines);
    let block = lines.join("\n");
    out.preview = preview.iter().map(|l| format!("+ {l}\n")).collect();
    out.token = token(current.as_deref(), &block);
    (out, Some(bytes))
}

fn token(current: Option<&[u8]>, block: &str) -> String {
    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);
    match current {
        Some(bytes) => {
            ctx.update(b"file\0");
            ctx.update(bytes);
        }
        None => ctx.update(b"absent\0"),
    }
    ctx.update(block.as_bytes());
    hex(&ctx.finish().as_ref()[..16])
}

/// Writes `bytes` over `path` atomically, keeping the file's permissions
/// (a new file is private).
pub fn write_env_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    match std::fs::metadata(path) {
        Ok(meta) => {
            let tmp = tmp_path(path);
            {
                use std::io::Write;
                let mut f = std::fs::File::create(&tmp)?;
                f.set_permissions(meta.permissions())?;
                f.write_all(bytes)?;
                f.sync_all()?;
            }
            std::fs::rename(&tmp, path)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => write_private(path, bytes),
        Err(e) => Err(e),
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("env");
    path.with_file_name(format!(".{name}.locdev-tmp"))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Index {
    backups: Vec<EnvBackup>,
}

/// Encrypted copies of a project's `.env*` files, taken before LocDev
/// writes one. They are sealed with the vault key, like secrets.
pub struct Backups {
    dir: PathBuf,
}

impl Backups {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn project_dir(&self, project: &str) -> PathBuf {
        self.dir.join(project)
    }

    fn index(&self, project: &str) -> Index {
        std::fs::read(self.project_dir(project).join("index.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn list(&self, project: &str) -> Vec<EnvBackup> {
        let mut list = self.index(project).backups;
        list.reverse();
        list
    }

    pub fn take(
        &self,
        vault: &Vault,
        project: &str,
        file: &str,
        bytes: &[u8],
    ) -> Result<EnvBackup, String> {
        let at = locdev_core::model::now_ms();
        let id = format!("{at}-{}", file.trim_start_matches('.').replace('.', "-"));
        let aad = format!("backup/{project}/{id}");
        let sealed = vault.seal(&aad, bytes).map_err(|e| e.0)?;
        let dir = self.project_dir(project);
        create_private_dir(&dir).map_err(|e| e.to_string())?;
        write_private(&dir.join(format!("{id}.bin")), &sealed).map_err(|e| e.to_string())?;
        let backup = EnvBackup {
            id,
            project: project.into(),
            file: file.into(),
            at,
            size: bytes.len() as u64,
        };
        let mut index = self.index(project);
        index.backups.push(backup.clone());
        while index.backups.len() > BACKUPS_KEPT {
            let old = index.backups.remove(0);
            let _ = std::fs::remove_file(dir.join(format!("{}.bin", old.id)));
        }
        let json = serde_json::to_vec_pretty(&index).map_err(|e| e.to_string())?;
        write_private(&dir.join("index.json"), &json).map_err(|e| e.to_string())?;
        Ok(backup)
    }

    pub fn read(
        &self,
        vault: &Vault,
        project: &str,
        id: &str,
    ) -> Result<(EnvBackup, Vec<u8>), String> {
        let backup = self
            .index(project)
            .backups
            .into_iter()
            .find(|b| b.id == id)
            .ok_or_else(|| format!("no backup {id} for {project}"))?;
        let sealed = std::fs::read(self.project_dir(project).join(format!("{id}.bin")))
            .map_err(|e| e.to_string())?;
        let plain = vault
            .open(&format!("backup/{project}/{id}"), &sealed)
            .map_err(|e| e.0)?;
        Ok((backup, plain))
    }

    pub fn forget(&self, project: &str) {
        let _ = std::fs::remove_dir_all(self.project_dir(project));
    }
}

fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::resolve::{Input, resolve};
    use crate::rules::{rules, scan};
    use crate::store::StoredVar;
    use crate::vault::MemoryKey;
    use locdev_core::deps::EnvConflictPolicy;
    use locdev_core::env::ProjectEnvSettings;
    use locdev_core::model::ProjectKind;

    fn var(key: &str, secret: bool) -> StoredVar {
        StoredVar {
            key: key.into(),
            profile: "global".into(),
            secret,
            value: None,
            description: String::new(),
            updated_at: 0,
        }
    }

    fn run(dir: &Path, file: &str, kind: ProjectKind) -> (SyncPlan, Option<Vec<u8>>) {
        let vars = vec![
            var("STRIPE_SECRET_KEY", true),
            var("SENTRY_DSN", false),
            var("APP_NAME", false),
        ];
        let values: BTreeMap<_, _> = [
            (
                ("global".into(), "STRIPE_SECRET_KEY".into()),
                "sk_test_42".to_string(),
            ),
            (
                ("global".into(), "SENTRY_DSN".into()),
                "https://k@sentry.io/1".to_string(),
            ),
            (("global".into(), "APP_NAME".into()), "My Shop".to_string()),
        ]
        .into();
        let r = rules(kind, dir, None);
        let s = scan(dir, &r);
        let res = resolve(&Input {
            settings: &ProjectEnvSettings::default(),
            vars: &vars,
            profiles: &BTreeSet::from(["global".to_string()]),
            values: &values,
            rules: &r,
            scan: &s,
            system: &BTreeSet::new(),
            locdev: &BTreeSet::new(),
            policy: EnvConflictPolicy::ActiveFiles,
        });
        plan("shop", dir, file, &res, &values, &s)
    }

    #[test]
    fn appends_missing_keys_and_masks_preview() {
        let dir = tempfile::tempdir().unwrap();
        let original = "# Laravel\nAPP_NAME=Laravel\nAPP_ENV=local\n";
        std::fs::write(dir.path().join(".env"), original).unwrap();
        let (plan, bytes) = run(dir.path(), ".env", ProjectKind::Laravel);
        assert!(plan.refused.is_none());
        let added: Vec<_> = plan.add.iter().map(|l| l.key.as_str()).collect();
        assert_eq!(added, vec!["SENTRY_DSN", "STRIPE_SECRET_KEY"]);
        assert_eq!(plan.skip[0].key, "APP_NAME");
        assert!(!plan.preview.contains("sk_test_42"));
        assert!(plan.preview.contains("STRIPE_SECRET_KEY=********"));

        let text = String::from_utf8(bytes.unwrap()).unwrap();
        assert!(text.starts_with(original));
        assert!(text.contains("STRIPE_SECRET_KEY=sk_test_42\n"));
        assert!(text.contains("SENTRY_DSN=https://k@sentry.io/1\n"));
    }

    #[test]
    fn example_gets_placeholders_production_no_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let (example, bytes) = run(dir.path(), ".env.example", ProjectKind::Laravel);
        let text = String::from_utf8(bytes.unwrap()).unwrap();
        assert!(text.contains("STRIPE_SECRET_KEY=\n"));
        assert!(!text.contains("sk_test_42"));
        assert!(example.add.iter().all(|l| l.placeholder));
        assert!(text.contains("APP_NAME=\n"));
        assert!(!text.contains("sentry.io"));

        let (prod, bytes) = run(dir.path(), ".env.production", ProjectKind::Laravel);
        assert!(prod.skip.iter().any(|s| s.key == "STRIPE_SECRET_KEY"));
        assert!(
            !String::from_utf8(bytes.unwrap())
                .unwrap()
                .contains("sk_test_42")
        );
    }

    #[test]
    fn active_variant_blocks_the_key() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env.local"), "SENTRY_DSN=mine\n").unwrap();
        let (plan, _) = run(dir.path(), ".env", ProjectKind::Next);
        let skip = plan.skip.iter().find(|s| s.key == "SENTRY_DSN").unwrap();
        assert!(skip.reason.contains(".env.local"));
    }

    #[test]
    fn refuses_bad_targets() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            run(dir.path(), "../.env", ProjectKind::Laravel)
                .0
                .refused
                .is_some()
        );
        assert!(
            run(dir.path(), "config.php", ProjectKind::Laravel)
                .0
                .refused
                .is_some()
        );
        std::fs::write(dir.path().join(".env"), [0xff, 0x00]).unwrap();
        assert!(
            run(dir.path(), ".env", ProjectKind::Laravel)
                .0
                .refused
                .is_some()
        );
        #[cfg(unix)]
        {
            let other = tempfile::tempdir().unwrap();
            std::fs::write(other.path().join("real"), "A=1\n").unwrap();
            std::os::unix::fs::symlink(other.path().join("real"), dir.path().join(".env.local"))
                .unwrap();
            assert!(
                run(dir.path(), ".env.local", ProjectKind::Laravel)
                    .0
                    .refused
                    .is_some()
            );
        }
    }

    #[test]
    fn token_tracks_file_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env"), "A=1\n").unwrap();
        let first = run(dir.path(), ".env", ProjectKind::Laravel).0.token;
        assert_eq!(first, run(dir.path(), ".env", ProjectKind::Laravel).0.token);
        std::fs::write(dir.path().join(".env"), "A=2\n").unwrap();
        assert_ne!(first, run(dir.path(), ".env", ProjectKind::Laravel).0.token);
    }

    #[test]
    fn write_keeps_permissions_and_backups_are_sealed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        std::fs::write(&path, "A=1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        }
        write_env_file(&path, b"A=1\nB=2\n").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"A=1\nB=2\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o640
            );
        }

        let vault =
            Vault::load(&dir.path().join("v.json"), Box::new(MemoryKey::default())).unwrap();
        let backups = Backups::new(dir.path().join("backups"));
        let b = backups
            .take(&vault, "shop", ".env", b"SECRET=sk_live_1\n")
            .unwrap();
        let raw = std::fs::read(
            dir.path()
                .join("backups/shop")
                .join(format!("{}.bin", b.id)),
        )
        .unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("sk_live"));
        assert_eq!(
            backups.read(&vault, "shop", &b.id).unwrap().1,
            b"SECRET=sk_live_1\n"
        );
        assert_eq!(backups.list("shop").len(), 1);
    }
}
