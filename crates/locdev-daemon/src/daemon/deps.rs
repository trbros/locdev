//! Dependencies: listing (from disk and the registry cache, no network),
//! the registry cache itself, the request limiter every registry call goes
//! through, and update plans.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use locdev_core::deps::{
    Advisory, DepsReport, Ecosystem, PackageLinks, PlannedChange, UpdateKind, UpdatePlan,
};
use locdev_core::model::{Project, RuntimeKind, now_ms};
use locdev_core::{AppError, ErrorCode};
use locdev_deps::analysis::{self, Context};
use locdev_deps::local::{self, Declared};
use locdev_deps::registry::{self, FetchError, PackageInfo};
use locdev_deps::update::{self, Pick};
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::Daemon;
use crate::fsutil::write_atomic;

/// Registry metadata is reused for this long before it is asked again.
const CACHE_TTL_MS: u64 = 6 * 3_600_000;

/// Package metadata from registries, kept on disk between daemon runs.
pub struct RegistryCache {
    path: PathBuf,
    entries: Mutex<HashMap<String, (u64, PackageInfo)>>,
}

fn key(ecosystem: Ecosystem, name: &str) -> String {
    format!("{}:{name}", ecosystem.label())
}

impl RegistryCache {
    pub fn load(path: &Path) -> Self {
        let entries = std::fs::read(path)
            .ok()
            .and_then(|raw| serde_json::from_slice(&raw).ok())
            .unwrap_or_default();
        Self {
            path: path.to_path_buf(),
            entries: Mutex::new(entries),
        }
    }

    /// Cached metadata, however old (listing never goes to the network).
    pub fn get(&self, ecosystem: Ecosystem, name: &str) -> Option<PackageInfo> {
        self.entries
            .lock()
            .unwrap()
            .get(&key(ecosystem, name))
            .map(|(_, info)| info.clone())
    }

    pub fn fresh(&self, ecosystem: Ecosystem, name: &str) -> Option<PackageInfo> {
        let entries = self.entries.lock().unwrap();
        let (at, info) = entries.get(&key(ecosystem, name))?;
        (now_ms().saturating_sub(*at) < CACHE_TTL_MS).then(|| info.clone())
    }

    pub fn put(&self, info: PackageInfo) {
        self.entries
            .lock()
            .unwrap()
            .insert(key(info.ecosystem, &info.name), (now_ms(), info));
    }

    pub fn set_links(&self, ecosystem: Ecosystem, name: &str, links: PackageLinks) {
        if let Some((_, info)) = self.entries.lock().unwrap().get_mut(&key(ecosystem, name)) {
            info.links = links;
        }
    }

    pub fn save(&self) {
        let raw = serde_json::to_vec(&*self.entries.lock().unwrap());
        match raw {
            Ok(raw) => {
                if let Some(parent) = self.path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(error) = write_atomic(&self.path, &raw) {
                    warn!(%error, "could not save the registry cache");
                }
            }
            Err(error) => warn!(%error, "could not serialize the registry cache"),
        }
    }
}

/// Spaces registry requests out across every job: at most `per_second`.
pub struct Limiter {
    next: tokio::sync::Mutex<Instant>,
}

impl Limiter {
    pub fn new() -> Self {
        Self {
            next: tokio::sync::Mutex::new(Instant::now()),
        }
    }

    pub async fn wait(&self, per_second: u8) {
        let interval = Duration::from_millis(1000 / u64::from(per_second.max(1)));
        let mut next = self.next.lock().await;
        let now = Instant::now();
        if *next > now {
            tokio::time::sleep(*next - now).await;
        }
        *next = Instant::now().max(*next) + interval;
    }
}

/// Advisories found by the last check, per package and version: they only
/// apply while the project still uses that version.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Findings {
    pub checked_at: Option<u64>,
    pub advisories: HashMap<String, (String, Vec<Advisory>)>,
}

impl Daemon {
    fn findings_path(&self, project: &str) -> PathBuf {
        self.paths
            .data_dir
            .join("state")
            .join("deps")
            .join(format!("{project}.json"))
    }

    pub(super) fn findings(&self, project: &str) -> Findings {
        std::fs::read(self.findings_path(project))
            .ok()
            .and_then(|raw| serde_json::from_slice(&raw).ok())
            .unwrap_or_default()
    }

    pub(super) fn save_findings(&self, project: &str, findings: &Findings) {
        let path = self.findings_path(project);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(raw) = serde_json::to_vec_pretty(findings) {
            let _ = write_atomic(&path, &raw);
        }
    }

    /// What the analysis needs to know about the site's runtimes.
    pub(super) fn deps_context(&self, project: &Project, local: &[Declared]) -> Context {
        let resolved = self.runtimes.for_project(project);
        // Without a pin or default, the site runs whatever is on PATH.
        let version_of = |kind| {
            resolved
                .iter()
                .find(|r| r.kind == kind)
                .and_then(|r| r.version.clone())
                .or_else(|| self.runtimes.on_path(kind))
                .map(|v| v.version)
        };
        Context {
            node: version_of(RuntimeKind::Node),
            php: version_of(RuntimeKind::Php),
            current: local
                .iter()
                .filter_map(|d| Some((d.name.clone(), analysis::current(d)?.to_string())))
                .collect(),
        }
    }

    /// The project's dependencies from disk, judged with cached registry
    /// data and the last check's advisories. No network.
    pub(super) async fn deps_report(&self, project: &Project) -> DepsReport {
        let path = project.path.clone();
        let local = tokio::task::spawn_blocking(move || local::read(&path))
            .await
            .unwrap_or_default();
        let ctx = self.deps_context(project, &local.packages);
        let findings = self.findings(&project.name);
        let packages = local
            .packages
            .iter()
            .map(|dep| {
                let advisories = findings
                    .advisories
                    .get(&key(dep.ecosystem, &dep.name))
                    .filter(|(version, _)| Some(version.as_str()) == analysis::current(dep))
                    .map(|(_, list)| list.clone())
                    .unwrap_or_default();
                analysis::report(
                    dep,
                    self.registry_cache.get(dep.ecosystem, &dep.name).as_ref(),
                    advisories,
                    &ctx,
                )
            })
            .collect();
        DepsReport {
            project: project.name.clone(),
            manager: local.manager,
            files: local.files,
            packages,
            checked_at: findings.checked_at,
            errors: local.errors,
        }
    }

    /// Metadata for one package: the cache while fresh, else the registry
    /// (through the limiter).
    pub(super) async fn package_info(
        &self,
        ecosystem: Ecosystem,
        name: &str,
    ) -> Result<PackageInfo, FetchError> {
        if let Some(info) = self.registry_cache.fresh(ecosystem, name) {
            return Ok(info);
        }
        self.limiter
            .wait(self.settings.get().automation.requests_per_second)
            .await;
        let name_owned = name.to_string();
        let info = tokio::task::spawn_blocking(move || registry::fetch(ecosystem, &name_owned))
            .await
            .map_err(|e| FetchError::Network(e.to_string()))??;
        self.registry_cache.put(info.clone());
        Ok(info)
    }

    /// What updating `packages` would do, computed fresh. The same function
    /// builds what the user confirms and what the job then runs.
    pub(super) async fn plan_update(
        &self,
        project: &Project,
        packages: &[String],
        allow_major: bool,
        run_scripts: bool,
    ) -> Result<UpdatePlan, AppError> {
        if packages.is_empty() {
            return Err(AppError::invalid("choose at least one package"));
        }
        let path = project.path.clone();
        let local = tokio::task::spawn_blocking(move || local::read(&path))
            .await
            .unwrap_or_default();
        let manager = local.manager.unwrap_or(RuntimeKind::Npm);
        let ctx = self.deps_context(project, &local.packages);
        let mut changes = Vec::new();
        let mut picks = Vec::new();
        let mut warnings = Vec::new();

        for name in packages {
            let dep = local
                .packages
                .iter()
                .find(|d| &d.name == name)
                .ok_or_else(|| {
                    AppError::not_found(format!("{} doesn't declare {name}", project.name))
                })?;
            let info = self
                .package_info(dep.ecosystem, &dep.name)
                .await
                .map_err(|e| AppError::new(ErrorCode::Remote, format!("{name}: {e}")))?;
            let Some((to, candidate)) = analysis::target(dep, &info, allow_major) else {
                warnings.push(format!(
                    "{name} is already at the newest {} version",
                    if allow_major { "stable" } else { "allowed" }
                ));
                continue;
            };
            let from = analysis::current(dep).map(str::to_string);
            let from_version = from.as_deref().and_then(|v| match dep.ecosystem {
                Ecosystem::Npm => locdev_core::semver::Version::parse(v),
                Ecosystem::Composer => locdev_core::semver::Version::parse_composer(v),
            });
            let (compatibility, reasons) = match &from_version {
                Some(from) => analysis::judge(dep, from, &to, candidate, &ctx),
                None => (
                    locdev_core::deps::Compatibility::Unknown,
                    vec!["not installed yet".into()],
                ),
            };
            let within_range = match dep.ecosystem {
                Ecosystem::Npm => locdev_core::semver::parse_npm(&dep.constraint).ok(),
                Ecosystem::Composer => locdev_core::semver::parse_composer(&dep.constraint).ok(),
            }
            .is_some_and(|range| range.matches(&to));
            let pick = Pick {
                name: dep.name.clone(),
                ecosystem: dep.ecosystem,
                kind: dep.kind,
                declared: dep.constraint.clone(),
                target: candidate.version.clone(),
                within_range,
            };
            if compatibility == locdev_core::deps::Compatibility::Incompatible {
                warnings.push(format!(
                    "{name} {}: {}",
                    candidate.version,
                    reasons.join("; ")
                ));
            }
            // Release links for what the user is about to read up on.
            let mut links = info.links.clone();
            if dep.ecosystem == Ecosystem::Npm && links.repository.is_none() {
                self.limiter
                    .wait(self.settings.get().automation.requests_per_second)
                    .await;
                let (n, v) = (dep.name.clone(), candidate.version.clone());
                if let Ok(Ok(found)) =
                    tokio::task::spawn_blocking(move || registry::npm_links(&n, &v)).await
                {
                    self.registry_cache
                        .set_links(dep.ecosystem, &dep.name, found.clone());
                    links = found;
                }
            }
            changes.push(PlannedChange {
                name: dep.name.clone(),
                ecosystem: dep.ecosystem,
                from: from.clone(),
                to: candidate.version.clone(),
                constraint: (!within_range).then(|| pick.new_constraint()),
                update: from_version
                    .as_ref()
                    .map(|f| analysis::update_kind(f, &to))
                    .unwrap_or(UpdateKind::None),
                compatibility,
                reasons,
                links,
            });
            picks.push(pick);
        }

        if !local
            .files
            .iter()
            .any(|f| f.ends_with(".lock") || f.ends_with("lock.json") || f.ends_with("lock.yaml"))
        {
            warnings.push("no lockfile: the manager will create one".into());
        }
        let berry = manager == RuntimeKind::Yarn && update::yarn_berry(&project.path);
        let commands = update::commands(manager, berry, &picks, run_scripts);
        Ok(UpdatePlan {
            project: project.name.clone(),
            files: update::files(manager, &picks),
            commands: commands.iter().map(|c| c.argv.clone()).collect(),
            changes,
            run_scripts,
            warnings,
        })
    }
}
