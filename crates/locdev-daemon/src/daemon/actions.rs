//! The GitHub Actions builder: templates, validation and saving workflows
//! into a site's `.github/workflows/`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use locdev_actions::{ActionsError, Facts, Versions, templates, validate, yaml};
use locdev_core::actions::{SavePlan, Validation, Workflow};
use locdev_core::model::{Project, now_ms};
use locdev_core::protocol::Response;
use locdev_core::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::Daemon;
use crate::fsutil::write_atomic;

/// Action versions from GitHub, kept in `data/state/action-versions.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VersionCache {
    pub checked_at: Option<u64>,
    /// repo → major tag (`v7`).
    pub versions: BTreeMap<String, String>,
}

impl VersionCache {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }
}

fn app_error(error: ActionsError) -> AppError {
    AppError::new(ErrorCode::Conflict, error.0)
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AppError::internal(e.to_string()))
}

impl Daemon {
    fn versions_path(&self) -> PathBuf {
        self.paths
            .data_dir
            .join("state")
            .join("action-versions.json")
    }

    fn versions(&self) -> Versions {
        Versions::with(&self.action_versions.lock().unwrap().versions)
    }

    /// Where LocDev's own actionlint goes.
    fn actionlint_dir(&self) -> PathBuf {
        self.paths.data_dir.join("tools").join("actionlint")
    }

    /// The daemon's PATH with LocDev's actionlint first.
    fn tools_path(&self) -> String {
        let current = std::env::var_os("PATH").unwrap_or_default();
        std::env::join_paths(
            std::iter::once(self.actionlint_dir()).chain(std::env::split_paths(&current)),
        )
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
    }

    /// Validates `text`, with actionlint when installed (by LocDev or on PATH).
    pub(super) async fn validate(&self, text: String) -> Result<Validation, AppError> {
        let versions = self.versions();
        let path = self.tools_path();
        blocking(move || validate::validate(&text, &versions, Some(&path))).await
    }

    /// Every workflow of the site, validated one by one.
    pub(super) async fn actions_audit(&self, project: &Project) -> Result<Response, AppError> {
        let root = project.path.clone();
        let existing = blocking(move || locdev_actions::existing(&root)).await?;
        let mut files = Vec::new();
        for workflow in existing {
            let root = project.path.clone();
            let file = workflow.file.clone();
            let findings = match blocking(move || locdev_actions::read(&root, &file)).await? {
                Ok((text, _)) => {
                    self.workflow_keep(project, &workflow.file, &text, "read");
                    self.validate(text).await?.findings
                }
                Err(e) => vec![locdev_core::actions::Finding {
                    level: locdev_core::actions::FindingLevel::Error,
                    rule: "read".into(),
                    message: e.0,
                    line: None,
                    source: "locdev".into(),
                    fix: None,
                }],
            };
            files.push(locdev_core::actions::WorkflowAudit {
                file: workflow.file,
                name: workflow.name,
                findings,
            });
        }
        let path = self.tools_path();
        let actionlint = blocking(move || actionlint_version(&path)).await?;
        Ok(Response::ActionsAudit { files, actionlint })
    }

    fn workflow_history_dir(&self, project: &Project, file: &str) -> PathBuf {
        self.paths
            .data_dir
            .join("state")
            .join("workflow-history")
            .join(&project.name)
            .join(file.replace(['/', '\\'], "_"))
    }

    /// Keeps `text` as a version of `file` unless it's what was kept last.
    /// Failing to keep one never blocks reading or saving.
    fn workflow_keep(&self, project: &Project, file: &str, text: &str, reason: &str) {
        let dir = self.workflow_history_dir(project, file);
        let hash = locdev_actions::hash(text.as_bytes());
        let last = workflow_versions(&dir).into_iter().next();
        if let Some(last) = last
            && std::fs::read(dir.join(format!("{}.yml", last.id)))
                .is_ok_and(|old| locdev_actions::hash(&old) == hash)
        {
            return;
        }
        let id = format!("{}-{reason}", now_ms());
        if std::fs::create_dir_all(&dir).is_err()
            || write_atomic(&dir.join(format!("{id}.yml")), text.as_bytes()).is_err()
        {
            warn!(project = %project.name, %file, "workflow version not kept");
            return;
        }
        // The newest HISTORY versions stay.
        for old in workflow_versions(&dir).into_iter().skip(WORKFLOW_HISTORY) {
            let _ = std::fs::remove_file(dir.join(format!("{}.yml", old.id)));
        }
    }

    pub(super) fn actions_history(
        &self,
        project: &Project,
        file: &str,
    ) -> Result<Response, AppError> {
        locdev_actions::check_file_name(file).map_err(app_error)?;
        Ok(Response::ActionsHistory {
            versions: workflow_versions(&self.workflow_history_dir(project, file)),
        })
    }

    pub(super) async fn actions_history_version(
        &self,
        project: &Project,
        file: &str,
        id: &str,
    ) -> Result<Response, AppError> {
        locdev_actions::check_file_name(file).map_err(app_error)?;
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(AppError::invalid("bad version id"));
        }
        let path = self
            .workflow_history_dir(project, file)
            .join(format!("{id}.yml"));
        let text = std::fs::read_to_string(&path)
            .map_err(|_| AppError::not_found(format!("no version {id} of {file}")))?;
        let validation = self.validate(text.clone()).await?;
        Ok(Response::ActionsYaml {
            yaml: text,
            validation,
        })
    }

    /// actionlint from its latest GitHub release, only when the SHA-256 in
    /// the release's checksums file and GitHub's own digest of the asset agree.
    pub(super) async fn actionlint_install(&self) -> Result<Response, AppError> {
        if self.settings.get().network.offline {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "offline mode is on (Settings → Automations)",
            ));
        }
        let dir = self.actionlint_dir();
        let version = blocking(move || install_actionlint(&dir))
            .await?
            .map_err(|e| AppError::new(ErrorCode::Remote, e))?;
        info!(%version, "actionlint installed");
        Ok(Response::Ok)
    }

    pub(super) async fn actions_overview(&self, project: &Project) -> Result<Response, AppError> {
        let root = project.path.clone();
        let facts = facts(root.clone()).await?;
        let versions = self.versions();
        let checked_at = self.action_versions.lock().unwrap().checked_at;
        let path = self.tools_path();
        let (existing, actionlint) =
            blocking(move || (locdev_actions::existing(&root), actionlint_version(&path))).await?;
        Ok(Response::ActionsOverview {
            templates: templates::list(&facts),
            existing,
            actions: versions.list(),
            versions_checked_at: checked_at,
            actionlint,
        })
    }

    pub(super) async fn actions_template(
        &self,
        project: &Project,
        template: &str,
    ) -> Result<Response, AppError> {
        let facts = facts(project.path.clone()).await?;
        let (workflow, file) = templates::build(template, &facts, &self.versions())
            .ok_or_else(|| AppError::not_found(format!("no template named '{template}'")))?;
        Ok(Response::ActionsWorkflow { workflow, file })
    }

    pub(super) async fn actions_render(&self, workflow: Workflow) -> Result<Response, AppError> {
        let text = yaml::render(&workflow);
        let validation = self.validate(text.clone()).await?;
        Ok(Response::ActionsYaml {
            yaml: text,
            validation,
        })
    }

    pub(super) async fn actions_read(
        &self,
        project: &Project,
        file: String,
    ) -> Result<Response, AppError> {
        let root = project.path.clone();
        let name = file.clone();
        let (text, _) = blocking(move || locdev_actions::read(&root, &file))
            .await?
            .map_err(app_error)?;
        // An edit made outside LocDev becomes a version too.
        self.workflow_keep(project, &name, &text, "read");
        let validation = self.validate(text.clone()).await?;
        Ok(Response::ActionsYaml {
            yaml: text,
            validation,
        })
    }

    pub(super) async fn actions_plan(
        &self,
        project: &Project,
        file: String,
        text: String,
    ) -> Result<SavePlan, AppError> {
        let validation = self.validate(text.clone()).await?;
        let root = project.path.clone();
        blocking(move || locdev_actions::plan(&root, &file, &text, validation))
            .await?
            .map_err(app_error)
    }

    /// Saving is refused while LocDev's own checks report errors (syntax,
    /// structure, injection, unpinned actions…). actionlint's findings are
    /// shown but don't block: it also reports style and shellcheck notes.
    pub(super) async fn actions_save(
        &self,
        project: &Project,
        file: String,
        text: String,
        replace: Option<String>,
    ) -> Result<Response, AppError> {
        let validation = self.validate(text.clone()).await?;
        if let Some(error) = validation
            .findings
            .iter()
            .find(|f| f.level == locdev_core::actions::FindingLevel::Error && f.source == "locdev")
        {
            return Err(AppError::invalid(format!(
                "fix the errors first: {}",
                error.message
            )));
        }
        let root = project.path.clone();
        let replacing = replace.is_some();
        if replacing && let Ok((current, _)) = locdev_actions::read(&project.path, &file) {
            self.workflow_keep(project, &file, &current, "before-save");
        }
        let (name, saved) = (file.clone(), text.clone());
        let path = blocking(move || locdev_actions::save(&root, &file, &text, replace.as_deref()))
            .await?
            .map_err(app_error)?;
        self.workflow_keep(project, &name, &saved, "saved");
        info!(project = %project.name, %path, replacing, "workflow saved");
        Ok(Response::ActionsSaved { path })
    }

    pub(super) fn action_versions(&self) -> Response {
        Response::ActionVersions {
            actions: self.versions().list(),
            checked_at: self.action_versions.lock().unwrap().checked_at,
        }
    }

    /// Asks GitHub for the latest release of every known action (with the
    /// connected account when there is one, for the higher rate limit).
    pub(super) async fn refresh_action_versions(&self) -> Result<Response, AppError> {
        if self.settings.get().network.offline {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "offline mode is on (Settings → Automations)",
            ));
        }
        let token = tokio::task::spawn_blocking(locdev_github::load)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .ok()
            .flatten()
            .map(|(token, _)| token);
        let (found, errors) = blocking(move || {
            let client = match token {
                Some(token) => locdev_github::Client::new(token),
                None => locdev_github::Client::anonymous(),
            };
            let mut found = BTreeMap::new();
            let mut errors = Vec::new();
            for (repo, _, _) in locdev_actions::versions::SNAPSHOT {
                match client.latest_tag(repo) {
                    Ok(tag) => {
                        if let Some(major) = locdev_actions::versions::major_tag(&tag) {
                            found.insert(repo.to_string(), major);
                        }
                    }
                    Err(e) => errors.push(format!("{repo}: {e}")),
                }
            }
            (found, errors)
        })
        .await?;
        if found.is_empty() {
            return Err(AppError::new(
                ErrorCode::Remote,
                errors
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "no versions found".into()),
            ));
        }
        for error in &errors {
            warn!(%error, "action version not refreshed");
        }
        let cache = VersionCache {
            checked_at: Some(now_ms()),
            versions: found,
        };
        let raw =
            serde_json::to_vec_pretty(&cache).map_err(|e| AppError::internal(e.to_string()))?;
        write_atomic(&self.versions_path(), &raw)?;
        *self.action_versions.lock().unwrap() = cache.clone();
        Ok(self.action_versions())
    }
}

async fn facts(root: PathBuf) -> Result<Facts, AppError> {
    blocking(move || {
        let branch = locdev_git::Repo::open(&root)
            .ok()
            .and_then(|repo| repo.status().ok())
            .and_then(|status| status.branch);
        Facts::read(&root, branch.as_deref())
    })
    .await
}

/// Versions kept per workflow file.
const WORKFLOW_HISTORY: usize = 50;

/// Kept versions in `dir`, newest first (ids start with the time).
fn workflow_versions(dir: &Path) -> Vec<locdev_core::actions::WorkflowVersion> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let id = name.strip_suffix(".yml")?.to_string();
            let (at, reason) = id.split_once('-')?;
            Some(locdev_core::actions::WorkflowVersion {
                at: at.parse().ok()?,
                reason: reason.into(),
                size: e.metadata().map(|m| m.len()).unwrap_or(0),
                id,
            })
        })
        .collect();
    out.sort_by(|a, b| b.at.cmp(&a.at).then(b.id.cmp(&a.id)));
    out
}

/// `actionlint_<v>_<os>_<arch>.<ext>` for this machine.
fn actionlint_asset(version: &str) -> Option<String> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "darwin",
        "windows" => "windows",
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => return None,
    };
    let ext = if os == "windows" { "zip" } else { "tar.gz" };
    Some(format!("actionlint_{version}_{os}_{arch}.{ext}"))
}

fn install_actionlint(dir: &Path) -> Result<String, String> {
    use locdev_runtimes::download::{checksum_for, download, fetch_text};
    let release: serde_json::Value = serde_json::from_str(
        &fetch_text("https://api.github.com/repos/rhysd/actionlint/releases/latest")
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let tag = release["tag_name"].as_str().ok_or("no release tag")?;
    let version = tag.trim_start_matches('v');
    let file = actionlint_asset(version).ok_or("actionlint has no build for this machine")?;
    let asset = release["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["name"] == file.as_str())
        .ok_or_else(|| format!("the release has no {file}"))?;
    let url = asset["browser_download_url"]
        .as_str()
        .ok_or("no download URL")?;
    let listing = fetch_text(&format!(
        "https://github.com/rhysd/actionlint/releases/download/{tag}/actionlint_{version}_checksums.txt"
    ))
    .map_err(|e| e.to_string())?;
    let listed = checksum_for(&listing, &file).ok_or("the checksums file doesn't list it")?;
    // Two sources must agree: the release's own file and GitHub's digest.
    if let Some(digest) = asset["digest"]
        .as_str()
        .and_then(|d| d.strip_prefix("sha256:"))
        && !digest.eq_ignore_ascii_case(&listed)
    {
        return Err("the checksums file and GitHub's digest disagree".into());
    }
    let staging = tempfile::tempdir().map_err(|e| e.to_string())?;
    let archive = staging.path().join(&file);
    let actual = download(url, &archive, &|_, _| {}).map_err(|e| e.to_string())?;
    if !actual.eq_ignore_ascii_case(&listed) {
        return Err(format!("SHA-256 mismatch for {file}: refused"));
    }
    let unpacked = staging.path().join("out");
    if file.ends_with(".zip") {
        locdev_runtimes::extract::zip(&archive, &unpacked, false)
    } else {
        locdev_runtimes::extract::tar_gz_flat(&archive, &unpacked)
    }
    .map_err(|e| e.to_string())?;
    let name = if cfg!(windows) {
        "actionlint.exe"
    } else {
        "actionlint"
    };
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::copy(unpacked.join(name), dir.join(name)).map_err(|e| e.to_string())?;
    Ok(version.to_string())
}

fn actionlint_version(path: &str) -> Option<String> {
    let name = if cfg!(windows) {
        "actionlint.exe"
    } else {
        "actionlint"
    };
    let program = std::env::split_paths(path)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())?;
    let output = locdev_core::process::command(program)
        .arg("-version")
        .output()
        .ok()?;
    String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .next()
        .map(str::to_string)
}
