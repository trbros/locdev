//! Wire types for local Git and GitHub.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    /// Where a rename or copy came from.
    pub orig_path: Option<String>,
    /// Change in the index (staged).
    pub staged: Option<ChangeKind>,
    /// Change in the working tree (not staged).
    pub unstaged: Option<ChangeKind>,
    pub untracked: bool,
    /// Unmerged: a merge left conflicts in it.
    pub conflicted: bool,
    /// Lines added/removed (staged + unstaged), when Git can count them.
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

/// An operation Git is in the middle of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoState {
    #[default]
    Clean,
    Merging,
    Rebasing,
    CherryPicking,
    Reverting,
    Bisecting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubRepo {
    pub owner: String,
    pub name: String,
}

impl GithubRepo {
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    pub fn url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner, self.name)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatus {
    /// `None` when HEAD is detached.
    pub branch: Option<String>,
    /// Full sha of HEAD; `None` before the first commit.
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub state: RepoState,
    pub changes: Vec<FileChange>,
    /// The first remote that points at GitHub.
    pub github: Option<GithubRepo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub short: String,
    pub author: String,
    pub email: String,
    /// Seconds since the epoch.
    pub time: u64,
    /// Branches and tags pointing at it.
    pub refs: Vec<String>,
    pub subject: String,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStat {
    pub path: String,
    /// `None` for binary files.
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitDetail {
    pub commit: Commit,
    pub body: String,
    pub files: Vec<FileStat>,
    pub diff: String,
    /// The diff was cut at the size limit.
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    /// `main`, or `origin/main` for remote branches.
    pub name: String,
    pub remote: bool,
    pub sha: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub current: bool,
    pub time: u64,
    pub subject: String,
    /// Fully merged into the current branch (deleting it loses nothing).
    pub merged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stash {
    pub index: u32,
    pub message: String,
    pub time: u64,
}

/// A working tree of the repository (`git worktree list`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worktree {
    pub path: std::path::PathBuf,
    /// Checked-out branch; `None` when detached.
    #[serde(default)]
    pub branch: Option<String>,
    pub head: String,
    /// The repository's main working tree (can't be removed).
    pub main: bool,
    #[serde(default)]
    pub locked: bool,
    /// Its folder is gone; `git worktree prune` forgets it.
    #[serde(default)]
    pub prunable: bool,
    /// The LocDev site registered for this folder, if any.
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remote {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
}

/// One entry of HEAD's reflog: what moved it, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefLogEntry {
    pub sha: String,
    pub message: String,
    pub time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetMode {
    /// Moves the branch; changes stay staged.
    Soft,
    /// Moves the branch; changes stay in the working tree.
    Mixed,
    /// Moves the branch and throws uncommitted changes away.
    Hard,
}

/// Every change LocDev makes to a repository. Reads have their own requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum GitOp {
    Stage {
        paths: Vec<String>,
    },
    Unstage {
        paths: Vec<String>,
    },
    /// Throws away working-tree changes (and deletes untracked files) in `paths`.
    Discard {
        paths: Vec<String>,
    },
    Commit {
        message: String,
    },
    CreateBranch {
        name: String,
        #[serde(default)]
        start: Option<String>,
        #[serde(default)]
        switch: bool,
    },
    Switch {
        name: String,
    },
    DeleteBranch {
        name: String,
        /// Delete even if not merged.
        #[serde(default)]
        force: bool,
    },
    Merge {
        branch: String,
    },
    AbortMerge,
    Fetch {
        #[serde(default)]
        remote: Option<String>,
    },
    /// Fast-forward only: never creates a merge on its own.
    Pull,
    Push {
        /// `--force-with-lease`: rewrites the remote branch.
        #[serde(default)]
        force: bool,
    },
    StashPush {
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        untracked: bool,
    },
    StashApply {
        index: u32,
    },
    StashPop {
        index: u32,
    },
    StashDrop {
        index: u32,
    },
    RemoteAdd {
        name: String,
        url: String,
    },
    RemoteRemove {
        name: String,
    },
    RemoteSetUrl {
        name: String,
        url: String,
    },
    /// A new commit that undoes `sha` (history is kept).
    Revert {
        sha: String,
    },
    /// Puts `paths` back as they were at `sha`, in the working tree.
    RestoreFiles {
        sha: String,
        paths: Vec<String>,
    },
    Reset {
        target: String,
        mode: ResetMode,
    },
    /// A new working tree for `branch` in its own folder, so two branches
    /// run side by side (optionally as their own LocDev site).
    WorktreeAdd {
        branch: String,
        /// Create `branch` (from `start`, or HEAD).
        #[serde(default)]
        create: bool,
        #[serde(default)]
        start: Option<String>,
        /// Where; next to the repository (`<repo>-<branch>`) when omitted.
        #[serde(default)]
        path: Option<std::path::PathBuf>,
        /// Register the folder as a LocDev site.
        #[serde(default)]
        site: bool,
        /// Copy the untracked `.env*` files of the main tree that the new one
        /// lacks (they aren't in Git, and the site needs them).
        #[serde(default)]
        copy_env: bool,
    },
    /// Deletes a working tree's folder (and the site LocDev registered for it).
    WorktreeRemove {
        path: std::path::PathBuf,
        /// Even with uncommitted changes or untracked files.
        #[serde(default)]
        force: bool,
    },
    /// Forgets working trees whose folders are gone.
    WorktreePrune,
}

/// What a destructive operation will lose, shown before it runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestructivePreview {
    pub summary: String,
    /// Concrete things that go away (files, commits, a stash…).
    pub lost: Vec<String>,
    /// Text the user must type to confirm.
    pub phrase: String,
    /// A LocDev password is set and required.
    pub password: bool,
    /// What LocDev saves first, if anything.
    pub backup: Option<String>,
    /// Why it can't run right now (conflicts, a merge in progress…).
    pub blocked: Option<String>,
}

/// The user's confirmation of a destructive operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Confirmation {
    pub phrase: String,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitResult {
    pub message: String,
    /// Git's own output.
    pub output: String,
    /// Where the pre-operation backup was saved.
    pub backup: Option<String>,
}

/// Protection of destructive operations (Settings → Security).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityStatus {
    pub password_set: bool,
    /// Ask for the password on destructive operations (when one is set).
    pub require_password: bool,
    /// Save a backup before destructive operations.
    pub backups: bool,
    /// Locked out after too many wrong passwords, until (ms).
    pub locked_until: Option<u64>,
}

// ── GitHub ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubAuthMethod {
    /// OAuth device flow (needs a registered OAuth App client id).
    Device,
    /// A personal access token pasted by the user.
    Token,
    /// The token of an installed, logged-in GitHub CLI, copied once.
    GhCli,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubAccount {
    pub login: String,
    pub name: Option<String>,
    pub method: GithubAuthMethod,
    /// OAuth scopes (classic tokens); empty for fine-grained tokens.
    pub scopes: Vec<String>,
    /// Where the token is kept ("system keyring").
    pub storage: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    /// Seconds until the code expires.
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubOverview {
    pub repo: GithubRepo,
    pub description: Option<String>,
    pub private: bool,
    pub default_branch: String,
    pub html_url: String,
    pub open_issues: u64,
    pub stars: u64,
    /// Requests left in the current rate-limit window.
    pub rate_remaining: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubItem {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub author: String,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
    pub labels: Vec<String>,
    /// PRs: head → base.
    pub branch: Option<String>,
    pub draft: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: String,
    pub branch: String,
    pub event: String,
    /// queued, in_progress, completed…
    pub status: String,
    /// success, failure, cancelled… once completed.
    pub conclusion: Option<String>,
    pub sha: String,
    pub url: String,
    pub created_at: String,
    pub workflow_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub tag: String,
    pub name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub published_at: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRun {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub url: String,
}

/// What a GitHub tab shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubSection {
    Overview,
    Issues,
    Pulls,
    Workflows,
    Runs,
    Releases,
    Branches,
    Commits,
    Checks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "section", rename_all = "snake_case")]
pub enum GithubData {
    Overview(GithubOverview),
    Issues { items: Vec<GithubItem> },
    Pulls { items: Vec<GithubItem> },
    Workflows { workflows: Vec<Workflow> },
    Runs { runs: Vec<WorkflowRun> },
    Releases { releases: Vec<Release> },
    Branches { names: Vec<String> },
    Commits { commits: Vec<Commit> },
    Checks { sha: String, checks: Vec<CheckRun> },
}

/// Changes on GitHub, each confirmed by the user first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum GithubAction {
    RerunFailed { run_id: u64 },
    CancelRun { run_id: u64 },
    Dispatch { workflow_id: u64, git_ref: String },
}
