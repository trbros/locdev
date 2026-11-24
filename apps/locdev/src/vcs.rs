//! `locdev git …` (a site's repository: reads and every change the app makes,
//! destructive ones previewed and confirmed by the daemon's own rules),
//! `locdev github …` (the account, the repository on GitHub and confirmed
//! changes there) and `locdev security …` (the password that guards
//! destructive operations).

use std::io::{BufRead, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::bail;
use clap::{Subcommand, ValueEnum};
use locdev_core::git::{Confirmation, GitOp, GithubAction, GithubSection, ResetMode};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::output::Output;
use crate::work::confirm;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum GitCommand {
    /// Branch, upstream and changed files.
    Status { site: Option<String> },
    /// Recent commits.
    Log {
        site: Option<String>,
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
    },
    /// Local and remote branches.
    Branches { site: Option<String> },
    /// Stashes.
    Stashes { site: Option<String> },
    /// What HEAD did (reflog) and what LocDev ran.
    History { site: Option<String> },
    /// A file's changes (working tree, or the index with --staged).
    Diff {
        path: String,
        #[arg(long)]
        staged: bool,
        #[arg(long)]
        site: Option<String>,
    },
    /// One commit: message, files and diff.
    Show {
        sha: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// The repository's remotes.
    Remotes { site: Option<String> },
    /// Stage files for the next commit.
    Stage {
        #[arg(required = true)]
        paths: Vec<String>,
        #[arg(long)]
        site: Option<String>,
    },
    /// Take files out of the index (their changes stay).
    Unstage {
        #[arg(required = true)]
        paths: Vec<String>,
        #[arg(long)]
        site: Option<String>,
    },
    /// Throw away working-tree changes in files (untracked ones are deleted).
    ///
    /// Destructive: shows what is lost and asks for the confirmation phrase
    /// (and the LocDev password, when one is set).
    Discard {
        #[arg(required = true)]
        paths: Vec<String>,
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
    /// Commit what is staged.
    Commit {
        #[arg(short, long)]
        message: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// Create a branch (from HEAD, or --from).
    Branch {
        name: String,
        #[arg(long)]
        from: Option<String>,
        /// Switch to it right away.
        #[arg(long)]
        switch: bool,
        #[arg(long)]
        site: Option<String>,
    },
    /// Switch to a branch.
    Switch {
        name: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// Delete a local branch (--force when it isn't merged).
    DeleteBranch {
        name: String,
        #[arg(long)]
        force: bool,
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
    /// Merge a branch into the current one.
    Merge {
        branch: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// Abort a merge in progress.
    MergeAbort {
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
    /// Fetch from a remote (every remote when omitted).
    Fetch {
        remote: Option<String>,
        #[arg(long)]
        site: Option<String>,
    },
    /// Pull, fast-forward only: never creates a merge on its own.
    Pull {
        #[arg(long)]
        site: Option<String>,
    },
    /// Push the current branch (asks first: it changes the remote).
    Push {
        /// --force-with-lease: rewrites the remote branch.
        #[arg(long)]
        force: bool,
        /// Do not ask before pushing.
        #[arg(short, long)]
        yes: bool,
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
    /// Working trees: each branch in its own folder, optionally as its own site.
    #[command(subcommand)]
    Worktree(WorktreeCommand),
    /// Stashes: save, apply, pop, drop.
    #[command(subcommand)]
    Stash(StashCommand),
    /// Remotes: add, remove, change the URL.
    #[command(subcommand)]
    Remote(RemoteCommand),
    /// A new commit that undoes another (history is kept).
    Revert {
        sha: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// Put files back as they were at a commit (in the working tree).
    Restore {
        sha: String,
        #[arg(required = true)]
        paths: Vec<String>,
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
    /// Move the branch to another commit (soft, mixed or hard).
    Reset {
        target: String,
        #[arg(long, value_enum, default_value_t = Mode::Mixed)]
        mode: Mode,
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
}

#[derive(Subcommand)]
pub(crate) enum StashCommand {
    /// Save the working tree's changes in a new stash.
    Push {
        #[arg(short, long)]
        message: Option<String>,
        /// Include untracked files.
        #[arg(short, long)]
        untracked: bool,
        #[arg(long)]
        site: Option<String>,
    },
    /// Apply a stash and keep it.
    Apply {
        index: u32,
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
    /// Apply a stash and drop it.
    Pop {
        index: u32,
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
    /// Delete a stash.
    Drop {
        index: u32,
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
}

#[derive(Subcommand)]
pub(crate) enum RemoteCommand {
    Add {
        name: String,
        url: String,
        #[arg(long)]
        site: Option<String>,
    },
    Remove {
        name: String,
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
    SetUrl {
        name: String,
        url: String,
        #[arg(long)]
        site: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum WorktreeCommand {
    /// The repository's working trees and the LocDev site of each.
    #[command(visible_alias = "ls")]
    List {
        #[arg(long)]
        site: Option<String>,
    },
    /// Check out a branch in a new folder (next to the repository as
    /// `<repo>-<branch>` unless --path).
    Add {
        branch: String,
        /// Create the branch (from --from, or HEAD).
        #[arg(long, short = 'b')]
        create: bool,
        #[arg(long)]
        from: Option<String>,
        /// An absolute folder for it.
        #[arg(long)]
        path: Option<std::path::PathBuf>,
        /// Register the folder as a LocDev site.
        #[arg(long = "site-for-it")]
        register: bool,
        /// Copy the untracked .env files the new folder lacks.
        #[arg(long)]
        copy_env: bool,
        #[arg(long)]
        site: Option<String>,
    },
    /// Delete a working tree's folder (and its LocDev site). Destructive:
    /// shows what is lost and asks for the folder name.
    Remove {
        path: std::path::PathBuf,
        /// Even with uncommitted changes.
        #[arg(long)]
        force: bool,
        #[command(flatten)]
        confirm: ConfirmArgs,
    },
    /// Forget working trees whose folders are gone.
    Prune {
        #[arg(long)]
        site: Option<String>,
    },
}

/// The site and, for scripts, the confirmation of a destructive operation.
#[derive(clap::Args)]
pub(crate) struct ConfirmArgs {
    #[arg(long)]
    site: Option<String>,
    /// The phrase the preview asks for, to confirm without a terminal.
    #[arg(long, value_name = "PHRASE")]
    confirm: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum Mode {
    Soft,
    Mixed,
    Hard,
}

#[derive(Subcommand)]
pub(crate) enum GithubCommand {
    /// The connected account.
    Status,
    /// Connect with a personal access token (read from stdin) or the GitHub CLI.
    Connect {
        /// Copy the token of a logged-in GitHub CLI (gh auth token), once.
        #[arg(long)]
        gh: bool,
    },
    /// Forget the token (removes it from the system keyring).
    Disconnect,
    /// Connect in the browser with a one-time code (OAuth device flow).
    Login,
    /// The site's repository on GitHub: overview, issues, pull requests, runs…
    Show {
        #[arg(value_enum, default_value_t = Section::Overview)]
        section: Section,
        site: Option<String>,
    },
    /// Re-run the failed jobs of a workflow run (asks first).
    Rerun {
        run: u64,
        #[arg(long)]
        site: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    /// Cancel a workflow run (asks first).
    CancelRun {
        run: u64,
        #[arg(long)]
        site: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    /// Start a workflow that has a workflow_dispatch trigger (asks first).
    Dispatch {
        /// The workflow's id (`locdev github show workflows`).
        workflow: u64,
        /// The branch or tag to run on (the default branch when omitted).
        #[arg(long = "ref")]
        git_ref: Option<String>,
        #[arg(long)]
        site: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum Section {
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

#[derive(Subcommand)]
pub(crate) enum SecurityCommand {
    /// Whether a password guards destructive operations.
    Status,
    /// Set, change or remove the password (asked on the terminal).
    Password {
        /// Remove the password.
        #[arg(long)]
        remove: bool,
    },
    /// Whether destructive operations ask for the password, and back up first.
    Options {
        /// Ask for the password on destructive operations (when one is set).
        #[arg(long)]
        require_password: Option<bool>,
        /// Save a backup before destructive operations.
        #[arg(long)]
        backups: Option<bool>,
    },
}

async fn name(paths: &Paths, site: Option<String>) -> anyhow::Result<String> {
    Ok(site::resolve(paths, site).await?.project.name)
}

pub async fn git(command: GitCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    let response = match command {
        GitCommand::Status { site } => Request::GitStatus {
            name: name(paths, site).await?,
        },
        GitCommand::Log { site, limit } => Request::GitLog {
            name: name(paths, site).await?,
            rev: None,
            skip: 0,
            limit: Some(limit),
        },
        GitCommand::Branches { site } => Request::GitBranches {
            name: name(paths, site).await?,
        },
        GitCommand::Stashes { site } => Request::GitStashes {
            name: name(paths, site).await?,
        },
        GitCommand::History { site } => Request::GitHistory {
            name: name(paths, site).await?,
        },
        GitCommand::Diff { path, staged, site } => Request::GitDiff {
            name: name(paths, site).await?,
            path,
            staged,
        },
        GitCommand::Show { sha, site } => Request::GitShow {
            name: name(paths, site).await?,
            sha,
        },
        GitCommand::Remotes { site } => Request::GitRemotes {
            name: name(paths, site).await?,
        },
        GitCommand::Worktree(WorktreeCommand::List { site }) => Request::GitWorktrees {
            name: name(paths, site).await?,
        },
        change => {
            let (op, site, confirmation) = git_op(change);
            return run_op(paths, out, site, op, confirmation).await;
        }
    };
    match daemon_call(paths, response).await? {
        Response::Text { text, truncated } => out.text(&text, truncated),
        Response::Commit(detail) => out.commit_detail(&detail),
        Response::Remotes { remotes } => out.remotes(&remotes),
        Response::GitStatus(status) => out.git_status(&status),
        Response::Commits { commits } => out.commits(&commits),
        Response::Branches { branches } => out.branches(&branches),
        Response::Stashes { stashes } => out.stashes(&stashes),
        Response::Worktrees { worktrees } => {
            if out.is_json() {
                out.print_json(&worktrees);
            }
            for w in worktrees.iter().filter(|_| !out.is_json()) {
                println!(
                    "{}  {}{}{}",
                    w.path.display(),
                    w.branch.as_deref().unwrap_or("(detached)"),
                    if w.main { "  main" } else { "" },
                    w.site
                        .as_deref()
                        .map(|s| format!("  site {s}"))
                        .unwrap_or_default()
                );
            }
        }
        Response::GitHistory { reflog, operations } => out.git_history(&reflog, &operations),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
    Ok(ExitCode::SUCCESS)
}

/// What a change needs besides the op: the site, and how it is confirmed.
struct Confirm {
    phrase: Option<String>,
    /// A plain y/N first (remote changes), unless `--yes`.
    ask: Option<(String, bool)>,
}

fn git_op(command: GitCommand) -> (GitOp, Option<String>, Confirm) {
    let none = |site| {
        (
            site,
            Confirm {
                phrase: None,
                ask: None,
            },
        )
    };
    let phrase = |c: ConfirmArgs| {
        (
            c.site,
            Confirm {
                phrase: c.confirm,
                ask: None,
            },
        )
    };
    let (op, (site, confirm)) = match command {
        GitCommand::Stage { paths, site } => (GitOp::Stage { paths }, none(site)),
        GitCommand::Unstage { paths, site } => (GitOp::Unstage { paths }, none(site)),
        GitCommand::Discard { paths, confirm } => (GitOp::Discard { paths }, phrase(confirm)),
        GitCommand::Commit { message, site } => (GitOp::Commit { message }, none(site)),
        GitCommand::Branch {
            name,
            from,
            switch,
            site,
        } => (
            GitOp::CreateBranch {
                name,
                start: from,
                switch,
            },
            none(site),
        ),
        GitCommand::Switch { name, site } => (GitOp::Switch { name }, none(site)),
        GitCommand::DeleteBranch {
            name,
            force,
            confirm,
        } => (GitOp::DeleteBranch { name, force }, phrase(confirm)),
        GitCommand::Merge { branch, site } => (GitOp::Merge { branch }, none(site)),
        GitCommand::MergeAbort { confirm } => (GitOp::AbortMerge, phrase(confirm)),
        GitCommand::Fetch { remote, site } => (GitOp::Fetch { remote }, none(site)),
        GitCommand::Pull { site } => (GitOp::Pull, none(site)),
        GitCommand::Push {
            force,
            yes,
            confirm,
        } => {
            let (site, mut c) = phrase(confirm);
            c.ask = Some(("Push the current branch to its remote?".into(), yes));
            (GitOp::Push { force }, (site, c))
        }
        GitCommand::Stash(StashCommand::Push {
            message,
            untracked,
            site,
        }) => (GitOp::StashPush { message, untracked }, none(site)),
        GitCommand::Stash(StashCommand::Apply { index, confirm }) => {
            (GitOp::StashApply { index }, phrase(confirm))
        }
        GitCommand::Stash(StashCommand::Pop { index, confirm }) => {
            (GitOp::StashPop { index }, phrase(confirm))
        }
        GitCommand::Stash(StashCommand::Drop { index, confirm }) => {
            (GitOp::StashDrop { index }, phrase(confirm))
        }
        GitCommand::Worktree(WorktreeCommand::Add {
            branch,
            create,
            from,
            path,
            register,
            copy_env,
            site,
        }) => (
            GitOp::WorktreeAdd {
                branch,
                create,
                start: from,
                path: path.map(std::path::absolute).transpose().unwrap_or(None),
                site: register,
                copy_env,
            },
            none(site),
        ),
        GitCommand::Worktree(WorktreeCommand::Remove {
            path,
            force,
            confirm,
        }) => (
            GitOp::WorktreeRemove {
                path: std::path::absolute(&path).unwrap_or(path),
                force,
            },
            phrase(confirm),
        ),
        GitCommand::Worktree(WorktreeCommand::Prune { site }) => (GitOp::WorktreePrune, none(site)),
        GitCommand::Worktree(WorktreeCommand::List { .. }) => unreachable!("read above"),
        GitCommand::Remote(RemoteCommand::Add { name, url, site }) => {
            (GitOp::RemoteAdd { name, url }, none(site))
        }
        GitCommand::Remote(RemoteCommand::Remove { name, confirm }) => {
            (GitOp::RemoteRemove { name }, phrase(confirm))
        }
        GitCommand::Remote(RemoteCommand::SetUrl { name, url, site }) => {
            (GitOp::RemoteSetUrl { name, url }, none(site))
        }
        GitCommand::Revert { sha, site } => (GitOp::Revert { sha }, none(site)),
        GitCommand::Restore {
            sha,
            paths,
            confirm,
        } => (GitOp::RestoreFiles { sha, paths }, phrase(confirm)),
        GitCommand::Reset {
            target,
            mode,
            confirm,
        } => (
            GitOp::Reset {
                target,
                mode: match mode {
                    Mode::Soft => ResetMode::Soft,
                    Mode::Mixed => ResetMode::Mixed,
                    Mode::Hard => ResetMode::Hard,
                },
            },
            phrase(confirm),
        ),
        GitCommand::Status { .. }
        | GitCommand::Log { .. }
        | GitCommand::Branches { .. }
        | GitCommand::Stashes { .. }
        | GitCommand::History { .. }
        | GitCommand::Diff { .. }
        | GitCommand::Show { .. }
        | GitCommand::Remotes { .. } => unreachable!("reads are handled in `git`"),
    };
    (op, site, confirm)
}

/// Runs a change the way the app does: the daemon previews it, a
/// destructive one shows what is lost and needs the phrase (and the
/// password), and the daemon checks both again when it runs.
async fn run_op(
    paths: &Paths,
    out: &Output,
    site: Option<String>,
    op: GitOp,
    confirm_with: Confirm,
) -> anyhow::Result<ExitCode> {
    let name = name(paths, site).await?;
    if let Some((question, yes)) = &confirm_with.ask
        && !yes
    {
        confirm(question)?;
    }
    let preview = match daemon_call(
        paths,
        Request::GitPreview {
            name: name.clone(),
            op: op.clone(),
        },
    )
    .await?
    {
        Response::GitPreview { preview } => preview,
        other => bail!("unexpected response from daemon: {other:?}"),
    };
    let confirmation = match preview {
        None => None,
        Some(preview) => {
            out.destructive_preview(&preview);
            if let Some(reason) = &preview.blocked {
                bail!("{reason}");
            }
            let phrase = match confirm_with.phrase {
                Some(phrase) => phrase,
                None => {
                    if !std::io::stdin().is_terminal() {
                        bail!(
                            "this is destructive; re-run with --confirm \"{}\"",
                            preview.phrase
                        );
                    }
                    eprint!("Type \"{}\" to confirm: ", preview.phrase);
                    read_secret()?
                }
            };
            if phrase.trim() != preview.phrase {
                bail!("the phrase doesn't match; nothing was changed");
            }
            let password = if preview.password {
                eprint!("LocDev password: ");
                Some(read_secret()?)
            } else {
                None
            };
            Some(Confirmation { phrase, password })
        }
    };
    match daemon_call(
        paths,
        Request::Git {
            name,
            op,
            confirm: confirmation,
        },
    )
    .await?
    {
        Response::GitResult(result) => out.git_result(&result),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
    Ok(ExitCode::SUCCESS)
}

pub async fn github(
    command: GithubCommand,
    paths: &Paths,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    let request = match command {
        GithubCommand::Login => return github_login(paths, out).await,
        GithubCommand::Show { section, site } => {
            let section = match section {
                Section::Overview => GithubSection::Overview,
                Section::Issues => GithubSection::Issues,
                Section::Pulls => GithubSection::Pulls,
                Section::Workflows => GithubSection::Workflows,
                Section::Runs => GithubSection::Runs,
                Section::Releases => GithubSection::Releases,
                Section::Branches => GithubSection::Branches,
                Section::Commits => GithubSection::Commits,
                Section::Checks => GithubSection::Checks,
            };
            let request = Request::Github {
                name: name(paths, site).await?,
                section,
            };
            match daemon_call(paths, request).await? {
                Response::GithubData(data) => out.github_data(&data),
                other => bail!("unexpected response from daemon: {other:?}"),
            }
            return Ok(ExitCode::SUCCESS);
        }
        GithubCommand::Rerun { run, site, yes } => {
            return github_action(
                paths,
                out,
                site,
                yes,
                format!("Re-run the failed jobs of run {run} on GitHub?"),
                GithubAction::RerunFailed { run_id: run },
            )
            .await;
        }
        GithubCommand::CancelRun { run, site, yes } => {
            return github_action(
                paths,
                out,
                site,
                yes,
                format!("Cancel run {run} on GitHub?"),
                GithubAction::CancelRun { run_id: run },
            )
            .await;
        }
        GithubCommand::Dispatch {
            workflow,
            git_ref,
            site,
            yes,
        } => {
            let site = name(paths, site).await?;
            let git_ref = match git_ref {
                Some(r) => r,
                None => match daemon_call(
                    paths,
                    Request::Github {
                        name: site.clone(),
                        section: GithubSection::Overview,
                    },
                )
                .await?
                {
                    Response::GithubData(locdev_core::git::GithubData::Overview(o)) => {
                        o.default_branch
                    }
                    other => bail!("unexpected response from daemon: {other:?}"),
                },
            };
            return github_action(
                paths,
                out,
                Some(site),
                yes,
                format!("Start workflow {workflow} on {git_ref} on GitHub?"),
                GithubAction::Dispatch {
                    workflow_id: workflow,
                    git_ref,
                },
            )
            .await;
        }
        GithubCommand::Status => Request::GithubAccount,
        GithubCommand::Disconnect => Request::GithubDisconnect,
        GithubCommand::Connect { gh: true } => Request::GithubConnectGhCli,
        GithubCommand::Connect { gh: false } => {
            // From stdin, never from an argument (which shell history keeps).
            if std::io::stdin().is_terminal() {
                eprintln!(
                    "Paste a GitHub personal access token (it stays in your system keyring), then Enter:"
                );
            }
            let token = read_secret()?;
            Request::GithubConnectToken { token }
        }
    };
    match daemon_call(paths, request).await? {
        Response::Github { account } => out.github_account(account.as_ref()),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
    Ok(ExitCode::SUCCESS)
}

/// Remote changes are always confirmed here first; the daemon only acts on
/// the one action it is sent.
async fn github_action(
    paths: &Paths,
    out: &Output,
    site: Option<String>,
    yes: bool,
    question: String,
    action: GithubAction,
) -> anyhow::Result<ExitCode> {
    let name = name(paths, site).await?;
    if !yes {
        confirm(&question)?;
    }
    match daemon_call(paths, Request::GithubAction { name, action }).await? {
        Response::GitResult(result) => out.git_result(&result),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
    Ok(ExitCode::SUCCESS)
}

/// The OAuth device flow: show the code, open the page, poll until GitHub
/// says yes (or the code expires).
async fn github_login(paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    let code = match daemon_call(paths, Request::GithubDeviceStart).await? {
        Response::DeviceCode(code) => code,
        other => bail!("unexpected response from daemon: {other:?}"),
    };
    eprintln!(
        "Open {} and enter the code {}",
        code.verification_uri, code.user_code
    );
    if std::io::stdout().is_terminal() {
        let _ = site::open_url(&code.verification_uri);
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(code.expires_in);
    let interval = std::time::Duration::from_secs(code.interval.max(1));
    loop {
        tokio::time::sleep(interval).await;
        match daemon_call(paths, Request::GithubDevicePoll).await? {
            Response::Github {
                account: Some(account),
            } => {
                out.github_account(Some(&account));
                return Ok(ExitCode::SUCCESS);
            }
            Response::Github { account: None } => {}
            other => bail!("unexpected response from daemon: {other:?}"),
        }
        if std::time::Instant::now() > deadline {
            bail!("the code expired; run `locdev github login` again");
        }
    }
}

pub async fn security(
    command: SecurityCommand,
    paths: &Paths,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    let request = match command {
        SecurityCommand::Status => Request::GetSecurity,
        SecurityCommand::Options {
            require_password,
            backups,
        } => {
            let Response::Security(status) = daemon_call(paths, Request::GetSecurity).await? else {
                bail!("unexpected response from daemon");
            };
            if require_password.is_none() && backups.is_none() {
                out.security(&status);
                return Ok(ExitCode::SUCCESS);
            }
            // With a password set, changing how it protects needs it.
            let password = if status.password_set {
                eprint!("LocDev password: ");
                Some(read_secret()?)
            } else {
                None
            };
            Request::SetSecurityOptions {
                password,
                require_password: require_password.unwrap_or(status.require_password),
                backups: backups.unwrap_or(status.backups),
            }
        }
        SecurityCommand::Password { remove } => {
            let Response::Security(status) = daemon_call(paths, Request::GetSecurity).await? else {
                bail!("unexpected response from daemon");
            };
            let current = if status.password_set {
                eprint!("Current password: ");
                Some(read_secret()?)
            } else {
                None
            };
            let new = if remove {
                None
            } else {
                eprint!("New password (8+ characters): ");
                let first = read_secret()?;
                eprint!("Again: ");
                if read_secret()? != first {
                    bail!("the passwords don't match");
                }
                Some(first)
            };
            Request::SetPassword { current, new }
        }
    };
    match daemon_call(paths, request).await? {
        Response::Security(status) => out.security(&status),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
    Ok(ExitCode::SUCCESS)
}

/// A line from stdin. On a terminal the text is visible (no extra crate for
/// hiding input); piping the secret in keeps it off the screen.
pub(crate) fn read_secret() -> anyhow::Result<String> {
    std::io::stderr().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    let line = line.trim_end_matches(['\r', '\n']).to_string();
    if line.is_empty() {
        bail!("nothing entered");
    }
    Ok(line)
}
