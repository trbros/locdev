//! A site's folder on disk: deleting it from the app (to the trash), and
//! noticing when it was deleted some other way.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use locdev_core::deps::{NotificationCategory, NotificationLink, Severity};
use locdev_core::model::Project;
use locdev_core::{AppError, ErrorCode};
use tracing::{info, warn};

use super::Daemon;
use crate::notify::New;

/// How often registered folders are checked.
const EVERY: Duration = Duration::from_secs(5);
/// Checks in a row a folder must be missing before its site goes: a folder
/// renamed or moved back within seconds keeps it.
const MISSES: u8 = 2;

/// Places where removable drives and network shares are mounted: a site
/// there that disappears was most likely unmounted, not deleted.
const MOUNT_ROOTS: &[&str] = &["/media", "/run/media", "/mnt", "/Volumes"];

/// Why `path` must not be deleted as a site's folder, if it mustn't: the
/// home folder, a drive's root, LocDev's sites folder itself, or a folder
/// that holds another registered site.
fn refuse_delete(path: &Path, home: &Path, sites_dir: &Path, others: &[PathBuf]) -> Option<String> {
    if path.parent().is_none() || path == home || home.starts_with(path) || path == sites_dir {
        return Some(format!("{} is not a site's own folder", path.display()));
    }
    if let Some(other) = others
        .iter()
        .find(|o| o.as_path() != path && o.starts_with(path))
    {
        return Some(format!(
            "{} also holds another site ({})",
            path.display(),
            other.display()
        ));
    }
    None
}

/// A folder that's gone because it was deleted: its parent is still there
/// (a whole drive or share that went away takes the parent with it) and it
/// isn't under a mount location.
fn deleted(path: &Path) -> bool {
    !path.exists()
        && path.parent().is_some_and(Path::is_dir)
        && !MOUNT_ROOTS.iter().any(|root| path.starts_with(root))
}

impl Daemon {
    /// Moves a site's folder to the trash, after stopping what runs from it.
    /// The caller removes the site afterwards.
    pub(super) async fn trash_site_folder(&self, name: &str) -> Result<(), AppError> {
        let project = self.project(name).await?;
        let path = project.path.clone();
        let others: Vec<PathBuf> = self
            .registry
            .lock()
            .await
            .all()
            .iter()
            .filter(|p| p.name != name)
            .map(|p| p.path.clone())
            .collect();
        if let Some(reason) = refuse_delete(&path, &Self::home_dir(), &Self::sites_dir(), &others) {
            return Err(AppError::invalid(format!(
                "not deleting the folder: {reason}"
            )));
        }
        if !path.is_dir() {
            // Already gone: nothing to move.
            return Ok(());
        }
        // Nothing may hold files in it while it moves (Windows refuses).
        self.tunnel_stop(name).await;
        self.stop_all_of(name).await;
        let target = path.clone();
        tokio::task::spawn_blocking(move || trash::delete(&target))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .map_err(|e| {
                AppError::new(
                    ErrorCode::Io,
                    format!("could not move {} to the trash: {e}", path.display()),
                )
            })?;
        info!(project = %name, path = %path.display(), "site folder moved to the trash");
        Ok(())
    }

    /// Sites whose folder was deleted outside LocDev are stopped and removed,
    /// with a notification. Parked folders follow their own watcher.
    pub async fn watch_site_folders(self: Arc<Self>) {
        let mut misses: HashMap<String, u8> = HashMap::new();
        let mut every = tokio::time::interval(EVERY);
        every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            every.tick().await;
            let projects: Vec<Project> = self.registry.lock().await.all().to_vec();
            misses.retain(|name, _| projects.iter().any(|p| &p.name == name));
            for project in projects {
                if !deleted(&project.path) {
                    misses.remove(&project.name);
                    continue;
                }
                let seen = misses.entry(project.name.clone()).or_insert(0);
                *seen += 1;
                if *seen < MISSES {
                    continue;
                }
                misses.remove(&project.name);
                match self.remove_site(&project.name).await {
                    Ok(()) => {
                        info!(project = %project.name, path = %project.path.display(), "folder deleted: site removed");
                        self.notify(New {
                            category: NotificationCategory::Action,
                            severity: Severity::Info,
                            title: format!("{} was removed", project.domain),
                            body: format!(
                                "Its folder {} was deleted, so LocDev stopped its processes and removed the site.",
                                project.path.display()
                            ),
                            project: None,
                            link: Some(NotificationLink {
                                view: "sites".into(),
                                sub: None,
                                project: None,
                            }),
                            dedup: format!("folder-deleted:{}", project.name),
                        });
                    }
                    Err(error) => {
                        warn!(project = %project.name, %error, "folder deleted, but the site couldn't be removed");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_deletes_home_roots_or_folders_with_other_sites() {
        let home = Path::new("/home/u");
        let sites = Path::new("/home/u/LocDev/Sites");
        let others = vec![PathBuf::from("/home/u/code/api")];
        assert!(refuse_delete(Path::new("/"), home, sites, &others).is_some());
        assert!(refuse_delete(home, home, sites, &others).is_some());
        assert!(refuse_delete(Path::new("/home"), home, sites, &others).is_some());
        assert!(refuse_delete(sites, home, sites, &others).is_some());
        assert!(refuse_delete(Path::new("/home/u/code"), home, sites, &others).is_some());
        assert!(
            refuse_delete(Path::new("/home/u/LocDev/Sites/shop"), home, sites, &others).is_none()
        );
        assert!(
            refuse_delete(
                Path::new("/home/u/code/api"),
                home,
                sites,
                &[PathBuf::from("/home/u/code/api")]
            )
            .is_none()
        );
    }

    #[test]
    fn a_deleted_folder_is_told_from_an_unmounted_drive() {
        let dir = tempfile::tempdir().unwrap();
        let site = dir.path().join("shop");
        std::fs::create_dir(&site).unwrap();
        assert!(!deleted(&site));
        std::fs::remove_dir(&site).unwrap();
        assert!(deleted(&site));
        // The parent went too (a drive or share that went away).
        assert!(!deleted(&dir.path().join("gone/shop")));
        assert!(!deleted(Path::new("/media/u/Drive/shop")));
    }
}
