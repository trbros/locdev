//! Local Git for a site: reads, changes, and the guard around destructive
//! operations (a preview of what is lost, a typed phrase, the password when
//! one is set, and a backup first). Git runs off the async threads, one
//! operation at a time per site.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use locdev_core::git::{Confirmation, DestructivePreview, GitOp, GitResult, RepoState, ResetMode};
use locdev_core::model::{Project, now_ms};
use locdev_core::{AppError, ErrorCode};
use locdev_git::{GitError, Repo};
use tracing::info;

use super::Daemon;

/// Operations kept in a site's LocDev history.
const HISTORY: usize = 200;

fn git_error(error: GitError) -> AppError {
    AppError::new(ErrorCode::Conflict, error.0)
}

impl Daemon {
    /// Runs `f` on the site's repository, off the async threads, holding the
    /// site's Git lock (Git itself refuses concurrent index changes).
    pub(super) async fn with_repo<T: Send + 'static>(
        &self,
        project: &Project,
        f: impl FnOnce(&Repo) -> Result<T, GitError> + Send + 'static,
    ) -> Result<T, AppError> {
        if !locdev_git::available() {
            return Err(AppError::new(
                ErrorCode::NotRunnable,
                "git is not installed (or not on the daemon's PATH)",
            ));
        }
        let lock = self
            .git_locks
            .lock()
            .unwrap()
            .entry(project.name.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone();
        let _guard = lock.lock().await;
        let path = project.path.clone();
        tokio::task::spawn_blocking(move || {
            let repo =
                Repo::open(&path).map_err(|e| GitError(format!("{}: {e}", path.display())))?;
            f(&repo)
        })
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(git_error)
    }

    pub(super) async fn git_preview(
        &self,
        project: &Project,
        op: GitOp,
    ) -> Result<Option<DestructivePreview>, AppError> {
        let password = self.security.password_required();
        let backups = self.security.backups();
        self.with_repo(project, move |repo| preview(repo, &op, password, backups))
            .await
    }

    /// Runs `op`; destructive ones only with a matching confirmation.
    pub(super) async fn git_run(
        &self,
        project: &Project,
        op: GitOp,
        confirm: Option<Confirmation>,
    ) -> Result<GitResult, AppError> {
        let preview = self.git_preview(project, op.clone()).await?;
        if let Err(error) = self.check_confirmation(preview.as_ref(), confirm) {
            // Refusals are part of the history too.
            self.git_log_append(
                &project.name,
                format!(
                    "{} refused {}: {}",
                    locdev_core::model::civil_date(now_ms()),
                    describe(&op),
                    error.message
                ),
            );
            return Err(error);
        }
        // Confirmed: the worktree's own site goes down first, or its server
        // keeps the folder open and Windows refuses to delete it.
        if let GitOp::WorktreeRemove { path, .. } = &op {
            let site = self
                .registry
                .lock()
                .await
                .all()
                .iter()
                .find(|p| p.name != project.name && locdev_git::same_path(&p.path, path))
                .map(|p| p.name.clone());
            if let Some(site) = site {
                self.tunnel_stop(&site).await;
                self.stop_all_of(&site).await;
            }
        }
        self.finish_git(project, op, preview.is_some()).await
    }

    fn check_confirmation(
        &self,
        preview: Option<&DestructivePreview>,
        confirm: Option<Confirmation>,
    ) -> Result<(), AppError> {
        if let Some(preview) = preview {
            if let Some(reason) = &preview.blocked {
                return Err(AppError::new(ErrorCode::Conflict, reason.clone()));
            }
            let confirm = confirm.ok_or_else(|| {
                AppError::new(
                    ErrorCode::Unauthorized,
                    format!(
                        "{} Type \"{}\" to confirm.",
                        preview.summary, preview.phrase
                    ),
                )
            })?;
            if confirm.phrase.trim() != preview.phrase {
                return Err(AppError::new(
                    ErrorCode::Unauthorized,
                    format!(
                        "the confirmation doesn't match; type \"{}\"",
                        preview.phrase
                    ),
                ));
            }
            if preview.password {
                self.security.verify(confirm.password.as_deref())?;
            }
        }
        Ok(())
    }

    async fn finish_git(
        &self,
        project: &Project,
        op: GitOp,
        destructive: bool,
    ) -> Result<GitResult, AppError> {
        let backups = destructive && self.security.backups();
        let stamp = format!("{}-{}", now_ms(), op_name(&op));
        let backup_dir = self
            .paths
            .data_dir
            .join("backups")
            .join(&project.name)
            .join(&stamp);
        let label = describe(&op);
        let result = self
            .with_repo(project, move |repo| {
                let backup = if backups {
                    save_backup(repo, &op, &stamp, &backup_dir)?
                } else {
                    None
                };
                let output = execute(repo, &op)?;
                Ok(GitResult {
                    message: done_message(&op),
                    output,
                    backup,
                })
            })
            .await;

        let line = match &result {
            Ok(r) => format!(
                "{} ok {label}{}",
                locdev_core::model::civil_date(now_ms()),
                r.backup
                    .as_deref()
                    .map(|b| format!(" (backup {b})"))
                    .unwrap_or_default()
            ),
            Err(e) => format!(
                "{} failed {label}: {}",
                locdev_core::model::civil_date(now_ms()),
                e.message
            ),
        };
        self.git_log_append(&project.name, line);
        info!(project = %project.name, op = %label, ok = result.is_ok(), "git operation");
        result
    }

    fn git_log_path(&self, project: &str) -> PathBuf {
        self.paths
            .data_dir
            .join("state")
            .join("git")
            .join(format!("{project}.log"))
    }

    fn git_log_append(&self, project: &str, line: String) {
        let path = self.git_log_path(project);
        let mut lines = self.git_operations(project);
        lines.insert(0, line);
        lines.truncate(HISTORY);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = crate::fsutil::write_atomic(&path, lines.join("\n").as_bytes());
    }

    /// Operations LocDev ran on the site's repository, newest first.
    pub(super) fn git_operations(&self, project: &str) -> Vec<String> {
        std::fs::read_to_string(self.git_log_path(project))
            .map(|t| t.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }
}

fn op_name(op: &GitOp) -> &'static str {
    match op {
        GitOp::Stage { .. } => "stage",
        GitOp::Unstage { .. } => "unstage",
        GitOp::Discard { .. } => "discard",
        GitOp::Commit { .. } => "commit",
        GitOp::CreateBranch { .. } => "create-branch",
        GitOp::Switch { .. } => "switch",
        GitOp::DeleteBranch { .. } => "delete-branch",
        GitOp::Merge { .. } => "merge",
        GitOp::AbortMerge => "abort-merge",
        GitOp::Fetch { .. } => "fetch",
        GitOp::Pull => "pull",
        GitOp::Push { .. } => "push",
        GitOp::StashPush { .. } => "stash",
        GitOp::StashApply { .. } => "stash-apply",
        GitOp::StashPop { .. } => "stash-pop",
        GitOp::StashDrop { .. } => "stash-drop",
        GitOp::RemoteAdd { .. } => "remote-add",
        GitOp::RemoteRemove { .. } => "remote-remove",
        GitOp::RemoteSetUrl { .. } => "remote-set-url",
        GitOp::Revert { .. } => "revert",
        GitOp::RestoreFiles { .. } => "restore",
        GitOp::Reset { .. } => "reset",
        GitOp::WorktreeAdd { .. } => "worktree-add",
        GitOp::WorktreeRemove { .. } => "worktree-remove",
        GitOp::WorktreePrune => "worktree-prune",
    }
}

fn describe(op: &GitOp) -> String {
    match op {
        GitOp::Stage { paths } | GitOp::Unstage { paths } | GitOp::Discard { paths } => {
            format!("{} {}", op_name(op), paths.join(", "))
        }
        GitOp::Commit { message } => format!("commit \"{}\"", message.lines().next().unwrap_or("")),
        GitOp::CreateBranch { name, .. }
        | GitOp::Switch { name }
        | GitOp::DeleteBranch { name, .. } => {
            format!("{} {name}", op_name(op))
        }
        GitOp::Merge { branch } => format!("merge {branch}"),
        GitOp::Push { force: true } => "push --force-with-lease".into(),
        GitOp::StashApply { index } | GitOp::StashPop { index } | GitOp::StashDrop { index } => {
            format!("{} stash@{{{index}}}", op_name(op))
        }
        GitOp::RemoteAdd { name, .. }
        | GitOp::RemoteRemove { name }
        | GitOp::RemoteSetUrl { name, .. } => {
            format!("{} {name}", op_name(op))
        }
        GitOp::Revert { sha } => format!("revert {}", short(sha)),
        GitOp::RestoreFiles { sha, paths } => {
            format!("restore {} from {}", paths.join(", "), short(sha))
        }
        GitOp::Reset { target, mode } => format!(
            "reset --{} {}",
            format!("{mode:?}").to_lowercase(),
            short(target)
        ),
        GitOp::WorktreeAdd { branch, path, .. } => format!(
            "worktree add {branch}{}",
            path.as_ref()
                .map(|p| format!(" at {}", p.display()))
                .unwrap_or_default()
        ),
        GitOp::WorktreeRemove { path, .. } => format!("worktree remove {}", path.display()),
        _ => op_name(op).to_string(),
    }
}

fn short(sha: &str) -> &str {
    &sha[..sha.len().min(10)]
}

fn done_message(op: &GitOp) -> String {
    match op {
        GitOp::Commit { .. } => "committed".into(),
        GitOp::Discard { paths } => format!("discarded changes in {} file(s)", paths.len()),
        GitOp::Pull => "pulled (fast-forward)".into(),
        GitOp::Push { .. } => "pushed".into(),
        GitOp::Fetch { .. } => "fetched".into(),
        other => format!("{} done", op_name(other).replace('-', " ")),
    }
}

/// What a destructive operation loses; `None` for safe ones.
fn preview(
    repo: &Repo,
    op: &GitOp,
    password: bool,
    backups: bool,
) -> Result<Option<DestructivePreview>, GitError> {
    let status = repo.status()?;
    let conflicts = status.changes.iter().any(|c| c.conflicted);
    let snapshot =
        backups.then(|| "a snapshot of uncommitted changes under refs/locdev/backups/".to_string());
    let make = |summary: String,
                lost: Vec<String>,
                phrase: String,
                backup: Option<String>,
                blocked: Option<String>| {
        Some(DestructivePreview {
            summary,
            lost,
            phrase,
            password,
            backup: if backups { backup } else { None },
            blocked,
        })
    };

    Ok(match op {
        GitOp::Discard { paths } => {
            let lost: Vec<String> = paths
                .iter()
                .map(|p| match status.changes.iter().find(|c| &c.path == p) {
                    Some(c) if c.untracked => format!("{p} (untracked file, deleted)"),
                    Some(_) => format!("{p} (uncommitted changes)"),
                    None => format!("{p} (no changes)"),
                })
                .collect();
            make(
                format!(
                    "Discarding throws away uncommitted changes in {} file(s).",
                    paths.len()
                ),
                lost,
                "discard".into(),
                Some("a snapshot of tracked changes, and copies of untracked files".into()),
                None,
            )
        }
        GitOp::DeleteBranch { name, force } => {
            let branch = repo
                .branches()?
                .into_iter()
                .find(|b| !b.remote && &b.name == name);
            match branch {
                None => None,
                Some(b) if b.current => make(
                    "You can't delete the branch you're on.".into(),
                    vec![],
                    name.clone(),
                    None,
                    Some("switch to another branch first".into()),
                ),
                Some(b) if b.merged && !force => None,
                Some(_) => {
                    let lost: Vec<String> = repo
                        .unique_commits(name)?
                        .iter()
                        .map(|c| format!("{} {}", c.short, c.subject))
                        .collect();
                    make(
                        format!(
                            "{name} has {} commit(s) that aren't in your current branch.",
                            lost.len()
                        ),
                        lost,
                        name.clone(),
                        Some(format!("a ref to {name}'s last commit")),
                        None,
                    )
                }
            }
        }
        GitOp::Push { force: true } => {
            let Some(upstream) = status.upstream.clone() else {
                return Ok(make(
                    "There is no upstream branch to overwrite.".into(),
                    vec![],
                    String::new(),
                    None,
                    Some("push normally first".into()),
                ));
            };
            let lost: Vec<String> = repo
                .unique_commits(&upstream)?
                .iter()
                .map(|c| format!("{} {} (on {upstream})", c.short, c.subject))
                .collect();
            make(
                format!(
                    "Force-pushing rewrites {upstream}: commits there that you don't have are removed for everyone."
                ),
                lost,
                status.branch.clone().unwrap_or_default(),
                Some(format!("a ref to {upstream}'s current commit")),
                None,
            )
        }
        GitOp::StashDrop { index } => {
            let stash = repo.stashes()?.into_iter().find(|s| s.index == *index);
            let mut lost: Vec<String> = stash.iter().map(|s| s.message.clone()).collect();
            lost.extend(repo.stash_files(*index).unwrap_or_default());
            make(
                format!("Dropping stash@{{{index}}} deletes the changes saved in it."),
                lost,
                "drop".into(),
                Some("a ref to the stash's commit".into()),
                None,
            )
        }
        GitOp::Reset { target, mode } => {
            let blocked = (status.state != RepoState::Clean || conflicts)
                .then(|| "finish or abort the merge/rebase in progress first".to_string());
            let mut lost: Vec<String> = repo
                .commits_between(target)?
                .iter()
                .map(|c| {
                    format!(
                        "{} {} (off the branch; still in the reflog)",
                        c.short, c.subject
                    )
                })
                .collect();
            if *mode == ResetMode::Hard {
                lost.extend(
                    status
                        .changes
                        .iter()
                        .filter(|c| !c.untracked)
                        .map(|c| format!("{} (uncommitted changes)", c.path)),
                );
            }
            let target_sha = repo.check_rev(target)?;
            make(
                format!(
                    "Resetting moves {} to {}{}.",
                    status.branch.as_deref().unwrap_or("HEAD"),
                    short(&target_sha),
                    if *mode == ResetMode::Hard {
                        " and throws uncommitted changes away"
                    } else {
                        ""
                    }
                ),
                lost,
                short(&target_sha).to_string(),
                Some("a ref to the current commit, plus a snapshot of uncommitted changes".into()),
                blocked,
            )
        }
        GitOp::RestoreFiles { sha, paths } => {
            let lost: Vec<String> = paths
                .iter()
                .filter(|p| status.changes.iter().any(|c| &c.path == *p))
                .map(|p| format!("{p} (uncommitted changes)"))
                .collect();
            make(
                format!(
                    "Restoring overwrites {} file(s) with their version from {}.",
                    paths.len(),
                    short(sha)
                ),
                lost,
                "restore".into(),
                snapshot.clone(),
                None,
            )
        }
        GitOp::AbortMerge => make(
            "Aborting the merge throws away your conflict resolutions.".into(),
            status
                .changes
                .iter()
                .filter(|c| c.conflicted)
                .map(|c| c.path.clone())
                .collect(),
            "abort".into(),
            snapshot.clone(),
            (status.state != RepoState::Merging).then(|| "no merge in progress".to_string()),
        ),
        GitOp::RemoteRemove { name } => make(
            format!(
                "Removing {name} deletes its remote-tracking branches here (nothing on the server changes)."
            ),
            repo.branches()?
                .into_iter()
                .filter(|b| b.remote && b.name.starts_with(&format!("{name}/")))
                .map(|b| b.name)
                .collect(),
            name.clone(),
            None,
            None,
        ),
        GitOp::WorktreeRemove { path, force } => {
            let trees = repo.worktrees()?;
            let tree = trees.iter().find(|w| locdev_git::same_path(&w.path, path));
            let blocked = match tree {
                None => Some(format!(
                    "{} is not a working tree of this repository",
                    path.display()
                )),
                Some(t) if t.main => Some("the main working tree can't be removed".to_string()),
                Some(t) if t.locked => {
                    Some("the working tree is locked (`git worktree unlock` first)".to_string())
                }
                _ => None,
            };
            let changes = match (&blocked, Repo::open(path)) {
                (None, Ok(tree)) => tree.status()?.changes,
                _ => Vec::new(),
            };
            let blocked = blocked.or_else(|| {
                (!changes.is_empty() && !force).then(|| {
                    format!(
                        "{} has {} uncommitted change(s); remove it with force to throw them away",
                        path.display(),
                        changes.len()
                    )
                })
            });
            let mut lost = vec![format!("the folder {}", path.display())];
            lost.extend(changes.iter().map(|c| {
                if c.untracked {
                    format!("{} (untracked file)", c.path)
                } else {
                    format!("{} (uncommitted changes)", c.path)
                }
            }));
            let phrase = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "remove".into());
            make(
                "Removing a working tree deletes its folder; its branch and commits stay in the repository.".into(),
                lost,
                phrase,
                Some("a snapshot of its tracked changes under refs/locdev/backups/".into()),
                blocked,
            )
        }
        _ => None,
    })
}

/// Saves what the operation is about to lose. Returns where.
fn save_backup(
    repo: &Repo,
    op: &GitOp,
    stamp: &str,
    dir: &Path,
) -> Result<Option<String>, GitError> {
    let mut saved = Vec::new();
    match op {
        GitOp::Discard { paths } => {
            if let Some(r) = repo.backup(stamp)? {
                saved.push(r);
            }
            // `git stash create` doesn't hold untracked files: copy them.
            let status = repo.status()?;
            for path in paths
                .iter()
                .filter(|p| status.changes.iter().any(|c| c.untracked && &c.path == *p))
            {
                let target = dir.join(path);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| GitError(e.to_string()))?;
                }
                std::fs::copy(repo.root().join(path), &target)
                    .map_err(|e| GitError(format!("backup of {path}: {e}")))?;
            }
            if dir.exists() {
                saved.push(dir.display().to_string());
            }
        }
        GitOp::Reset { .. } | GitOp::RestoreFiles { .. } | GitOp::AbortMerge => {
            if let Some(head) = repo.head() {
                saved.push(repo.keep_ref(&format!("{stamp}-head"), &head)?);
            }
            if let Some(r) = repo.backup(stamp)? {
                saved.push(r);
            }
        }
        GitOp::DeleteBranch { name, .. } => {
            let sha = repo.check_rev(name)?;
            saved.push(repo.keep_ref(stamp, &sha)?);
        }
        GitOp::StashDrop { index } => {
            let sha = repo.stash_sha(*index)?;
            saved.push(repo.keep_ref(stamp, &sha)?);
        }
        GitOp::WorktreeRemove { path, .. } => {
            // Refs are shared between working trees: the snapshot outlives it.
            if let Some(r) = Repo::open(path)?.backup(stamp)? {
                saved.push(r);
            }
        }
        GitOp::Push { force: true } => {
            if let Some(upstream) = repo.status()?.upstream {
                let sha = repo.check_rev(&upstream)?;
                saved.push(repo.keep_ref(stamp, &sha)?);
            }
        }
        _ => {}
    }
    Ok((!saved.is_empty()).then(|| saved.join(", ")))
}

fn execute(repo: &Repo, op: &GitOp) -> Result<String, GitError> {
    match op {
        GitOp::Stage { paths } => repo.stage(paths),
        GitOp::Unstage { paths } => repo.unstage(paths),
        GitOp::Discard { paths } => repo.discard(paths),
        GitOp::Commit { message } => repo.commit(message),
        GitOp::CreateBranch {
            name,
            start,
            switch,
        } => repo.create_branch(name, start.as_deref(), *switch),
        GitOp::Switch { name } => repo.switch(name),
        GitOp::DeleteBranch { name, force } => {
            // Unmerged branches need -D; the preview already asked about that.
            let merged = repo
                .branches()?
                .iter()
                .any(|b| !b.remote && &b.name == name && b.merged);
            repo.delete_branch(name, *force || !merged)
        }
        GitOp::Merge { branch } => repo.merge(branch),
        GitOp::AbortMerge => repo.abort_merge(),
        GitOp::Fetch { remote } => repo.fetch(remote.as_deref()),
        GitOp::Pull => repo.pull(),
        GitOp::Push { force } => repo.push(*force),
        GitOp::StashPush { message, untracked } => repo.stash_push(message.as_deref(), *untracked),
        GitOp::StashApply { index } => repo.stash_apply(*index, false),
        GitOp::StashPop { index } => repo.stash_apply(*index, true),
        GitOp::StashDrop { index } => repo.stash_drop(*index),
        GitOp::RemoteAdd { name, url } => repo.remote_add(name, url),
        GitOp::RemoteRemove { name } => repo.remote_remove(name),
        GitOp::RemoteSetUrl { name, url } => repo.remote_set_url(name, url),
        GitOp::Revert { sha } => repo.revert(sha),
        GitOp::RestoreFiles { sha, paths } => repo.restore_files(sha, paths),
        GitOp::Reset { target, mode } => repo.reset(target, *mode),
        GitOp::WorktreeAdd {
            branch,
            create,
            start,
            path,
            ..
        } => {
            let path = path
                .as_deref()
                .ok_or_else(|| GitError("no folder for the working tree".into()))?;
            repo.worktree_add(path, branch, *create, start.as_deref())
        }
        GitOp::WorktreeRemove { path, force } => repo.worktree_remove(path, *force),
        GitOp::WorktreePrune => repo.worktree_prune(),
    }
}

/// `feature/login` → `feature-login`: a folder and site name.
pub(super) fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

impl Daemon {
    /// Working trees of the site's repository, with the site of each folder.
    pub(super) async fn git_worktrees(
        &self,
        project: &Project,
    ) -> Result<Vec<locdev_core::git::Worktree>, AppError> {
        let mut trees = self.with_repo(project, |repo| repo.worktrees()).await?;
        let registry = self.registry.lock().await;
        for tree in &mut trees {
            tree.site = registry
                .all()
                .iter()
                .find(|p| locdev_git::same_path(&p.path, &tree.path))
                .map(|p| p.name.clone());
        }
        Ok(trees)
    }

    /// Fills what the client may leave out: a new working tree goes next to
    /// the repository, as `<repo>-<branch>`. The daemon decides the folder.
    pub(super) fn worktree_defaults(
        &self,
        project: &Project,
        op: GitOp,
    ) -> Result<GitOp, AppError> {
        let GitOp::WorktreeAdd {
            branch,
            create,
            start,
            path,
            site,
            copy_env,
        } = op
        else {
            return Ok(op);
        };
        let path = match path {
            Some(path) if path.is_absolute() => path,
            Some(_) => {
                return Err(AppError::invalid(
                    "the working tree folder must be an absolute path",
                ));
            }
            None => {
                let parent = project.path.parent().ok_or_else(|| {
                    AppError::invalid("the site's folder has no parent for a working tree")
                })?;
                let base = project
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| project.name.clone());
                parent.join(format!("{base}-{}", slug(&branch)))
            }
        };
        Ok(GitOp::WorktreeAdd {
            branch,
            create,
            start,
            path: Some(path),
            site,
            copy_env,
        })
    }

    /// After a worktree change: copy `.env` files and register the new folder
    /// as a site, or remove the site of a folder that is gone. A failure here
    /// is reported in the result; the Git part already happened.
    pub(super) async fn worktree_after(
        &self,
        project: &Project,
        op: &GitOp,
        mut result: GitResult,
    ) -> GitResult {
        match op {
            GitOp::WorktreeAdd {
                path: Some(path),
                site,
                copy_env,
                ..
            } => {
                if *copy_env {
                    let copied = copy_env_files(&project.path, path);
                    if !copied.is_empty() {
                        result
                            .output
                            .push_str(&format!("\ncopied {}", copied.join(", ")));
                    }
                }
                if *site {
                    let name = path
                        .file_name()
                        .map(|n| slug(&n.to_string_lossy()))
                        .unwrap_or_default();
                    match self.add(path.clone(), Some(name), None, None, false).await {
                        Ok(_) => result.message.push_str(" and registered as a site"),
                        Err(e) => result
                            .output
                            .push_str(&format!("\nnot registered as a site: {}", e.message)),
                    }
                }
            }
            GitOp::WorktreeRemove { path, .. } => {
                let site = self
                    .registry
                    .lock()
                    .await
                    .all()
                    .iter()
                    .find(|p| p.name != project.name && locdev_git::same_path(&p.path, path))
                    .map(|p| p.name.clone());
                if let Some(site) = site {
                    match self.remove_site(&site).await {
                        Ok(()) => result.message.push_str(&format!("; site {site} removed")),
                        Err(e) => result
                            .output
                            .push_str(&format!("\nsite {site} not removed: {}", e.message)),
                    }
                }
            }
            _ => {}
        }
        result
    }
}

/// Untracked `.env*` files of the main tree that `to` lacks; never overwrites.
fn copy_env_files(from: &Path, to: &Path) -> Vec<String> {
    let mut copied = Vec::new();
    let Ok(entries) = std::fs::read_dir(from) else {
        return copied;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(".env") || !entry.path().is_file() {
            continue;
        }
        let target = to.join(&name);
        if target.exists() {
            continue;
        }
        if std::fs::copy(entry.path(), &target).is_ok() {
            copied.push(name);
        }
    }
    copied.sort();
    copied
}
