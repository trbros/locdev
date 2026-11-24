//! `locdev` / `locdev tui`: a full-screen terminal UI over the same daemon
//! the app talks to. Like the app, it only sends `Request`s and shows
//! `Response`s; anything it has no screen for runs as a regular `locdev`
//! command from the `:` prompt, with the terminal handed back to it.
//!
//! Threads: the UI loop runs on a blocking thread (crossterm input is
//! synchronous); daemon calls run as tasks on the CLI's tokio runtime and
//! come back as [`Event`]s.

mod app;
mod ui;

pub mod theme;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use locdev_core::deps::{Job, Notification};
use locdev_core::mail::MailSummary;
use locdev_core::model::{
    Check, DaemonStatus, LogLine, ProcessRole, ProcessView, ProjectView, RuntimeInfo,
};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};
use locdev_core::services::InstanceView;
use locdev_ipc::IpcError;
use ratatui::crossterm::event::{self, Event as TermEvent, KeyEventKind};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

pub use app::split_args;
use app::{App, Effect};

/// Everything the dashboard and the sidebar show, fetched together.
pub struct Snapshot {
    pub status: DaemonStatus,
    pub projects: Vec<ProjectView>,
    pub instances: Vec<InstanceView>,
    pub processes: Vec<ProcessView>,
    pub notifications: Vec<Notification>,
    pub jobs: Vec<Job>,
}

/// Whose output a log view follows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogSource {
    Site {
        name: String,
        role: ProcessRole,
        aux: Option<String>,
    },
    Instance(String),
}

pub enum Event {
    Snapshot(Box<Snapshot>),
    /// The daemon could not be reached (or speaks another protocol).
    Offline(String),
    Doctor(Vec<Check>),
    Runtimes(Vec<RuntimeInfo>),
    Mail(Vec<MailSummary>),
    MailBody {
        id: String,
        text: String,
    },
    Logs {
        source: LogSource,
        lines: Vec<LogLine>,
    },
    /// An action finished: what to tell the user.
    Done(Result<String, String>),
}

pub enum Cmd {
    Refresh,
    Doctor,
    Runtimes,
    Mail,
    MailGet(String),
    Logs {
        source: LogSource,
        after: Option<u64>,
    },
    /// Sends `request`; `done` is the message shown when it succeeds.
    Act {
        done: String,
        request: Box<Request>,
    },
    StartDaemon,
}

/// Visual options from the command line.
pub struct Options {
    pub light: bool,
}

/// Whether stdin and stdout are a terminal the UI can take over.
pub fn interactive() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal() && std::io::stdin().is_terminal()
}

pub async fn run(paths: Paths, options: Options) -> anyhow::Result<ExitCode> {
    if !interactive() {
        anyhow::bail!("the terminal UI needs an interactive terminal; see `locdev --help`");
    }

    let (cmd_tx, mut cmd_rx) = unbounded_channel::<Cmd>();
    let (event_tx, event_rx) = mpsc::channel::<Event>();

    // The worker: one task per command, so a slow request never holds the
    // others back. Refreshes don't pile up behind a slow daemon.
    let worker_paths = paths.clone();
    let worker = tokio::spawn(async move {
        let refreshing = Arc::new(AtomicBool::new(false));
        while let Some(cmd) = cmd_rx.recv().await {
            let paths = worker_paths.clone();
            let tx = event_tx.clone();
            if matches!(cmd, Cmd::Refresh) && refreshing.swap(true, Ordering::SeqCst) {
                continue;
            }
            let refreshing = refreshing.clone();
            tokio::spawn(async move {
                let is_refresh = matches!(cmd, Cmd::Refresh);
                handle(&paths, cmd, &tx).await;
                if is_refresh {
                    refreshing.store(false, Ordering::SeqCst);
                }
            });
        }
    });

    let theme = if options.light {
        theme::LIGHT
    } else {
        theme::DARK
    };
    let ui_paths = paths.clone();
    let result = tokio::task::spawn_blocking(move || ui_loop(ui_paths, theme, cmd_tx, event_rx))
        .await
        .map_err(|e| anyhow::anyhow!("the terminal UI stopped: {e}"))?;
    worker.abort();
    result.map(|()| ExitCode::SUCCESS)
}

fn ui_loop(
    paths: Paths,
    theme: theme::Theme,
    cmd: UnboundedSender<Cmd>,
    events: mpsc::Receiver<Event>,
) -> anyhow::Result<()> {
    let mut app = App::new(paths, theme);
    let mut terminal = ratatui::init();
    let send = |c: Cmd| {
        let _ = cmd.send(c);
    };

    send(Cmd::Refresh);
    send(Cmd::Doctor);
    let mut last_refresh = Instant::now();
    let mut last_doctor = Instant::now();
    let mut last_logs = Instant::now() - Duration::from_secs(5);

    let outcome = loop {
        while let Ok(event) = events.try_recv() {
            app.apply(event);
        }

        if last_refresh.elapsed() >= Duration::from_millis(1500) {
            last_refresh = Instant::now();
            app.sample_machine();
            send(Cmd::Refresh);
        }
        if last_doctor.elapsed() >= Duration::from_secs(60) {
            last_doctor = Instant::now();
            send(Cmd::Doctor);
        }
        if last_logs.elapsed() >= Duration::from_millis(900) {
            last_logs = Instant::now();
            if let Some((source, after)) = app.watched_logs() {
                send(Cmd::Logs { source, after });
            }
        }

        if let Err(error) = terminal.draw(|frame| ui::draw(frame, &mut app)) {
            break Err(error.into());
        }

        match event::poll(Duration::from_millis(120)) {
            Ok(true) => match event::read() {
                Ok(TermEvent::Key(key)) if key.kind == KeyEventKind::Press => app.on_key(key),
                Ok(TermEvent::Paste(text)) => app.on_paste(&text),
                Ok(_) => {}
                Err(error) => break Err(error.into()),
            },
            Ok(false) => {}
            Err(error) => break Err(error.into()),
        }

        for effect in app.take_effects() {
            match effect {
                Effect::Quit => {
                    ratatui::restore();
                    return Ok(());
                }
                Effect::Send(c) => send(c),
                Effect::Open(url) => {
                    if let Err(error) = crate::site::open_url(&url) {
                        app.toast(Err(format!("{error:#}")));
                    }
                }
                Effect::Run(args) => {
                    ratatui::restore();
                    run_command(&args);
                    terminal = ratatui::init();
                    let _ = terminal.clear();
                    send(Cmd::Refresh);
                    send(Cmd::Doctor);
                }
            }
        }
    };
    ratatui::restore();
    outcome
}

/// Runs `locdev <args>` with the real terminal (so confirmations, password
/// prompts and `--follow` work), then waits for Enter.
fn run_command(args: &[String]) {
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "\x1b[2m$ locdev {}\x1b[0m\n", args.join(" "));
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("locdev"));
    // Run by the user from the prompt, in the user's terminal.
    match std::process::Command::new(exe).args(args).status() {
        Ok(status) if status.success() => {}
        Ok(status) => {
            let _ = writeln!(stdout, "\n\x1b[31mexited with {status}\x1b[0m");
        }
        Err(error) => {
            let _ = writeln!(stdout, "\x1b[31mcould not run locdev: {error}\x1b[0m");
        }
    }
    let _ = write!(stdout, "\n\x1b[2mPress Enter to return to LocDev…\x1b[0m");
    let _ = stdout.flush();
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
}

async fn call(paths: &Paths, request: Request) -> Result<Response, String> {
    match locdev_ipc::request(paths, request).await {
        Ok(Response::Error(error)) => Err(error.message),
        Ok(response) => Ok(response),
        Err(IpcError::Io(_)) => Err("the daemon is not running".into()),
        Err(IpcError::IncompatibleProtocol { daemon, client }) => Err(format!(
            "the daemon speaks protocol {daemon}, this CLI speaks {client}; run `locdev restart`"
        )),
        Err(error) => Err(error.to_string()),
    }
}

async fn handle(paths: &Paths, cmd: Cmd, tx: &mpsc::Sender<Event>) {
    let event = match cmd {
        Cmd::Refresh => match refresh(paths).await {
            Ok(snapshot) => Event::Snapshot(Box::new(snapshot)),
            Err(error) => Event::Offline(error),
        },
        Cmd::Doctor => match call(paths, Request::Doctor).await {
            Ok(Response::Doctor { checks }) => Event::Doctor(checks),
            _ => return,
        },
        Cmd::Runtimes => match call(paths, Request::ListRuntimes { refresh: false }).await {
            Ok(Response::Runtimes { runtimes }) => Event::Runtimes(runtimes),
            Ok(_) => return,
            Err(error) => Event::Done(Err(error)),
        },
        Cmd::Mail => {
            let request = Request::MailList {
                site: None,
                query: None,
                unread: false,
                attachments: false,
                limit: Some(200),
            };
            match call(paths, request).await {
                Ok(Response::MailList { messages }) => Event::Mail(messages),
                Ok(_) => return,
                Err(error) => Event::Done(Err(error)),
            }
        }
        Cmd::MailGet(id) => match call(paths, Request::MailGet { id: id.clone() }).await {
            Ok(Response::MailDetail(detail)) => Event::MailBody {
                id,
                text: mail_text(&detail),
            },
            Ok(_) => return,
            Err(error) => Event::Done(Err(error)),
        },
        Cmd::Logs { source, after } => {
            let request = match &source {
                LogSource::Site { name, role, aux } => Request::ProjectLogs {
                    name: name.clone(),
                    after,
                    limit: Some(500),
                    process: *role,
                    aux: aux.clone(),
                },
                LogSource::Instance(name) => Request::InstanceLogs {
                    name: name.clone(),
                    after,
                    limit: Some(500),
                },
            };
            match call(paths, request).await {
                Ok(Response::Logs { lines }) => Event::Logs { source, lines },
                _ => return,
            }
        }
        Cmd::Act { done, request } => Event::Done(call(paths, *request).await.map(|_| done)),
        Cmd::StartDaemon => Event::Done(
            locdev_ipc::launcher::ensure_running(paths)
                .await
                .map(|_| "daemon started".to_string())
                .map_err(|e| format!("could not start the daemon: {e}")),
        ),
    };
    let _ = tx.send(event);
}

async fn refresh(paths: &Paths) -> Result<Snapshot, String> {
    let status = match call(paths, Request::Status).await? {
        Response::Status(status) => status,
        other => return Err(format!("unexpected response: {other:?}")),
    };
    let (projects, services, processes, notifications, jobs) = tokio::join!(
        call(paths, Request::ListProjects),
        call(paths, Request::ListServices { refresh: false }),
        call(paths, Request::ListProcesses),
        call(paths, Request::ListNotifications { all: false }),
        call(paths, Request::ListJobs { project: None }),
    );
    Ok(Snapshot {
        status,
        projects: match projects {
            Ok(Response::Projects { projects }) => projects,
            _ => vec![],
        },
        instances: match services {
            Ok(Response::Services { instances, .. }) => instances,
            _ => vec![],
        },
        processes: match processes {
            Ok(Response::Processes { processes }) => processes,
            _ => vec![],
        },
        notifications: match notifications {
            Ok(Response::Notifications { notifications, .. }) => notifications,
            _ => vec![],
        },
        jobs: match jobs {
            Ok(Response::Jobs { jobs }) => jobs,
            _ => vec![],
        },
    })
}

/// Headers and the text part of a captured message (HTML stays in the app).
fn mail_text(detail: &locdev_core::mail::MailDetail) -> String {
    let s = &detail.summary;
    let mut text = format!(
        "From:    {}\nTo:      {}\nSubject: {}\n",
        s.from,
        s.to.join(", "),
        s.subject
    );
    if !detail.cc.is_empty() {
        text.push_str(&format!("Cc:      {}\n", detail.cc.join(", ")));
    }
    if !detail.attachments.is_empty() {
        text.push_str(&format!("Files:   {}\n", detail.attachments.len()));
    }
    text.push('\n');
    match &detail.text {
        Some(body) => {
            if detail.text_generated {
                text.push_str("(text generated from the HTML part)\n\n");
            }
            text.push_str(body);
        }
        None => text.push_str("(no text part; open it in the app or with `locdev mail show`)"),
    }
    if !detail.problems.is_empty() {
        text.push_str("\n\nProblems:\n");
        for p in &detail.problems {
            text.push_str(&format!("  • {p}\n"));
        }
    }
    text
}
