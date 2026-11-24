//! Laravel Tinker for a site: snippets run with `php artisan tinker
//! --execute` (limited time and output), or the interactive REPL in a
//! terminal. Both use the site's own PHP and environment, like its server.
//! Code runs only when the user asks; it's their app.

use std::process::Stdio;
use std::time::{Duration, Instant};

use locdev_core::model::{ProcessRole, Project, RuntimeKind, TinkerOutput};
use locdev_core::scaffold::{Step, TerminalInfo};
use locdev_core::{AppError, ErrorCode};
use tokio::io::AsyncReadExt;

use super::Daemon;

const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_OUTPUT: usize = 256 * 1024;
const MAX_CODE: usize = 64 * 1024;

/// Tinker ships with Laravel (`laravel/tinker` in composer.json).
fn has_tinker(project: &Project) -> Result<(), AppError> {
    if !project.path.join("artisan").is_file() {
        return Err(AppError::new(
            ErrorCode::NotRunnable,
            format!("{} is not a Laravel app (no artisan)", project.name),
        ));
    }
    let composer = std::fs::read_to_string(project.path.join("composer.json")).unwrap_or_default();
    if !composer.contains("\"laravel/tinker\"") {
        return Err(AppError::new(
            ErrorCode::NotRunnable,
            "laravel/tinker isn't installed; add it with `composer require laravel/tinker`",
        ));
    }
    if !project.path.join("vendor").is_dir() {
        return Err(AppError::new(
            ErrorCode::NotRunnable,
            "the dependencies aren't installed; run `composer install` first",
        ));
    }
    Ok(())
}

impl Daemon {
    /// The site's PHP and the environment its server gets.
    async fn tinker_env(
        &self,
        project: &Project,
    ) -> Result<(std::path::PathBuf, Vec<(String, String)>), AppError> {
        self.runtimes.wait_detected().await;
        let php = self
            .runtimes
            .for_project(project)
            .into_iter()
            .find(|r| r.kind == RuntimeKind::Php)
            .and_then(|r| r.version)
            .or_else(|| self.runtimes.on_path(RuntimeKind::Php))
            .map_or_else(|| "php".into(), |v| v.executable);
        let (mut env, _) = self.shared_env(project, ProcessRole::Main);
        env.extend(self.runtimes.env_for(project)?);
        self.adapter_env(project, project.port, &mut env);
        Ok((php, env))
    }

    pub(super) async fn tinker_run(
        &self,
        project: &Project,
        code: &str,
    ) -> Result<TinkerOutput, AppError> {
        has_tinker(project)?;
        let code = code.trim();
        if code.is_empty() {
            return Err(AppError::invalid("no code to run"));
        }
        if code.len() > MAX_CODE {
            return Err(AppError::invalid("the code is too long"));
        }
        let (php, env) = self.tinker_env(project).await?;
        let started = Instant::now();
        let mut child = tokio::process::Command::from(locdev_core::process::command(&php))
            // One argument, no shell: the code is never parsed by one.
            .args(["artisan", "tinker", &format!("--execute={code}")])
            .current_dir(&project.path)
            .envs(env)
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| AppError::internal(format!("could not run {}: {e}", php.display())))?;
        let mut stdout = child.stdout.take().expect("piped");
        let mut stderr = child.stderr.take().expect("piped");
        let read = async {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let limit = (MAX_OUTPUT + 1) as u64;
            let (mut o, mut e) = ((&mut stdout).take(limit), (&mut stderr).take(limit));
            let _ = tokio::join!(o.read_to_end(&mut out), e.read_to_end(&mut err));
            out.extend(err);
            let status = child.wait().await.ok();
            (out, status)
        };
        let (output, code, timed_out) = match tokio::time::timeout(TIMEOUT, read).await {
            Ok((out, status)) => (out, status.and_then(|s| s.code()), false),
            // Dropping the future drops the child: kill_on_drop.
            Err(_) => (Vec::new(), None, true),
        };
        let truncated = output.len() > MAX_OUTPUT;
        let text = String::from_utf8_lossy(&output[..output.len().min(MAX_OUTPUT)]).into_owned();
        Ok(TinkerOutput {
            output: text,
            code,
            duration_ms: started.elapsed().as_millis() as u64,
            timed_out,
            truncated,
        })
    }

    pub(super) async fn tinker_open(&self, project: &Project) -> Result<TerminalInfo, AppError> {
        has_tinker(project)?;
        let (php, env) = self.tinker_env(project).await?;
        let step = Step {
            label: "Tinker".into(),
            program: php.to_string_lossy().into_owned(),
            args: vec!["artisan".into(), "tinker".into()],
            cwd: project.path.clone(),
            env: Vec::new(),
        };
        let me = self.me()?;
        let session = self
            .terminals
            .open(format!("Tinker: {}", project.name), 1)?;
        let info = session.info();
        tokio::spawn(async move {
            let code = me.run_steps(&session, &[step], 0, &env).await;
            session.finish(code);
        });
        Ok(info)
    }
}
