//! The Mailbox: LocDev's capture-only SMTP server on loopback, the messages
//! it keeps (data/mail), and how each site is pointed at it. The wiring goes
//! only into the site's process environment; a real SMTP or API transport is
//! replaced only with the user's explicit `override_real`.

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex, OnceLock};

use locdev_core::deps::{NotificationCategory, NotificationLink, Severity};
use locdev_core::mail::{MailEnvelope, MailSetup, MailSummary, MailboxStatus};
use locdev_core::model::{MailSettings, Project};
use locdev_core::protocol::{Request, Response};
use locdev_core::{AppError, ErrorCode};
use locdev_mail::smtp::Sink;
use locdev_mail::store::{Filter, Store};
use tracing::{info, warn};

use super::Daemon;
use crate::config::MailConfig;
use crate::notify::New;

#[derive(Default)]
pub struct MailState {
    store: OnceLock<Arc<Store>>,
    status: Mutex<MailboxStatus>,
}

struct Deliver {
    store: Arc<Store>,
    /// Each captured message, for its notification.
    captured: tokio::sync::mpsc::UnboundedSender<MailSummary>,
}

impl Sink for Deliver {
    fn deliver(&self, envelope: MailEnvelope, raw: Vec<u8>) {
        // The AUTH user is the site's name when LocDev wired the site.
        let site = envelope.auth_user.as_ref().map(|u| u.to_ascii_lowercase());
        match self.store.add(envelope, site, &raw) {
            Ok(summary) => {
                info!(id = %summary.id, site = ?summary.site, subject = %summary.subject, "mail captured");
                let _ = self.captured.send(summary);
            }
            Err(error) => warn!(%error, "could not store a captured message"),
        }
    }
}

/// Binds the SMTP server (the configured port or the next nine).
pub async fn start(
    config: &MailConfig,
    dir: &std::path::Path,
    state: &MailState,
    captured: tokio::sync::mpsc::UnboundedSender<MailSummary>,
) {
    let mut status = MailboxStatus {
        enabled: config.enabled,
        ..Default::default()
    };
    let store = match Store::open(dir) {
        Ok(store) => Arc::new(store),
        Err(error) => {
            status.error = Some(format!("{}: {error}", dir.display()));
            *state.status.lock().unwrap() = status;
            return;
        }
    };
    let _ = state.store.set(store.clone());
    if config.enabled {
        let ports: Vec<u16> = if config.port == 0 {
            vec![0]
        } else {
            (config.port..=config.port.saturating_add(9)).collect()
        };
        let mut last = None;
        for port in ports {
            match tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).await
            {
                Ok(listener) => {
                    let bound = listener.local_addr().map(|a| a.port()).unwrap_or(port);
                    info!(port = bound, "mail server listening");
                    status.port = Some(bound);
                    tokio::spawn(locdev_mail::smtp::serve(
                        listener,
                        Arc::new(Deliver {
                            store: store.clone(),
                            captured: captured.clone(),
                        }),
                    ));
                    break;
                }
                Err(error) => last = Some(format!("127.0.0.1:{port}: {error}")),
            }
        }
        if status.port.is_none() {
            status.error = last.or(Some("no port to listen on".into()));
        }
    }
    *state.status.lock().unwrap() = status;
}

fn error(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message.into())
}

impl Daemon {
    /// One notification per captured message (category Mail), shown by the
    /// OS when no LocDev window is in front. Opening or deleting the message
    /// marks it read.
    pub async fn mail_notifications(
        self: Arc<Self>,
        mut captured: tokio::sync::mpsc::UnboundedReceiver<MailSummary>,
    ) {
        while let Some(m) = captured.recv().await {
            let from = if m.from.is_empty() {
                "unknown sender"
            } else {
                m.from.as_str()
            };
            let subject = if m.subject.is_empty() {
                "(no subject)"
            } else {
                m.subject.as_str()
            };
            self.notify(New {
                category: NotificationCategory::Mail,
                severity: Severity::Info,
                title: match &m.site {
                    Some(site) => format!("New email from {site}"),
                    None => "New email in the Mailbox".into(),
                },
                body: format!("{from}: {subject}"),
                project: m.site.clone(),
                link: Some(NotificationLink {
                    view: "tools".into(),
                    sub: Some("mailbox".into()),
                    project: m.site.clone(),
                }),
                dedup: format!("mail:{}", m.id),
            });
        }
    }

    /// Mail notifications follow the messages: read or gone, read.
    fn sync_mail_notifications(&self) {
        let Some(store) = self.mail.store.get() else {
            return;
        };
        let unread = store.unread_ids();
        self.notifications
            .read_where(NotificationCategory::Mail, |dedup| {
                dedup
                    .strip_prefix("mail:")
                    .is_some_and(|id| unread.contains(id))
            });
    }

    fn mail_store(&self) -> Result<Arc<Store>, AppError> {
        self.mail
            .store
            .get()
            .cloned()
            .ok_or_else(|| AppError::new(ErrorCode::Internal, "the mailbox isn't available"))
    }

    pub(super) fn mailbox_status(&self) -> MailboxStatus {
        let mut status = self.mail.status.lock().unwrap().clone();
        if let Some(store) = self.mail.store.get() {
            (status.messages, status.unread) = store.counts();
        }
        status
    }

    /// The project's own variables, as its framework reads them (.env, then .env.local).
    fn project_env_values(project: &Project) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for file in [".env", ".env.local"] {
            if let Ok(bytes) = std::fs::read(project.path.join(file))
                && let Ok(doc) = locdev_env::dotenv::parse(&bytes)
            {
                out.extend(doc.entries.into_iter().map(|e| (e.key, e.value)));
            }
        }
        out
    }

    fn mail_plan(&self, project: &Project) -> Option<locdev_mail::wiring::Plan> {
        let port = self.mail.status.lock().unwrap().port?;
        let stack = self.stack_of(project);
        let components: Vec<String> = stack.components.iter().map(|c| c.id.clone()).collect();
        let env = Self::project_env_values(project);
        let sendmail = std::env::current_exe()
            .ok()
            // Single quotes for `sh -c` (PHP runs sendmail_path through it); the
            // site name is a slug, so it needs none.
            .map(|exe| {
                format!(
                    "'{}' sendmail --site {} --port {port}",
                    exe.display().to_string().replace('\'', "'\\''"),
                    project.name
                )
            });
        let ini_dir = self.paths.data_dir.join("mail").join("php");
        let plan = locdev_mail::wiring::plan(&locdev_mail::wiring::Input {
            site: &project.name,
            framework: &stack.framework,
            components: &components,
            root: &project.path,
            port,
            env: &env,
            sendmail,
            ini_dir: &ini_dir,
        });
        Some(plan)
    }

    /// What every process of the site gets when its Mailbox is on.
    pub(super) fn mail_env(&self, project: &Project) -> Vec<(String, String)> {
        if !project.mail.enabled {
            return Vec::new();
        }
        match self.mail_plan(project) {
            Some(plan) if !plan.setup.real_smtp || project.mail.override_real => plan.env,
            Some(_) => {
                warn!(site = %project.name, "mailbox on, but the project points at a real mail server; not overriding without consent");
                Vec::new()
            }
            None => Vec::new(),
        }
    }

    fn mail_setup(&self, project: &Project) -> Result<MailSetup, AppError> {
        let plan = self.mail_plan(project).ok_or_else(|| {
            let why = self
                .mail
                .status
                .lock()
                .unwrap()
                .error
                .clone()
                .unwrap_or_else(|| "it is off in config.toml".into());
            AppError::new(
                ErrorCode::Conflict,
                format!("LocDev's mail server isn't running: {why}"),
            )
        })?;
        let mut setup = plan.setup;
        setup.enabled = project.mail.enabled;
        if setup.real_smtp && project.mail.enabled && !project.mail.override_real {
            setup.notes.insert(0, "On, but not applied: the project points at a real mail server and replacing it wasn't allowed.".into());
        }
        Ok(setup)
    }

    pub(super) async fn mail_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::MailboxStatus => Ok(Response::MailboxStatus(self.mailbox_status())),
            Request::MailList {
                site,
                query,
                unread,
                attachments,
                limit,
            } => {
                let messages = self.mail_store()?.list(&Filter {
                    site,
                    query,
                    unread,
                    with_attachments: attachments,
                    limit,
                });
                Ok(Response::MailList { messages })
            }
            Request::MailGet { id } => {
                let store = self.mail_store()?;
                let mut detail = store
                    .get(&id)
                    .ok_or_else(|| AppError::not_found(format!("no message {id}")))?;
                self.sync_mail_notifications();
                if let Some(site) = detail.summary.site.clone()
                    && let Ok(project) = self.project(&site).await
                {
                    detail.framework = Some(self.stack_of(&project).framework);
                }
                Ok(Response::MailDetail(Box::new(detail)))
            }
            Request::MailRaw { id } => {
                let raw = self
                    .mail_store()?
                    .raw(&id)
                    .ok_or_else(|| AppError::not_found(format!("no message {id}")))?;
                Ok(Response::MailRaw {
                    base64: locdev_mail::parse::base64(&raw),
                })
            }
            Request::MailAttachment { id, index } => {
                let raw = self
                    .mail_store()?
                    .raw(&id)
                    .ok_or_else(|| AppError::not_found(format!("no message {id}")))?;
                let (name, content_type, bytes) = locdev_mail::parse::attachment(&raw, index)
                    .ok_or_else(|| {
                        AppError::not_found(format!("message {id} has no attachment {index}"))
                    })?;
                Ok(Response::MailAttachment {
                    name,
                    content_type,
                    base64: locdev_mail::parse::base64(&bytes),
                })
            }
            Request::MailDelete { ids } => {
                let count = self.mail_store()?.delete(&ids);
                self.sync_mail_notifications();
                Ok(Response::MailDeleted { count })
            }
            Request::MailClear { site } => {
                let count = self.mail_store()?.clear(site.as_deref());
                self.sync_mail_notifications();
                Ok(Response::MailDeleted { count })
            }
            Request::MailSetup { name } => {
                let project = self.project(&name).await?;
                Ok(Response::MailSetup(self.mail_setup(&project)?))
            }
            Request::MailEnable {
                name,
                enabled,
                override_real,
            } => {
                let project = self.project(&name).await?;
                if enabled {
                    let setup = self.mail_setup(&project)?;
                    if setup.real_smtp && !override_real {
                        return Err(AppError::new(
                            ErrorCode::Conflict,
                            format!(
                                "{name} sends through a real mail server ({}); allow replacing it while the Mailbox is on to continue",
                                setup.current.join(", ")
                            ),
                        ));
                    }
                }
                let project = {
                    let mut registry = self.registry.lock().await;
                    let p = registry.get_mut(&name)?;
                    p.mail = MailSettings {
                        enabled,
                        override_real: enabled && override_real,
                    };
                    let p = p.clone();
                    self.commit(&registry)?;
                    p
                };
                info!(site = %name, enabled, override_real, "mailbox changed");
                Ok(Response::MailSetup(self.mail_setup(&project)?))
            }
            Request::MailSendTest { name, to } => {
                let port = self
                    .mail
                    .status
                    .lock()
                    .unwrap()
                    .port
                    .ok_or_else(|| error("LocDev's mail server isn't running"))?;
                let site = match &name {
                    Some(n) => Some(self.project(n).await?.name),
                    None => None,
                };
                let to = to.unwrap_or_else(|| "you@example.test".into());
                let from = format!("{}@locdev.test", site.as_deref().unwrap_or("locdev"));
                let raw = locdev_mail::client::test_message(&from, &to, site.as_deref());
                locdev_mail::client::send(
                    SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
                    site.as_deref(),
                    &from,
                    &[to],
                    &raw,
                )
                .await
                .map_err(|e| AppError::new(ErrorCode::Internal, e))?;
                Ok(Response::Ok)
            }
            _ => unreachable!("not a mail request"),
        }
    }
}
