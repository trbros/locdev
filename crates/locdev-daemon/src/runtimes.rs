//! Runtime and package-manager versions for projects: managed installs,
//! detected system ones, global defaults (`runtimes.toml`), per-project pins
//! and the versions a project's own files ask for (`.nvmrc`, `engines`…).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use locdev_core::detect::requirements;
use locdev_core::model::{
    InstallJob, JobState, Project, Release, Requirement, ResolvedRuntime, RuntimeCategory,
    RuntimeInfo, RuntimeKind, RuntimeSource, RuntimeUpdate, RuntimeVersion, today,
};
use locdev_core::semver::{self, Range, Version};
use locdev_core::{AppError, ErrorCode};
use locdev_runtimes::lifecycle::Lifecycle;
use locdev_runtimes::{PhpSource, Store, version};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::fsutil::write_atomic;

const STATE_VERSION: u32 = 1;
const CATALOG_TTL: Duration = Duration::from_secs(3600);
/// Project version files are re-read at most this often.
const REQUIREMENTS_TTL: Duration = Duration::from_secs(3);
/// Keep install logs short; they describe steps, not output.
const JOB_LOG_LINES: usize = 50;

#[derive(Debug, Default, Serialize, Deserialize)]
struct StateFile {
    version: u32,
    /// Global default pin per runtime ("node" → "22").
    #[serde(default)]
    defaults: BTreeMap<RuntimeKind, String>,
}

pub struct Runtimes {
    store: Arc<Store>,
    /// Shims of the defaults for the user's terminal (`<data>/bin`).
    terminal_dir: PathBuf,
    state_path: PathBuf,
    lifecycle_path: PathBuf,
    defaults: Mutex<BTreeMap<RuntimeKind, String>>,
    /// Detected system versions; refreshed on demand (detection runs them).
    system: Mutex<HashMap<RuntimeKind, Vec<RuntimeVersion>>>,
    catalog: Mutex<HashMap<RuntimeKind, (Instant, Vec<Release>)>>,
    jobs: Mutex<HashMap<RuntimeKind, InstallJob>>,
    lifecycle: Mutex<Lifecycle>,
    requirements: Mutex<HashMap<PathBuf, (Instant, Vec<Requirement>)>>,
    /// Set once the first system scan finished; until then system versions
    /// are unknown, not absent.
    detected: tokio::sync::watch::Sender<bool>,
}

impl Runtimes {
    /// `php` is where PHP builds are downloaded from.
    pub fn load(data_dir: &Path, config_dir: &Path, php: PhpSource) -> Result<Self, AppError> {
        let state_path = config_dir.join("runtimes.toml");
        let state = match std::fs::read_to_string(&state_path) {
            Ok(raw) => toml::from_str::<StateFile>(&raw).map_err(|e| {
                AppError::new(ErrorCode::Config, format!("{}: {e}", state_path.display()))
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => StateFile::default(),
            Err(e) => return Err(e.into()),
        };
        if state.version > STATE_VERSION {
            return Err(AppError::new(
                ErrorCode::Config,
                format!("{}: written by a newer LocDev", state_path.display()),
            ));
        }

        // The cached schedule wins over the bundled one only when newer.
        let lifecycle_path = data_dir.join("runtimes").join("lifecycle.json");
        let bundled = Lifecycle::bundled();
        let lifecycle = match Lifecycle::load(&lifecycle_path) {
            Some(cached) if cached.fetched > bundled.fetched => cached,
            _ => bundled,
        };

        Ok(Self {
            store: Arc::new(Store::new(data_dir.join("runtimes"), php)),
            terminal_dir: data_dir.join("bin"),
            state_path,
            lifecycle_path,
            defaults: Mutex::new(state.defaults),
            system: Mutex::default(),
            catalog: Mutex::default(),
            jobs: Mutex::default(),
            lifecycle: Mutex::new(lifecycle),
            requirements: Mutex::default(),
            detected: tokio::sync::watch::Sender::new(false),
        })
    }

    /// Re-detects runtimes already on the machine (runs each candidate once).
    pub async fn refresh_system(&self) {
        let found = tokio::task::spawn_blocking(|| {
            RuntimeKind::ALL
                .into_iter()
                .map(|kind| (kind, locdev_runtimes::system(kind)))
                .collect::<HashMap<_, _>>()
        })
        .await
        .unwrap_or_default();
        *self.system.lock().unwrap() = found;
        self.detected.send_replace(true);
        self.sync_terminal();
    }

    pub fn terminal_dir(&self) -> &Path {
        &self.terminal_dir
    }

    /// Rewrites the terminal shims from the defaults: one per default that
    /// resolves (plus npm/npx/corepack with Node.js). Kinds without a
    /// default get no shim, so the terminal finds the system's own.
    pub fn sync_terminal(&self) -> Vec<String> {
        let defaults = self.defaults.lock().unwrap().clone();
        let mut shims = Vec::new();
        for (kind, pin) in defaults {
            let Some(version) = self.resolve(kind, &pin) else {
                continue;
            };
            if kind == RuntimeKind::Node {
                shims.extend(locdev_runtimes::terminal::node_companions(
                    &version.executable,
                ));
            }
            shims.push(locdev_runtimes::terminal::Shim {
                name: kind.executable().to_string(),
                target: version.executable,
            });
        }
        match locdev_runtimes::terminal::sync(&self.terminal_dir, &shims) {
            Ok(written) => written,
            Err(error) => {
                warn!(%error, dir = %self.terminal_dir.display(), "terminal shims not written");
                Vec::new()
            }
        }
    }

    /// Waits (bounded) for the first system scan, so a start right after
    /// the daemon boots doesn't mistake "not scanned yet" for "missing".
    pub async fn wait_detected(&self) {
        let mut rx = self.detected.subscribe();
        let _ = tokio::time::timeout(Duration::from_secs(15), rx.wait_for(|done| *done)).await;
    }

    /// Re-downloads the support schedules when the copy is older than
    /// `max_age_days`. Failures keep the current data.
    pub async fn refresh_lifecycle(&self, max_age_days: u64) {
        let stale = {
            let current = self.lifecycle.lock().unwrap();
            let cutoff = locdev_core::model::civil_date(
                locdev_core::model::now_ms().saturating_sub(max_age_days * 86_400_000),
            );
            current.fetched <= cutoff
        };
        if !stale {
            return;
        }
        let path = self.lifecycle_path.clone();
        let fetched = tokio::task::spawn_blocking(move || {
            let data = Lifecycle::fetch(&today())?;
            let _ = data.save(&path);
            Ok::<_, locdev_runtimes::RuntimeError>(data)
        })
        .await;
        match fetched {
            Ok(Ok(data)) => {
                info!(fetched = %data.fetched, "runtime support schedules refreshed");
                *self.lifecycle.lock().unwrap() = data;
            }
            Ok(Err(error)) => warn!(%error, "could not refresh runtime support schedules"),
            Err(error) => warn!(%error, "support schedule refresh panicked"),
        }
    }

    /// Managed and system versions, newest first; a managed install wins
    /// over a system one with the same version.
    pub fn versions(&self, kind: RuntimeKind) -> Vec<RuntimeVersion> {
        let mut all = self.store.installed(kind);
        let system = self
            .system
            .lock()
            .unwrap()
            .get(&kind)
            .cloned()
            .unwrap_or_default();
        let bundled = if kind == RuntimeKind::Npm {
            self.bundled_npm()
        } else {
            Vec::new()
        };
        for candidate in system.into_iter().chain(bundled) {
            if !all.iter().any(|v| v.version == candidate.version) {
                all.push(candidate);
            }
        }
        all.sort_by(|a, b| version::compare(&b.version, &a.version));

        let lifecycle = self.lifecycle.lock().unwrap();
        let day = today();
        for v in &mut all {
            v.support = lifecycle.support(kind, &v.version, &day);
        }
        all
    }

    /// npm inside each managed Node.js (read from its package.json, nothing
    /// is run). It follows its Node.js, so it is listed but never pinned.
    fn bundled_npm(&self) -> Vec<RuntimeVersion> {
        self.store
            .installed(RuntimeKind::Node)
            .into_iter()
            .filter_map(|node| {
                let bin = node.executable.parent()?;
                let (package, npm) = if cfg!(windows) {
                    (
                        bin.join("node_modules/npm/package.json"),
                        bin.join("npm.cmd"),
                    )
                } else {
                    (
                        bin.parent()?.join("lib/node_modules/npm/package.json"),
                        bin.join("npm"),
                    )
                };
                let raw = std::fs::read_to_string(package).ok()?;
                let version = serde_json::from_str::<serde_json::Value>(&raw).ok()?["version"]
                    .as_str()?
                    .to_string();
                Some(RuntimeVersion {
                    version,
                    source: RuntimeSource::Managed,
                    executable: npm,
                    origin: format!("node {}", node.version),
                    support: None,
                })
            })
            .collect()
    }

    pub fn info(&self, projects: &[Project]) -> Vec<RuntimeInfo> {
        let defaults = self.defaults.lock().unwrap().clone();
        let jobs = self.jobs.lock().unwrap().clone();

        let mut usage: HashMap<RuntimeKind, BTreeMap<String, Vec<String>>> = HashMap::new();
        for project in projects {
            for resolved in self.for_project(project) {
                if !uses(project, &resolved) {
                    continue;
                }
                if let Some(v) = resolved.version {
                    usage
                        .entry(resolved.kind)
                        .or_default()
                        .entry(v.version)
                        .or_default()
                        .push(project.name.clone());
                }
            }
        }

        RuntimeKind::ALL
            .into_iter()
            .map(|kind| {
                let versions = self.versions(kind);
                RuntimeInfo {
                    kind,
                    default: defaults.get(&kind).cloned(),
                    updates: self.updates(kind, &versions),
                    versions,
                    job: jobs.get(&kind).cloned(),
                    downloads: self.store.downloads(kind),
                    category: kind.category(),
                    usage: usage.remove(&kind).unwrap_or_default(),
                }
            })
            .collect()
    }

    /// Newer releases of each managed line, from the catalog cache only (no
    /// network on a status call).
    fn updates(&self, kind: RuntimeKind, versions: &[RuntimeVersion]) -> Vec<RuntimeUpdate> {
        let catalog = self.catalog.lock().unwrap();
        let Some((_, releases)) = catalog.get(&kind) else {
            return Vec::new();
        };
        let mut newest_per_line: BTreeMap<String, &str> = BTreeMap::new();
        for v in versions
            .iter()
            .filter(|v| v.source == RuntimeSource::Managed)
        {
            let line = release_line(kind, &v.version);
            let entry = newest_per_line.entry(line).or_insert(&v.version);
            if version::compare(&v.version, entry).is_gt() {
                *entry = &v.version;
            }
        }
        newest_per_line
            .into_iter()
            .filter_map(|(line, installed)| {
                let latest = version::best(&line, releases, |r| r.version.as_str())?;
                version::compare(&latest.version, installed)
                    .is_gt()
                    .then(|| RuntimeUpdate {
                        line,
                        installed: installed.to_string(),
                        latest: latest.version.clone(),
                    })
            })
            .collect()
    }

    /// The version a bare `node`/`php` finds on the daemon's PATH: what a
    /// site without any pin or default runs with.
    pub fn on_path(&self, kind: RuntimeKind) -> Option<RuntimeVersion> {
        let versions = self.versions(kind);
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path).find_map(|dir| {
            versions
                .iter()
                .find(|v| {
                    v.executable.parent() == Some(dir.as_path())
                        && v.executable
                            .file_stem()
                            .is_some_and(|stem| stem == kind.executable())
                })
                .cloned()
        })
    }

    pub fn resolve(&self, kind: RuntimeKind, pin: &str) -> Option<RuntimeVersion> {
        let versions = self.versions(kind);
        self.pick(kind, pin, &versions)
    }

    /// The newest of `versions` matching a pin or a version-file entry
    /// (`lts/*`, `lts/iron` and `node` too); on a tie the managed one wins.
    fn pick(
        &self,
        kind: RuntimeKind,
        spec: &str,
        versions: &[RuntimeVersion],
    ) -> Option<RuntimeVersion> {
        let spec = spec.trim();
        let best = if kind == RuntimeKind::Node && spec.to_ascii_lowercase().starts_with("lts/") {
            let lifecycle = self.lifecycle.lock().unwrap();
            let codename = &spec[4..];
            let majors: Vec<u64> = if codename == "*" {
                lifecycle.lts_majors()
            } else {
                lifecycle.codename_major(codename).into_iter().collect()
            };
            versions
                .iter()
                .filter(|v| {
                    version::parts(&v.version)
                        .first()
                        .is_some_and(|m| majors.contains(m))
                })
                .max_by(|a, b| version::compare(&a.version, &b.version))
        } else if matches!(spec, "node" | "latest" | "current" | "stable" | "*") {
            versions.first()
        } else {
            version::best(spec, versions, |v| v.version.as_str())
        }?;
        versions
            .iter()
            .find(|v| v.version == best.version && v.source == RuntimeSource::Managed)
            .or(Some(best))
            .cloned()
    }

    fn project_requirements(&self, path: &Path) -> Vec<Requirement> {
        let mut cache = self.requirements.lock().unwrap();
        if let Some((at, found)) = cache.get(path)
            && at.elapsed() < REQUIREMENTS_TTL
        {
            return found.clone();
        }
        let found = requirements(path);
        cache.insert(path.to_path_buf(), (Instant::now(), found.clone()));
        found
    }

    /// What the project's files ask for (cached for a few seconds).
    pub fn requirements_of(&self, project: &Project) -> Vec<Requirement> {
        self.project_requirements(&project.path)
    }

    /// The versions a project runs with. Per kind, the first that applies:
    /// its own pin, a version file (`.nvmrc`…), the newest installed version
    /// in a declared range (`engines`, `require.php`) unless the default
    /// already fits, then the global default.
    pub fn for_project(&self, project: &Project) -> Vec<ResolvedRuntime> {
        let defaults = self.defaults.lock().unwrap().clone();
        let reqs = self.project_requirements(&project.path);
        RuntimeKind::ALL
            .into_iter()
            .filter(|kind| kind.pinnable())
            .filter_map(|kind| self.resolve_kind(project, kind, &reqs, defaults.get(&kind)))
            .collect()
    }

    fn resolve_kind(
        &self,
        project: &Project,
        kind: RuntimeKind,
        reqs: &[Requirement],
        default: Option<&String>,
    ) -> Option<ResolvedRuntime> {
        let range_req = reqs.iter().find(|r| r.kind == kind && !r.exact);
        let range = range_req.and_then(|r| parse_range(kind, &r.constraint));
        let versions = self.versions(kind);

        let mut resolved = if let Some(pin) = project.pin(kind) {
            ResolvedRuntime {
                kind,
                requested: pin.to_string(),
                pinned: true,
                version: self.pick(kind, pin, &versions),
                from: "projects.toml".into(),
                unmet: None,
            }
        } else if let Some(req) = reqs.iter().find(|r| r.kind == kind && r.exact) {
            ResolvedRuntime {
                kind,
                requested: req.constraint.clone(),
                pinned: true,
                version: self.pick(kind, &req.constraint, &versions),
                from: req.file.clone(),
                unmet: None,
            }
        } else {
            let from_default = default.and_then(|d| self.pick(kind, d, &versions));
            let default_fits = match (&range, &from_default) {
                (Some(range), Some(v)) => satisfies(range, &v.version),
                _ => true,
            };
            let in_range = range.as_ref().filter(|_| !default_fits).and_then(|range| {
                versions
                    .iter()
                    .filter(|v| satisfies(range, &v.version))
                    .max_by(|a, b| version::compare(&a.version, &b.version))
                    .cloned()
            });
            match (in_range, range_req) {
                (Some(v), Some(req)) => ResolvedRuntime {
                    kind,
                    requested: req.constraint.clone(),
                    pinned: true,
                    version: Some(v),
                    from: req.file.clone(),
                    unmet: None,
                },
                _ => ResolvedRuntime {
                    kind,
                    requested: default?.clone(),
                    pinned: false,
                    version: from_default,
                    from: "default".into(),
                    unmet: None,
                },
            }
        };

        if let (Some(range), Some(req), Some(v)) = (&range, range_req, &resolved.version)
            && !satisfies(range, &v.version)
        {
            resolved.unmet = Some(req.clone());
        }
        Some(resolved)
    }

    /// Environment for a project's process: its runtimes first on PATH.
    /// A runtime the project itself asks for (pin or version file) is a
    /// requirement: nothing matching means no start. Defaults and package
    /// managers are preferences, skipped when nothing matches.
    pub fn env_for(&self, project: &Project) -> Result<Vec<(String, String)>, AppError> {
        let mut entries = Vec::new();
        let mut env = Vec::new();

        // Package managers first: their launchers call `node`/`php`, which
        // must then be the project's own.
        let mut resolved = self.for_project(project);
        resolved.sort_by_key(|r| r.kind.category() == RuntimeCategory::Runtime);

        for resolved in resolved {
            let kind = resolved.kind;
            let required = resolved.pinned && kind.category() == RuntimeCategory::Runtime;
            let Some(runtime) = resolved.version else {
                if required {
                    return Err(AppError::new(
                        ErrorCode::NotRunnable,
                        format!(
                            "{} {} ({}) is not installed; run `locdev runtime install {} {}` or change the project's pin",
                            kind.label(),
                            resolved.requested,
                            resolved.from,
                            kind.executable(),
                            resolved.requested
                        ),
                    ));
                }
                warn!(
                    project = %project.name,
                    runtime = kind.executable(),
                    requested = %resolved.requested,
                    "runtime not installed; using the system PATH"
                );
                continue;
            };
            let dir = self.store.path_entry(kind, &runtime).map_err(|e| {
                AppError::internal(format!("could not prepare {}: {e}", kind.label()))
            })?;
            entries.push(dir);
            env.push((
                format!("LOCDEV_{}_VERSION", kind.executable().to_uppercase()),
                runtime.version,
            ));
        }

        if !entries.is_empty() {
            let current = std::env::var_os("PATH").unwrap_or_default();
            let joined =
                std::env::join_paths(entries.into_iter().chain(std::env::split_paths(&current)))
                    .map_err(|e| AppError::internal(format!("invalid PATH: {e}")))?;
            env.push(("PATH".into(), joined.to_string_lossy().into_owned()));
        }
        Ok(env)
    }

    pub async fn catalog(&self, kind: RuntimeKind) -> Result<Vec<Release>, AppError> {
        if let Some((at, releases)) = self.catalog.lock().unwrap().get(&kind)
            && at.elapsed() < CATALOG_TTL
        {
            return Ok(releases.clone());
        }
        if !self.store.downloads(kind) {
            return Err(not_downloadable(kind));
        }

        let store = self.store.clone();
        let releases = tokio::task::spawn_blocking(move || store.catalog(kind))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
        self.catalog
            .lock()
            .unwrap()
            .insert(kind, (Instant::now(), releases.clone()));
        Ok(releases)
    }

    /// Refreshes the catalogs of kinds with managed installs, so `info()`
    /// can show updates. Errors are logged, not returned.
    pub async fn refresh_catalogs(&self) {
        for kind in RuntimeKind::ALL {
            if self.store.downloads(kind)
                && !self.store.installed(kind).is_empty()
                && let Err(error) = self.catalog(kind).await
            {
                warn!(runtime = kind.executable(), %error, "could not refresh the release catalog");
            }
        }
    }

    /// Turns "lts", "22" or "22.11.0" into an exact released version.
    async fn pick_release(&self, kind: RuntimeKind, spec: &str) -> Result<String, AppError> {
        let releases = self.catalog(kind).await?;
        let found = if spec.eq_ignore_ascii_case("lts") {
            releases.iter().find(|r| r.lts.is_some())
        } else if spec.eq_ignore_ascii_case("latest") {
            releases.first()
        } else {
            version::best(spec, &releases, |r| r.version.as_str())
        };
        found.map(|r| r.version.clone()).ok_or_else(|| {
            AppError::not_found(format!("no {} release matches '{spec}'", kind.label()))
        })
    }

    /// Resolves `spec` to a release and marks its install as queued (the
    /// progress shows up in `info()`). The work itself is a job that runs
    /// [`Runtimes::run_install`]. Returns the exact version.
    pub async fn begin_install(&self, kind: RuntimeKind, spec: &str) -> Result<String, AppError> {
        if !self.store.downloads(kind) {
            return Err(not_downloadable(kind));
        }
        let version = self.pick_release(kind, spec).await?;

        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get(&kind)
            && !matches!(job.state, JobState::Done | JobState::Failed)
        {
            return Err(AppError::new(
                ErrorCode::AlreadyExists,
                format!(
                    "{} {} is already being installed",
                    kind.label(),
                    job.version
                ),
            ));
        }
        jobs.insert(
            kind,
            InstallJob {
                kind,
                version: version.clone(),
                state: JobState::Downloading,
                downloaded: 0,
                total: None,
                error: None,
                log: vec![format!(
                    "installing {} {version} (asked for '{spec}')",
                    kind.label()
                )],
            },
        );
        Ok(version)
    }

    /// Downloads, verifies and installs `version`, updating the progress
    /// `info()` reports. Blocking; safe to run again after an interruption.
    pub fn run_install(&self, kind: RuntimeKind, version: &str) -> Result<(), String> {
        info!(runtime = kind.executable(), %version, "installing runtime");
        // After a daemon restart the progress entry is gone: recreate it.
        self.jobs
            .lock()
            .unwrap()
            .entry(kind)
            .or_insert_with(|| InstallJob {
                kind,
                version: version.to_string(),
                state: JobState::Downloading,
                downloaded: 0,
                total: None,
                error: None,
                log: vec![format!("installing {} {version} (resumed)", kind.label())],
            });
        let push_log = |line: String| {
            if let Some(job) = self.jobs.lock().unwrap().get_mut(&kind)
                && job.log.len() < JOB_LOG_LINES
            {
                job.log.push(line);
            }
        };
        let result = self.store.install_logged(
            kind,
            version,
            &|state, downloaded, total| {
                if let Some(job) = self.jobs.lock().unwrap().get_mut(&kind) {
                    job.state = state;
                    if state == JobState::Downloading {
                        job.downloaded = downloaded;
                        job.total = total;
                    }
                }
            },
            &push_log,
        );
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.get_mut(&kind);
        match result {
            Ok(_) => {
                info!(runtime = kind.executable(), %version, "runtime installed");
                if let Some(job) = job {
                    job.state = JobState::Done;
                }
                drop(jobs);
                // A default like "8.4" may now resolve to the new version.
                self.sync_terminal();
                Ok(())
            }
            Err(error) => {
                warn!(runtime = kind.executable(), %version, %error, "runtime install failed");
                if let Some(job) = job {
                    job.state = JobState::Failed;
                    job.log.push(format!("failed: {error}"));
                    job.error = Some(error.to_string());
                }
                Err(error.to_string())
            }
        }
    }

    /// The install log of `kind`, for the job that ran it.
    pub fn install_log(&self, kind: RuntimeKind) -> Vec<String> {
        self.jobs
            .lock()
            .unwrap()
            .get(&kind)
            .map(|j| j.log.clone())
            .unwrap_or_default()
    }

    /// Removes a managed version. Refused while a running site uses it, and
    /// (unless `force`) while any site or the default resolves to it.
    pub fn uninstall(
        &self,
        kind: RuntimeKind,
        version: &str,
        force: bool,
        projects: &[Project],
        is_running: impl Fn(&str) -> bool,
    ) -> Result<(), AppError> {
        if !self
            .store
            .installed(kind)
            .iter()
            .any(|v| v.version == version)
        {
            return Err(AppError::not_found(format!(
                "{} {version} is not a LocDev-managed install (system versions are never removed)",
                kind.label()
            )));
        }

        let users: Vec<&Project> = projects
            .iter()
            .filter(|p| {
                self.for_project(p).iter().any(|r| {
                    r.kind == kind
                        && uses(p, r)
                        && r.version.as_ref().is_some_and(|v| {
                            v.version == version && v.source == RuntimeSource::Managed
                        })
                })
            })
            .collect();
        let running: Vec<&str> = users
            .iter()
            .filter(|p| is_running(&p.name))
            .map(|p| p.name.as_str())
            .collect();
        if !running.is_empty() {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!(
                    "{} {version} is in use by running site(s): {}; stop them first",
                    kind.label(),
                    running.join(", ")
                ),
            ));
        }
        if !force {
            let mut reasons: Vec<String> = Vec::new();
            if !users.is_empty() {
                let names: Vec<&str> = users.iter().map(|p| p.name.as_str()).collect();
                reasons.push(format!("used by {}", names.join(", ")));
            }
            if let Some(default) = self.defaults.lock().unwrap().get(&kind)
                && self
                    .resolve(kind, default)
                    .is_some_and(|v| v.version == version)
            {
                reasons.push(format!("the default ({default}) resolves to it"));
            }
            if !reasons.is_empty() {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    format!(
                        "{} {version} is {}; pass --force to remove it anyway",
                        kind.label(),
                        reasons.join(" and ")
                    ),
                ));
            }
        }

        self.store
            .uninstall(kind, version)
            .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
        self.sync_terminal();
        Ok(())
    }

    pub fn set_default(&self, kind: RuntimeKind, pin: Option<String>) -> Result<(), AppError> {
        let pin = pin
            .map(|p| p.trim().trim_start_matches('v').to_string())
            .filter(|p| !p.is_empty());
        if let Some(pin) = &pin {
            validate_pin(kind, pin)?;
        }

        let mut defaults = self.defaults.lock().unwrap();
        match pin {
            Some(pin) => defaults.insert(kind, pin),
            None => defaults.remove(&kind),
        };
        let file = StateFile {
            version: STATE_VERSION,
            defaults: defaults.clone(),
        };
        let raw = toml::to_string_pretty(&file).map_err(|e| AppError::internal(e.to_string()))?;
        write_atomic(&self.state_path, raw.as_bytes())?;
        drop(defaults);
        self.sync_terminal();
        Ok(())
    }

    /// The installed versions a package manager needs (`node >=18.12`) that
    /// the project's runtime does not meet.
    pub fn tool_conflicts(&self, resolved: &[ResolvedRuntime]) -> Vec<String> {
        let mut out = Vec::new();
        for tool in resolved
            .iter()
            .filter(|r| r.kind.category() == RuntimeCategory::PackageManager)
        {
            let Some(v) = tool
                .version
                .as_ref()
                .filter(|v| v.source == RuntimeSource::Managed)
            else {
                continue;
            };
            for (needs, range) in self.store.tool_meta(tool.kind, &v.version).requires {
                let runtime = resolved
                    .iter()
                    .find(|r| r.kind == needs)
                    .and_then(|r| r.version.as_ref());
                if let (Some(runtime), Some(parsed)) = (runtime, parse_range(needs, &range))
                    && !satisfies(&parsed, &runtime.version)
                {
                    out.push(format!(
                        "{} {} needs {} {range}, but the site runs {}",
                        tool.kind.label(),
                        v.version,
                        needs.label(),
                        runtime.version
                    ));
                }
            }
        }
        out
    }
}

/// Whether `project` actually runs `resolved`: a pin or version file always
/// counts; a default only for the project's own ecosystem (a Node.js site
/// has the default PHP on its PATH but doesn't use it).
fn uses(project: &Project, resolved: &ResolvedRuntime) -> bool {
    use locdev_core::model::ProjectKind as K;
    if resolved.from != "default" {
        return true;
    }
    let has = |file: &str| project.path.join(file).is_file();
    match resolved.kind {
        RuntimeKind::Php | RuntimeKind::Composer => {
            matches!(project.kind, K::Laravel | K::Wordpress | K::Php) || has("composer.json")
        }
        RuntimeKind::Node
        | RuntimeKind::Npm
        | RuntimeKind::Pnpm
        | RuntimeKind::Yarn
        | RuntimeKind::Bun => {
            matches!(
                project.kind,
                K::Next | K::Nuxt | K::Astro | K::Sveltekit | K::Vite | K::Node
            ) || has("package.json")
        }
    }
}

fn not_downloadable(kind: RuntimeKind) -> AppError {
    AppError::new(
        ErrorCode::NotRunnable,
        match kind {
            RuntimeKind::Php => "PHP downloads are not configured in this LocDev build \
                (no signed build source); installed PHP versions are detected automatically"
                .to_string(),
            RuntimeKind::Npm => {
                "npm comes with each Node.js version; install a Node.js version instead".to_string()
            }
            RuntimeKind::Bun => "LocDev doesn't install Bun yet; versions from bun.sh's installer \
                or a package manager are detected automatically"
                .to_string(),
            _ => format!("LocDev can't download {} on this platform", kind.label()),
        },
    )
}

/// The line people pin: Node.js and package managers by major, PHP by minor.
pub fn release_line(kind: RuntimeKind, v: &str) -> String {
    let parts = version::parts(v);
    let take = if kind == RuntimeKind::Php { 2 } else { 1 };
    parts
        .iter()
        .take(take)
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

fn parse_range(kind: RuntimeKind, raw: &str) -> Option<Range> {
    match kind {
        RuntimeKind::Php | RuntimeKind::Composer => semver::parse_composer(raw).ok(),
        _ => semver::parse_npm(raw).ok(),
    }
}

fn satisfies(range: &Range, v: &str) -> bool {
    Version::parse(v).is_some_and(|v| range.matches(&v))
}

/// A pin is one to three numeric components: "22", "8.3", "22.11.0".
pub fn validate_pin(kind: RuntimeKind, pin: &str) -> Result<(), AppError> {
    if !kind.pinnable() {
        return Err(AppError::invalid(
            "npm follows the site's Node.js version; pin Node.js instead",
        ));
    }
    let segments: Vec<&str> = pin.trim_start_matches('v').split('.').collect();
    let well_formed = (1..=3).contains(&segments.len())
        && segments
            .iter()
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
    if well_formed {
        Ok(())
    } else {
        Err(AppError::invalid(format!(
            "'{pin}' is not a version; use something like 22, 8.3 or 22.11.0"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtimes(dir: &Path) -> Runtimes {
        Runtimes::load(dir, dir, PhpSource::default()).unwrap()
    }

    fn fake_system(rt: &Runtimes, kind: RuntimeKind, versions: &[&str]) {
        let found = versions
            .iter()
            .map(|v| RuntimeVersion {
                version: v.to_string(),
                source: RuntimeSource::System,
                executable: PathBuf::from(format!(
                    "/opt/{}/{v}/bin/{}",
                    kind.executable(),
                    kind.executable()
                )),
                origin: "test".into(),
                support: None,
            })
            .collect();
        rt.system.lock().unwrap().insert(kind, found);
    }

    fn project(dir: &Path) -> Project {
        Project {
            name: "blog".into(),
            domain: "blog.test".into(),
            path: dir.into(),
            ..Default::default()
        }
    }

    #[test]
    fn defaults_count_only_for_their_own_ecosystem() {
        let dir = tempfile::tempdir().unwrap();
        let rt = runtimes(dir.path());
        let node_site = tempfile::tempdir().unwrap();
        std::fs::write(node_site.path().join("package.json"), "{}").unwrap();
        let resolved = |kind, from: &str| ResolvedRuntime {
            kind,
            requested: "8.4".into(),
            pinned: false,
            version: None,
            from: from.into(),
            unmet: None,
        };
        let site = project(node_site.path());
        assert!(!uses(&site, &resolved(RuntimeKind::Php, "default")));
        assert!(uses(&site, &resolved(RuntimeKind::Node, "default")));
        assert!(uses(&site, &resolved(RuntimeKind::Php, "projects.toml")));
        let laravel = Project {
            kind: locdev_core::model::ProjectKind::Laravel,
            ..project(dir.path())
        };
        assert!(uses(&laravel, &resolved(RuntimeKind::Composer, "default")));
        drop(rt);
    }

    #[test]
    fn pins_are_numeric() {
        for ok in ["22", "8.3", "22.11.0", "v20"] {
            assert!(validate_pin(RuntimeKind::Node, ok).is_ok(), "{ok}");
        }
        for bad in ["", "lts", "22.x", "1.2.3.4", "8.3-beta"] {
            assert!(validate_pin(RuntimeKind::Node, bad).is_err(), "{bad}");
        }
        assert!(validate_pin(RuntimeKind::Pnpm, "9").is_ok());
        assert!(validate_pin(RuntimeKind::Npm, "10").is_err());
    }

    #[test]
    fn defaults_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let rt = runtimes(dir.path());
        rt.set_default(RuntimeKind::Node, Some("v22".into()))
            .unwrap();
        rt.set_default(RuntimeKind::Php, Some("8.3".into()))
            .unwrap();
        rt.set_default(RuntimeKind::Php, None).unwrap();
        rt.set_default(RuntimeKind::Pnpm, Some("9".into())).unwrap();

        let info = runtimes(dir.path()).info(&[]);
        let get = |k| info.iter().find(|i| i.kind == k).unwrap();
        assert_eq!(get(RuntimeKind::Node).default.as_deref(), Some("22"));
        assert_eq!(get(RuntimeKind::Php).default, None);
        assert_eq!(get(RuntimeKind::Pnpm).default.as_deref(), Some("9"));
        assert_eq!(
            get(RuntimeKind::Pnpm).category,
            RuntimeCategory::PackageManager
        );
    }

    #[test]
    fn missing_pins_refuse_to_start() {
        let dir = tempfile::tempdir().unwrap();
        let rt = runtimes(dir.path());
        let pinned = Project {
            node: Some("1.2".into()),
            ..project(dir.path())
        };
        let error = rt.env_for(&pinned).unwrap_err();
        assert_eq!(error.code, ErrorCode::NotRunnable);
        assert!(error.message.contains("locdev runtime install node 1.2"));

        // Without pins nothing changes: the system PATH applies.
        let unpinned = project(dir.path());
        assert!(rt.env_for(&unpinned).unwrap().is_empty());

        // A missing *default* is not an error: it falls back to PATH.
        rt.set_default(RuntimeKind::Node, Some("1.2".into()))
            .unwrap();
        assert!(rt.env_for(&unpinned).unwrap().is_empty());

        // Nor is a missing package manager.
        let mut tool = project(dir.path());
        tool.tools.insert(RuntimeKind::Pnpm, "9".into());
        assert!(rt.env_for(&tool).is_ok());
    }

    #[test]
    fn version_files_act_like_pins() {
        let dir = tempfile::tempdir().unwrap();
        let rt = runtimes(dir.path());
        fake_system(&rt, RuntimeKind::Node, &["18.20.0", "20.11.1", "22.11.0"]);
        rt.set_default(RuntimeKind::Node, Some("22".into()))
            .unwrap();
        std::fs::write(dir.path().join(".nvmrc"), "20\n").unwrap();

        let resolved = rt.for_project(&project(dir.path()));
        let node = resolved
            .iter()
            .find(|r| r.kind == RuntimeKind::Node)
            .unwrap();
        assert_eq!(node.version.as_ref().unwrap().version, "20.11.1");
        assert_eq!(node.from, ".nvmrc");
        assert!(node.pinned);

        // The project's own pin beats the file.
        let pinned = Project {
            node: Some("18".into()),
            ..project(dir.path())
        };
        let node = rt
            .for_project(&pinned)
            .into_iter()
            .find(|r| r.kind == RuntimeKind::Node)
            .unwrap();
        assert_eq!(node.version.unwrap().version, "18.20.0");

        // `lts/*` picks the newest installed LTS line.
        std::fs::write(dir.path().join(".nvmrc"), "lts/*\n").unwrap();
        rt.requirements.lock().unwrap().clear();
        let node = rt
            .for_project(&project(dir.path()))
            .into_iter()
            .find(|r| r.kind == RuntimeKind::Node)
            .unwrap();
        assert_eq!(node.version.unwrap().version, "22.11.0");
    }

    #[test]
    fn ranges_steer_the_choice() {
        let dir = tempfile::tempdir().unwrap();
        let rt = runtimes(dir.path());
        fake_system(&rt, RuntimeKind::Php, &["8.1.30", "8.3.14", "8.4.2"]);
        rt.set_default(RuntimeKind::Php, Some("8.1".into()))
            .unwrap();
        std::fs::write(
            dir.path().join("composer.json"),
            r#"{"require":{"php":">=8.2 <8.4"}}"#,
        )
        .unwrap();

        // The default (8.1) doesn't fit, so the newest fitting version wins.
        let php = rt
            .for_project(&project(dir.path()))
            .into_iter()
            .find(|r| r.kind == RuntimeKind::Php)
            .unwrap();
        assert_eq!(php.version.as_ref().unwrap().version, "8.3.14");
        assert_eq!(php.from, "composer.json (require)");
        assert!(php.unmet.is_none());

        // An explicit pin outside the range is kept, but flagged.
        let pinned = Project {
            php: Some("8.4".into()),
            ..project(dir.path())
        };
        let php = rt
            .for_project(&pinned)
            .into_iter()
            .find(|r| r.kind == RuntimeKind::Php)
            .unwrap();
        assert_eq!(php.version.unwrap().version, "8.4.2");
        assert_eq!(php.unmet.unwrap().constraint, ">=8.2 <8.4");
    }

    #[test]
    fn reports_usage_and_support() {
        let dir = tempfile::tempdir().unwrap();
        let rt = runtimes(dir.path());
        fake_system(&rt, RuntimeKind::Node, &["18.20.0", "22.11.0"]);
        let p = Project {
            node: Some("22".into()),
            ..project(dir.path())
        };
        let info = rt.info(std::slice::from_ref(&p));
        let node = info.iter().find(|i| i.kind == RuntimeKind::Node).unwrap();
        assert_eq!(node.usage.get("22.11.0"), Some(&vec!["blog".to_string()]));
        let old = node
            .versions
            .iter()
            .find(|v| v.version == "18.20.0")
            .unwrap();
        assert_eq!(
            old.support.as_ref().unwrap().state,
            locdev_core::model::SupportState::Eol
        );
    }

    #[test]
    fn release_lines() {
        assert_eq!(release_line(RuntimeKind::Node, "22.11.0"), "22");
        assert_eq!(release_line(RuntimeKind::Php, "8.3.14"), "8.3");
        assert_eq!(release_line(RuntimeKind::Pnpm, "9.12.0"), "9");
    }
}
