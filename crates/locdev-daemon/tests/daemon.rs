//! Runs the real `locdevd` binary in an isolated home and drives it over IPC.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use locdev_core::ErrorCode;
use locdev_core::paths::{HOME_ENV, Paths};
use locdev_core::protocol::{Request, Response};
use locdev_ipc::{IpcError, request};

struct Daemon {
    child: Child,
    paths: Paths,
    _home: tempfile::TempDir,
}

impl Daemon {
    async fn start() -> Self {
        Self::start_in(tempfile::tempdir().unwrap()).await
    }

    /// Starts a daemon in an existing home (to test what survives a restart).
    // The child is killed and reaped in `Drop` (or `stop`).
    #[allow(clippy::zombie_processes)]
    async fn start_in(home: tempfile::TempDir) -> Self {
        Self::start_with(home, "port = 0").await
    }

    /// `dns` is the body of the `[dns]` section.
    #[allow(clippy::zombie_processes)]
    async fn start_with(home: tempfile::TempDir, dns: &str) -> Self {
        Self::start_full(home, dns, None).await
    }

    /// With `path`, the daemon's PATH (fake programs for scaffold tests).
    #[allow(clippy::zombie_processes)]
    async fn start_full(home: tempfile::TempDir, dns: &str, path: Option<String>) -> Self {
        let env: Vec<(&str, String)> = path.map(|p| ("PATH", p)).into_iter().collect();
        Self::start_env(home, dns, &env).await
    }

    /// With extra environment variables for the daemon (PATH, HOME…).
    #[allow(clippy::zombie_processes)]
    async fn start_env(home: tempfile::TempDir, dns: &str, env: &[(&str, String)]) -> Self {
        let paths = Paths::under(home.path().to_path_buf());
        // Keep the proxy off the machine's real ports 80/7080.
        std::fs::create_dir_all(&paths.config_dir).unwrap();
        std::fs::write(
            paths.config_file(),
            // The vault key goes to a file: tests never touch the system keyring.
            format!("[proxy]\nhttp_port = 0\nfallback_port = 0\n[dns]\n{dns}\n[tls]\nhttps_port = 0\nfallback_port = 0\n[env]\nkey_store = \"file\"\n[mail]\nport = 0\n"),
        )
        .unwrap();
        // Native notifications are on by default; tests must not pop real ones.
        // Nor ask GitHub for LocDev releases on their own.
        std::fs::write(
            paths.config_dir.join("settings.toml"),
            "[notifications]\nnative = false\n[automation]\nupdates = false\n",
        )
        .unwrap();

        let mut command = Command::new(env!("CARGO_BIN_EXE_locdevd"));
        command
            .env(HOME_ENV, home.path())
            .env("LOCDEV_LOG", "warn")
            .stdout(Stdio::null());
        for (key, value) in env {
            command.env(key, value);
        }
        let child = command.spawn().unwrap();

        for _ in 0..100 {
            if request(&paths, Request::Status).await.is_ok() {
                return Self {
                    child,
                    paths,
                    _home: home,
                };
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("daemon did not come up");
    }

    async fn call(&self, req: Request) -> Result<Response, IpcError> {
        request(&self.paths, req).await
    }
}

impl Daemon {
    /// Stops the daemon and hands back its home.
    fn stop(mut self) -> tempfile::TempDir {
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::mem::replace(&mut self._home, tempfile::tempdir().unwrap())
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[tokio::test]
async fn registers_lists_and_removes_projects() {
    let daemon = Daemon::start().await;
    let project_dir = tempfile::tempdir().unwrap();
    std::fs::write(project_dir.path().join("index.php"), "<?php echo 'hi';").unwrap();

    let response = daemon
        .call(Request::AddProject {
            path: project_dir.path().to_path_buf(),
            name: Some("My_Site".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();
    let Response::Project { project } = response else {
        panic!("unexpected {response:?}");
    };
    assert_eq!(project.project.domain, "my-site.test");
    assert!(project.project.command.unwrap().starts_with("php -S"));
    assert!(project.project.port.is_some());

    // The registry survives on disk.
    let saved = std::fs::read_to_string(daemon.paths.registry_file()).unwrap();
    assert!(saved.contains("name = \"my-site\""));

    let error = daemon
        .call(Request::StartProject {
            name: "nope".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(error, IpcError::App(e) if e.code == ErrorCode::NotFound));

    daemon
        .call(Request::RemoveProject {
            name: "my-site".into(),
        })
        .await
        .unwrap();
    let Response::Projects { projects } = daemon.call(Request::ListProjects).await.unwrap() else {
        panic!();
    };
    assert!(projects.is_empty());
    assert!(
        project_dir.path().join("index.php").exists(),
        "files must survive removal"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn runs_and_stops_a_project() {
    use locdev_core::model::ProcessState;

    let daemon = Daemon::start().await;
    let project_dir = tempfile::tempdir().unwrap();

    daemon
        .call(Request::AddProject {
            path: project_dir.path().to_path_buf(),
            name: Some("worker".into()),
            command: Some("echo ready on $PORT; exec sleep 60".into()),
            port: Some(9555),
            auto_start: false,
        })
        .await
        .unwrap();

    let started = daemon
        .call(Request::StartProject {
            name: "worker".into(),
        })
        .await
        .unwrap();
    let Response::Project { project } = started else {
        panic!("unexpected {started:?}");
    };
    assert_eq!(project.process.state, ProcessState::Running);
    assert_eq!(project.direct_url.as_deref(), Some("http://127.0.0.1:9555"));
    assert!(
        project
            .https_url
            .as_deref()
            .is_some_and(|u| u.starts_with("https://worker."))
    );
    let url = project.url.expect("routable through the proxy");
    assert!(url.starts_with("http://worker."), "{url}");

    tokio::time::sleep(Duration::from_millis(300)).await;
    let Response::Logs { lines } = daemon
        .call(Request::ProjectLogs {
            name: "worker".into(),
            after: None,
            limit: None,
            process: Default::default(),
            aux: None,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(lines.iter().any(|l| l.text == "ready on 9555"), "{lines:?}");

    let stopped = daemon
        .call(Request::StopProject {
            name: "worker".into(),
        })
        .await
        .unwrap();
    assert!(
        matches!(stopped, Response::Project { project } if project.process.state == ProcessState::Stopped)
    );
}

#[tokio::test]
async fn shutdown_over_ipc() {
    let mut daemon = Daemon::start().await;

    daemon.call(Request::Shutdown).await.unwrap();

    let status = tokio::task::spawn_blocking(move || {
        let status = daemon.child.wait();
        (daemon, status)
    })
    .await
    .unwrap()
    .1
    .unwrap();
    assert!(status.success());
}

/// A stand-in server: prints `line` and keeps running, in the shell the
/// supervisor uses (`sh` on Unix, `cmd` on Windows).
fn print_and_wait(line: &str) -> String {
    if cfg!(windows) {
        format!("echo {line}& ping -n 61 127.0.0.1 >nul")
    } else {
        format!("echo {line}; exec sleep 60")
    }
}

#[tokio::test]
async fn helper_dev_server_needs_consent() {
    use locdev_core::model::{DevServerMode, DevTool, ProcessRole, ProcessState};

    let daemon = Daemon::start().await;
    let dir = tempfile::tempdir().unwrap();
    for (file, body) in [
        ("artisan", ""),
        ("composer.json", "{}"),
        (
            "package.json",
            r#"{"scripts":{"dev":"vite"},"devDependencies":{"vite":"7"}}"#,
        ),
    ] {
        std::fs::write(dir.path().join(file), body).unwrap();
    }

    let Response::Project { project } = daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("shop".into()),
            command: Some(print_and_wait(if cfg!(windows) {
                "main on %PORT%"
            } else {
                "main on $PORT"
            })),
            port: None,
            auto_start: false,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    // Detected and recorded, but waiting for a decision.
    let detected = project.project.dev_server.clone().expect("vite detected");
    assert_eq!(
        (detected.tool, detected.mode),
        (DevTool::Vite, DevServerMode::Ask)
    );
    assert_eq!(project.stack.framework, "Laravel + Vite");

    let refused = daemon
        .call(Request::StartDevServer {
            name: "shop".into(),
        })
        .await;
    assert!(matches!(refused, Err(IpcError::App(e)) if e.code == ErrorCode::NotRunnable));

    // Consent, with a stand-in command, and start it now.
    let Response::Project { project } = daemon
        .call(Request::SetDevServer {
            name: "shop".into(),
            mode: DevServerMode::Manual,
            command: Some(print_and_wait("helper on {port}")),
            start: true,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    let port = project
        .project
        .dev_server
        .as_ref()
        .unwrap()
        .port
        .expect("port allocated");
    assert_eq!(project.dev_process.unwrap().state, ProcessState::Running);
    assert_eq!(project.process.state, ProcessState::Stopped);

    tokio::time::sleep(Duration::from_millis(300)).await;
    let Response::Logs { lines } = daemon
        .call(Request::ProjectLogs {
            name: "shop".into(),
            after: None,
            limit: None,
            process: ProcessRole::Dev,
            aux: None,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(
        lines.iter().any(|l| l.text == format!("helper on {port}")),
        "{lines:?}"
    );

    let Response::Processes { processes } = daemon.call(Request::ListProcesses).await.unwrap()
    else {
        panic!();
    };
    let dev = processes
        .iter()
        .find(|p| p.project == "shop" && p.role == ProcessRole::Dev)
        .unwrap();
    assert_eq!(dev.label, "Vite");
    assert!(dev.memory.is_some_and(|m| m > 0), "{dev:?}");
    assert!(dev.log_file.is_some());

    // Stopping the site stops the helper too.
    daemon
        .call(Request::StartProject {
            name: "shop".into(),
        })
        .await
        .unwrap();
    let Response::Project { project } = daemon
        .call(Request::StopProject {
            name: "shop".into(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(project.process.state, ProcessState::Stopped);
    assert_eq!(project.dev_process.unwrap().state, ProcessState::Stopped);

    // HTTPS diagnosis and the HTTPS-only switch.
    let Response::Doctor { checks } = daemon
        .call(Request::CheckSiteHttps {
            name: "shop".into(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    for id in [
        "https-server",
        "dns",
        "certificate",
        "trust",
        "port",
        "secure",
    ] {
        assert!(
            checks.iter().any(|c| c.id == id),
            "missing {id}: {checks:?}"
        );
    }
    let Response::Project { project } = daemon
        .call(Request::SetSecure {
            name: "shop".into(),
            secure: true,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(project.project.secure);
}

#[tokio::test]
async fn dependencies_jobs_and_notifications() {
    use locdev_core::deps::{AuditInterval, JobStatus, NotificationCategory};

    let daemon = Daemon::start().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"left-pad":"^1.3.0"},"devDependencies":{"local-thing":"file:../thing"}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"node_modules/left-pad":{"version":"1.3.0"}}}"#,
    )
    .unwrap();
    daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("deps".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();

    // Listing reads the disk only: locked versions, nothing checked yet.
    let Response::Deps(report) = daemon
        .call(Request::ListDeps {
            name: "deps".into(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(report.files, vec!["package.json", "package-lock.json"]);
    let pad = report
        .packages
        .iter()
        .find(|p| p.name == "left-pad")
        .unwrap();
    assert_eq!(pad.locked.as_deref(), Some("1.3.0"));
    assert!(report.checked_at.is_none());
    let local = report
        .packages
        .iter()
        .find(|p| p.name == "local-thing")
        .unwrap();
    assert!(
        local
            .reasons
            .iter()
            .any(|r| r.contains("isn't a registry version range"))
    );

    // Settings round trip (no automatic checks in this test).
    let Response::Settings(mut settings) = daemon.call(Request::GetSettings).await.unwrap() else {
        panic!();
    };
    settings.automation.audit = AuditInterval::Manual;
    settings.network.offline = true;
    let Response::Settings(saved) = daemon
        .call(Request::UpdateSettings {
            settings: settings.clone(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(saved, settings);

    // A project without dependencies checks without any network request.
    let empty = tempfile::tempdir().unwrap();
    std::fs::write(empty.path().join("package.json"), "{}").unwrap();
    daemon
        .call(Request::AddProject {
            path: empty.path().to_path_buf(),
            name: Some("empty".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();
    let Response::Job { job } = daemon
        .call(Request::CheckDeps {
            name: "empty".into(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    let mut status = job.status;
    for _ in 0..100 {
        let Response::Job { job } = daemon.call(Request::GetJob { id: job.id }).await.unwrap()
        else {
            panic!();
        };
        status = job.status;
        if status.finished() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(status, JobStatus::Completed);

    // The finished job left a notification, and notifications persist.
    daemon.call(Request::TestNotification).await.unwrap();
    let Response::Notifications {
        notifications,
        unread,
    } = daemon
        .call(Request::ListNotifications { all: false })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(unread, 2);
    assert!(
        notifications
            .iter()
            .any(|n| n.category == NotificationCategory::Action && n.title.contains("finished"))
    );

    let home = daemon.stop();
    let daemon = Daemon::start_in(home).await;
    let Response::Notifications { unread, .. } = daemon
        .call(Request::ListNotifications { all: false })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(unread, 2, "notifications survive a daemon restart");
    daemon
        .call(Request::MarkNotifications {
            ids: vec![],
            read: Some(true),
            dismissed: None,
        })
        .await
        .unwrap();
    let Response::Notifications { unread, .. } = daemon
        .call(Request::ListNotifications { all: false })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(unread, 0);
}

#[tokio::test]
async fn destructive_git_needs_confirmation_and_password() {
    use locdev_core::git::{Confirmation, GitOp};

    let daemon = Daemon::start().await;
    let dir = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    };
    git(&["init", "--quiet", "--initial-branch=main"]);
    git(&["config", "user.name", "T"]);
    git(&["config", "user.email", "t@example.com"]);
    git(&["config", "commit.gpgsign", "false"]);
    // Git for Windows turns autocrlf on system-wide; keep the bytes as written.
    git(&["config", "core.autocrlf", "false"]);
    std::fs::write(dir.path().join("a.txt"), "one\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "--quiet", "-m", "first"]);
    std::fs::write(dir.path().join("a.txt"), "precious work\n").unwrap();
    std::fs::write(dir.path().join("notes.txt"), "untracked\n").unwrap();
    daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("repo".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();

    let Response::GitStatus(status) = daemon
        .call(Request::GitStatus {
            name: "repo".into(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(status.changes.len(), 2);

    let discard = GitOp::Discard {
        paths: vec!["a.txt".into(), "notes.txt".into()],
    };
    // Without confirmation: refused, and nothing changes.
    let refused = daemon
        .call(Request::Git {
            name: "repo".into(),
            op: discard.clone(),
            confirm: None,
        })
        .await;
    assert!(matches!(refused, Err(IpcError::App(e)) if e.code == ErrorCode::Unauthorized));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "precious work\n"
    );

    let Response::GitPreview { preview } = daemon
        .call(Request::GitPreview {
            name: "repo".into(),
            op: discard.clone(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    let preview = preview.expect("discard is destructive");
    assert_eq!(preview.phrase, "discard");
    assert!(
        preview
            .lost
            .iter()
            .any(|l| l.contains("notes.txt (untracked file, deleted)"))
    );
    assert!(!preview.password);

    // A password, then the wrong one is refused.
    daemon
        .call(Request::SetPassword {
            current: None,
            new: Some("correct horse".into()),
        })
        .await
        .unwrap();
    let wrong = daemon
        .call(Request::Git {
            name: "repo".into(),
            op: discard.clone(),
            confirm: Some(Confirmation {
                phrase: "discard".into(),
                password: Some("nope".into()),
            }),
        })
        .await;
    assert!(
        matches!(wrong, Err(IpcError::App(e)) if e.code == ErrorCode::Unauthorized && e.message.contains("wrong"))
    );

    let Response::GitResult(result) = daemon
        .call(Request::Git {
            name: "repo".into(),
            op: discard,
            confirm: Some(Confirmation {
                phrase: "discard".into(),
                password: Some("correct horse".into()),
            }),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    let backup = result.backup.expect("a backup was made");
    assert!(backup.contains("refs/locdev/backups/"), "{backup}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "one\n"
    );
    assert!(!dir.path().join("notes.txt").exists());

    // Safe operations need nothing.
    daemon
        .call(Request::Git {
            name: "repo".into(),
            op: GitOp::CreateBranch {
                name: "topic".into(),
                start: None,
                switch: false,
            },
            confirm: None,
        })
        .await
        .unwrap();
    let Response::GitHistory { operations, .. } = daemon
        .call(Request::GitHistory {
            name: "repo".into(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(operations[0].contains("create-branch topic"));
    assert!(operations.iter().any(|l| l.contains("refused discard")));
}

#[tokio::test]
async fn actions_builder_saves_without_overwriting() {
    let daemon = Daemon::start().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{ "scripts": { "test": "vitest", "build": "vite build" }, "devDependencies": { "vite": "^8.0.0" } }"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("package-lock.json"), "{}").unwrap();
    daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("app".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();
    let name = || "app".to_string();

    let Response::ActionsOverview {
        templates,
        existing,
        ..
    } = daemon
        .call(Request::ActionsOverview { name: name() })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(existing.is_empty());
    assert!(templates.iter().any(|t| t.id == "node-ci" && t.recommended));

    let Response::ActionsWorkflow { workflow, file } = daemon
        .call(Request::ActionsTemplate {
            name: name(),
            template: "node-ci".into(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    let Response::ActionsYaml { yaml, validation } = daemon
        .call(Request::ActionsRender { workflow })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(!validation.has_errors(), "{validation:#?}");
    assert!(yaml.contains("run: npm ci") && yaml.contains("run: npm run test"));

    // New file: plan, then save.
    let Response::ActionsPlan(plan) = daemon
        .call(Request::ActionsPlan {
            name: name(),
            file: file.clone(),
            yaml: yaml.clone(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(!plan.exists && plan.diff.contains("+name: CI"));
    daemon
        .call(Request::ActionsSave {
            name: name(),
            file: file.clone(),
            yaml: yaml.clone(),
            replace: None,
        })
        .await
        .unwrap();
    let saved = dir.path().join(".github/workflows/ci.yml");
    assert_eq!(std::fs::read_to_string(&saved).unwrap(), yaml);

    // Saving again without the reviewed hash is refused; with it, replaced.
    let changed = yaml.replace("name: CI", "name: Checks");
    let again = daemon
        .call(Request::ActionsSave {
            name: name(),
            file: file.clone(),
            yaml: changed.clone(),
            replace: None,
        })
        .await;
    assert!(matches!(again, Err(IpcError::App(e)) if e.message.contains("already exists")));
    let Response::ActionsPlan(plan) = daemon
        .call(Request::ActionsPlan {
            name: name(),
            file: file.clone(),
            yaml: changed.clone(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(plan.exists && plan.diff.contains("-name: CI") && plan.diff.contains("+name: Checks"));
    daemon
        .call(Request::ActionsSave {
            name: name(),
            file: file.clone(),
            yaml: changed.clone(),
            replace: plan.current_hash,
        })
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(&saved).unwrap(), changed);

    // Security errors block saving.
    let injected = changed.replace(
        "run: npm run test",
        "run: echo ${{ github.event.pull_request.title }}",
    );
    let refused = daemon
        .call(Request::ActionsSave {
            name: name(),
            file: "other.yml".into(),
            yaml: injected,
            replace: None,
        })
        .await;
    assert!(matches!(refused, Err(IpcError::App(e)) if e.message.contains("fix the errors first")));
    assert!(!dir.path().join(".github/workflows/other.yml").exists());
}

/// Shared variables end to end: stored encrypted, injected into the real
/// process, skipped where the project defines them, synced on request with
/// a backup, and still readable after the daemon restarts.
#[cfg(unix)]
#[tokio::test]
async fn shared_environment_variables() {
    use locdev_core::env::{EntryStatus, ProjectEnvSettings};

    let daemon = Daemon::start().await;
    let project_dir = tempfile::tempdir().unwrap();
    let dir = project_dir.path();
    std::fs::write(dir.join(".env"), "# mine\nAPP_NAME=Shop\n").unwrap();
    std::fs::write(dir.join(".env.local"), "SENTRY_DSN=project-value\n").unwrap();

    let set = |key: &str, profile: &str, value: &str, secret: Option<bool>| Request::EnvSet {
        key: key.into(),
        profile: profile.into(),
        value: Some(value.into()),
        secret,
        description: None,
    };
    daemon
        .call(set("STRIPE_SECRET_KEY", "global", "sk_test_123", None))
        .await
        .unwrap();
    daemon
        .call(set("SENTRY_DSN", "global", "https://shared", Some(false)))
        .await
        .unwrap();
    daemon
        .call(Request::EnvSaveProfile {
            id: "laravel".into(),
            label: "Laravel".into(),
            description: String::new(),
        })
        .await
        .unwrap();
    daemon
        .call(set("QUEUE_CONNECTION", "laravel", "redis", Some(false)))
        .await
        .unwrap();

    // Secrets never land in clear on disk.
    let config = std::fs::read_to_string(daemon.paths.config_dir.join("env.toml")).unwrap();
    assert!(config.contains("STRIPE_SECRET_KEY") && !config.contains("sk_test_123"));
    let state = daemon.paths.data_dir.join("state");
    let vault = std::fs::read_to_string(state.join("env-vault.json")).unwrap();
    assert!(!vault.contains("sk_test_123"));

    let Response::Project { project } = daemon
        .call(Request::AddProject {
            path: dir.to_path_buf(),
            name: Some("envsite".into()),
            command: Some("env > env.out; sleep 30".into()),
            port: None,
            auto_start: false,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    let name = project.project.name.clone();
    daemon
        .call(Request::StartProject { name: name.clone() })
        .await
        .unwrap();
    let out = dir.join("env.out");
    for _ in 0..100 {
        if std::fs::metadata(&out).is_ok_and(|m| m.len() > 0) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let env = std::fs::read_to_string(&out).unwrap();
    assert!(env.contains("STRIPE_SECRET_KEY=sk_test_123"), "{env}");
    assert!(!env.contains("SENTRY_DSN"), "the project's .env.local wins");
    assert!(!env.contains("QUEUE_CONNECTION"), "profile not enabled");

    let Response::Logs { lines } = daemon
        .call(Request::ProjectLogs {
            name: name.clone(),
            after: None,
            limit: None,
            process: Default::default(),
            aux: None,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    let log: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
    assert!(
        log.contains(&"[LocDev] Warning: SENTRY_DSN already exists in .env.local"),
        "{log:?}"
    );
    assert!(log.iter().all(|l| !l.contains("sk_test_123")));

    // Enabling the profile: the report says a restart is needed.
    let Response::EnvProject(report) = daemon
        .call(Request::EnvSetProject {
            name: name.clone(),
            settings: ProjectEnvSettings {
                profiles: vec!["laravel".into()],
                ..Default::default()
            },
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(report.restart_needed);
    let status = |key: &str| report.entries.iter().find(|e| e.key == key).unwrap().status;
    assert_eq!(status("QUEUE_CONNECTION"), EntryStatus::Injected);
    assert_eq!(status("SENTRY_DSN"), EntryStatus::Skipped);
    assert!(
        report
            .entries
            .iter()
            .all(|e| !e.secret || e.value.is_none())
    );
    daemon
        .call(Request::StopProject { name: name.clone() })
        .await
        .unwrap();

    // Sync: plan, a stale token is refused, the real one writes with a backup.
    let Response::EnvSyncPlans { plans } = daemon
        .call(Request::EnvSyncPlan {
            name: Some(name.clone()),
            file: None,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    let plan = &plans[0];
    let added: Vec<&str> = plan.add.iter().map(|l| l.key.as_str()).collect();
    assert_eq!(added, vec!["QUEUE_CONNECTION", "STRIPE_SECRET_KEY"]);
    assert!(!plan.preview.contains("sk_test_123"));
    let error = daemon
        .call(Request::EnvSync {
            name: name.clone(),
            file: None,
            token: "stale".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(error, IpcError::App(e) if e.code == ErrorCode::Conflict));
    let Response::EnvSynced(result) = daemon
        .call(Request::EnvSync {
            name: name.clone(),
            file: None,
            token: plan.token.clone(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    let synced = std::fs::read_to_string(dir.join(".env")).unwrap();
    assert!(synced.starts_with("# mine\nAPP_NAME=Shop\n"));
    assert!(synced.contains("STRIPE_SECRET_KEY=sk_test_123\n"));
    let backup = result.backup.expect("a backup of the old .env");

    // Restore puts the original back.
    daemon
        .call(Request::EnvRestore {
            name: name.clone(),
            id: backup,
        })
        .await
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join(".env")).unwrap(),
        "# mine\nAPP_NAME=Shop\n"
    );
    let Response::EnvBackups { backups } = daemon
        .call(Request::EnvBackups { name: name.clone() })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(backups.len(), 2, "the synced file was kept too");

    // After a restart, secrets are still readable (the key persisted).
    let home = daemon.stop();
    let daemon = Daemon::start_in(home).await;
    let Response::EnvValue { value } = daemon
        .call(Request::EnvReveal {
            key: "STRIPE_SECRET_KEY".into(),
            profile: "global".into(),
            password: None,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(value, "sk_test_123");
    let Response::EnvOverview {
        vars,
        vault,
        history,
        ..
    } = daemon.call(Request::EnvOverview).await.unwrap()
    else {
        panic!();
    };
    assert!(vault.available && vault.backend == "key file");
    let secret = vars.iter().find(|v| v.key == "STRIPE_SECRET_KEY").unwrap();
    assert!(secret.secret && secret.value.is_none());
    assert_eq!(history[0].action, "revealed");
    assert!(
        !serde_json::to_string(&history)
            .unwrap()
            .contains("sk_test_123")
    );

    // A secret can't take a browser prefix.
    let error = daemon
        .call(set("NEXT_PUBLIC_KEY", "global", "x", Some(true)))
        .await
        .unwrap_err();
    assert!(matches!(error, IpcError::App(e) if e.code == ErrorCode::InvalidInput));
}

/// Framework adapters end to end: definitions in use, the adapter's port
/// variables and Python venv reaching the real process, manual stack
/// choices persisted, and a rollback refused when nothing was downloaded.
#[cfg(unix)]
#[tokio::test]
async fn framework_adapters_and_manual_stacks() {
    use locdev_core::frameworks::DefinitionsOrigin;
    use locdev_core::model::Integration;

    let daemon = Daemon::start().await;
    let Response::StackDefinitions {
        definitions,
        registry,
    } = daemon.call(Request::StackDefinitions).await.unwrap()
    else {
        panic!();
    };
    assert!(definitions.len() >= 46);
    assert_eq!(registry.origin, DefinitionsOrigin::Bundled);
    let error = daemon.call(Request::RollbackDefinitions).await.unwrap_err();
    assert!(matches!(error, IpcError::App(e) if e.code == ErrorCode::Conflict));

    let run = |dir: &std::path::Path, name: &str| {
        let dir = dir.to_path_buf();
        let name = name.to_string();
        let daemon = &daemon;
        async move {
            let Response::Project { project } = daemon
                .call(Request::AddProject {
                    path: dir.clone(),
                    name: Some(name.clone()),
                    command: Some("env > env.out; sleep 30".into()),
                    port: None,
                    auto_start: false,
                })
                .await
                .unwrap()
            else {
                panic!();
            };
            daemon
                .call(Request::StartProject { name: name.clone() })
                .await
                .unwrap();
            let out = dir.join("env.out");
            for _ in 0..100 {
                if std::fs::metadata(&out).is_ok_and(|m| m.len() > 0) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            daemon.call(Request::StopProject { name }).await.unwrap();
            (
                project.project.port.unwrap(),
                std::fs::read_to_string(out).unwrap(),
            )
        }
    };

    // Spring Boot: the port also travels in SERVER_PORT.
    let spring = tempfile::tempdir().unwrap();
    std::fs::write(
        spring.path().join("pom.xml"),
        "<project><parent><artifactId>spring-boot-starter-parent</artifactId><version>3.4.1</version></parent></project>",
    )
    .unwrap();
    std::fs::write(spring.path().join("mvnw"), "").unwrap();
    let (port, env) = run(spring.path(), "spring").await;
    assert!(env.contains(&format!("SERVER_PORT={port}\n")), "{env}");
    assert!(env.contains("SERVER_ADDRESS=127.0.0.1\n"));

    // Django with a virtualenv: its bin comes first on PATH.
    let django = tempfile::tempdir().unwrap();
    let dir = std::fs::canonicalize(django.path()).unwrap();
    std::fs::write(dir.join("manage.py"), "").unwrap();
    std::fs::write(dir.join("requirements.txt"), "Django==5.1.2\n").unwrap();
    std::fs::create_dir_all(dir.join(".venv/bin")).unwrap();
    std::fs::write(dir.join(".venv/pyvenv.cfg"), "home = /usr/bin\n").unwrap();
    let (_, env) = run(&dir, "djangosite").await;
    let venv = dir.join(".venv");
    assert!(
        env.contains(&format!("VIRTUAL_ENV={}\n", venv.display())),
        "{env}"
    );
    assert!(
        env.contains(&format!("PATH={}/bin:", venv.display())),
        "{env}"
    );

    // Manual choices: add, hide, pick an adapter; refused when it can't run.
    let set =
        |add: &[&str], remove: &[&str], adapter: Option<&str>, apply: bool| Request::StackSet {
            name: "djangosite".into(),
            add: add.iter().map(|s| s.to_string()).collect(),
            remove: remove.iter().map(|s| s.to_string()).collect(),
            adapter: adapter.map(str::to_string),
            apply_command: apply,
        };
    let Response::StackReport(report) = daemon
        .call(set(&["tailwind"], &["python"], None, false))
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(
        report
            .stack
            .components
            .iter()
            .any(|c| c.id == "tailwind" && c.manual)
    );
    assert!(!report.stack.components.iter().any(|c| c.id == "python"));
    assert_eq!(report.hidden, vec!["python".to_string()]);
    let error = daemon
        .call(set(&[], &[], Some("flask"), true))
        .await
        .unwrap_err();
    assert!(matches!(error, IpcError::App(e) if e.code == ErrorCode::NotRunnable));
    let Response::StackReport(report) = daemon
        .call(set(&[], &[], Some("fastapi"), false))
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(report.stack.adapter.as_deref(), Some("fastapi"));
    assert_eq!(report.stack.integration, Integration::Custom);
    let error = daemon
        .call(set(&["nope"], &[], None, false))
        .await
        .unwrap_err();
    assert!(matches!(error, IpcError::App(e) if e.code == ErrorCode::NotFound));

    // Persisted in projects.toml, and back to detection with "".
    let saved = std::fs::read_to_string(daemon.paths.registry_file()).unwrap();
    assert!(
        saved.contains("adapter = \"fastapi\"") && saved.contains("hidden_stacks = [\"python\"]")
    );
    let Response::StackReport(report) = daemon.call(set(&[], &[], Some(""), false)).await.unwrap()
    else {
        panic!();
    };
    assert_eq!(report.stack.adapter.as_deref(), Some("django"));
}

/// Auxiliary processes: offered by what the project has installed, never
/// run without a decision, alternatives refused together, custom ones with
/// their own logs, and everything stopped with the site.
#[cfg(unix)]
#[tokio::test]
async fn auxiliary_processes() {
    use locdev_core::model::{DevServerMode, ProcessState};
    use locdev_core::protocol::ProcessAction;

    let daemon = Daemon::start().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("artisan"), "").unwrap();
    std::fs::write(
        dir.path().join("composer.json"),
        r#"{"require":{"laravel/framework":"^12","laravel/horizon":"^5"}}"#,
    )
    .unwrap();
    daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("lara".into()),
            command: Some("sleep 30".into()),
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();
    let list = |r: Response| match r {
        Response::ProjectProcesses { processes } => processes,
        other => panic!("{other:?}"),
    };
    let procs = list(
        daemon
            .call(Request::ProjectProcesses {
                name: "lara".into(),
            })
            .await
            .unwrap(),
    );
    let ids: Vec<&str> = procs.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["queue", "horizon", "scheduler"]);
    assert!(procs.iter().all(|p| p.mode == DevServerMode::Ask));

    let control = |process: &str, action| Request::ControlProcess {
        name: "lara".into(),
        process: process.into(),
        action,
    };
    // Nothing runs before a decision.
    let error = daemon
        .call(control("queue", ProcessAction::Start))
        .await
        .unwrap_err();
    assert!(matches!(error, IpcError::App(e) if e.code == ErrorCode::NotRunnable));

    // Horizon (its command replaced so it runs here) excludes queue:work.
    let set =
        |process: &str, mode, command: Option<&str>, label: Option<&str>| Request::SetProcess {
            name: "lara".into(),
            process: process.into(),
            mode: Some(mode),
            command: command.map(str::to_string),
            label: label.map(str::to_string),
        };
    daemon
        .call(set(
            "horizon",
            DevServerMode::Manual,
            Some("sleep 30"),
            None,
        ))
        .await
        .unwrap();
    daemon
        .call(control("horizon", ProcessAction::Start))
        .await
        .unwrap();
    let error = daemon
        .call(set("queue", DevServerMode::Auto, None, None))
        .await
        .unwrap_err();
    assert!(matches!(error, IpcError::App(e) if e.code == ErrorCode::Conflict));

    // A custom process with its own output; it starts with the site (auto).
    daemon
        .call(set(
            "mailer",
            DevServerMode::Auto,
            Some("echo mailer-ready; sleep 30"),
            Some("Mail queue"),
        ))
        .await
        .unwrap();
    daemon
        .call(Request::StartProject {
            name: "lara".into(),
        })
        .await
        .unwrap();
    let mut seen = false;
    for _ in 0..60 {
        let Response::Logs { lines } = daemon
            .call(Request::ProjectLogs {
                name: "lara".into(),
                after: None,
                limit: None,
                process: Default::default(),
                aux: Some("mailer".into()),
            })
            .await
            .unwrap()
        else {
            panic!();
        };
        if lines.iter().any(|l| l.text == "mailer-ready") {
            seen = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(seen, "the custom process ran with the site");
    let procs = list(
        daemon
            .call(Request::ProjectProcesses {
                name: "lara".into(),
            })
            .await
            .unwrap(),
    );
    let mailer = procs.iter().find(|p| p.id == "mailer").unwrap();
    assert!(mailer.custom && mailer.name == "Mail queue");
    assert_eq!(mailer.info.state, ProcessState::Running);

    // The global list shows them; stopping the site stops all of them.
    let Response::Processes { processes } = daemon.call(Request::ListProcesses).await.unwrap()
    else {
        panic!();
    };
    assert!(
        processes
            .iter()
            .any(|p| p.aux.as_deref() == Some("horizon"))
    );
    daemon
        .call(Request::StopProject {
            name: "lara".into(),
        })
        .await
        .unwrap();
    let procs = list(
        daemon
            .call(Request::ProjectProcesses {
                name: "lara".into(),
            })
            .await
            .unwrap(),
    );
    assert!(
        procs.iter().all(|p| p.info.state != ProcessState::Running),
        "{procs:?}"
    );

    // Decisions are kept in projects.toml.
    let saved = std::fs::read_to_string(daemon.paths.registry_file()).unwrap();
    assert!(
        saved.contains("[projects.processes.mailer]") || saved.contains("processes.mailer"),
        "{saved}"
    );
}

/// Service instances with the system's own servers (skipped when absent):
/// created with their own data and port, supervised, ready by a real
/// connection, backed up cold, and never deleted without the name.
#[cfg(unix)]
#[tokio::test]
async fn service_instances_with_system_servers() {
    use locdev_core::protocol::ProcessAction;
    use locdev_core::services::{InstallSource, ServiceKind};

    let daemon = Daemon::start().await;
    let Response::Services { services, .. } = daemon
        .call(Request::ListServices { refresh: true })
        .await
        .unwrap()
    else {
        panic!();
    };
    let mysql = services
        .iter()
        .find(|s| s.kind == ServiceKind::Mysql)
        .unwrap();
    assert!(mysql.no_download.as_deref().unwrap().contains("MD5"));
    let available: Vec<ServiceKind> = [ServiceKind::Redis, ServiceKind::Postgres]
        .into_iter()
        .filter(|k| {
            services.iter().any(|s| {
                s.kind == *k && s.installs.iter().any(|i| i.source == InstallSource::System)
            })
        })
        .collect();
    if available.is_empty() {
        eprintln!("no system Redis/PostgreSQL; skipped");
        return;
    }
    for kind in available {
        let name = format!("t-{}", kind.id());
        let Response::Instance(view) = daemon
            .call(Request::CreateInstance {
                name: name.clone(),
                kind,
                version: None,
                port: None,
                autostart: false,
            })
            .await
            .unwrap()
        else {
            panic!();
        };
        assert!(view.data_dir.starts_with(&daemon.paths.data_dir));
        let port = view.instance.port;
        daemon
            .call(Request::ControlInstance {
                name: name.clone(),
                action: ProcessAction::Start,
            })
            .await
            .unwrap();
        let mut ready = false;
        for _ in 0..100 {
            let Response::Services { instances, .. } = daemon
                .call(Request::ListServices { refresh: false })
                .await
                .unwrap()
            else {
                panic!();
            };
            if instances.iter().any(|i| i.instance.name == name && i.ready) {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(ready, "{kind:?} on {port}");

        let Response::InstanceBackup(backup) = daemon
            .call(Request::BackupInstance { name: name.clone() })
            .await
            .unwrap()
        else {
            panic!();
        };
        assert!(backup.path.is_file() && backup.size > 0);

        let error = daemon
            .call(Request::RemoveInstance {
                name: name.clone(),
                delete_data: true,
                confirm: Some("nope".into()),
            })
            .await
            .unwrap_err();
        assert!(matches!(error, IpcError::App(e) if e.code == ErrorCode::Unauthorized));
        assert!(
            view.data_dir.exists(),
            "data untouched after a wrong confirmation"
        );
        daemon
            .call(Request::RemoveInstance {
                name: name.clone(),
                delete_data: true,
                confirm: Some(name.clone()),
            })
            .await
            .unwrap();
        assert!(!view.data_dir.exists());
        assert!(backup.path.is_file(), "backups are kept");
    }
}

/// The database manager on a real SQLite file: connections from the
/// site's .env (no secret on the wire), destructive SQL gated by the name,
/// row edits, and a remote .env database refused until allowed.
#[tokio::test]
async fn database_manager_on_sqlite() {
    use locdev_core::db::{DbFilter, RowEdit};

    let daemon = Daemon::start().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("database")).unwrap();
    std::fs::write(dir.path().join("artisan"), "").unwrap();
    std::fs::write(dir.path().join("composer.json"), "{}").unwrap();
    std::fs::write(
        dir.path().join(".env"),
        "DB_CONNECTION=sqlite\nREDIS_URL=redis://:topsecret@cache.example.com:6379\n",
    )
    .unwrap();
    daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("shop".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();

    let Response::DbConnections { connections } = daemon
        .call(Request::DbConnections {
            name: Some("shop".into()),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(
        !serde_json::to_string(&connections)
            .unwrap()
            .contains("topsecret")
    );
    let sqlite = connections
        .iter()
        .find(|c| c.engine == locdev_core::db::DbEngine::Sqlite)
        .unwrap()
        .id
        .clone();
    let redis = connections.iter().find(|c| c.remote).unwrap().id.clone();

    // The file doesn't exist yet: reported, then created on request.
    let error = daemon
        .call(Request::DbSchema {
            connection: sqlite.clone(),
            allow_remote: false,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(&error, IpcError::App(e) if e.code == ErrorCode::NotFound),
        "{error:?}"
    );
    daemon
        .call(Request::DbCreateDatabase {
            connection: sqlite.clone(),
            database: None,
        })
        .await
        .unwrap();
    assert!(dir.path().join("database/database.sqlite").is_file());

    let query = |sql: &str, confirm: Option<&str>| Request::DbQuery {
        connection: sqlite.clone(),
        sql: sql.into(),
        confirm: confirm.map(str::to_string),
        allow_remote: false,
    };
    daemon
        .call(query(
            "CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT)",
            None,
        ))
        .await
        .unwrap();
    daemon
        .call(Request::DbEditRow {
            connection: sqlite.clone(),
            table: "posts".into(),
            schema: String::new(),
            edit: RowEdit::Insert {
                values: vec![("title".into(), Some("Olá".into()))],
            },
            allow_remote: false,
        })
        .await
        .unwrap();
    let Response::DbRows(rows) = daemon
        .call(Request::DbRows {
            connection: sqlite.clone(),
            table: "posts".into(),
            schema: String::new(),
            page: 0,
            page_size: None,
            order_by: Some("id".into()),
            desc: false,
            filters: vec![DbFilter {
                column: "title".into(),
                op: "like".into(),
                value: Some("Ol%".into()),
            }],
            allow_remote: false,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(rows.total, Some(1));
    assert_eq!(rows.rows[0][1].as_deref(), Some("Olá"));

    // Destructive SQL: refused without the name, run with it.
    let error = daemon
        .call(query("DELETE FROM posts", None))
        .await
        .unwrap_err();
    assert!(
        matches!(&error, IpcError::App(e) if e.code == ErrorCode::Unauthorized),
        "{error:?}"
    );
    let error = daemon
        .call(query("DELETE FROM posts", Some("wrong")))
        .await
        .unwrap_err();
    assert!(matches!(&error, IpcError::App(e) if e.code == ErrorCode::Unauthorized));
    let Response::DbRows(done) = daemon
        .call(query("DELETE FROM posts", Some("database.sqlite")))
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(done.affected, Some(1));

    // A database on another host is never opened implicitly.
    let error = daemon
        .call(Request::RedisKeys {
            connection: redis,
            cursor: None,
            pattern: None,
            allow_remote: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(&error, IpcError::App(e) if e.code == ErrorCode::Unauthorized));
}

/// Config Files through the real daemon: only existing files, options gated
/// by the installed version, a save that keeps comments and needs the
/// reviewed hash, the history and a revert, and the analysis.
#[tokio::test]
async fn config_files_edit_safely() {
    use locdev_core::config::{ConfigChange, OptionState};

    let daemon = Daemon::start().await;
    let dir = tempfile::tempdir().unwrap();
    let vite = "import { defineConfig } from 'vite'\n\nexport default defineConfig({\n  server: {\n    port: 5173, // LocDev proxies this\n    host: true,\n  },\n})\n";
    std::fs::write(dir.path().join("vite.config.ts"), vite).unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        "{\n  \"name\": \"web\",\n  \"scripts\": { \"dev\": \"vite\" },\n  \"devDependencies\": { \"vite\": \"^7.1.0\" }\n}\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("node_modules/vite")).unwrap();
    std::fs::write(
        dir.path().join("node_modules/vite/package.json"),
        "{\"version\":\"7.1.3\"}",
    )
    .unwrap();
    daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("web".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();

    let Response::ConfigFiles { files } = daemon
        .call(Request::ConfigFiles { name: "web".into() })
        .await
        .unwrap()
    else {
        panic!();
    };
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["vite.config.ts", "package.json"]);
    assert_eq!(files[0].version.as_deref(), Some("7.1.3"));

    let Response::ConfigFile(view) = daemon
        .call(Request::ConfigFile {
            name: "web".into(),
            path: "vite.config.ts".into(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    let port = view
        .options
        .iter()
        .find(|o| o.key == "server.port")
        .unwrap();
    assert_eq!(
        port.state,
        OptionState::Set {
            value: serde_json::json!(5173)
        }
    );

    // Outside the list: refused, whatever the path.
    let err = daemon
        .call(Request::ConfigFile {
            name: "web".into(),
            path: "../../etc/passwd".into(),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(err, IpcError::App(ref e) if e.code == locdev_core::ErrorCode::NotFound),
        "{err:?}"
    );

    let change = ConfigChange::Set {
        key: "server.port".into(),
        value: serde_json::json!(5180),
    };
    let Response::ConfigPreview(preview) = daemon
        .call(Request::ConfigPreview {
            name: "web".into(),
            path: "vite.config.ts".into(),
            change: change.clone(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(
        preview
            .diff
            .contains("+    port: 5180, // LocDev proxies this")
    );
    // Nothing written by a preview.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("vite.config.ts")).unwrap(),
        vite
    );

    let stale = daemon
        .call(Request::ConfigSave {
            name: "web".into(),
            path: "vite.config.ts".into(),
            change: change.clone(),
            base_hash: "0".repeat(64),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(stale, IpcError::App(ref e) if e.code == locdev_core::ErrorCode::Conflict),
        "{stale:?}"
    );

    let Response::ConfigSaved { hash } = daemon
        .call(Request::ConfigSave {
            name: "web".into(),
            path: "vite.config.ts".into(),
            change,
            base_hash: view.hash.clone(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(
        std::fs::read_to_string(dir.path().join("vite.config.ts")).unwrap(),
        vite.replace("5173", "5180")
    );

    let Response::ConfigHistory { entries } = daemon
        .call(Request::ConfigHistory {
            name: "web".into(),
            path: "vite.config.ts".into(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(entries.len(), 1);
    let Response::ConfigHistoryVersion { content } = daemon
        .call(Request::ConfigHistoryVersion {
            name: "web".into(),
            path: "vite.config.ts".into(),
            id: entries[0].id.clone(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(content, vite);

    let Response::ConfigFindings { findings } = daemon
        .call(Request::ConfigCheck { name: "web".into() })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(
        findings.iter().any(|f| f.rule == "vite.host-exposed"),
        "{findings:?}"
    );

    daemon
        .call(Request::ConfigRevert {
            name: "web".into(),
            path: "vite.config.ts".into(),
            id: entries[0].id.clone(),
            base_hash: hash,
        })
        .await
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("vite.config.ts")).unwrap(),
        vite
    );
}

/// Deploy assistant through the real daemon: the analysis names no secret,
/// platforms come ranked by fit, and a config file is written only as
/// reviewed and never over an existing one.
#[tokio::test]
async fn deploy_assistant_recommends_and_prepares() {
    use locdev_core::deploy::Verdict;

    let daemon = Daemon::start().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name":"web","scripts":{"dev":"next dev","build":"next build","start":"next start"},"dependencies":{"next":"16.0.0","react":"19.0.0"}}"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("node_modules/next")).unwrap();
    std::fs::write(
        dir.path().join("node_modules/next/package.json"),
        r#"{"version":"16.0.0"}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("next.config.mjs"), "export default {}\n").unwrap();
    std::fs::write(dir.path().join(".env"), "DATABASE_URL=postgresql://app:s3cr3t-value@127.0.0.1:5432/web\nAUTH_SECRET=another-s3cret\n").unwrap();
    daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("web".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();

    let Response::DeployReport(report) = daemon
        .call(Request::DeployAnalysis { name: "web".into() })
        .await
        .unwrap()
    else {
        panic!();
    };
    let json = serde_json::to_string(&report).unwrap();
    assert!(
        !json.contains("s3cr3t") && !json.contains("another-s3cret"),
        "no value leaves the daemon"
    );
    assert_eq!(
        report.profile.rendering,
        locdev_core::deploy::Rendering::Ssr
    );
    assert!(
        report.profile.needs.iter().any(|n| n.id == "postgres"),
        "{:?}",
        report.profile.needs
    );
    let verdicts: Vec<Verdict> = report.providers.iter().map(|p| p.verdict).collect();
    assert!(verdicts.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(
        report
            .providers
            .iter()
            .find(|p| p.id == "github-pages")
            .unwrap()
            .verdict,
        Verdict::NotSuitable
    );

    let Response::DeployConfig(config) = daemon
        .call(Request::DeployConfig {
            name: "web".into(),
            provider: "render".into(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(!config.exists && !config.content.contains("s3cr3t"));
    assert!(
        !dir.path().join("render.yaml").exists(),
        "a preview writes nothing"
    );
    daemon
        .call(Request::DeployWriteConfig {
            name: "web".into(),
            provider: "render".into(),
            hash: config.hash.clone(),
        })
        .await
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("render.yaml")).unwrap(),
        config.content
    );
    let again = daemon
        .call(Request::DeployWriteConfig {
            name: "web".into(),
            provider: "render".into(),
            hash: config.hash,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(again, IpcError::App(ref e) if e.code == locdev_core::ErrorCode::Conflict),
        "{again:?}"
    );
}

/// Subdomains through the real daemon and proxy: the wildcard, a rule to
/// another port, the wildcard off, and `ExplainHost` telling DNS, route and
/// certificate apart.
#[tokio::test]
async fn subdomain_rules_route_through_the_proxy() {
    use locdev_core::subdomains::{SubdomainRule, SubdomainTarget, Subdomains};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn upstream(name: &'static str) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            loop {
                let (mut s, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let _ = s.read(&mut buf).await;
                    let r = format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{name}",
                        name.len()
                    );
                    let _ = s.write_all(r.as_bytes()).await;
                });
            }
        });
        port
    }
    async fn get(proxy: u16, host: &str) -> String {
        let mut s = tokio::net::TcpStream::connect(("127.0.0.1", proxy))
            .await
            .unwrap();
        s.write_all(
            format!("GET / HTTP/1.1\r\nhost: {host}\r\nconnection: close\r\n\r\n").as_bytes(),
        )
        .await
        .unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        out
    }

    let daemon = Daemon::start().await;
    let site = upstream("site").await;
    let api = upstream("api").await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "x").unwrap();
    daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("shop".into()),
            command: None,
            port: Some(site),
            auto_start: false,
        })
        .await
        .unwrap();
    let Response::Status(status) = daemon.call(Request::Status).await.unwrap() else {
        panic!()
    };
    let proxy = status.proxy.port.unwrap();
    let body = |r: String| r.rsplit("\r\n\r\n").next().unwrap_or("").to_string();
    // Before any rule: every subdomain is the site (how it always worked).
    assert_eq!(body(get(proxy, "cliente1.shop.test").await), "site");

    let rules = |wildcard| Subdomains {
        wildcard,
        rules: vec![SubdomainRule {
            pattern: "API".into(),
            target: SubdomainTarget::Port { port: api },
        }],
    };
    let Response::Project { project } = daemon
        .call(Request::SetSubdomains {
            name: "shop".into(),
            subdomains: rules(true),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        project.project.subdomains.rules[0].pattern, "api",
        "normalized"
    );
    assert_eq!(body(get(proxy, "api.shop.test").await), "api");
    assert_eq!(body(get(proxy, "x.y.shop.test").await), "site");

    daemon
        .call(Request::SetSubdomains {
            name: "shop".into(),
            subdomains: rules(false),
        })
        .await
        .unwrap();
    assert!(get(proxy, "x.shop.test").await.starts_with("HTTP/1.1 404"));
    assert_eq!(body(get(proxy, "shop.test").await), "site");

    let bad = daemon
        .call(Request::SetSubdomains {
            name: "shop".into(),
            subdomains: Subdomains {
                wildcard: true,
                rules: vec![SubdomainRule {
                    pattern: "q".into(),
                    target: SubdomainTarget::Process {
                        id: "reverb".into(),
                    },
                }],
            },
        })
        .await
        .unwrap_err();
    assert!(
        matches!(bad, IpcError::App(ref e) if e.message.contains("no auxiliary process")),
        "{bad:?}"
    );

    let Response::HostExplain(explain) = daemon
        .call(Request::ExplainHost {
            host: "API.shop.test:443".into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(explain.host, "api.shop.test");
    assert_eq!(explain.route.rule.as_deref(), Some("api"));
    assert_eq!(
        (explain.route.port, explain.route.reachable),
        (Some(api), Some(true))
    );
    assert!(explain.tls.names.contains(&"*.shop.test".to_string()));
    let Response::HostExplain(off) = daemon
        .call(Request::ExplainHost {
            host: "nope.shop.test".into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        off.route.problem.as_deref(),
        Some("the wildcard is off and no rule matches")
    );
    let Response::HostExplain(deep) = daemon
        .call(Request::ExplainHost {
            host: "a.tenants.shop.test".into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(deep.tls.names.contains(&"*.tenants.shop.test".to_string()));
}

/// The DNS server falls back to the next loopback address when the first is
/// taken (how Windows avoids another tool on 127.0.0.1:53), and reports it.
/// macOS only puts 127.0.0.1 on `lo0`, so 127.53.0.1 can't be bound there
/// (the fallback is only configured by default on Windows).
#[tokio::test]
#[cfg_attr(target_os = "macos", ignore = "macOS has no 127.53.0.1 on lo0")]
async fn dns_falls_back_to_its_own_loopback_address() {
    let taken = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();
    let daemon = Daemon::start_with(
        tempfile::tempdir().unwrap(),
        &format!("port = {port}\naddresses = [\"127.0.0.1\", \"127.53.0.1\"]"),
    )
    .await;
    let Response::Status(status) = daemon.call(Request::Status).await.unwrap() else {
        panic!()
    };
    assert_eq!(
        status.dns.address.as_deref(),
        Some("127.53.0.1"),
        "{:?}",
        status.dns.error
    );
    assert_eq!(
        status.dns.listen(),
        Some(format!("127.53.0.1:{port}").parse().unwrap())
    );

    // It answers there.
    let client = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client
        .send_to(
            &locdev_dns::wire::query(9, "a.b.shop.test", 1, true),
            ("127.53.0.1", port),
        )
        .await
        .unwrap();
    let mut buf = [0u8; 512];
    let (len, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut buf))
        .await
        .unwrap()
        .unwrap();
    assert!(buf[..len].windows(4).any(|w| w == [127, 0, 0, 1]));
    drop(taken);
}

/// The Mailbox through the real daemon: a Laravel site's processes get the
/// SMTP variables only when it's on, a real SMTP needs consent, and a
/// message sent like the framework would is captured under the site.
#[tokio::test]
async fn mailbox_wires_sites_and_captures() {
    let daemon = Daemon::start().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("artisan"), "").unwrap();
    std::fs::write(
        dir.path().join("composer.json"),
        r#"{"require":{"laravel/framework":"^12.0"}}"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("config")).unwrap();
    std::fs::write(
        dir.path().join("config/mail.php"),
        "'scheme' => env('MAIL_SCHEME'),",
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".env"),
        "MAIL_MAILER=smtp\nMAIL_HOST=smtp.postmarkapp.com\nMAIL_PASSWORD=real-secret\n",
    )
    .unwrap();
    daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("shop".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();

    let Response::MailboxStatus(status) = daemon.call(Request::MailboxStatus).await.unwrap() else {
        panic!()
    };
    let port = status.port.expect("mail server listening");

    let Response::MailSetup(setup) = daemon
        .call(Request::MailSetup {
            name: "shop".into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(setup.real_smtp);
    assert!(
        !serde_json::to_string(&setup)
            .unwrap()
            .contains("real-secret")
    );
    assert!(
        setup
            .injects
            .iter()
            .any(|h| h.name == "MAIL_PORT" && h.value == port.to_string())
    );

    // Real SMTP: refused without consent, accepted with it.
    let refused = daemon
        .call(Request::MailEnable {
            name: "shop".into(),
            enabled: true,
            override_real: false,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(refused, IpcError::App(ref e) if e.code == locdev_core::ErrorCode::Conflict),
        "{refused:?}"
    );
    let Response::MailSetup(on) = daemon
        .call(Request::MailEnable {
            name: "shop".into(),
            enabled: true,
            override_real: true,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(on.enabled);

    // A Laravel-style send: AUTH as the site, then the message.
    let raw = b"From: Shop <noreply@shop.test>\r\nTo: ana@example.com\r\nSubject: Order shipped\r\nDate: Tue, 03 Nov 2026 10:00:00 +0000\r\nMessage-ID: <9@shop.test>\r\n\r\nYour order is on its way.\r\n";
    locdev_mail::client::send(
        ([127, 0, 0, 1], port).into(),
        Some("shop"),
        "noreply@shop.test",
        &["ana@example.com".into()],
        raw,
    )
    .await
    .unwrap();
    daemon
        .call(Request::MailSendTest {
            name: None,
            to: None,
        })
        .await
        .unwrap();

    let Response::MailList { messages } = daemon
        .call(Request::MailList {
            site: Some("shop".into()),
            query: None,
            unread: false,
            attachments: false,
            limit: None,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].subject, "Order shipped");
    // The capture became a notification (category mail), and the Status
    // counts it as unread mail, apart from the other notifications.
    let mail_notification = |paths: Paths| async move {
        let Response::Notifications { notifications, .. } =
            request(&paths, Request::ListNotifications { all: false })
                .await
                .unwrap()
        else {
            panic!()
        };
        notifications.into_iter().find(|n| {
            n.category == locdev_core::deps::NotificationCategory::Mail
                && n.project.as_deref() == Some("shop")
        })
    };
    let mut found = None;
    for _ in 0..50 {
        found = mail_notification(daemon.paths.clone()).await;
        if found.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let notification = found.expect("a notification for the captured message");
    assert!(!notification.read);
    assert_eq!(notification.title, "New email from shop");
    let Response::Status(status) = daemon.call(Request::Status).await.unwrap() else {
        panic!()
    };
    assert!(status.mail.unread >= 1);
    assert_eq!(status.unread_notifications, 0);
    let Response::MailDetail(detail) = daemon
        .call(Request::MailGet {
            id: messages[0].id.clone(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(detail.envelope.auth_user.as_deref(), Some("shop"));
    assert_eq!(detail.framework.as_deref(), Some("Laravel"));
    // Opening the message reads its notification too.
    assert!(mail_notification(daemon.paths.clone()).await.unwrap().read);
    let Response::MailList { messages: all } = daemon
        .call(Request::MailList {
            site: None,
            query: Some("locdev test".into()),
            unread: false,
            attachments: false,
            limit: None,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(all.len(), 1);
    let Response::MailDeleted { count } = daemon
        .call(Request::MailClear { site: None })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(count, 2);
}

/// Email Studio through the real daemon: generator (plain HTML stack, no
/// overwrite), a handwritten Blade template whose logic survives a visual
/// edit, the reviewed-hash save, an external edit noticed by its hash, and a
/// test send captured by the Mailbox.
#[tokio::test]
async fn email_studio_keeps_code_and_saves_safely() {
    use locdev_core::email::{PreviewMode, Segment};

    let daemon = Daemon::start().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("artisan"), "").unwrap();
    std::fs::write(
        dir.path().join("composer.json"),
        r#"{"require":{"laravel/framework":"^12.0"}}"#,
    )
    .unwrap();
    let rel = "resources/views/emails/order.blade.php";
    std::fs::create_dir_all(dir.path().join("resources/views/emails")).unwrap();
    let handwritten = "<p>Hi {{ $user->name }}</p>\n@foreach ($items as $item)\n  <li>{{ $item }}</li>\n@endforeach\n";
    std::fs::write(dir.path().join(rel), handwritten).unwrap();
    daemon
        .call(Request::AddProject {
            path: dir.path().to_path_buf(),
            name: Some("shop".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();

    let Response::EmailCatalog(catalog) = daemon
        .call(Request::EmailCatalog {
            name: "shop".into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        catalog
            .templates
            .iter()
            .map(|t| t.path.as_str())
            .collect::<Vec<_>>(),
        vec![rel]
    );

    let Response::EmailDoc(doc) = daemon
        .call(Request::EmailOpen {
            name: "shop".into(),
            path: rel.into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(doc.variables, vec!["user", "items", "item"]);
    // Add a button after the handwritten code.
    let mut segments = doc.segments.clone();
    segments.push(Segment::Block {
        id: "new-1".into(),
        block: "button".into(),
        props: serde_json::json!({ "text": "Track", "url": "{{ $url }}" }),
        edited: false,
    });
    let Response::EmailBuilt { content, diff } = daemon
        .call(Request::EmailBuild {
            name: "shop".into(),
            path: rel.into(),
            segments: segments.clone(),
            theme: doc.theme.clone(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(
        content.starts_with(handwritten),
        "the handwritten Blade is kept byte for byte"
    );
    assert!(diff.contains("+{{-- locdev:block") && !diff.contains("-@foreach"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join(rel)).unwrap(),
        handwritten,
        "building writes nothing"
    );

    let Response::EmailPreview(preview) = daemon
        .call(Request::EmailPreview {
            name: "shop".into(),
            path: rel.into(),
            segments: Some(segments),
            theme: Some(doc.theme.clone()),
            data: serde_json::json!({ "user": { "name": "Ana" }, "url": "https://shop.test/t", "item": "Mug" }),
            mode: PreviewMode::Static,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(
        preview.html.contains("Hi Ana") && preview.html.contains("href=\"https://shop.test/t\"")
    );

    let stale = daemon
        .call(Request::EmailSave {
            name: "shop".into(),
            path: rel.into(),
            content: content.clone(),
            base_hash: "0".repeat(64),
        })
        .await
        .unwrap_err();
    assert!(matches!(stale, IpcError::App(ref e) if e.code == locdev_core::ErrorCode::Conflict));
    let Response::EmailSaved { hash } = daemon
        .call(Request::EmailSave {
            name: "shop".into(),
            path: rel.into(),
            content: content.clone(),
            base_hash: doc.hash.clone(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        std::fs::read_to_string(dir.path().join(rel)).unwrap(),
        content
    );

    // Edited in an IDE: the hash moves, so the app notices.
    std::fs::write(
        dir.path().join(rel),
        format!("{content}<!-- from the IDE -->\n"),
    )
    .unwrap();
    let Response::EmailStat { hash: now } = daemon
        .call(Request::EmailStat {
            name: "shop".into(),
            path: rel.into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_ne!(now, hash);
    let Response::EmailDoc(reopened) = daemon
        .call(Request::EmailOpen {
            name: "shop".into(),
            path: rel.into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(
        reopened
            .segments
            .iter()
            .any(|s| matches!(s, Segment::Block { block, edited: false, .. } if block == "button"))
    );

    // Components: created once, never over themselves.
    let Response::EmailCreated(created) = daemon
        .call(Request::EmailNew {
            name: "shop".into(),
            component: true,
            item: "footer".into(),
            markdown: false,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        created.files,
        vec!["resources/views/components/mail/footer.blade.php"]
    );
    let again = daemon
        .call(Request::EmailNew {
            name: "shop".into(),
            component: true,
            item: "footer".into(),
            markdown: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(again, IpcError::App(ref e) if e.code == locdev_core::ErrorCode::Conflict));

    daemon
        .call(Request::EmailSendTest {
            name: "shop".into(),
            path: rel.into(),
            segments: None,
            theme: None,
            data: serde_json::json!({}),
            mode: PreviewMode::Static,
            to: None,
        })
        .await
        .unwrap();
    let Response::MailList { messages } = daemon
        .call(Request::MailList {
            site: Some("shop".into()),
            query: None,
            unread: false,
            attachments: false,
            limit: None,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(messages[0].subject, format!("[Email Studio] {rel}"));
}

/// A project created end to end: plan, missing tools, the steps in a real
/// terminal (with a question answered through it), and the site registered.
/// `npm` is a fake on PATH, so nothing is downloaded.
#[cfg(unix)]
#[tokio::test]
async fn creates_a_project_in_a_terminal() {
    use locdev_core::scaffold::NewProject;
    use std::os::unix::fs::PermissionsExt;

    let bin = tempfile::tempdir().unwrap();
    let npm = bin.path().join("npm");
    std::fs::write(
        &npm,
        "#!/bin/sh\nif [ \"$1\" = create ]; then\n  printf 'Project name? '\n  read answer\n  mkdir -p \"$4\"\n  printf '{\"name\":\"%s\",\"scripts\":{\"dev\":\"vite\"},\"devDependencies\":{\"vite\":\"7.0.0\"}}' \"$answer\" > \"$4/package.json\"\n  echo \"created $4 with $6 $7\"\nelse\n  echo \"npm $*\"\nfi\n",
    )
    .unwrap();
    std::fs::set_permissions(&npm, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:/usr/bin:/bin", bin.path().display());
    let daemon = Daemon::start_full(tempfile::tempdir().unwrap(), "port = 0", Some(path)).await;
    let code = tempfile::tempdir().unwrap();
    let project = NewProject {
        starter: "vite".into(),
        method: "npm".into(),
        name: "shop".into(),
        parent: code.path().to_path_buf(),
        register: true,
        ..Default::default()
    };

    let Response::ProjectPlan(plan) = daemon
        .call(Request::PlanProject {
            project: project.clone(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        plan.steps[0].display(),
        "npm create --yes vite@latest shop -- --template react-ts --no-interactive"
    );
    assert!(plan.missing.is_empty(), "{:?}", plan.missing);
    // A tool that isn't there blocks the run.
    let mut bun = project.clone();
    bun.method = "bun".into();
    let Response::ProjectPlan(plan) = daemon
        .call(Request::PlanProject {
            project: bun.clone(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(plan.missing, ["Bun"]);
    assert!(
        daemon
            .call(Request::CreateProject { project: bun })
            .await
            .is_err()
    );

    let Response::Terminal(info) = daemon
        .call(Request::CreateProject { project })
        .await
        .unwrap()
    else {
        panic!()
    };
    let mut seen = String::new();
    let mut from = 0;
    let mut answered = false;
    let done = loop {
        let Response::TerminalOutput(out) = daemon
            .call(Request::TerminalRead { id: info.id, from })
            .await
            .unwrap()
        else {
            panic!()
        };
        seen.push_str(&out.data);
        from = out.next;
        if !answered && seen.contains("Project name?") {
            daemon
                .call(Request::TerminalWrite {
                    id: info.id,
                    data: "acme\r".into(),
                })
                .await
                .unwrap();
            answered = true;
        }
        if !out.info.running {
            break out.info;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert_eq!(done.code, Some(0), "{seen}");
    assert!(
        seen.contains("created shop with --template react-ts"),
        "{seen}"
    );
    assert!(seen.contains("npm install"), "{seen}");
    assert_eq!(done.site.as_deref(), Some("shop"));
    let manifest = std::fs::read_to_string(code.path().join("shop/package.json")).unwrap();
    assert!(
        manifest.contains("\"name\":\"acme\""),
        "the answer went through the terminal: {manifest}"
    );
    let Response::Projects { projects } = daemon.call(Request::ListProjects).await.unwrap() else {
        panic!()
    };
    assert!(projects.iter().any(|p| p.project.name == "shop"));

    // The folder exists now: creating it again is refused.
    let again = NewProject {
        starter: "vite".into(),
        method: "npm".into(),
        name: "shop".into(),
        parent: code.path().to_path_buf(),
        ..Default::default()
    };
    assert!(
        daemon
            .call(Request::PlanProject { project: again })
            .await
            .is_err()
    );
}

/// Waits until `check` holds for the parked folders (file events are
/// gathered for a moment before a rescan).
async fn wait_parked(
    daemon: &Daemon,
    check: impl Fn(&[locdev_core::park::ParkedView]) -> bool,
) -> Vec<locdev_core::park::ParkedView> {
    for _ in 0..150 {
        let Response::Parked { dirs, .. } = daemon.call(Request::ParkList).await.unwrap() else {
            panic!();
        };
        if check(&dirs) {
            return dirs;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("parked folders never reached the expected state");
}

async fn site_names(daemon: &Daemon) -> Vec<String> {
    let Response::Projects { projects } = daemon.call(Request::ListProjects).await.unwrap() else {
        panic!();
    };
    projects.into_iter().map(|p| p.project.name).collect()
}

#[tokio::test]
async fn parked_folders_follow_their_projects() {
    use locdev_core::park::{OnNew, ParkOptions, ParkedState};

    let daemon = Daemon::start().await;
    let dev = tempfile::Builder::new()
        .prefix("devroot")
        .tempdir()
        .unwrap();
    let root = dev.path().canonicalize().unwrap();
    let write = |rel: &str, body: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    };
    write("shop/artisan", "");
    write("shop/composer.json", "{}");
    write("api/Cargo.toml", "[package]\nname = \"api\"\n");
    write("notes/todo.txt", "not a project");
    write("node_modules/x/package.json", "{}");
    write("web/index.php", "<?php");
    // Already a site, added by hand: left alone.
    daemon
        .call(Request::AddProject {
            path: root.join("web"),
            name: Some("my-web".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();

    // A folder that is itself a project, and one inside a parked folder,
    // are refused.
    let error = daemon
        .call(Request::ParkSet {
            path: root.join("shop"),
            options: ParkOptions::default(),
        })
        .await
        .unwrap_err();
    assert!(matches!(&error, IpcError::App(e) if e.message.contains("itself a project")));

    let Response::Parked { dirs, added, .. } = daemon
        .call(Request::ParkSet {
            path: root.clone(),
            options: ParkOptions::default(),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(added, ["api", "shop"]);
    let states: Vec<(&str, ParkedState)> = dirs[0]
        .projects
        .iter()
        .map(|p| (p.rel.as_str(), p.state))
        .collect();
    assert_eq!(
        states,
        [
            ("api", ParkedState::Registered),
            ("shop", ParkedState::Registered),
            ("web", ParkedState::Manual)
        ]
    );
    let error = daemon
        .call(Request::ParkSet {
            path: root.join("notes"),
            options: ParkOptions::default(),
        })
        .await
        .unwrap_err();
    assert!(matches!(&error, IpcError::App(e) if e.code == ErrorCode::AlreadyExists));

    // New projects and removed folders are picked up from file events.
    assert!(dirs[0].watching, "{:?}", dirs[0].watch_error);
    write("blog/index.php", "<?php");
    wait_parked(&daemon, |d| d[0].projects.iter().any(|p| p.rel == "blog")).await;
    assert!(site_names(&daemon).await.contains(&"blog".to_string()));
    std::fs::remove_dir_all(root.join("api")).unwrap();
    wait_parked(&daemon, |d| !d[0].projects.iter().any(|p| p.rel == "api")).await;
    assert!(!site_names(&daemon).await.contains(&"api".to_string()));

    // A name that's taken gets the parent folder's name.
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(elsewhere.path().join("index.php"), "<?php").unwrap();
    daemon
        .call(Request::AddProject {
            path: elsewhere.path().to_path_buf(),
            name: Some("dash".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();
    write("dash/index.php", "<?php");
    let Response::Parked { added, dirs, .. } =
        daemon.call(Request::ParkScan { path: None }).await.unwrap()
    else {
        panic!();
    };
    let dash_name = format!(
        "dash-{}",
        locdev_core::slugify(&root.file_name().unwrap().to_string_lossy())
    );
    assert_eq!(added, std::slice::from_ref(&dash_name));
    let dash = dirs[0].projects.iter().find(|p| p.rel == "dash").unwrap();
    assert!(dash.note.as_deref().unwrap().contains("taken"));

    // A parked site removed by hand stays away until it's un-ignored.
    daemon
        .call(Request::RemoveProject {
            name: "shop".into(),
        })
        .await
        .unwrap();
    let Response::Parked { dirs, added, .. } =
        daemon.call(Request::ParkScan { path: None }).await.unwrap()
    else {
        panic!();
    };
    assert!(added.is_empty());
    assert!(dirs[0].dir.ignored.contains(&"shop".to_string()));
    let Response::Parked { added, .. } = daemon
        .call(Request::ParkIgnore {
            path: root.join("shop"),
            ignore: false,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(added, ["shop"]);

    // Suggest mode lists new projects without adding them.
    daemon
        .call(Request::ParkSet {
            path: root.clone(),
            options: ParkOptions {
                on_new: Some(OnNew::Suggest),
                ..Default::default()
            },
        })
        .await
        .unwrap();
    write("later/package.json", r#"{"scripts":{"dev":"vite"}}"#);
    let Response::Parked { dirs, added, .. } = daemon
        .call(Request::ParkScan {
            path: Some(root.clone()),
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(added.is_empty());
    let later = dirs[0].projects.iter().find(|p| p.rel == "later").unwrap();
    assert_eq!(later.state, ParkedState::Suggested);

    // The list and the sites survive a restart.
    let home = daemon.stop();
    let daemon = Daemon::start_in(home).await;
    let dirs = wait_parked(&daemon, |d| {
        d.first().is_some_and(|d| d.last_scan.is_some())
    })
    .await;
    assert_eq!(dirs[0].dir.on_new, OnNew::Suggest);
    let names = site_names(&daemon).await;
    for name in ["blog", &dash_name, "my-web", "shop"] {
        assert!(names.contains(&name.to_string()), "{names:?}");
    }

    // Unparking removes its sites (never their files) but not manual ones.
    let Response::Parked { dirs, removed, .. } = daemon
        .call(Request::ParkRemove {
            path: root.clone(),
            keep_sites: false,
        })
        .await
        .unwrap()
    else {
        panic!();
    };
    assert!(dirs.is_empty());
    assert_eq!(removed.len(), 3, "{removed:?}");
    assert_eq!(site_names(&daemon).await, ["dash", "my-web"]);
    assert!(root.join("shop/artisan").exists());
}

#[tokio::test]
async fn starter_files_are_written_before_the_steps() {
    use locdev_core::scaffold::NewProject;

    let daemon = Daemon::start().await;
    let code = tempfile::tempdir().unwrap();
    let project = NewProject {
        starter: "php".into(),
        method: "files".into(),
        name: "plain".into(),
        parent: code.path().to_path_buf(),
        options: [("git".to_string(), "false".to_string())].into(),
        register: true,
        ..Default::default()
    };

    // A version that isn't installed is refused before anything runs.
    let mut pinned = project.clone();
    pinned.node = Some("3".into());
    let error = daemon
        .call(Request::PlanProject { project: pinned })
        .await
        .unwrap_err();
    assert!(
        matches!(&error, IpcError::App(e) if e.message.contains("not installed")),
        "{error:?}"
    );

    let Response::ProjectPlan(plan) = daemon
        .call(Request::PlanProject {
            project: project.clone(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(plan.make_dir);
    assert_eq!(plan.files[0].path, "public/index.php");
    assert!(plan.steps.is_empty());

    let Response::Terminal(info) = daemon
        .call(Request::CreateProject { project })
        .await
        .unwrap()
    else {
        panic!()
    };
    let mut out = String::new();
    let mut from = 0;
    for _ in 0..100 {
        let Response::TerminalOutput(o) = daemon
            .call(Request::TerminalRead { id: info.id, from })
            .await
            .unwrap()
        else {
            panic!()
        };
        out.push_str(&o.data);
        from = o.next;
        if !o.info.running {
            assert_eq!(o.info.code, Some(0), "{out}");
            assert_eq!(o.info.site.as_deref(), Some("plain"));
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(out.contains("Wrote public/index.php"), "{out}");
    let index = code.path().join("plain/public/index.php");
    assert!(
        std::fs::read_to_string(index)
            .unwrap()
            .contains("PHP_VERSION")
    );
    // Registered with the command the PHP definition gives public/.
    let Response::Projects { projects } = daemon.call(Request::ListProjects).await.unwrap() else {
        panic!()
    };
    let site = projects.iter().find(|p| p.project.name == "plain").unwrap();
    assert!(
        site.project
            .command
            .as_deref()
            .unwrap()
            .ends_with("-t public"),
        "{:?}",
        site.project.command
    );
}

#[tokio::test]
async fn php_settings_reach_running_sites() {
    use locdev_core::phpini::PhpIniChange;
    use std::collections::BTreeMap;

    let daemon = Daemon::start().await;
    // Runtime detection runs in the background after startup.
    let mut lines = Vec::new();
    for _ in 0..100 {
        if let Ok(Response::PhpLines { lines: found }) = daemon.call(Request::PhpConfigLines).await
            && !found.is_empty()
        {
            lines = found;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let Some(php) = lines.first().cloned() else {
        eprintln!("no PHP found; skipped");
        return;
    };
    let line = php.line.clone();

    // A PHP site that prints what its PHP uses.
    let site = tempfile::tempdir().unwrap();
    std::fs::write(
        site.path().join("index.php"),
        "<?php echo ini_get('memory_limit'), '|', ini_get('max_execution_time');",
    )
    .unwrap();
    let Response::Project { project } = daemon
        .call(Request::AddProject {
            path: site.path().to_path_buf(),
            name: Some("phpcfg".into()),
            command: None,
            // A port from the OS: other tests' daemons run in parallel and
            // hand out ports from the same range.
            port: Some(
                std::net::TcpListener::bind(("127.0.0.1", 0))
                    .unwrap()
                    .local_addr()
                    .unwrap()
                    .port(),
            ),
            auto_start: false,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    let port = project.project.port.unwrap();
    daemon
        .call(Request::UpdateProject {
            name: "phpcfg".into(),
            command: None,
            port: None,
            auto_start: None,
            node: None,
            php: Some(line.clone()),
            tools: Default::default(),
            restart_on_failure: None,
        })
        .await
        .unwrap();
    daemon
        .call(Request::StartProject {
            name: "phpcfg".into(),
        })
        .await
        .unwrap();
    let get = || async move {
        for _ in 0..50 {
            if let Ok(body) = http_get(port, "/").await {
                return body;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("the site never answered");
    };
    let before = get().await;

    let set = |values: &[(&str, Option<&str>)]| PhpIniChange::Set {
        values: values
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.map(str::to_string)))
            .collect::<BTreeMap<_, _>>(),
    };
    // Preview: nothing written, the running site listed.
    let Response::PhpConfig { preview, .. } = daemon
        .call(Request::PhpConfigChange {
            line: line.clone(),
            change: set(&[
                ("memory_limit", Some("321M")),
                ("max_execution_time", Some("47")),
            ]),
            dry_run: true,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    let preview = preview.unwrap();
    assert!(preview.after.contains("memory_limit = 321M"));
    assert_eq!(preview.restart, ["phpcfg"]);
    assert_eq!(get().await, before, "a preview changes nothing");

    let Response::PhpConfig {
        config, restarted, ..
    } = daemon
        .call(Request::PhpConfigChange {
            line: line.clone(),
            change: set(&[
                ("memory_limit", Some("321M")),
                ("max_execution_time", Some("47")),
            ]),
            dry_run: false,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(restarted, ["phpcfg"]);
    let memory = config
        .directives
        .iter()
        .find(|d| d.key == "memory_limit")
        .unwrap();
    assert_eq!(memory.effective.as_deref(), Some("321M"));
    assert_ne!(memory.original.as_deref(), Some("321M"));
    // The site, served by PHP's built-in server, gets both values.
    assert_eq!(get().await, "321M|47");

    // Bad values and broken files are refused; the file stays as it was.
    let saved = std::fs::read_to_string(&config.file).unwrap();
    assert!(
        daemon
            .call(Request::PhpConfigChange {
                line: line.clone(),
                change: set(&[("memory_limit", Some("lots"))]),
                dry_run: false,
            })
            .await
            .is_err()
    );
    let broken = daemon
        .call(Request::PhpConfigChange {
            line: line.clone(),
            change: PhpIniChange::Text {
                text: "memory_limit == \"3\n".into(),
                hash: config.hash.clone(),
            },
            dry_run: false,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(&broken, IpcError::App(e) if e.message.contains("syntax error")),
        "{broken:?}"
    );
    let stale = daemon
        .call(Request::PhpConfigChange {
            line: line.clone(),
            change: PhpIniChange::Text {
                text: "memory_limit = 64M\n".into(),
                hash: "old".into(),
            },
            dry_run: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(&stale, IpcError::App(e) if e.code == ErrorCode::Conflict));
    assert_eq!(std::fs::read_to_string(&config.file).unwrap(), saved);

    // The advanced editor keeps lines it doesn't know; reset puts PHP's
    // own values back, and the previous file is in the history.
    let Response::PhpConfig { config, .. } = daemon
        .call(Request::PhpConfigChange {
            line: line.clone(),
            change: PhpIniChange::Text {
                text: format!("{saved}; mine\nmemory_limit = 300M\n"),
                hash: config.hash.clone(),
            },
            dry_run: false,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(config.text.contains("; mine"));
    assert!(!config.history.is_empty());
    let Response::PhpConfig { config, .. } = daemon
        .call(Request::PhpConfigChange {
            line: line.clone(),
            change: PhpIniChange::Reset,
            dry_run: false,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(!config.file.exists());
    assert_eq!(get().await, before);

    // A file PHP loads can be read; anything else can't.
    if let Some(ini) = config.loaded_ini.clone() {
        let Response::PhpIniFile { text, .. } = daemon
            .call(Request::PhpIniRead {
                line: line.clone(),
                path: ini,
            })
            .await
            .unwrap()
        else {
            panic!()
        };
        assert!(!text.is_empty());
    }
    assert!(
        daemon
            .call(Request::PhpIniRead {
                line,
                path: "/etc/passwd".into(),
            })
            .await
            .is_err()
    );
}

/// A plain HTTP GET to 127.0.0.1:`port`; the body.
async fn http_get(port: u16, path: &str) -> std::io::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port)).await?;
    s.write_all(format!("GET {path} HTTP/1.0\r\nHost: localhost\r\n\r\n").as_bytes())
        .await?;
    let mut out = String::new();
    s.read_to_string(&mut out).await?;
    Ok(out
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default())
}

#[cfg(unix)]
#[tokio::test]
async fn tunnels_share_only_the_running_site() {
    use std::os::unix::fs::PermissionsExt;

    // A stand-in for cloudflared: answers --version and serves /quicktunnel
    // on the --metrics port, like the real program's metrics server.
    let bin = tempfile::tempdir().unwrap();
    let fake = bin.path().join("cloudflared");
    std::fs::write(
        &fake,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "cloudflared version 2026.10.7 (built test)"; exit 0; fi
echo "args: $*"
m=""; prev=""
for a in "$@"; do if [ "$prev" = "--metrics" ]; then m="$a"; fi; prev="$a"; done
exec python3 -c '
import http.server, sys
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = b"{\"hostname\":\"calm-fox-river.trycloudflare.com\"}" if self.path == "/quicktunnel" else b"ok"
        self.send_response(200); self.end_headers(); self.wfile.write(body)
    def log_message(self, *a): pass
http.server.HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
' "${m##*:}"
"#,
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:/usr/bin:/bin", bin.path().display());
    let daemon = Daemon::start_full(tempfile::tempdir().unwrap(), "port = 0", Some(path)).await;

    let Response::TunnelProviders { providers } =
        daemon.call(Request::TunnelProviders).await.unwrap()
    else {
        panic!()
    };
    let program = providers[0]
        .program
        .as_ref()
        .expect("cloudflared found on PATH");
    assert_eq!(program.version, "2026.10.7");
    assert!(!program.managed && program.outdated.is_none());

    let site = tempfile::tempdir().unwrap();
    let port = std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    daemon
        .call(Request::AddProject {
            path: site.path().to_path_buf(),
            name: Some("shared".into()),
            command: Some("python3 -m http.server {port} --bind 127.0.0.1".into()),
            port: Some(port),
            auto_start: false,
        })
        .await
        .unwrap();
    let start = |allowed: Vec<String>| Request::TunnelStart {
        site: "shared".into(),
        provider: None,
        allowed,
    };
    // Only a running site can be shared.
    let error = daemon.call(start(Vec::new())).await.unwrap_err();
    assert!(
        matches!(&error, IpcError::App(e) if e.code == ErrorCode::NotRunnable),
        "{error:?}"
    );
    daemon
        .call(Request::StartProject {
            name: "shared".into(),
        })
        .await
        .unwrap();
    let error = daemon
        .call(start(vec!["not-an-email".into()]))
        .await
        .unwrap_err();
    assert!(matches!(&error, IpcError::App(e) if e.message.contains("not an e-mail")));

    let Response::Tunnel { tunnel } = daemon
        .call(start(vec!["Ana@Example.com, *@corp.io".into()]))
        .await
        .unwrap()
    else {
        panic!()
    };
    // The address comes from the program's metrics server once it's up;
    // on a busy machine that can take longer than the start call waits.
    let mut tunnel = tunnel;
    for _ in 0..120 {
        if tunnel.state == locdev_core::tunnel::TunnelState::Online {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        let Response::Tunnels { tunnels } = daemon.call(Request::TunnelList).await.unwrap() else {
            panic!()
        };
        tunnel = tunnels.into_iter().next().unwrap();
    }
    assert_eq!(
        tunnel.state,
        locdev_core::tunnel::TunnelState::Online,
        "{tunnel:?}"
    );
    assert_eq!(
        tunnel.url.as_deref(),
        Some("https://calm-fox-river.trycloudflare.com")
    );
    assert_eq!(tunnel.allowed, ["ana@example.com", "*@corp.io"]);
    assert_eq!(tunnel.target, format!("http://127.0.0.1:{port}"));

    // It points at the site's own server with the site's Host, protected.
    let Response::Logs { lines } = daemon
        .call(Request::TunnelLogs {
            site: "shared".into(),
            after: None,
            limit: None,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    let log: String = lines
        .iter()
        .map(|l| l.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        log.contains(&format!("--url http://127.0.0.1:{port}")),
        "{log}"
    );
    assert!(log.contains("--http-host-header shared.test"));
    assert!(log.contains("--allowed-mail ana@example.com --allowed-mail *@corp.io"));
    assert!(log.contains("--no-autoupdate"));

    let Response::Tunnels { tunnels } = daemon
        .call(Request::TunnelStop {
            site: "shared".into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(tunnels.is_empty());

    // Removing a shared site ends its tunnel too.
    daemon.call(start(Vec::new())).await.unwrap();
    daemon
        .call(Request::RemoveProject {
            name: "shared".into(),
        })
        .await
        .unwrap();
    let Response::Tunnels { tunnels } = daemon.call(Request::TunnelList).await.unwrap() else {
        panic!()
    };
    assert!(tunnels.is_empty());
}

/// A fake platform API on loopback: canned answers by "METHOD /path", and
/// every request it got (method, path, auth, body).
/// Requests a fake API got: method, path, `Authorization`, body.
type Seen = std::sync::Arc<std::sync::Mutex<Vec<(String, String, String, String)>>>;

fn fake_api(routes: Vec<(&'static str, &'static str)>) -> (String, Seen) {
    use std::io::{BufRead, BufReader, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = seen.clone();
    let address = base.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut parts = line.split_whitespace();
            let method = parts.next().unwrap_or("").to_string();
            let path = parts.next().unwrap_or("").to_string();
            let (mut auth, mut length) = (String::new(), 0usize);
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let header = header.trim_end().to_string();
                if header.is_empty() {
                    break;
                }
                let (k, v) = header.split_once(':').unwrap_or((&header, ""));
                match k.to_ascii_lowercase().as_str() {
                    "authorization" => auth = v.trim().to_string(),
                    "content-length" => length = v.trim().parse().unwrap_or(0),
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let key = format!("{method} {}", path.split('?').next().unwrap());
            log.lock().unwrap().push((
                method,
                path,
                auth,
                String::from_utf8_lossy(&body).into_owned(),
            ));
            let reply = routes.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
            // `{BASE}` in a reply is this server's own address.
            let (status, reply) = match reply {
                Some(r) => (200, r.replace("{BASE}", &address)),
                None => (404, "{}".to_string()),
            };
            let mut stream = stream;
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    (base, seen)
}

#[tokio::test]
async fn deploys_only_what_is_pushed() {
    let (base, seen) = fake_api(vec![
        (
            "GET /v2/user",
            r#"{"user":{"username":"ana","email":"ana@acme.com"}}"#,
        ),
        (
            "GET /v10/projects",
            r#"{"projects":[{"id":"prj_1","name":"shop","link":{"org":"acme","repo":"shop","productionBranch":"main"}}]}"#,
        ),
        (
            "GET /v9/projects/prj_1",
            r#"{"id":"prj_1","name":"shop","link":{"org":"acme","repo":"shop","productionBranch":"main"},"targets":{"production":{"id":"dpl_1","alias":["shop.acme.com"]}}}"#,
        ),
        (
            "GET /v7/deployments",
            r#"{"deployments":[{"uid":"dpl_1","name":"shop","url":"shop-1.vercel.app","readyState":"READY","created":1793700000000,"target":"production","meta":{"githubCommitRef":"main"}}]}"#,
        ),
        (
            "GET /v9/projects/prj_1/domains",
            r#"{"domains":[{"name":"shop.acme.com"}]}"#,
        ),
        (
            "GET /v10/projects/prj_1/env",
            r#"{"envs":[{"key":"DATABASE_URL","target":["production"]}]}"#,
        ),
        (
            "POST /v13/deployments",
            r#"{"id":"dpl_2","name":"shop","url":"shop-2.vercel.app","readyState":"QUEUED"}"#,
        ),
        ("POST /v1/projects/prj_1/rollback/dpl_1", "{}"),
        (
            "GET /v3/deployments/dpl_2/events",
            r#"[{"payload":{"text":"Cloning github.com/acme/shop"}}]"#,
        ),
    ]);
    let config = format!("port = 0\n[deploy]\ntoken_store = \"file\"\nvercel_api = \"{base}\"\n");
    let daemon = Daemon::start_with(tempfile::tempdir().unwrap(), &config).await;

    // A site whose repository is on GitHub, with main pushed.
    let site = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(site.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(site.path().join("index.html"), "<h1>shop</h1>").unwrap();
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=T",
        "-c",
        "user.email=t@e.st",
        "commit",
        "-q",
        "-m",
        "first",
    ]);
    git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/acme/shop.git",
    ]);
    git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&["branch", "-q", "--set-upstream-to=origin/main"]);
    let head = git(&["rev-parse", "HEAD"]);
    daemon
        .call(Request::AddProject {
            path: site.path().to_path_buf(),
            name: Some("shop".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();

    // A token is checked before it's kept, and never written next to the account.
    let Response::DeployPlatforms { platforms } = daemon
        .call(Request::DeployConnect {
            platform: "vercel".into(),
            token: "tok_secret".into(),
            team: Some("acme".into()),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    let vercel = platforms.iter().find(|p| p.id == "vercel").unwrap();
    assert_eq!(vercel.account.as_ref().unwrap().user, "ana");
    let accounts =
        std::fs::read_to_string(daemon.paths.config_dir.join("deploy-accounts.toml")).unwrap();
    assert!(!accounts.contains("tok_secret"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(daemon.paths.config_dir.join("deploy-tokens.toml"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    let Response::RemoteProjects { projects } = daemon
        .call(Request::DeployRemoteProjects {
            platform: "vercel".into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(projects[0].repo.as_deref(), Some("acme/shop"));
    daemon
        .call(Request::DeployLinkSite {
            site: "shop".into(),
            platform: Some("vercel".into()),
            project: Some("prj_1".into()),
        })
        .await
        .unwrap();
    let Response::RemoteStatus(status) = daemon
        .call(Request::DeployRemoteStatus {
            site: "shop".into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(status.git.problems.is_empty(), "{:?}", status.git.problems);
    assert_eq!(status.git.commit.as_deref(), Some(head.as_str()));
    assert!(status.deployments[0].current);
    assert_eq!(status.domains, ["shop.acme.com"]);
    assert_eq!(status.env[0].key, "DATABASE_URL");

    // Uncommitted work blocks a deploy: the platform would build something else.
    std::fs::write(site.path().join("index.html"), "<h1>changed</h1>").unwrap();
    let blocked = daemon
        .call(Request::DeployRun {
            site: "shop".into(),
            production: true,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(&blocked, IpcError::App(e) if e.code == ErrorCode::Conflict && e.message.contains("uncommitted")),
        "{blocked:?}"
    );
    git(&["checkout", "--", "index.html"]);

    let Response::RemoteDeployment { deployment } = daemon
        .call(Request::DeployRun {
            site: "shop".into(),
            production: true,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(deployment.id, "dpl_2");
    daemon
        .call(Request::DeployRollback {
            site: "shop".into(),
            deployment: "dpl_1".into(),
        })
        .await
        .unwrap();
    let Response::RemoteLog(log) = daemon
        .call(Request::DeployLogs {
            site: "shop".into(),
            deployment: "dpl_2".into(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(log.lines, ["Cloning github.com/acme/shop"]);

    // A copy, so no lock is held across the awaits below.
    let seen = seen.lock().unwrap().clone();
    assert!(
        seen.iter()
            .all(|(_, _, auth, _)| auth == "Bearer tok_secret")
    );
    let (_, path, _, body) = seen
        .iter()
        .find(|(m, p, _, _)| m == "POST" && p.starts_with("/v13/deployments"))
        .unwrap();
    assert!(path.contains("slug=acme"));
    let body: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(
        body["gitSource"]["sha"],
        head.as_str(),
        "the pushed HEAD is what's built"
    );
    assert_eq!(body["gitSource"]["org"], "acme");
    assert_eq!(body["target"], "production");

    daemon
        .call(Request::DeployDisconnect {
            platform: "vercel".into(),
        })
        .await
        .unwrap();
    let tokens =
        std::fs::read_to_string(daemon.paths.config_dir.join("deploy-tokens.toml")).unwrap();
    assert!(!tokens.contains("tok_secret"));
    let error = daemon
        .call(Request::DeployRemoteStatus {
            site: "shop".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(&error, IpcError::App(e) if e.code == ErrorCode::Unauthorized));
}

/// The update check reads LocDev's releases anonymously, skips drafts,
/// takes the versions from the newest stable one and notifies once.
#[tokio::test]
async fn update_check_reads_published_releases() {
    let (base, seen) = fake_api(vec![
        (
            "GET /repos/acme/locdev/releases",
            r#"[
              {"tag_name":"v99.1.0","draft":true,"html_url":"https://github.com/acme/locdev/releases/tag/v99.1.0","assets":[]},
              {"tag_name":"v99.0.0-rc.1","prerelease":true,"html_url":"https://github.com/acme/locdev/releases/tag/v99.0.0-rc.1","assets":[]},
              {"tag_name":"v98.0.0","published_at":"2026-11-04T10:00:00Z","html_url":"https://github.com/acme/locdev/releases/tag/v98.0.0","body":"Better things",
               "assets":[{"name":"versions.json","browser_download_url":"{BASE}/dl/v98.0.0/versions.json"},
                         {"name":"latest.json","browser_download_url":"{BASE}/dl/v98.0.0/latest.json"}]}
            ]"#,
        ),
        (
            "GET /dl/v98.0.0/versions.json",
            r#"{"app":"98.0.0","daemon":"97.0.0"}"#,
        ),
    ]);
    let config = format!("port = 0\n[updates]\nrepo = \"acme/locdev\"\napi = \"{base}\"\n");
    let daemon = Daemon::start_with(tempfile::tempdir().unwrap(), &config).await;

    let Response::Updates(status) = daemon
        .call(Request::UpdateCheck { refresh: true })
        .await
        .unwrap()
    else {
        panic!("expected updates");
    };
    assert_eq!(status.repo, "acme/locdev");
    assert_eq!(status.daemon, env!("CARGO_PKG_VERSION"));
    assert_eq!(status.error, None);
    let latest = status.latest.expect("a stable release");
    assert_eq!(
        latest.tag, "v98.0.0",
        "drafts and prereleases are not the latest"
    );
    assert_eq!(latest.daemon.as_deref(), Some("97.0.0"));
    assert!(latest.updater);
    assert!(status.daemon_update);
    assert_eq!(status.releases.len(), 2, "the draft is left out");
    assert!(status.releases[0].prerelease);

    // Cached: a second check without refresh doesn't ask again.
    let asked = seen.lock().unwrap().len();
    daemon
        .call(Request::UpdateCheck { refresh: false })
        .await
        .unwrap();
    assert_eq!(seen.lock().unwrap().len(), asked);
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .all(|(_, _, auth, _)| auth.is_empty()),
        "no token is ever sent"
    );

    // A bad repository name is refused before any request.
    let config = "port = 0\n[updates]\nrepo = \"../x\"\n";
    let other = Daemon::start_with(tempfile::tempdir().unwrap(), config).await;
    let error = other
        .call(Request::UpdateCheck { refresh: true })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("owner/name"), "{error}");
}

/// Editors are found on PATH and started with their documented arguments,
/// only on the site's folder or a file inside it.
#[cfg(unix)]
#[tokio::test]
async fn opens_sites_and_files_in_the_editor() {
    use std::os::unix::fs::PermissionsExt;

    let bin = tempfile::tempdir().unwrap();
    let record = bin.path().join("args.txt");
    let code = bin.path().join("code");
    std::fs::write(
        &code,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\n",
            record.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&code, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:/usr/bin:/bin", bin.path().display());
    let daemon = Daemon::start_full(tempfile::tempdir().unwrap(), "port = 0", Some(path)).await;

    let site = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(site.path().join("src")).unwrap();
    std::fs::write(site.path().join("index.php"), "<?php").unwrap();
    std::fs::write(site.path().join("src/app.php"), "<?php").unwrap();
    daemon
        .call(Request::AddProject {
            path: site.path().to_path_buf(),
            name: Some("shop".into()),
            command: None,
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();

    let Response::Editors(view) = daemon
        .call(Request::EditorList { refresh: true })
        .await
        .unwrap()
    else {
        panic!("expected editors");
    };
    let vscode = view
        .editors
        .iter()
        .find(|e| e.id == "vscode")
        .expect("code on PATH");
    assert_eq!(vscode.program, code);

    let open = |path: Option<&str>, line: Option<u32>| Request::EditorOpen {
        site: "shop".into(),
        path: path.map(str::to_string),
        line,
        column: None,
        editor: Some("vscode".into()),
    };
    daemon.call(open(None, None)).await.unwrap();
    // The editor is started without waiting: let the first one write before
    // the second starts, or the two lines can land in either order.
    for _ in 0..50 {
        if std::fs::read_to_string(&record).is_ok_and(|r| r.ends_with('\n')) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    daemon
        .call(open(Some("src/app.php"), Some(7)))
        .await
        .unwrap();
    let root = site.path().canonicalize().unwrap();
    // The site is registered by its canonical path (on macOS the temporary
    // folder is behind the /var -> /private/var link).
    let want = format!(
        "{}\n--goto\n{}:7\n",
        root.display(),
        root.join("src/app.php").display()
    );
    let mut got = String::new();
    for _ in 0..50 {
        got = std::fs::read_to_string(&record).unwrap_or_default();
        if got.len() >= want.len() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(got, want);

    // Nothing outside the site, and no unknown editor.
    let error = daemon
        .call(open(Some("../etc/passwd"), None))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("isn't a file inside"), "{error}");
    let mut other = open(None, None);
    if let Request::EditorOpen { editor, .. } = &mut other {
        *editor = Some("notepad".into());
    }
    assert!(daemon.call(other).await.is_err());
}

/// The Debug Center end to end: a real (Python) site answers 500 and prints
/// a traceback holding a password from its .env; a real VarDumper payload
/// reaches the dump server. Events are tied to the site, secrets hidden,
/// repeats grouped, and clearing works.
#[tokio::test]
async fn debug_center_collects_errors_and_dumps() {
    use locdev_core::observe::{EventFilter, EventKind};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Windows' `python3` can be the Store's stub: it starts but fails.
    if !std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        eprintln!("python3 not found; skipping");
        return;
    }
    let site = tempfile::tempdir().unwrap();
    std::fs::write(
        site.path().join(".env"),
        "DB_HOST=127.0.0.1\nDB_PASSWORD=pa55-w0rd-xyz\n",
    )
    .unwrap();
    std::fs::write(
        site.path().join("server.py"),
        r#"import sys, traceback
from http.server import BaseHTTPRequestHandler, HTTPServer

class H(BaseHTTPRequestHandler):
    def do_GET(self):
        try:
            raise ValueError("could not connect with pa55-w0rd-xyz")
        except ValueError:
            traceback.print_exc()
            sys.stderr.flush()
        self.send_response(500)
        self.end_headers()
        self.wfile.write(b"boom")
    def log_message(self, *args):
        pass

HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
"#,
    )
    .unwrap();
    let daemon = Daemon::start().await;
    daemon
        .call(Request::AddProject {
            path: site.path().to_path_buf(),
            name: Some("api".into()),
            command: Some("python3 server.py {port}".into()),
            port: None,
            auto_start: false,
        })
        .await
        .unwrap();
    daemon
        .call(Request::StartProject { name: "api".into() })
        .await
        .unwrap();
    let Response::Status(status) = daemon.call(Request::Status).await.unwrap() else {
        panic!()
    };
    let proxy = status.proxy.port.unwrap();
    let get = || async move {
        for _ in 0..50 {
            if let Ok(mut s) = tokio::net::TcpStream::connect(("127.0.0.1", proxy)).await {
                s.write_all(
                    b"GET /orders?id=7 HTTP/1.1\r\nhost: api.test\r\nconnection: close\r\n\r\n",
                )
                .await
                .unwrap();
                let mut out = String::new();
                s.read_to_string(&mut out).await.unwrap();
                if out.split("\r\n").next().is_some_and(|l| l.contains(" 500")) {
                    return out;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("the site never answered 500");
    };
    get().await;
    get().await;

    // The traceback is read once the output goes quiet.
    let events = |filter: EventFilter| {
        let daemon = &daemon;
        async move {
            let Response::ObserveEvents { events, .. } = daemon
                .call(Request::ObserveEvents {
                    filter,
                    limit: None,
                })
                .await
                .unwrap()
            else {
                panic!()
            };
            events
        }
    };
    let site_filter = EventFilter {
        site: Some("api".into()),
        ..Default::default()
    };
    let mut found = Vec::new();
    for _ in 0..60 {
        found = events(site_filter.clone()).await;
        let exceptions = found
            .iter()
            .filter(|e| e.kind == EventKind::Exception)
            .count();
        let http = found.iter().filter(|e| e.kind == EventKind::Http).count();
        if exceptions >= 2 && http >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let exception = found
        .iter()
        .find(|e| e.kind == EventKind::Exception)
        .unwrap_or_else(|| panic!("no exception in {found:#?}"));
    assert_eq!(exception.code.as_deref(), Some("ValueError"));
    assert_eq!(exception.runtime.as_deref(), Some("python"));
    assert!(exception.file.as_deref().unwrap().ends_with("server.py"));
    assert!(
        !exception.message.contains("pa55-w0rd-xyz"),
        "{}",
        exception.message
    );
    assert!(!exception.raw.contains("pa55-w0rd-xyz"));
    assert!(exception.message.contains("[redacted]"));
    let http = found.iter().find(|e| e.kind == EventKind::Http).unwrap();
    let request = http.request.as_ref().unwrap();
    assert_eq!(request.status, Some(500));
    assert_eq!(request.url.as_deref(), Some("api.test/orders?id=7"));

    let Response::ObserveGroups { groups } = daemon
        .call(Request::ObserveGroups {
            filter: site_filter.clone(),
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    let group = groups
        .iter()
        .find(|g| g.kind == EventKind::Exception)
        .unwrap();
    assert!(
        group.count >= 2,
        "the same traceback twice is one problem: {groups:#?}"
    );

    // A dump, as VarDumper's ServerDumper sends it.
    let Response::ObserveStatus(status) = daemon.call(Request::ObserveStatus).await.unwrap() else {
        panic!()
    };
    let addr = status.dump_server.expect("dump server listening");
    let payload = include_str!("../../locdev-observe/tests/fixtures/vardumper-7.4.txt")
        .lines()
        .next()
        .unwrap();
    let mut s = tokio::net::TcpStream::connect(&addr).await.unwrap();
    s.write_all(format!("{payload}\n").as_bytes())
        .await
        .unwrap();
    drop(s);
    let dumps = EventFilter {
        kinds: vec![EventKind::Dump],
        ..Default::default()
    };
    let mut got = Vec::new();
    for _ in 0..30 {
        got = events(dumps.clone()).await;
        if !got.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].message, "array:5");
    assert!(got[0].dump.is_some());

    // Dumps for a site are a per-site switch (kept in projects.toml).
    let Response::ObserveDumps {
        enabled, restarted, ..
    } = daemon
        .call(Request::ObserveDumps {
            site: "api".into(),
            enabled: true,
        })
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(enabled && restarted);
    let saved = std::fs::read_to_string(daemon.paths.registry_file()).unwrap();
    assert!(saved.contains("dumps = true"));

    daemon
        .call(Request::ObserveClear {
            site: Some("api".into()),
        })
        .await
        .unwrap();
    assert!(events(site_filter).await.is_empty());
    daemon
        .call(Request::StopProject { name: "api".into() })
        .await
        .unwrap();
}

/// The default versions reach the terminal: a shim per default in the
/// daemon's `bin` folder, and that folder in the shell's startup file only
/// while it's turned on. Nothing outside LocDev's block is touched.
#[cfg(unix)]
#[tokio::test]
async fn default_versions_reach_the_terminal() {
    use std::os::unix::fs::PermissionsExt;

    use locdev_core::model::RuntimeKind;

    let user = tempfile::tempdir().unwrap();
    let tools = user.path().join("tools");
    std::fs::create_dir_all(&tools).unwrap();
    let php = tools.join("php");
    std::fs::write(
        &php,
        // Detection runs `php -n -r 'echo PHP_VERSION;'`.
        "#!/bin/sh\nif [ \"$1\" = -n ]; then echo 8.3.99; else echo \"fake $*\"; fi\n",
    )
    .unwrap();
    std::fs::set_permissions(&php, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(user.path().join(".zshrc"), "alias ll='ls -l'\n").unwrap();

    let daemon = Daemon::start_env(
        tempfile::tempdir().unwrap(),
        "port = 0",
        &[
            ("PATH", format!("{}:/usr/bin:/bin", tools.display())),
            ("HOME", user.path().display().to_string()),
            ("SHELL", "/bin/zsh".into()),
        ],
    )
    .await;
    let terminal = |enable| Request::TerminalRuntimes { enable };

    let Response::TerminalRuntimes(state) = daemon.call(terminal(None)).await.unwrap() else {
        panic!()
    };
    assert!(!state.enabled && state.commands.is_empty(), "{state:?}");

    daemon
        .call(Request::SetDefaultRuntime {
            kind: RuntimeKind::Php,
            version: Some("8.3".into()),
        })
        .await
        .unwrap();
    let Response::TerminalRuntimes(state) = daemon.call(terminal(Some(true))).await.unwrap() else {
        panic!()
    };
    assert!(state.enabled, "{state:?}");
    assert_eq!(state.commands, ["php"]);
    assert_eq!(state.files, [user.path().join(".zshrc")]);
    let rc = std::fs::read_to_string(user.path().join(".zshrc")).unwrap();
    assert!(
        rc.starts_with("alias ll='ls -l'\n# >>> Managed by LocDev"),
        "{rc}"
    );
    assert!(
        rc.contains(&format!("export PATH='{}'", state.dir.display())),
        "{rc}"
    );

    // The shim runs the default version with the arguments it got.
    let out = Command::new(state.dir.join("php"))
        .args(["-r", "1"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "fake -r 1");

    // A re-scan doesn't list the shim as another PHP.
    let Response::Runtimes { runtimes } = daemon
        .call(Request::ListRuntimes { refresh: true })
        .await
        .unwrap()
    else {
        panic!()
    };
    let php_versions = &runtimes
        .iter()
        .find(|r| r.kind == RuntimeKind::Php)
        .unwrap()
        .versions;
    assert!(
        php_versions
            .iter()
            .all(|v| !v.executable.starts_with(&state.dir)),
        "{php_versions:?}"
    );

    // No default: no shim, so the terminal finds the system's own.
    daemon
        .call(Request::SetDefaultRuntime {
            kind: RuntimeKind::Php,
            version: None,
        })
        .await
        .unwrap();
    assert!(!state.dir.join("php").exists());

    let Response::TerminalRuntimes(state) = daemon.call(terminal(Some(false))).await.unwrap()
    else {
        panic!()
    };
    assert!(!state.enabled);
    assert_eq!(
        std::fs::read_to_string(user.path().join(".zshrc")).unwrap(),
        "alias ll='ls -l'\n"
    );
}
