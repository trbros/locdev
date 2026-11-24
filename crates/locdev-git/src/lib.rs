//! Local Git through the `git` program, never a shell: every call is a
//! program plus arguments, paths come after `--`, refs after
//! `--end-of-options`, and names are checked with `git check-ref-format`.
//! Using the user's own `git` keeps their config, hooks, credential helpers
//! and SSH agent; nothing here prompts (`GIT_TERMINAL_PROMPT=0`).

use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use locdev_core::git::{
    Branch, ChangeKind, Commit, CommitDetail, FileChange, FileStat, GitStatus, GithubRepo,
    RefLogEntry, Remote, RepoState, ResetMode, Stash, Worktree,
};

/// Diffs larger than this are cut (the UI is not an editor).
const MAX_DIFF: usize = 512 * 1024;
/// Network operations give up after this.
const NETWORK_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError(pub String);

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GitError {}

type Result<T> = std::result::Result<T, GitError>;

pub struct Repo {
    root: PathBuf,
}

/// Whether `git` is installed at all.
pub fn available() -> bool {
    locdev_core::process::command("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

impl Repo {
    /// The repository containing `path`, if it is in one.
    pub fn open(path: &Path) -> Result<Self> {
        let out = raw(path, &["rev-parse", "--show-toplevel"], None, None)?;
        if !out.success {
            return Err(GitError("not a Git repository".into()));
        }
        Ok(Self {
            root: PathBuf::from(out.stdout.trim()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn git(&self, args: &[&str]) -> Result<String> {
        let out = raw(&self.root, args, None, None)?;
        if out.success {
            Ok(out.stdout)
        } else {
            Err(GitError(error_text(&out)))
        }
    }

    /// Runs a command that changes things and returns everything it said.
    fn run(&self, args: &[&str]) -> Result<String> {
        let out = raw(&self.root, args, None, Some(NETWORK_TIMEOUT))?;
        let text = format!("{}{}", out.stdout, out.stderr).trim().to_string();
        if out.success {
            Ok(text)
        } else {
            Err(GitError(error_text(&out)))
        }
    }

    /// Checks that `paths` are plain relative paths inside the repository.
    fn check_paths<'a>(&self, paths: &'a [String]) -> Result<Vec<&'a str>> {
        if paths.is_empty() {
            return Err(GitError("no files given".into()));
        }
        paths
            .iter()
            .map(|p| {
                let path = Path::new(p);
                let ok = !p.is_empty()
                    && path
                        .components()
                        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
                if ok {
                    Ok(p.as_str())
                } else {
                    Err(GitError(format!(
                        "'{p}' is not a path inside the repository"
                    )))
                }
            })
            .collect()
    }

    /// Whether `path` is tracked (in the index), e.g. a `.env` that was committed.
    pub fn tracked(&self, path: &str) -> Result<bool> {
        let paths = [path.to_string()];
        let path = self.check_paths(&paths)?[0];
        Ok(raw(
            &self.root,
            &["ls-files", "--error-unmatch", "--", path],
            None,
            None,
        )?
        .success)
    }

    /// Whether `.gitignore` rules (or the global ones) ignore `path`.
    pub fn ignored(&self, path: &str) -> Result<bool> {
        let paths = [path.to_string()];
        let path = self.check_paths(&paths)?[0];
        Ok(raw(&self.root, &["check-ignore", "-q", "--", path], None, None)?.success)
    }

    /// A branch or ref name Git itself accepts (and that can't pass for an option).
    pub fn check_ref(&self, name: &str) -> Result<()> {
        if name.is_empty() || name.starts_with('-') {
            return Err(GitError(format!("'{name}' is not a valid name")));
        }
        let out = raw(
            &self.root,
            &["check-ref-format", "--branch", name],
            None,
            None,
        )?;
        if out.success {
            Ok(())
        } else {
            Err(GitError(format!("'{name}' is not a valid branch name")))
        }
    }

    /// A revision (sha, branch, `HEAD~2`) that exists.
    pub fn check_rev(&self, rev: &str) -> Result<String> {
        if rev.is_empty() || rev.starts_with('-') {
            return Err(GitError(format!("'{rev}' is not a revision")));
        }
        let sha = self
            .git(&[
                "rev-parse",
                "--verify",
                "--quiet",
                "--end-of-options",
                &format!("{rev}^{{commit}}"),
            ])
            .map_err(|_| GitError(format!("'{rev}' is not a commit in this repository")))?;
        Ok(sha.trim().to_string())
    }

    pub fn status(&self) -> Result<GitStatus> {
        let out = self.git(&[
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
        ])?;
        let mut status = parse_status(&out);
        status.state = self.state();
        // Line counts for both halves; binary files have none.
        for staged in [false, true] {
            let mut args = vec!["diff", "--numstat", "-z", "--no-renames"];
            if staged {
                args.push("--cached");
            }
            if let Ok(numstat) = self.git(&args) {
                for stat in parse_numstat(&numstat) {
                    if let Some(change) = status.changes.iter_mut().find(|c| c.path == stat.path) {
                        change.added = Some(change.added.unwrap_or(0) + stat.added.unwrap_or(0));
                        change.removed =
                            Some(change.removed.unwrap_or(0) + stat.removed.unwrap_or(0));
                    }
                }
            }
        }
        status.github = self
            .remotes()
            .ok()
            .and_then(|remotes| remotes.iter().find_map(|r| github_repo(&r.fetch_url)));
        Ok(status)
    }

    fn state(&self) -> RepoState {
        let Ok(dir) = self.git(&["rev-parse", "--git-dir"]) else {
            return RepoState::Clean;
        };
        let dir = self.root.join(dir.trim());
        if dir.join("MERGE_HEAD").exists() {
            RepoState::Merging
        } else if dir.join("rebase-merge").exists() || dir.join("rebase-apply").exists() {
            RepoState::Rebasing
        } else if dir.join("CHERRY_PICK_HEAD").exists() {
            RepoState::CherryPicking
        } else if dir.join("REVERT_HEAD").exists() {
            RepoState::Reverting
        } else if dir.join("BISECT_LOG").exists() {
            RepoState::Bisecting
        } else {
            RepoState::Clean
        }
    }

    /// One file's diff: staged, unstaged, or (untracked) the whole file.
    pub fn diff(&self, path: &str, staged: bool) -> Result<(String, bool)> {
        let path = self.check_paths(std::slice::from_ref(&path.to_string()))?[0].to_string();
        let tracked = raw(
            &self.root,
            &["ls-files", "--error-unmatch", "--", &path],
            None,
            None,
        )?
        .success;
        let out = if tracked {
            let mut args = vec!["diff", "--no-color", "--no-ext-diff"];
            if staged {
                args.push("--cached");
            }
            args.extend(["--", &path]);
            self.git(&args)?
        } else {
            // `--no-index` exits 1 when files differ, which is the point.
            raw(
                &self.root,
                &[
                    "diff",
                    "--no-color",
                    "--no-index",
                    "--",
                    null_device(),
                    &path,
                ],
                None,
                None,
            )?
            .stdout
        };
        Ok(cut(out))
    }

    pub fn stage(&self, paths: &[String]) -> Result<String> {
        let paths = self.check_paths(paths)?;
        let mut args = vec!["add", "--"];
        args.extend(paths);
        self.run(&args)
    }

    pub fn unstage(&self, paths: &[String]) -> Result<String> {
        let paths = self.check_paths(paths)?;
        // Before the first commit there is no HEAD to restore from.
        let mut args = if self.head().is_some() {
            vec!["restore", "--staged", "--"]
        } else {
            vec!["rm", "--cached", "--quiet", "--"]
        };
        args.extend(paths);
        self.run(&args)
    }

    /// Throws away working-tree changes; untracked files among `paths` are deleted.
    pub fn discard(&self, paths: &[String]) -> Result<String> {
        let paths = self.check_paths(paths)?;
        let status = self.status()?;
        let (untracked, tracked): (Vec<&str>, Vec<&str>) = paths
            .into_iter()
            .partition(|p| status.changes.iter().any(|c| c.untracked && c.path == *p));
        let mut out = String::new();
        if !tracked.is_empty() {
            let mut args = vec!["restore", "--worktree", "--"];
            args.extend(tracked);
            out += &self.run(&args)?;
        }
        if !untracked.is_empty() {
            let mut args = vec!["clean", "-f", "--"];
            args.extend(untracked);
            out += &self.run(&args)?;
        }
        Ok(out)
    }

    pub fn commit(&self, message: &str) -> Result<String> {
        if message.trim().is_empty() {
            return Err(GitError("the commit message is empty".into()));
        }
        // The message goes through stdin, never through an argument.
        let out = raw(
            &self.root,
            &["commit", "--file=-"],
            Some(message.as_bytes()),
            Some(NETWORK_TIMEOUT),
        )?;
        let text = format!("{}{}", out.stdout, out.stderr).trim().to_string();
        if out.success {
            Ok(text)
        } else {
            Err(GitError(error_text(&out)))
        }
    }

    pub fn head(&self) -> Option<String> {
        self.git(&["rev-parse", "--verify", "--quiet", "HEAD"])
            .ok()
            .map(|s| s.trim().to_string())
    }

    pub fn log(&self, rev: Option<&str>, skip: usize, limit: usize) -> Result<Vec<Commit>> {
        if self.head().is_none() {
            return Ok(Vec::new());
        }
        let rev = match rev {
            Some(rev) => self.check_rev(rev)?,
            None => "HEAD".into(),
        };
        let skip = format!("--skip={skip}");
        let max = format!("--max-count={}", limit.clamp(1, 500));
        let out = self.git(&[
            "log",
            LOG_FORMAT,
            "-z",
            &skip,
            &max,
            "--end-of-options",
            &rev,
        ])?;
        Ok(out
            .split('\0')
            .filter(|s| !s.trim().is_empty())
            .filter_map(parse_commit)
            .collect())
    }

    pub fn show(&self, rev: &str) -> Result<CommitDetail> {
        let sha = self.check_rev(rev)?;
        let commit = self
            .git(&[
                "log",
                LOG_FORMAT,
                "-z",
                "--max-count=1",
                "--end-of-options",
                &sha,
            ])?
            .split('\0')
            .find_map(parse_commit)
            .ok_or_else(|| GitError(format!("no commit {rev}")))?;
        let body = self.git(&[
            "log",
            "--format=%b",
            "--max-count=1",
            "--end-of-options",
            &sha,
        ])?;
        let files = parse_numstat(&self.git(&[
            "show",
            "--numstat",
            "-z",
            "--format=",
            "--no-renames",
            "--end-of-options",
            &sha,
        ])?);
        let (diff, truncated) = cut(self.git(&[
            "show",
            "--no-color",
            "--no-ext-diff",
            "--format=",
            "--end-of-options",
            &sha,
        ])?);
        Ok(CommitDetail {
            commit,
            body: body.trim().to_string(),
            files,
            diff,
            truncated,
        })
    }

    pub fn branches(&self) -> Result<Vec<Branch>> {
        let format = "--format=%(refname)%1f%(refname:short)%1f%(objectname:short)%1f%(upstream:short)%1f%(upstream:track,nobracket)%1f%(HEAD)%1f%(committerdate:unix)%1f%(contents:subject)";
        let out = self.git(&["for-each-ref", format, "refs/heads", "refs/remotes"])?;
        let merged: Vec<String> = if self.head().is_some() {
            self.git(&["branch", "--merged", "HEAD", "--format=%(refname:short)"])
                .map(|o| o.lines().map(str::to_string).collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(out
            .lines()
            .filter_map(|line| {
                let f: Vec<&str> = line.split('\u{1f}').collect();
                if f.len() < 8 || f[0].ends_with("/HEAD") {
                    return None;
                }
                let remote = f[0].starts_with("refs/remotes/");
                let (ahead, behind) = parse_track(f[4]);
                Some(Branch {
                    name: f[1].to_string(),
                    remote,
                    sha: f[2].to_string(),
                    upstream: (!f[3].is_empty()).then(|| f[3].to_string()),
                    ahead,
                    behind,
                    current: f[5] == "*",
                    time: f[6].parse().unwrap_or(0),
                    subject: f[7].to_string(),
                    merged: !remote && merged.iter().any(|m| m == f[1]),
                })
            })
            .collect())
    }

    pub fn create_branch(&self, name: &str, start: Option<&str>, switch: bool) -> Result<String> {
        self.check_ref(name)?;
        let start = match start {
            Some(start) => self.check_rev(start)?,
            None => "HEAD".into(),
        };
        if switch {
            self.run(&["switch", "--create", name, "--end-of-options", &start])
        } else {
            self.run(&["branch", "--end-of-options", name, &start])
        }
    }

    /// Switches branches; Git refuses when local changes would be overwritten.
    pub fn switch(&self, name: &str) -> Result<String> {
        if self.branches()?.iter().any(|b| !b.remote && b.name == name) {
            self.check_ref(name)?;
            return self.run(&["switch", "--end-of-options", name]);
        }
        // A remote branch (`origin/feature`): create the local tracking one.
        if let Some(remote) = self.branches()?.iter().find(|b| b.remote && b.name == name) {
            let local = remote.name.split_once('/').map_or(name, |(_, rest)| rest);
            self.check_ref(local)?;
            return self.run(&[
                "switch",
                "--create",
                local,
                "--track",
                "--end-of-options",
                name,
            ]);
        }
        Err(GitError(format!("no branch named '{name}'")))
    }

    pub fn delete_branch(&self, name: &str, force: bool) -> Result<String> {
        self.check_ref(name)?;
        self.run(&[
            "branch",
            if force { "-D" } else { "-d" },
            "--end-of-options",
            name,
        ])
    }

    /// Merges `branch`; on conflicts the merge stays in progress for the user.
    pub fn merge(&self, branch: &str) -> Result<String> {
        let sha = self.check_rev(branch)?;
        let out = raw(
            &self.root,
            &["merge", "--no-edit", "--end-of-options", &sha],
            None,
            Some(NETWORK_TIMEOUT),
        )?;
        let text = format!("{}{}", out.stdout, out.stderr).trim().to_string();
        if out.success {
            Ok(text)
        } else if self.state() == RepoState::Merging {
            Err(GitError(format!(
                "merge stopped with conflicts; resolve them or abort the merge\n{text}"
            )))
        } else {
            Err(GitError(error_text(&out)))
        }
    }

    pub fn abort_merge(&self) -> Result<String> {
        self.run(&["merge", "--abort"])
    }

    pub fn fetch(&self, remote: Option<&str>) -> Result<String> {
        match remote {
            Some(remote) => {
                self.check_remote(remote)?;
                self.run(&["fetch", "--prune", "--end-of-options", remote])
            }
            None => self.run(&["fetch", "--all", "--prune"]),
        }
    }

    pub fn pull(&self) -> Result<String> {
        self.run(&["pull", "--ff-only"])
    }

    pub fn push(&self, force: bool) -> Result<String> {
        if force {
            self.run(&["push", "--force-with-lease"])
        } else {
            self.run(&["push"])
        }
    }

    pub fn stashes(&self) -> Result<Vec<Stash>> {
        let out = self.git(&["stash", "list", "--format=%gd%x1f%ct%x1f%gs"])?;
        Ok(out
            .lines()
            .filter_map(|line| {
                let f: Vec<&str> = line.split('\u{1f}').collect();
                let index = f
                    .first()?
                    .strip_prefix("stash@{")?
                    .strip_suffix('}')?
                    .parse()
                    .ok()?;
                Some(Stash {
                    index,
                    time: f.get(1)?.parse().unwrap_or(0),
                    message: f.get(2)?.to_string(),
                })
            })
            .collect())
    }

    pub fn stash_push(&self, message: Option<&str>, untracked: bool) -> Result<String> {
        let mut args = vec!["stash", "push"];
        if untracked {
            args.push("--include-untracked");
        }
        if let Some(message) = message.filter(|m| !m.trim().is_empty()) {
            args.extend(["--message", message]);
        }
        self.run(&args)
    }

    fn stash_ref(&self, index: u32) -> Result<String> {
        if !self.stashes()?.iter().any(|s| s.index == index) {
            return Err(GitError(format!("no stash {index}")));
        }
        Ok(format!("stash@{{{index}}}"))
    }

    pub fn stash_apply(&self, index: u32, pop: bool) -> Result<String> {
        let reference = self.stash_ref(index)?;
        self.run(&["stash", if pop { "pop" } else { "apply" }, &reference])
    }

    pub fn stash_drop(&self, index: u32) -> Result<String> {
        let reference = self.stash_ref(index)?;
        self.run(&["stash", "drop", &reference])
    }

    /// The commit a stash points at (for backups).
    pub fn stash_sha(&self, index: u32) -> Result<String> {
        let reference = self.stash_ref(index)?;
        Ok(self.git(&["rev-parse", &reference])?.trim().to_string())
    }

    pub fn worktrees(&self) -> Result<Vec<Worktree>> {
        Ok(parse_worktrees(&self.git(&[
            "worktree",
            "list",
            "--porcelain",
        ])?))
    }

    /// Adds a working tree at `path` (which must not exist) for `branch`. An
    /// existing remote branch (`origin/x`) gets a local tracking branch.
    pub fn worktree_add(
        &self,
        path: &Path,
        branch: &str,
        create: bool,
        start: Option<&str>,
    ) -> Result<String> {
        if path.exists() {
            return Err(GitError(format!("{} already exists", path.display())));
        }
        let target = path.to_string_lossy().into_owned();
        if create {
            self.check_ref(branch)?;
            let start = match start {
                Some(start) => self.check_rev(start)?,
                None => "HEAD".into(),
            };
            return self.run(&[
                "worktree",
                "add",
                "-b",
                branch,
                "--end-of-options",
                &target,
                &start,
            ]);
        }
        let branches = self.branches()?;
        if branches.iter().any(|b| !b.remote && b.name == branch) {
            self.check_ref(branch)?;
            return self.run(&["worktree", "add", "--end-of-options", &target, branch]);
        }
        if let Some(remote) = branches.iter().find(|b| b.remote && b.name == branch) {
            let local = remote.name.split_once('/').map_or(branch, |(_, rest)| rest);
            self.check_ref(local)?;
            return self.run(&[
                "worktree",
                "add",
                "--track",
                "-b",
                local,
                "--end-of-options",
                &target,
                branch,
            ]);
        }
        Err(GitError(format!("no branch named '{branch}'")))
    }

    /// Removes a linked working tree (never the main one).
    pub fn worktree_remove(&self, path: &Path, force: bool) -> Result<String> {
        let known = self.worktrees()?;
        let tree = known
            .iter()
            .find(|w| same_path(&w.path, path))
            .ok_or_else(|| {
                GitError(format!(
                    "{} is not a working tree of this repository",
                    path.display()
                ))
            })?;
        if tree.main {
            return Err(GitError("the main working tree can't be removed".into()));
        }
        let target = tree.path.to_string_lossy().into_owned();
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.extend(["--end-of-options", &target]);
        self.run(&args)
    }

    pub fn worktree_prune(&self) -> Result<String> {
        self.run(&["worktree", "prune", "--verbose"])
    }

    pub fn remotes(&self) -> Result<Vec<Remote>> {
        let out = self.git(&["remote", "-v"])?;
        let mut remotes: Vec<Remote> = Vec::new();
        for line in out.lines() {
            let mut parts = line.split_whitespace();
            let (Some(name), Some(url), Some(kind)) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let entry = match remotes.iter_mut().find(|r| r.name == name) {
                Some(r) => r,
                None => {
                    remotes.push(Remote {
                        name: name.to_string(),
                        fetch_url: String::new(),
                        push_url: String::new(),
                    });
                    remotes.last_mut().expect("just pushed")
                }
            };
            if kind == "(push)" {
                entry.push_url = url.to_string();
            } else {
                entry.fetch_url = url.to_string();
            }
        }
        Ok(remotes)
    }

    fn check_remote(&self, name: &str) -> Result<()> {
        if self.remotes()?.iter().any(|r| r.name == name) {
            Ok(())
        } else {
            Err(GitError(format!("no remote named '{name}'")))
        }
    }

    fn check_url(url: &str) -> Result<()> {
        let ok = !url.starts_with('-')
            && !url.contains(char::is_whitespace)
            && (url.starts_with("https://")
                || url.starts_with("ssh://")
                || url.starts_with("git@")
                || url.starts_with("http://"));
        // `ext::` and other transports that run programs are refused.
        if ok {
            Ok(())
        } else {
            Err(GitError(format!(
                "'{url}' is not an https or ssh remote URL"
            )))
        }
    }

    pub fn remote_add(&self, name: &str, url: &str) -> Result<String> {
        self.check_ref(name)?;
        Self::check_url(url)?;
        self.run(&["remote", "add", "--", name, url])
    }

    pub fn remote_remove(&self, name: &str) -> Result<String> {
        self.check_remote(name)?;
        self.run(&["remote", "remove", "--", name])
    }

    pub fn remote_set_url(&self, name: &str, url: &str) -> Result<String> {
        self.check_remote(name)?;
        Self::check_url(url)?;
        self.run(&["remote", "set-url", "--", name, url])
    }

    /// A new commit that undoes `rev`.
    pub fn revert(&self, rev: &str) -> Result<String> {
        let sha = self.check_rev(rev)?;
        self.run(&["revert", "--no-edit", "--end-of-options", &sha])
    }

    /// Puts `paths` back as they were at `rev`, in the working tree only.
    pub fn restore_files(&self, rev: &str, paths: &[String]) -> Result<String> {
        let sha = self.check_rev(rev)?;
        let paths = self.check_paths(paths)?;
        let source = format!("--source={sha}");
        let mut args = vec!["restore", source.as_str(), "--worktree", "--"];
        args.extend(paths);
        self.run(&args)
    }

    pub fn reset(&self, target: &str, mode: ResetMode) -> Result<String> {
        let sha = self.check_rev(target)?;
        let flag = match mode {
            ResetMode::Soft => "--soft",
            ResetMode::Mixed => "--mixed",
            ResetMode::Hard => "--hard",
        };
        // `sha` is a full hash from `check_rev`, never an option. git 2.43
        // (Ubuntu 24.04) rejects `--end-of-options` in `reset`, so `--`
        // ends the revision instead.
        self.run(&["reset", flag, &sha, "--"])
    }

    /// Commits reachable from HEAD but not from `target`: what a reset to it
    /// takes off the branch.
    pub fn commits_between(&self, target: &str) -> Result<Vec<Commit>> {
        let sha = self.check_rev(target)?;
        let range = format!("{sha}..HEAD");
        let out = self.git(&[
            "log",
            LOG_FORMAT,
            "-z",
            "--max-count=200",
            "--end-of-options",
            &range,
        ])?;
        Ok(out
            .split('\0')
            .filter(|s| !s.trim().is_empty())
            .filter_map(parse_commit)
            .collect())
    }

    /// Commits reachable from `rev` but not from HEAD: what deleting that
    /// branch, or force-pushing over it, would lose.
    pub fn unique_commits(&self, rev: &str) -> Result<Vec<Commit>> {
        let sha = self.check_rev(rev)?;
        let range = format!("HEAD..{sha}");
        let out = self.git(&[
            "log",
            LOG_FORMAT,
            "-z",
            "--max-count=200",
            "--end-of-options",
            &range,
        ])?;
        Ok(out
            .split('\0')
            .filter(|s| !s.trim().is_empty())
            .filter_map(parse_commit)
            .collect())
    }

    /// Files a stash holds.
    pub fn stash_files(&self, index: u32) -> Result<Vec<String>> {
        let reference = self.stash_ref(index)?;
        Ok(self
            .git(&[
                "stash",
                "show",
                "--name-only",
                "--include-untracked",
                &reference,
            ])
            .or_else(|_| self.git(&["stash", "show", "--name-only", &reference]))?
            .lines()
            .map(str::to_string)
            .collect())
    }

    /// Saves the working tree and index as a commit (without touching them)
    /// under `refs/locdev/backups/<name>`, so they can be restored with
    /// `git stash apply <sha>`. `None` when there is nothing to save.
    pub fn backup(&self, name: &str) -> Result<Option<String>> {
        let sha = self.git(&[
            "stash",
            "create",
            "LocDev backup before a destructive operation",
        ])?;
        let sha = sha.trim();
        if sha.is_empty() {
            return Ok(None);
        }
        let reference = format!("refs/locdev/backups/{name}");
        self.git(&["update-ref", "-m", "LocDev backup", "--", &reference, sha])?;
        Ok(Some(format!("{reference} ({})", &sha[..sha.len().min(10)])))
    }

    /// Points `refs/locdev/backups/<name>` at `sha` (e.g. HEAD before a reset).
    pub fn keep_ref(&self, name: &str, sha: &str) -> Result<String> {
        let reference = format!("refs/locdev/backups/{name}");
        self.git(&["update-ref", "-m", "LocDev backup", "--", &reference, sha])?;
        Ok(reference)
    }

    /// Where HEAD has been (the reflog): the history of operations.
    pub fn reflog(&self, limit: usize) -> Result<Vec<RefLogEntry>> {
        if self.head().is_none() {
            return Ok(Vec::new());
        }
        let max = format!("--max-count={}", limit.clamp(1, 500));
        let out = self.git(&["reflog", "--format=%H%x1f%ct%x1f%gs", &max])?;
        Ok(out
            .lines()
            .filter_map(|line| {
                let f: Vec<&str> = line.split('\u{1f}').collect();
                Some(RefLogEntry {
                    sha: f.first()?.to_string(),
                    time: f.get(1)?.parse().unwrap_or(0),
                    message: f.get(2)?.to_string(),
                })
            })
            .collect())
    }
}

const LOG_FORMAT: &str = "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%at%x1f%D%x1f%P%x1f%s";

fn parse_commit(record: &str) -> Option<Commit> {
    let f: Vec<&str> = record.trim_start_matches('\n').split('\u{1f}').collect();
    if f.len() < 8 {
        return None;
    }
    Some(Commit {
        sha: f[0].to_string(),
        short: f[1].to_string(),
        author: f[2].to_string(),
        email: f[3].to_string(),
        time: f[4].parse().unwrap_or(0),
        refs: f[5]
            .split(", ")
            .filter(|r| !r.is_empty())
            .map(str::to_string)
            .collect(),
        parents: f[6].split_whitespace().map(str::to_string).collect(),
        subject: f[7].to_string(),
    })
}

/// `[ahead 2, behind 1]` (with `nobracket`: `ahead 2, behind 1`).
fn parse_track(track: &str) -> (u32, u32) {
    let mut out = (0, 0);
    for part in track.split(", ") {
        if let Some(n) = part.strip_prefix("ahead ") {
            out.0 = n.parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            out.1 = n.parse().unwrap_or(0);
        }
    }
    out
}

fn kind(code: char) -> Option<ChangeKind> {
    match code {
        'A' => Some(ChangeKind::Added),
        'M' => Some(ChangeKind::Modified),
        'D' => Some(ChangeKind::Deleted),
        'R' => Some(ChangeKind::Renamed),
        'C' => Some(ChangeKind::Copied),
        'T' => Some(ChangeKind::TypeChanged),
        _ => None,
    }
}

/// `git status --porcelain=v2 -z --branch`.
/// `git worktree list --porcelain`: blocks separated by blank lines.
pub fn parse_worktrees(out: &str) -> Vec<Worktree> {
    let mut trees: Vec<Worktree> = Vec::new();
    for block in out.split("\n\n") {
        let mut tree: Option<Worktree> = None;
        for line in block.lines() {
            let (key, value) = line.split_once(' ').unwrap_or((line, ""));
            match (key, tree.as_mut()) {
                ("worktree", _) => {
                    tree = Some(Worktree {
                        path: PathBuf::from(value),
                        branch: None,
                        head: String::new(),
                        main: trees.is_empty(),
                        locked: false,
                        prunable: false,
                        site: None,
                    });
                }
                ("HEAD", Some(t)) => t.head = value.to_string(),
                ("branch", Some(t)) => {
                    t.branch = Some(
                        value
                            .strip_prefix("refs/heads/")
                            .unwrap_or(value)
                            .to_string(),
                    );
                }
                ("locked", Some(t)) => t.locked = true,
                ("prunable", Some(t)) => t.prunable = true,
                _ => {}
            }
        }
        trees.extend(tree);
    }
    trees
}

/// Git prints forward slashes on Windows; compare the paths, not the text.
pub fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let p = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let text = p.to_string_lossy().replace('\\', "/");
        let text = text
            .strip_prefix("//?/")
            .unwrap_or(&text)
            .trim_end_matches('/')
            .to_string();
        if cfg!(windows) {
            text.to_lowercase()
        } else {
            text
        }
    };
    norm(a) == norm(b)
}

pub fn parse_status(out: &str) -> GitStatus {
    let mut status = GitStatus::default();
    let mut records = out.split('\0').peekable();
    while let Some(record) = records.next() {
        if let Some(header) = record.strip_prefix("# ") {
            if let Some(oid) = header.strip_prefix("branch.oid ") {
                status.head = (oid != "(initial)").then(|| oid.to_string());
            } else if let Some(head) = header.strip_prefix("branch.head ") {
                status.branch = (head != "(detached)").then(|| head.to_string());
            } else if let Some(upstream) = header.strip_prefix("branch.upstream ") {
                status.upstream = Some(upstream.to_string());
            } else if let Some(ab) = header.strip_prefix("branch.ab ") {
                let mut parts = ab.split_whitespace();
                status.ahead = parts
                    .next()
                    .and_then(|a| a.trim_start_matches('+').parse().ok())
                    .unwrap_or(0);
                status.behind = parts
                    .next()
                    .and_then(|b| b.trim_start_matches('-').parse().ok())
                    .unwrap_or(0);
            }
            continue;
        }
        let mut fields = record.splitn(2, ' ');
        let (Some(tag), Some(rest)) = (fields.next(), fields.next()) else {
            continue;
        };
        match tag {
            // `1 XY sub mH mI mW hH hI path`
            "1" => {
                let f: Vec<&str> = rest.splitn(8, ' ').collect();
                if f.len() == 8 {
                    let xy: Vec<char> = f[0].chars().collect();
                    status.changes.push(change(f[7], None, xy[0], xy[1]));
                }
            }
            // `2 XY sub mH mI mW hH hI Xscore path` then the original path.
            "2" => {
                let f: Vec<&str> = rest.splitn(9, ' ').collect();
                if f.len() == 9 {
                    let xy: Vec<char> = f[0].chars().collect();
                    let orig = records.next().map(str::to_string);
                    status.changes.push(change(f[8], orig, xy[0], xy[1]));
                }
            }
            // `u XY sub m1 m2 m3 mW h1 h2 h3 path`: unmerged.
            "u" => {
                let f: Vec<&str> = rest.splitn(10, ' ').collect();
                if f.len() == 10 {
                    let mut c = change(f[9], None, '.', 'M');
                    c.conflicted = true;
                    status.changes.push(c);
                }
            }
            "?" => status.changes.push(FileChange {
                path: rest.to_string(),
                orig_path: None,
                staged: None,
                unstaged: Some(ChangeKind::Added),
                untracked: true,
                conflicted: false,
                added: None,
                removed: None,
            }),
            _ => {}
        }
    }
    status
}

fn change(path: &str, orig: Option<String>, x: char, y: char) -> FileChange {
    FileChange {
        path: path.to_string(),
        orig_path: orig,
        staged: kind(x),
        unstaged: kind(y),
        untracked: false,
        conflicted: false,
        added: None,
        removed: None,
    }
}

/// `--numstat -z` (without renames): `added\tremoved\tpath\0`; `-` for binary.
pub fn parse_numstat(out: &str) -> Vec<FileStat> {
    out.split('\0')
        .filter_map(|record| {
            let mut f = record.trim_start_matches('\n').splitn(3, '\t');
            let (added, removed, path) = (f.next()?, f.next()?, f.next()?);
            if path.is_empty() {
                return None;
            }
            Some(FileStat {
                path: path.to_string(),
                added: added.parse().ok(),
                removed: removed.parse().ok(),
            })
        })
        .collect()
}

/// `owner/name` of a GitHub remote URL (https, ssh or scp-like).
pub fn github_repo(url: &str) -> Option<GithubRepo> {
    let rest = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
        .or_else(|| url.strip_prefix("git://github.com/"))?;
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");
    let (owner, name) = rest.split_once('/')?;
    let valid = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    (valid(owner) && valid(name) && !name.contains('/')).then(|| GithubRepo {
        owner: owner.to_string(),
        name: name.to_string(),
    })
}

fn cut(mut text: String) -> (String, bool) {
    if text.len() <= MAX_DIFF {
        return (text, false);
    }
    let mut end = MAX_DIFF;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    (text, true)
}

fn null_device() -> &'static str {
    if cfg!(windows) { "NUL" } else { "/dev/null" }
}

struct Output {
    success: bool,
    stdout: String,
    stderr: String,
}

fn error_text(out: &Output) -> String {
    let text = out.stderr.trim();
    let text = if text.is_empty() {
        out.stdout.trim()
    } else {
        text
    };
    text.lines()
        .map(|l| {
            l.trim_start_matches("fatal: ")
                .trim_start_matches("error: ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Runs `git -C dir …` with prompts and pagers off.
fn raw(
    dir: &Path,
    args: &[&str],
    stdin: Option<&[u8]>,
    timeout: Option<Duration>,
) -> Result<Output> {
    let mut command = locdev_core::process::command("git");
    command
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "core.quotepath=off",
            "-c",
            "color.ui=false",
            "--no-pager",
        ])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C")
        // An SSH key passphrase can't be typed here; fail instead of hanging.
        .env("GIT_SSH_COMMAND", ssh_command())
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| GitError(format!("could not run git: {e}")))?;
    if let (Some(input), Some(mut pipe)) = (stdin, child.stdin.take()) {
        pipe.write_all(input).map_err(|e| GitError(e.to_string()))?;
    }
    // Drain both pipes while waiting: a large diff would otherwise fill the
    // pipe buffer and block git forever.
    let drain = |pipe: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut buf);
            }
            buf
        })
    };
    let stdout = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    );
    let stderr = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    );
    let deadline = std::time::Instant::now() + timeout.unwrap_or(Duration::from_secs(30));
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if std::time::Instant::now() > deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitError("git took too long and was stopped".into()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => return Err(GitError(e.to_string())),
        }
    };
    Ok(Output {
        success: status.success(),
        stdout: String::from_utf8_lossy(&stdout.join().unwrap_or_default()).into_owned(),
        stderr: String::from_utf8_lossy(&stderr.join().unwrap_or_default()).into_owned(),
    })
}

/// The user's SSH command with batch mode on (keys from the agent only).
fn ssh_command() -> String {
    let base = std::env::var("GIT_SSH_COMMAND").unwrap_or_else(|_| "ssh".into());
    format!("{base} -o BatchMode=yes")
}

#[cfg(test)]
mod tests;
