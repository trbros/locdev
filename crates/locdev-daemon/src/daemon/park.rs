//! Parked folders (`locdev park`): their project subfolders become sites and
//! stay in step as projects appear and disappear. The list lives in
//! `config/parked.toml`; scans happen when a folder is parked or changed, on
//! file events (watched without recursion: the folder and the non-project
//! folders under it, never inside a project), and as a recovery rescan from
//! the job loop's minute tick. A site's files are never touched.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use locdev_core::model::{Project, now_ms};
use locdev_core::park::{
    OnMissing, OnNew, ParkError, ParkOptions, ParkedDir, ParkedProject, ParkedState, ParkedView,
};
use locdev_core::paths::plain;
use locdev_core::protocol::{Request, Response};
use locdev_core::{AppError, ErrorCode, project_name, slugify};
use locdev_frameworks::discover::{self, Found};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tracing::{info, warn};

use super::Daemon;
use crate::fsutil::write_atomic;

const FORMAT: u32 = 1;
/// A parked folder is rescanned at least this often, in case an event was
/// missed (network drives, watch limits, the daemon busy).
const RESCAN_MS: u64 = 5 * 60 * 1000;
/// Events are gathered until the folder has been quiet this long, so a
/// project being cloned or generated is scanned once, not per file.
const QUIET: Duration = Duration::from_millis(1500);

#[derive(Serialize, Deserialize)]
struct ParkFile {
    version: u32,
    #[serde(default, rename = "dir")]
    dirs: Vec<ParkedDir>,
}

#[derive(Default, Clone)]
struct Scan {
    at: u64,
    projects: Vec<ParkedProject>,
    errors: Vec<ParkError>,
}

struct Watching {
    watcher: Option<RecommendedWatcher>,
    error: Option<String>,
    /// What each parked folder has watched.
    paths: HashMap<PathBuf, HashSet<PathBuf>>,
}

pub struct ParkState {
    file: PathBuf,
    dirs: Mutex<Vec<ParkedDir>>,
    scans: Mutex<HashMap<PathBuf, Scan>>,
    /// One scan at a time: scans register sites and pick names.
    scanning: tokio::sync::Mutex<()>,
    watching: Mutex<Watching>,
    events: Mutex<Option<UnboundedReceiver<PathBuf>>>,
}

impl ParkState {
    pub fn load(file: PathBuf) -> Result<Self, AppError> {
        let dirs = match std::fs::read_to_string(&file) {
            Ok(raw) => {
                let parsed: ParkFile = toml::from_str(&raw).map_err(|e| {
                    AppError::new(ErrorCode::Config, format!("{}: {e}", file.display()))
                })?;
                if parsed.version > FORMAT {
                    return Err(AppError::new(
                        ErrorCode::Config,
                        format!(
                            "{}: written by a newer LocDev (format {})",
                            file.display(),
                            parsed.version
                        ),
                    ));
                }
                parsed.dirs
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e.into()),
        };
        let dirs = dirs
            .into_iter()
            .map(|mut dir: ParkedDir| {
                dir.path = plain(dir.path);
                dir
            })
            .collect();
        let (sender, receiver) = unbounded_channel();
        let (watcher, error) = match new_watcher(sender) {
            Ok(watcher) => (Some(watcher), None),
            Err(error) => {
                warn!(%error, "file watching unavailable; parked folders are rescanned periodically");
                (None, Some(error.to_string()))
            }
        };
        Ok(Self {
            file,
            dirs: Mutex::new(dirs),
            scans: Mutex::default(),
            scanning: tokio::sync::Mutex::new(()),
            watching: Mutex::new(Watching {
                watcher,
                error,
                paths: HashMap::new(),
            }),
            events: Mutex::new(Some(receiver)),
        })
    }

    fn save(&self, dirs: &[ParkedDir]) -> Result<(), AppError> {
        let file = ParkFile {
            version: FORMAT,
            dirs: dirs.to_vec(),
        };
        let raw = toml::to_string_pretty(&file).map_err(|e| AppError::internal(e.to_string()))?;
        write_atomic(&self.file, raw.as_bytes())
    }

    fn get(&self, root: &Path) -> Option<ParkedDir> {
        self.dirs
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.path == root)
            .cloned()
    }

    fn update(&self, root: &Path, change: impl FnOnce(&mut ParkedDir)) -> Result<(), AppError> {
        let mut dirs = self.dirs.lock().unwrap();
        let dir = dirs
            .iter_mut()
            .find(|d| d.path == root)
            .ok_or_else(|| not_parked(root))?;
        change(dir);
        self.save(&dirs)
    }

    /// Watches exactly `paths` for `root` (none when it's off or gone).
    fn rewatch(&self, root: &Path, paths: HashSet<PathBuf>) {
        let mut state = self.watching.lock().unwrap();
        let Watching {
            watcher,
            error,
            paths: watched,
        } = &mut *state;
        let Some(watcher) = watcher else { return };
        let before = watched.remove(root).unwrap_or_default();
        for path in before.difference(&paths) {
            let _ = watcher.unwatch(path);
        }
        let mut now = HashSet::new();
        for path in paths {
            if before.contains(&path) {
                now.insert(path);
                continue;
            }
            match watcher.watch(&path, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    now.insert(path);
                }
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "could not watch a parked folder");
                    *error = Some(e.to_string());
                }
            }
        }
        if !now.is_empty() {
            watched.insert(root.to_path_buf(), now);
        }
    }

    fn view(&self, dir: &ParkedDir) -> ParkedView {
        let scan = self
            .scans
            .lock()
            .unwrap()
            .get(&dir.path)
            .cloned()
            .unwrap_or_default();
        let watching = self.watching.lock().unwrap();
        ParkedView {
            dir: dir.clone(),
            exists: dir.path.is_dir(),
            watching: dir.enabled && watching.paths.contains_key(&dir.path),
            watch_error: dir.enabled.then(|| watching.error.clone()).flatten(),
            last_scan: (scan.at > 0).then_some(scan.at),
            projects: scan.projects,
            errors: scan.errors,
        }
    }

    fn views(&self) -> Vec<ParkedView> {
        let dirs = self.dirs.lock().unwrap().clone();
        dirs.iter().map(|d| self.view(d)).collect()
    }
}

fn new_watcher(sender: UnboundedSender<PathBuf>) -> notify::Result<RecommendedWatcher> {
    notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let Ok(event) = event else { return };
        if matches!(event.kind, EventKind::Access(_)) {
            return;
        }
        for path in event.paths {
            let _ = sender.send(plain(path));
        }
    })
}

fn not_parked(path: &Path) -> AppError {
    AppError::not_found(format!("{} is not a parked folder", path.display()))
}

fn relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The canonical form of a path the user typed (or the path as given when
/// it no longer exists, so a vanished folder can still be removed).
fn canonical(path: &Path) -> Result<PathBuf, AppError> {
    if !path.is_absolute() {
        return Err(AppError::invalid("the folder's path must be absolute"));
    }
    Ok(plain(
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
    ))
}

/// The site name for a found project: its folder's name, or, when taken,
/// the folder's name with its parent's, or a number.
fn free_name(found: &Found, root: &Path, taken: &dyn Fn(&str) -> bool) -> Result<String, String> {
    if found.name.is_empty() || project_name(&found.name).is_err() {
        return Err(format!("'{}' can't be turned into a site name", found.rel));
    }
    let parent = match found.rel.rsplit_once('/') {
        Some((parent, _)) => slugify(parent.rsplit('/').next().unwrap_or(parent)),
        None => root
            .file_name()
            .map(|n| slugify(&n.to_string_lossy()))
            .unwrap_or_default(),
    };
    let mut candidates = vec![found.name.clone()];
    if !parent.is_empty() {
        candidates.push(format!("{}-{parent}", found.name));
    }
    candidates.extend((2..=9).map(|n| format!("{}-{n}", found.name)));
    candidates
        .into_iter()
        .filter_map(|c| project_name(&c).ok())
        .find(|c| !taken(c))
        .ok_or_else(|| format!("no free site name for '{}'", found.rel))
}

impl Daemon {
    pub(super) async fn park_request(&self, request: Request) -> Result<Response, AppError> {
        let (added, removed) = match request {
            Request::ParkList => (Vec::new(), Vec::new()),
            Request::ParkSet { path, options } => self.park_set(&path, &options).await?,
            Request::ParkRemove { path, keep_sites } => {
                (Vec::new(), self.park_remove(&path, keep_sites).await?)
            }
            Request::ParkScan { path } => {
                let roots = match path {
                    Some(path) => {
                        let root = canonical(&path)?;
                        let dir = self.park.get(&root).ok_or_else(|| not_parked(&root))?;
                        if !dir.enabled {
                            return Err(AppError::invalid(format!(
                                "{} is turned off; turn it on to scan it",
                                root.display()
                            )));
                        }
                        vec![root]
                    }
                    None => self.park_enabled_roots(),
                };
                let mut added = Vec::new();
                let mut removed = Vec::new();
                for root in roots {
                    let (a, r) = self.park_scan(&root).await;
                    added.extend(a);
                    removed.extend(r);
                }
                (added, removed)
            }
            Request::ParkIgnore { path, ignore } => self.park_ignore(&path, ignore).await?,
            _ => unreachable!("not a park request"),
        };
        Ok(Response::Parked {
            dirs: self.park.views(),
            added,
            removed,
        })
    }

    fn park_enabled_roots(&self) -> Vec<PathBuf> {
        self.park
            .dirs
            .lock()
            .unwrap()
            .iter()
            .filter(|d| d.enabled)
            .map(|d| d.path.clone())
            .collect()
    }

    async fn park_set(
        &self,
        path: &Path,
        options: &ParkOptions,
    ) -> Result<(Vec<String>, Vec<String>), AppError> {
        let root = canonical(path)?;
        let existing = self.park.get(&root);
        let mut dir = match existing.clone() {
            Some(dir) => dir,
            None => {
                if !root.is_dir() {
                    return Err(AppError::invalid(format!(
                        "{} is not a folder",
                        root.display()
                    )));
                }
                if root.parent().is_none() {
                    return Err(AppError::invalid(
                        "the root of a drive can't be parked; park the folder that holds your projects",
                    ));
                }
                if let Some(framework) = discover::is_project(&self.catalog(), &root) {
                    return Err(AppError::invalid(format!(
                        "{} is itself a project ({framework}); add it as a site with `locdev add`, or park the folder that holds it",
                        root.display()
                    )));
                }
                let dirs = self.park.dirs.lock().unwrap().clone();
                if let Some(other) = dirs
                    .iter()
                    .find(|d| root.starts_with(&d.path) || d.path.starts_with(&root))
                {
                    return Err(AppError::new(
                        ErrorCode::AlreadyExists,
                        format!(
                            "{} overlaps the parked folder {}; a folder can only be found through one of them",
                            root.display(),
                            other.path.display()
                        ),
                    ));
                }
                ParkedDir::new(root.clone())
            }
        };
        options.apply(&mut dir).map_err(AppError::invalid)?;
        {
            let mut dirs = self.park.dirs.lock().unwrap();
            match dirs.iter_mut().find(|d| d.path == root) {
                Some(slot) => *slot = dir.clone(),
                None => dirs.push(dir.clone()),
            }
            dirs.sort_by(|a, b| a.path.cmp(&b.path));
            self.park.save(&dirs)?;
        }
        if existing.is_none() {
            info!(path = %root.display(), "folder parked");
        }
        if dir.enabled {
            Ok(self.park_scan(&root).await)
        } else {
            self.park.rewatch(&root, HashSet::new());
            Ok((Vec::new(), Vec::new()))
        }
    }

    async fn park_remove(&self, path: &Path, keep_sites: bool) -> Result<Vec<String>, AppError> {
        let root = canonical(path)?;
        let _scan = self.park.scanning.lock().await;
        {
            let mut dirs = self.park.dirs.lock().unwrap();
            let before = dirs.len();
            dirs.retain(|d| d.path != root);
            if dirs.len() == before {
                return Err(not_parked(&root));
            }
            self.park.save(&dirs)?;
        }
        self.park.rewatch(&root, HashSet::new());
        self.park.scans.lock().unwrap().remove(&root);

        let sites: Vec<String> = {
            let mut registry = self.registry.lock().await;
            let names: Vec<String> = registry
                .all()
                .iter()
                .filter(|p| p.parked.as_deref() == Some(root.as_path()))
                .map(|p| p.name.clone())
                .collect();
            if keep_sites {
                for name in &names {
                    registry.get_mut(name)?.parked = None;
                }
                self.commit(&registry)?;
                return Ok(Vec::new());
            }
            names
        };
        for name in &sites {
            self.park_unregister(name).await?;
        }
        info!(path = %root.display(), sites = sites.len(), "folder unparked");
        Ok(sites)
    }

    async fn park_ignore(
        &self,
        path: &Path,
        ignore: bool,
    ) -> Result<(Vec<String>, Vec<String>), AppError> {
        let target = canonical(path)?;
        let dirs = self.park.dirs.lock().unwrap().clone();
        let (root, rel) = dirs
            .iter()
            .find_map(|d| relative(&d.path, &target).map(|rel| (d.path.clone(), rel)))
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "{} is not inside a parked folder",
                    target.display()
                ))
            })?;
        self.park.update(&root, |dir| {
            dir.ignored.retain(|r| *r != rel);
            if ignore {
                dir.ignored.push(rel.clone());
                dir.ignored.sort();
            }
        })?;

        let mut removed = Vec::new();
        if ignore {
            let site = self
                .registry
                .lock()
                .await
                .all()
                .iter()
                .find(|p| p.path == target && p.parked.as_deref() == Some(root.as_path()))
                .map(|p| p.name.clone());
            if let Some(name) = site {
                self.park_unregister(&name).await?;
                removed.push(name);
            }
        }
        let (added, more) = self.park_scan(&root).await;
        removed.extend(more);
        Ok((added, removed))
    }

    /// Removes a site that came from a parked folder (never its files).
    async fn park_unregister(&self, name: &str) -> Result<(), AppError> {
        self.stop_all_of(name).await;
        let mut registry = self.registry.lock().await;
        registry.remove(name)?;
        self.commit(&registry)?;
        drop(registry);
        self.supervisor.forget(name);
        self.env_forget(name);
        info!(project = %name, "parked site removed");
        Ok(())
    }

    /// A site removed by hand that came from a parked folder would come
    /// back on the next scan; it's ignored there instead.
    pub(super) fn park_forget_site(&self, project: &Project) {
        let Some(root) = &project.parked else { return };
        if !project.path.exists() {
            return;
        }
        let Some(rel) = relative(root, &project.path) else {
            return;
        };
        let result = self.park.update(root, |dir| {
            if !dir.ignored.contains(&rel) {
                dir.ignored.push(rel.clone());
                dir.ignored.sort();
            }
        });
        if let Err(error) = result
            && error.code != ErrorCode::NotFound
        {
            warn!(%error, "could not record the removed site as ignored");
        }
    }

    /// Scans one parked folder: registers new projects (or suggests them),
    /// applies the folder's rule to sites whose folder is gone, and watches
    /// where new projects can appear. Returns the sites added and removed.
    pub(super) async fn park_scan(&self, root: &Path) -> (Vec<String>, Vec<String>) {
        let _scan = self.park.scanning.lock().await;
        let Some(dir) = self.park.get(root).filter(|d| d.enabled) else {
            return (Vec::new(), Vec::new());
        };
        if !root.is_dir() {
            // An unplugged drive or a renamed folder: nothing is removed,
            // so its sites come back as they were.
            self.park.rewatch(root, HashSet::new());
            self.park.scans.lock().unwrap().insert(
                root.to_path_buf(),
                Scan {
                    at: now_ms(),
                    projects: Vec::new(),
                    errors: vec![ParkError {
                        path: root.to_path_buf(),
                        message: "the folder doesn't exist (its sites are kept)".into(),
                    }],
                },
            );
            return (Vec::new(), Vec::new());
        }

        let catalog = self.catalog();
        let (owned_root, depth, exclude) = (root.to_path_buf(), dir.depth, dir.exclude.clone());
        let discovery = tokio::task::spawn_blocking(move || {
            discover::discover(&catalog, &owned_root, depth, &exclude)
        })
        .await
        .unwrap_or_default();

        let mut added = Vec::new();
        let mut missing = Vec::new();
        let mut projects = Vec::new();
        let mut errors: Vec<ParkError> = discovery
            .errors
            .iter()
            .map(|(path, message)| ParkError {
                path: path.clone(),
                message: message.clone(),
            })
            .collect();
        {
            let mut registry = self.registry.lock().await;
            let mut changed = false;
            for found in &discovery.projects {
                let mut item = ParkedProject {
                    path: found.path.clone(),
                    rel: found.rel.clone(),
                    name: found.name.clone(),
                    kind: found.kind,
                    framework: found.framework.clone(),
                    state: ParkedState::Suggested,
                    note: None,
                };
                if let Some(site) = registry.all().iter().find(|p| p.path == found.path) {
                    item.name = site.name.clone();
                    item.state = if site.parked.as_deref() == Some(root) {
                        ParkedState::Registered
                    } else {
                        ParkedState::Manual
                    };
                    projects.push(item);
                    continue;
                }
                if dir.ignored.contains(&found.rel) {
                    item.state = ParkedState::Ignored;
                    projects.push(item);
                    continue;
                }
                let name = free_name(found, root, &|n| registry.get(n).is_ok());
                match name {
                    Err(note) => {
                        item.state = ParkedState::Failed;
                        item.note = Some(note);
                    }
                    Ok(name) => {
                        if name != found.name {
                            item.note = Some(format!("'{}' was already taken", found.name));
                        }
                        item.name = name.clone();
                        if dir.on_new == OnNew::Register {
                            let made = self.new_project(
                                &registry,
                                found.path.clone(),
                                Some(name.clone()),
                                None,
                                None,
                                false,
                            );
                            match made.and_then(|mut project| {
                                project.parked = Some(root.to_path_buf());
                                registry.insert(project)
                            }) {
                                Ok(()) => {
                                    item.state = ParkedState::Registered;
                                    changed = true;
                                    added.push(name);
                                }
                                Err(error) => {
                                    item.state = ParkedState::Failed;
                                    item.note = Some(error.message);
                                }
                            }
                        }
                    }
                }
                projects.push(item);
            }
            if changed && let Err(error) = self.commit(&registry) {
                warn!(%error, "could not save the sites found in a parked folder");
                errors.push(ParkError {
                    path: root.to_path_buf(),
                    message: format!("could not save the new sites: {}", error.message),
                });
            }
            if dir.on_missing == OnMissing::Unregister {
                missing = registry
                    .all()
                    .iter()
                    .filter(|p| p.parked.as_deref() == Some(root) && !p.path.exists())
                    .map(|p| p.name.clone())
                    .collect();
            }
        }

        let mut removed = Vec::new();
        for name in missing {
            match self.park_unregister(&name).await {
                Ok(()) => removed.push(name),
                Err(error) => warn!(%error, site = %name, "could not remove a parked site"),
            }
        }
        if !added.is_empty() {
            info!(path = %root.display(), sites = ?added, "sites found in a parked folder");
        }

        self.park
            .rewatch(root, discovery.searched.into_iter().collect());
        self.park.scans.lock().unwrap().insert(
            root.to_path_buf(),
            Scan {
                at: now_ms(),
                projects,
                errors,
            },
        );
        (added, removed)
    }

    /// Rescans folders whose last scan is older than `RESCAN_MS` (called
    /// from the job loop's minute tick).
    pub(super) async fn park_rescan_stale(self: Arc<Self>) {
        let now = now_ms();
        let stale: Vec<PathBuf> = {
            let scans = self.park.scans.lock().unwrap();
            self.park_enabled_roots()
                .into_iter()
                .filter(|root| {
                    scans
                        .get(root)
                        .is_none_or(|s| now.saturating_sub(s.at) >= RESCAN_MS)
                })
                .collect()
        };
        for root in stale {
            self.park_scan(&root).await;
        }
    }

    /// Scans every parked folder once, then rescans the ones file events
    /// point at, after they've been quiet for a moment.
    pub async fn watch_parked(self: Arc<Self>) {
        let Some(mut events) = self.park.events.lock().unwrap().take() else {
            return;
        };
        for root in self.park_enabled_roots() {
            self.park_scan(&root).await;
        }
        while let Some(first) = events.recv().await {
            let mut paths = vec![first];
            loop {
                match tokio::time::timeout(QUIET, events.recv()).await {
                    Ok(Some(path)) => paths.push(path),
                    Ok(None) => return,
                    Err(_) => break,
                }
            }
            let roots: HashSet<PathBuf> = self
                .park_enabled_roots()
                .into_iter()
                .filter(|root| paths.iter().any(|p| p.starts_with(root)))
                .collect();
            for root in roots {
                self.park_scan(&root).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use locdev_core::model::ProjectKind;

    fn found(rel: &str) -> Found {
        let name = slugify(rel.rsplit('/').next().unwrap());
        Found {
            path: PathBuf::from("/dev").join(rel),
            rel: rel.into(),
            name,
            kind: ProjectKind::Node,
            framework: "Node.js".into(),
        }
    }

    #[test]
    fn loads_legacy_windows_parked_roots_in_plain_form() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("parked.toml");
        let data = ParkFile {
            version: FORMAT,
            dirs: vec![ParkedDir::new(r"\\?\C:\dev".into())],
        };
        std::fs::write(&file, toml::to_string(&data).unwrap()).unwrap();
        let state = ParkState::load(file).unwrap();
        assert!(state.get(Path::new(r"C:\dev")).is_some());
    }

    #[test]
    fn names_avoid_taken_ones() {
        let root = Path::new("/home/me/Development");
        let free = |_: &str| false;
        assert_eq!(free_name(&found("shop"), root, &free).unwrap(), "shop");

        let shop_taken = |n: &str| n == "shop";
        assert_eq!(
            free_name(&found("shop"), root, &shop_taken).unwrap(),
            "shop-development"
        );
        assert_eq!(
            free_name(&found("clients/shop"), root, &shop_taken).unwrap(),
            "shop-clients"
        );
        let both = |n: &str| n == "shop" || n == "shop-clients";
        assert_eq!(
            free_name(&found("clients/shop"), root, &both).unwrap(),
            "shop-2"
        );
        let all = |_: &str| true;
        assert!(free_name(&found("shop"), root, &all).is_err());
        assert!(free_name(&found("___"), root, &free).is_err());
    }

    #[test]
    fn relative_paths() {
        let root = Path::new("/a/b");
        assert_eq!(
            relative(root, Path::new("/a/b/c/d")).as_deref(),
            Some("c/d")
        );
        assert_eq!(relative(root, Path::new("/a/b")), None);
        assert_eq!(relative(root, Path::new("/a/x")), None);
    }

    #[test]
    fn file_round_trip_and_newer_format() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("parked.toml");
        let state = ParkState::load(file.clone()).unwrap();
        let mut parked = ParkedDir::new("/srv/dev".into());
        parked.on_new = OnNew::Suggest;
        parked.ignored = vec!["old".into()];
        state.save(std::slice::from_ref(&parked)).unwrap();
        let loaded = ParkState::load(file.clone()).unwrap();
        assert_eq!(loaded.get(Path::new("/srv/dev")), Some(parked));

        std::fs::write(&file, "version = 9\n").unwrap();
        assert!(ParkState::load(file).is_err());
    }
}
