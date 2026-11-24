//! Email Studio. Files are read and written by the daemon through
//! `locdev-email` (blocks between comment markers, code kept as written).
//! Project code runs only when the user asks for it (the framework preview,
//! `php artisan make:mail` from `locdev new email`), as a separate process
//! with a reduced environment, a time limit and bounded output; mail it
//! might send goes to the Mailbox.

use std::path::PathBuf;
use std::time::Duration;

use locdev_config::history::History;
use locdev_core::email::{EmailCreated, EmailFormat, EmailPreview, PreviewMode, Segment};
use locdev_core::model::Project;
use locdev_core::protocol::{Request, Response};
use locdev_core::{AppError, ErrorCode};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;

use super::Daemon;

const RUN_LIMIT: Duration = Duration::from_secs(25);
const OUTPUT_LIMIT: usize = 5 * 1024 * 1024;

fn error(message: String) -> AppError {
    let code = if message.contains("changed since") || message.contains("already exists") {
        ErrorCode::Conflict
    } else if message.contains("isn't an email template") {
        ErrorCode::NotFound
    } else {
        ErrorCode::InvalidInput
    };
    AppError::new(code, message)
}

/// `a/b.blade.php` → a file name for LocDev's own state folders.
fn slot(rel: &str) -> String {
    rel.replace(['/', '\\'], "%2F")
}

impl Daemon {
    fn email_state(&self, kind: &str, site: &str) -> PathBuf {
        self.paths
            .data_dir
            .join("state")
            .join(kind)
            .join(slot(site))
    }

    fn email_history(&self, site: &str, rel: &str) -> History {
        History::new(
            &self.paths.data_dir.join("state").join("email-history"),
            site,
            rel,
        )
    }

    fn sample_of(&self, site: &str, rel: &str, variables: &[String]) -> Value {
        let file = self
            .email_state("email-samples", site)
            .join(format!("{}.json", slot(rel)));
        std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_else(|| locdev_email::default_sample(variables))
    }

    /// The process environment for project code LocDev runs on request:
    /// the site's runtimes on PATH, and its mail pointed at the Mailbox.
    fn isolated_env(&self, project: &Project) -> Result<Vec<(String, String)>, AppError> {
        let mut env: Vec<(String, String)> = Vec::new();
        for key in ["HOME", "USERPROFILE", "SystemRoot", "TEMP", "TMP", "LANG"] {
            if let Ok(v) = std::env::var(key) {
                env.push((key.into(), v));
            }
        }
        env.extend(self.runtimes.env_for(project)?);
        if !env.iter().any(|(k, _)| k == "PATH")
            && let Ok(path) = std::env::var("PATH")
        {
            env.push(("PATH".into(), path));
        }
        env.push(("APP_ENV".into(), "local".into()));
        let mut forced = project.clone();
        forced.mail.enabled = true;
        forced.mail.override_real = true;
        env.extend(self.mail_env(&forced));
        Ok(env)
    }

    /// Runs `program args` in the project, stdin piped in, killed after the limit.
    async fn run_isolated(
        &self,
        project: &Project,
        program: &str,
        args: &[String],
        stdin: &[u8],
    ) -> Result<(bool, String, String), AppError> {
        let env = self.isolated_env(project)?;
        let mut command = tokio::process::Command::from(locdev_core::process::command(program));
        command
            .args(args)
            .current_dir(&project.path)
            .env_clear()
            .envs(env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn().map_err(|e| {
            AppError::new(
                ErrorCode::NotRunnable,
                format!("couldn't run {program}: {e}"),
            )
        })?;
        if let Some(mut input) = child.stdin.take() {
            let _ = input.write_all(stdin).await;
        }
        let mut stdout = child.stdout.take().expect("piped");
        let mut stderr = child.stderr.take().expect("piped");
        let run = async {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let mut out_limited = (&mut stdout).take(OUTPUT_LIMIT as u64);
            let mut err_limited = (&mut stderr).take(64 * 1024);
            let _ = tokio::join!(
                out_limited.read_to_end(&mut out),
                err_limited.read_to_end(&mut err)
            );
            let status = child.wait().await;
            (status, out, err)
        };
        match tokio::time::timeout(RUN_LIMIT, run).await {
            Ok((status, out, err)) => Ok((
                status.is_ok_and(|s| s.success()),
                String::from_utf8_lossy(&out).into_owned(),
                String::from_utf8_lossy(&err).into_owned(),
            )),
            Err(_) => Err(AppError::new(
                ErrorCode::Internal,
                format!(
                    "{program} took longer than {}s and was stopped",
                    RUN_LIMIT.as_secs()
                ),
            )),
        }
    }

    fn artisan_available(project: &Project) -> bool {
        project.path.join("artisan").is_file() && project.path.join("vendor/autoload.php").is_file()
    }

    async fn framework_render(
        &self,
        project: &Project,
        content: &str,
        format: EmailFormat,
        data: &Value,
    ) -> Result<String, AppError> {
        if !locdev_email::framework::laravel_ready(&project.path) {
            return Err(AppError::new(
                ErrorCode::NotRunnable,
                "the framework preview needs a Laravel project with vendor/ installed (`composer install`)",
            ));
        }
        let dir = self.email_state("email-preview", &project.name);
        std::fs::create_dir_all(&dir).map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
        let script = dir.join("render.php");
        std::fs::write(&script, locdev_email::framework::LARAVEL_RENDER)
            .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
        let view = format!(
            "locdev_preview_{}",
            locdev_email::hash(content.as_bytes())
                .get(..12)
                .unwrap_or("x")
        );
        let file = dir.join(format!("{view}.blade.php"));
        std::fs::write(&file, content).map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
        let mode = if format == EmailFormat::Markdown {
            "markdown"
        } else {
            "html"
        };
        let args = vec![
            script.display().to_string(),
            project.path.display().to_string(),
            dir.display().to_string(),
            view,
            mode.into(),
        ];
        let result = self
            .run_isolated(project, "php", &args, data.to_string().as_bytes())
            .await;
        let _ = std::fs::remove_file(&file);
        let (ok, out, err) = result?;
        if ok {
            Ok(out)
        } else {
            let detail = err
                .lines()
                .chain(out.lines())
                .filter(|l| !l.trim().is_empty())
                .take(8)
                .collect::<Vec<_>>()
                .join("\n");
            Err(AppError::new(
                ErrorCode::Internal,
                format!("Laravel couldn't render it:\n{detail}"),
            ))
        }
    }

    async fn email_preview(
        &self,
        project: &Project,
        rel: &str,
        segments: Option<Vec<Segment>>,
        theme: Option<Value>,
        data: &Value,
        mode: PreviewMode,
    ) -> Result<EmailPreview, AppError> {
        let root = project.path.clone();
        let path = rel.to_string();
        let (content, tagged, format) = {
            let (_, format) = locdev_email::resolve(&root, &path).map_err(error)?;
            match segments {
                Some(segments) => {
                    let theme = theme.unwrap_or_else(locdev_email::blocks::default_theme);
                    let (content, _) = locdev_email::build(&root, &path, &segments, &theme, false)
                        .map_err(error)?;
                    let (tagged, _) = locdev_email::build(&root, &path, &segments, &theme, true)
                        .map_err(error)?;
                    (content, tagged, format)
                }
                None => {
                    let content = std::fs::read_to_string(root.join(&path))
                        .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
                    (content.clone(), content, format)
                }
            }
        };
        match mode {
            PreviewMode::Static => {
                let r = locdev_email::preview::render(&tagged, format, data, &root);
                Ok(EmailPreview {
                    html: r.html,
                    text: Some(r.text),
                    content,
                    notes: r.notes,
                    error: None,
                })
            }
            PreviewMode::Framework => {
                match self.framework_render(project, &tagged, format, data).await {
                    Ok(html) => {
                        let (html, text) = if format == EmailFormat::Text {
                            (
                                format!(
                                    "<pre style=\"white-space:pre-wrap;font-family:ui-monospace,monospace;padding:16px\">{}</pre>",
                                    locdev_email::blocks::escape(&html)
                                ),
                                html.trim().to_string(),
                            )
                        } else {
                            let text = locdev_email::preview::text_of(&html);
                            (html, text)
                        };
                        Ok(EmailPreview {
                            html,
                            text: Some(text),
                            content,
                            notes: vec![
                                "Rendered by Laravel in a separate process with the sample data."
                                    .into(),
                            ],
                            error: None,
                        })
                    }
                    Err(e) => Ok(EmailPreview {
                        html: String::new(),
                        text: None,
                        content,
                        notes: Vec::new(),
                        error: Some(e.message),
                    }),
                }
            }
        }
    }

    async fn email_new(
        &self,
        project: &Project,
        kind: locdev_email::generate::Kind,
        item: &str,
        markdown: bool,
    ) -> Result<EmailCreated, AppError> {
        let root = project.path.clone();
        let artisan_ok = Self::artisan_available(project);
        let plan =
            locdev_email::generate::plan(&root, kind, item, markdown, artisan_ok).map_err(error)?;
        let mut files = Vec::new();
        let mut ran = None;
        let mut notes = plan.notes.clone();
        if let Some(args) = &plan.artisan {
            let mut full = vec!["artisan".to_string()];
            full.extend(args.iter().cloned());
            full.push("--no-interaction".into());
            let shown = format!("php {}", full.join(" "));
            let (ok, out, err) = self.run_isolated(project, "php", &full, b"").await?;
            if !ok {
                return Err(AppError::new(
                    ErrorCode::Internal,
                    format!(
                        "`{shown}` failed:\n{}",
                        err.trim()
                            .lines()
                            .chain(out.lines())
                            .take(6)
                            .collect::<Vec<_>>()
                            .join("\n")
                    ),
                ));
            }
            info!(site = %project.name, command = %shown, "ran the framework's generator");
            ran = Some(shown);
            if let Some(class) = args.get(1) {
                let class_rel = format!("app/Mail/{class}.php");
                if root.join(&class_rel).is_file() {
                    files.push(class_rel);
                }
            }
        }
        if let Some((rel, content)) = &plan.fill_after_artisan {
            let path = root.join(rel);
            // We checked it didn't exist; make:mail just wrote its stub there.
            if path.is_file() {
                std::fs::write(&path, content)
                    .map_err(|e| AppError::new(ErrorCode::Io, format!("{rel}: {e}")))?;
            } else {
                locdev_email::generate::create(&root, rel, content).map_err(error)?;
                notes.push("make:mail didn't write the view; LocDev created it.".into());
            }
            files.push(rel.clone());
        }
        for (rel, content) in &plan.files {
            locdev_email::generate::create(&root, rel, content).map_err(error)?;
            files.push(rel.clone());
        }
        info!(site = %project.name, ?files, "email files created");
        Ok(EmailCreated { files, ran, notes })
    }

    pub(super) async fn email_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::EmailCatalog { name } => {
                let project = self.project(&name).await?;
                let framework = self.stack_of(&project).framework;
                let artisan = Self::artisan_available(&project);
                let root = project.path.clone();
                let catalog = tokio::task::spawn_blocking(move || {
                    locdev_email::catalog(&root, &framework, artisan)
                })
                .await
                .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?;
                Ok(Response::EmailCatalog(catalog))
            }
            Request::EmailOpen { name, path } => {
                let project = self.project(&name).await?;
                let content = std::fs::read_to_string(project.path.join(&path)).unwrap_or_default();
                let sample =
                    self.sample_of(&name, &path, &locdev_email::discover::variables(&content));
                let doc = locdev_email::open(&project.path, &path, sample).map_err(error)?;
                Ok(Response::EmailDoc(Box::new(doc)))
            }
            Request::EmailStat { name, path } => {
                let project = self.project(&name).await?;
                let (file, _) = locdev_email::resolve(&project.path, &path).map_err(error)?;
                let bytes = std::fs::read(&file)
                    .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
                Ok(Response::EmailStat {
                    hash: locdev_email::hash(&bytes),
                })
            }
            Request::EmailPreview {
                name,
                path,
                segments,
                theme,
                data,
                mode,
            } => {
                let project = self.project(&name).await?;
                Ok(Response::EmailPreview(
                    self.email_preview(&project, &path, segments, theme, &data, mode)
                        .await?,
                ))
            }
            Request::EmailBuild {
                name,
                path,
                segments,
                theme,
            } => {
                let project = self.project(&name).await?;
                let (content, _) =
                    locdev_email::build(&project.path, &path, &segments, &theme, false)
                        .map_err(error)?;
                let current = std::fs::read_to_string(project.path.join(&path)).unwrap_or_default();
                Ok(Response::EmailBuilt {
                    diff: locdev_email::diff(&path, &current, &content),
                    content,
                })
            }
            Request::EmailSave {
                name,
                path,
                content,
                base_hash,
            } => {
                let project = self.project(&name).await?;
                let history = self.email_history(&name, &path);
                let hash = locdev_email::save(
                    &project.path,
                    &path,
                    &content,
                    &base_hash,
                    &history,
                    "email studio",
                )
                .map_err(error)?;
                info!(site = %name, file = %path, "email template saved");
                Ok(Response::EmailSaved { hash })
            }
            Request::EmailSample { name, path, data } => {
                let project = self.project(&name).await?;
                locdev_email::resolve(&project.path, &path).map_err(error)?;
                let dir = self.email_state("email-samples", &name);
                std::fs::create_dir_all(&dir)
                    .and_then(|_| {
                        std::fs::write(dir.join(format!("{}.json", slot(&path))), data.to_string())
                    })
                    .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
                Ok(Response::Ok)
            }
            Request::EmailNew {
                name,
                component,
                item,
                markdown,
            } => {
                let project = self.project(&name).await?;
                let kind = if component {
                    locdev_email::generate::Kind::Component
                } else {
                    locdev_email::generate::Kind::Template
                };
                Ok(Response::EmailCreated(
                    self.email_new(&project, kind, &item, markdown).await?,
                ))
            }
            Request::EmailSendTest {
                name,
                path,
                segments,
                theme,
                data,
                mode,
                to,
            } => {
                let project = self.project(&name).await?;
                let preview = self
                    .email_preview(&project, &path, segments, theme, &data, mode)
                    .await?;
                if let Some(e) = preview.error {
                    return Err(AppError::new(ErrorCode::Internal, e));
                }
                let port = self.mailbox_status().port.ok_or_else(|| {
                    AppError::new(ErrorCode::NotRunnable, "LocDev's mail server isn't running")
                })?;
                let to = to.unwrap_or_else(|| "you@example.test".into());
                let from = format!("{}@locdev.test", project.name);
                let raw = locdev_mail::client::html_message(
                    &from,
                    &to,
                    &format!("[Email Studio] {path}"),
                    &preview.html,
                    preview.text.as_deref().unwrap_or(""),
                );
                locdev_mail::client::send(
                    std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                    Some(&project.name),
                    &from,
                    &[to],
                    &raw,
                )
                .await
                .map_err(|e| AppError::new(ErrorCode::Internal, e))?;
                Ok(Response::Ok)
            }
            Request::EmailHistory { name, path } => {
                let project = self.project(&name).await?;
                locdev_email::resolve(&project.path, &path).map_err(error)?;
                Ok(Response::ConfigHistory {
                    entries: self.email_history(&name, &path).list(),
                })
            }
            Request::EmailHistoryVersion { name, path, id } => {
                let project = self.project(&name).await?;
                locdev_email::resolve(&project.path, &path).map_err(error)?;
                let content = self.email_history(&name, &path).load(&id).map_err(error)?;
                Ok(Response::ConfigHistoryVersion { content })
            }
            _ => unreachable!("not an email request"),
        }
    }
}
