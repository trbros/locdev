//! TUI state and key handling. Rendering lives in `ui.rs`.

use std::collections::HashMap;
use std::time::Instant;

use locdev_core::deps::{Job, JobStatus, Notification};
use locdev_core::mail::MailSummary;
use locdev_core::model::{
    Check, CheckLevel, DaemonStatus, LogLine, ProcessRole, ProcessState, ProcessView, ProjectView,
    RuntimeInfo,
};
use locdev_core::paths::Paths;
use locdev_core::protocol::{JobAction, ProcessAction, Request};
use locdev_core::services::InstanceView;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sysinfo::System;

use super::theme::Theme;
use super::{Cmd, Event, LogSource, Snapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Page {
    Dashboard,
    Sites,
    Services,
    Runtimes,
    Processes,
    Mailbox,
    Activity,
    Doctor,
    About,
}

impl Page {
    pub const ALL: [Page; 9] = [
        Page::Dashboard,
        Page::Sites,
        Page::Services,
        Page::Runtimes,
        Page::Processes,
        Page::Mailbox,
        Page::Activity,
        Page::Doctor,
        Page::About,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Dashboard => "Dashboard",
            Page::Sites => "Sites",
            Page::Services => "Services",
            Page::Runtimes => "Runtimes",
            Page::Processes => "Processes",
            Page::Mailbox => "Mailbox",
            Page::Activity => "Activity",
            Page::Doctor => "Doctor",
            Page::About => "About",
        }
    }

    /// Single-width glyphs only: wide ones break the sidebar's alignment.
    pub fn glyph(self) -> &'static str {
        match self {
            Page::Dashboard => "⌂",
            Page::Sites => "◇",
            Page::Services => "≡",
            Page::Runtimes => "λ",
            Page::Processes => "»",
            Page::Mailbox => "@",
            Page::Activity => "◔",
            Page::Doctor => "+",
            Page::About => "i",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
}

/// One row of the sidebar the cursor can rest on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SideItem {
    Nav(Page),
    Site(String),
    Service(String),
}

/// Something the palette can do.
#[derive(Debug, Clone)]
pub enum Action {
    Go(Page),
    Site(String),
    Service(String),
    Send {
        done: String,
        request: Box<Request>,
    },
    Logs {
        title: String,
        source: LogSource,
    },
    Browse(String),
    /// Opens the `:` prompt with this text.
    Prefill(String),
    StartDaemon,
    Doctor,
}

pub struct PaletteItem {
    pub label: String,
    pub hint: String,
    pub action: Action,
}

pub enum Overlay {
    Help,
    Palette { input: String, selected: usize },
    Prompt { input: String },
    Filter,
    Confirm { message: String, cmd: Box<Cmd> },
}

pub struct LogView {
    pub title: String,
    pub source: LogSource,
    /// Lines up from the bottom; 0 follows new output.
    pub scroll: usize,
}

pub enum Effect {
    Quit,
    Send(Cmd),
    Open(String),
    Run(Vec<String>),
}

pub struct Machine {
    pub cpu: f32,
    pub used: u64,
    pub total: u64,
}

pub struct App {
    pub paths: Paths,
    pub theme: Theme,
    pub page: Page,
    pub focus: Focus,
    pub side_cursor: usize,
    /// Selected row per page.
    pub rows: HashMap<Page, usize>,
    pub filter: String,
    pub overlay: Option<Overlay>,
    pub logs: Option<LogView>,
    pub mail_open: Option<(String, String)>,
    pub mail_scroll: u16,

    pub snapshot: Option<Snapshot>,
    pub offline: Option<String>,
    pub checks: Option<Vec<Check>>,
    pub runtimes: Option<Vec<RuntimeInfo>>,
    pub mail: Option<Vec<MailSummary>>,
    pub log_lines: HashMap<LogSource, Vec<LogLine>>,
    pub toast: Option<(Result<String, String>, Instant)>,
    pub machine: Machine,
    pub startup: Option<bool>,

    system: Option<System>,
    effects: Vec<Effect>,
}

/// Lines kept per followed process.
const MAX_LINES: usize = 3000;

impl App {
    pub fn new(paths: Paths, theme: Theme) -> Self {
        // Machine figures are a nicety: an unsupported OS just shows zeros.
        let mut system = System::new().ok();
        if let Some(system) = &mut system {
            system.refresh_memory();
            system.refresh_cpu_usage();
        }
        let startup = locdev_platform::service::status();
        let startup = (!startup.manager.is_empty()).then_some(startup.installed);
        Self {
            paths,
            theme,
            page: Page::Dashboard,
            focus: Focus::Sidebar,
            side_cursor: 0,
            rows: HashMap::new(),
            filter: String::new(),
            overlay: None,
            logs: None,
            mail_open: None,
            mail_scroll: 0,
            snapshot: None,
            offline: None,
            checks: None,
            runtimes: None,
            mail: None,
            log_lines: HashMap::new(),
            toast: None,
            machine: Machine {
                cpu: 0.0,
                used: system.as_ref().map_or(0, System::used_memory),
                total: system.as_ref().map_or(0, System::total_memory),
            },
            startup,
            system,
            effects: vec![],
        }
    }

    pub fn sample_machine(&mut self) {
        let Some(system) = &mut self.system else {
            return;
        };
        system.refresh_cpu_usage();
        system.refresh_memory();
        self.machine = Machine {
            cpu: system.global_cpu_usage(),
            used: system.used_memory(),
            total: system.total_memory(),
        };
    }

    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }

    pub fn toast(&mut self, message: Result<String, String>) {
        self.toast = Some((message, Instant::now()));
    }

    fn send(&mut self, cmd: Cmd) {
        self.effects.push(Effect::Send(cmd));
    }

    fn act(&mut self, done: impl Into<String>, request: Request) {
        self.send(Cmd::Act {
            done: done.into(),
            request: Box::new(request),
        });
    }

    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Snapshot(snapshot) => {
                self.snapshot = Some(*snapshot);
                self.offline = None;
                self.clamp();
            }
            Event::Offline(reason) => {
                self.offline = Some(reason);
            }
            Event::Doctor(checks) => self.checks = Some(checks),
            Event::Runtimes(runtimes) => self.runtimes = Some(runtimes),
            Event::Mail(messages) => self.mail = Some(messages),
            Event::MailBody { id, text } => {
                self.mail_open = Some((id, text));
                self.mail_scroll = 0;
            }
            Event::Logs { source, lines } => {
                let kept = self.log_lines.entry(source).or_default();
                let last = kept.last().map(|l| l.seq);
                kept.extend(lines.into_iter().filter(|l| last.is_none_or(|s| l.seq > s)));
                if kept.len() > MAX_LINES {
                    kept.drain(..kept.len() - MAX_LINES);
                }
            }
            Event::Done(result) => {
                let ok = result.is_ok();
                self.toast(result);
                if ok {
                    self.send(Cmd::Refresh);
                    match self.page {
                        Page::Mailbox => self.send(Cmd::Mail),
                        Page::Runtimes => self.send(Cmd::Runtimes),
                        _ => {}
                    }
                }
            }
        }
    }

    // ── What's shown ────────────────────────────────────────────────────────

    pub fn status(&self) -> Option<&DaemonStatus> {
        self.snapshot.as_ref().map(|s| &s.status)
    }

    fn matches(&self, text: &str) -> bool {
        self.filter.is_empty() || text.to_lowercase().contains(&self.filter.to_lowercase())
    }

    pub fn sites(&self) -> Vec<&ProjectView> {
        let Some(s) = &self.snapshot else {
            return vec![];
        };
        s.projects
            .iter()
            .filter(|p| self.matches(&p.project.name) || self.matches(&p.stack.framework))
            .collect()
    }

    pub fn instances(&self) -> Vec<&InstanceView> {
        let Some(s) = &self.snapshot else {
            return vec![];
        };
        s.instances
            .iter()
            .filter(|i| self.matches(&i.instance.name) || self.matches(i.instance.kind.label()))
            .collect()
    }

    pub fn processes(&self) -> Vec<&ProcessView> {
        let Some(s) = &self.snapshot else {
            return vec![];
        };
        s.processes
            .iter()
            .filter(|p| self.matches(&p.project) || self.matches(&p.label))
            .collect()
    }

    pub fn notifications(&self) -> Vec<&Notification> {
        let Some(s) = &self.snapshot else {
            return vec![];
        };
        s.notifications
            .iter()
            .filter(|n| !n.dismissed && (self.matches(&n.title) || self.matches(&n.body)))
            .collect()
    }

    pub fn jobs(&self) -> Vec<&Job> {
        let Some(s) = &self.snapshot else {
            return vec![];
        };
        s.jobs
            .iter()
            .filter(|j| {
                self.matches(j.kind.label())
                    || j.project.as_deref().is_some_and(|p| self.matches(p))
            })
            .collect()
    }

    pub fn messages(&self) -> Vec<&MailSummary> {
        self.mail
            .iter()
            .flatten()
            .filter(|m| {
                self.matches(&m.subject)
                    || self.matches(&m.from)
                    || m.site.as_deref().is_some_and(|s| self.matches(s))
            })
            .collect()
    }

    pub fn row(&self, page: Page) -> usize {
        self.rows.get(&page).copied().unwrap_or(0)
    }

    fn page_len(&self, page: Page) -> usize {
        match page {
            Page::Sites => self.sites().len(),
            Page::Services => self.instances().len(),
            Page::Processes => self.processes().len(),
            Page::Mailbox => self.messages().len(),
            Page::Activity => self.notifications().len() + self.jobs().len(),
            Page::Doctor => self.checks.as_ref().map_or(0, Vec::len),
            Page::Runtimes => self.runtimes.as_ref().map_or(0, Vec::len),
            Page::Dashboard | Page::About => 0,
        }
    }

    pub fn side_items(&self) -> Vec<SideItem> {
        let mut items: Vec<SideItem> = Page::ALL.iter().map(|p| SideItem::Nav(*p)).collect();
        items.extend(
            self.sites()
                .iter()
                .map(|p| SideItem::Site(p.project.name.clone())),
        );
        items.extend(
            self.instances()
                .iter()
                .map(|i| SideItem::Service(i.instance.name.clone())),
        );
        items
    }

    fn clamp(&mut self) {
        let side = self.side_items().len();
        self.side_cursor = self.side_cursor.min(side.saturating_sub(1));
        for page in Page::ALL {
            let len = self.page_len(page);
            if let Some(row) = self.rows.get_mut(&page) {
                *row = (*row).min(len.saturating_sub(1));
            }
        }
    }

    pub fn selected_site(&self) -> Option<&ProjectView> {
        self.sites().get(self.row(Page::Sites)).copied()
    }

    pub fn selected_instance(&self) -> Option<&InstanceView> {
        self.instances().get(self.row(Page::Services)).copied()
    }

    /// The process whose output should be polled right now, and from where.
    pub fn watched_logs(&self) -> Option<(LogSource, Option<u64>)> {
        let source = if let Some(view) = &self.logs {
            view.source.clone()
        } else {
            match self.page {
                Page::Sites => LogSource::Site {
                    name: self.selected_site()?.project.name.clone(),
                    role: ProcessRole::Main,
                    aux: None,
                },
                Page::Services => {
                    LogSource::Instance(self.selected_instance()?.instance.name.clone())
                }
                _ => return None,
            }
        };
        let after = self
            .log_lines
            .get(&source)
            .and_then(|l| l.last())
            .map(|l| l.seq);
        Some((source, after))
    }

    pub fn running_count(&self) -> usize {
        self.snapshot.as_ref().map_or(0, |s| {
            s.projects
                .iter()
                .filter(|p| p.process.state == ProcessState::Running)
                .count()
        })
    }

    // ── Navigation ──────────────────────────────────────────────────────────

    pub fn go(&mut self, page: Page) {
        if self.page != page {
            self.logs = None;
            self.mail_open = None;
        }
        self.page = page;
        if let Some(i) = self
            .side_items()
            .iter()
            .position(|item| *item == SideItem::Nav(page))
            && matches!(
                self.side_items().get(self.side_cursor),
                Some(SideItem::Nav(_))
            )
        {
            self.side_cursor = i;
        }
        match page {
            Page::Runtimes if self.runtimes.is_none() => self.send(Cmd::Runtimes),
            Page::Mailbox => self.send(Cmd::Mail),
            Page::Doctor if self.checks.is_none() => self.send(Cmd::Doctor),
            _ => {}
        }
    }

    fn open_site(&mut self, name: &str) {
        self.go(Page::Sites);
        if let Some(i) = self.sites().iter().position(|p| p.project.name == name) {
            self.rows.insert(Page::Sites, i);
        }
    }

    fn open_service(&mut self, name: &str) {
        self.go(Page::Services);
        if let Some(i) = self
            .instances()
            .iter()
            .position(|p| p.instance.name == name)
        {
            self.rows.insert(Page::Services, i);
        }
    }

    fn open_logs(&mut self, title: String, source: LogSource) {
        self.logs = Some(LogView {
            title,
            source,
            scroll: 0,
        });
    }

    fn move_by(&mut self, delta: isize) {
        if let Some(view) = &mut self.logs {
            view.scroll = view.scroll.saturating_add_signed(-delta);
            return;
        }
        if self.mail_open.is_some() {
            self.mail_scroll = self.mail_scroll.saturating_add_signed(delta as i16);
            return;
        }
        match self.focus {
            Focus::Sidebar => {
                let len = self.side_items().len();
                self.side_cursor = step(self.side_cursor, delta, len);
                self.follow_sidebar();
            }
            Focus::Main => {
                let len = self.page_len(self.page);
                let row = step(self.row(self.page), delta, len);
                self.rows.insert(self.page, row);
            }
        }
    }

    /// Moving over the sidebar previews what's under the cursor.
    fn follow_sidebar(&mut self) {
        match self.side_items().get(self.side_cursor).cloned() {
            Some(SideItem::Nav(page)) => self.go(page),
            Some(SideItem::Site(name)) => self.open_site(&name),
            Some(SideItem::Service(name)) => self.open_service(&name),
            None => {}
        }
    }

    // ── Input ───────────────────────────────────────────────────────────────

    pub fn on_paste(&mut self, text: &str) {
        let text = text.replace(['\n', '\r'], " ");
        match &mut self.overlay {
            Some(Overlay::Prompt { input }) | Some(Overlay::Palette { input, .. }) => {
                input.push_str(&text)
            }
            Some(Overlay::Filter) => self.filter.push_str(&text),
            _ => {}
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            self.effects.push(Effect::Quit);
            return;
        }
        if self.overlay.is_some() {
            self.overlay_key(key);
            return;
        }
        if ctrl && matches!(key.code, KeyCode::Char('p') | KeyCode::Char('k')) {
            self.overlay = Some(Overlay::Palette {
                input: String::new(),
                selected: 0,
            });
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.effects.push(Effect::Quit),
            KeyCode::Char('?') => self.overlay = Some(Overlay::Help),
            KeyCode::Char(':') => {
                self.overlay = Some(Overlay::Prompt {
                    input: String::new(),
                })
            }
            KeyCode::Char('/') => self.overlay = Some(Overlay::Filter),
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Sidebar => Focus::Main,
                    Focus::Main => Focus::Sidebar,
                }
            }
            KeyCode::Esc => {
                if self.logs.take().is_some() || self.mail_open.take().is_some() {
                } else if !self.filter.is_empty() {
                    self.filter.clear();
                } else {
                    self.focus = Focus::Sidebar;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_by(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_by(1),
            KeyCode::PageUp => self.move_by(-10),
            KeyCode::PageDown => self.move_by(10),
            KeyCode::Home | KeyCode::Char('g') => self.move_by(-10_000),
            KeyCode::End | KeyCode::Char('G') => self.move_by(10_000),
            KeyCode::Char(c @ '1'..='9') => {
                let page = Page::ALL[c as usize - '1' as usize];
                self.go(page);
            }
            KeyCode::Char('r') if self.page != Page::Doctor => {
                self.send(Cmd::Refresh);
                match self.page {
                    Page::Runtimes => self.send(Cmd::Runtimes),
                    Page::Mailbox => self.send(Cmd::Mail),
                    _ => {}
                }
                self.toast(Ok("refreshed".into()));
            }
            KeyCode::Char('S') if self.offline.is_some() => {
                self.send(Cmd::StartDaemon);
                self.toast(Ok("starting the daemon…".into()));
            }
            KeyCode::Enter if self.focus == Focus::Sidebar => {
                self.follow_sidebar();
                self.focus = Focus::Main;
            }
            _ => self.page_key(key),
        }
    }

    fn page_key(&mut self, key: KeyEvent) {
        match self.page {
            Page::Sites => self.sites_key(key),
            Page::Services => self.services_key(key),
            Page::Processes => self.processes_key(key),
            Page::Mailbox => self.mail_key(key),
            Page::Activity => self.activity_key(key),
            Page::Doctor => self.doctor_key(key),
            Page::Runtimes => self.runtimes_key(key),
            Page::Dashboard | Page::About => {
                if key.code == KeyCode::Enter {
                    self.focus = Focus::Sidebar;
                }
            }
        }
    }

    fn sites_key(&mut self, key: KeyEvent) {
        let Some(site) = self.selected_site() else {
            if key.code == KeyCode::Char('a') {
                self.prefill("add ");
            }
            return;
        };
        let name = site.project.name.clone();
        let running = site.process.state == ProcessState::Running;
        let has_dev = site.project.dev_server.is_some();
        let url = site.https_url.clone().or_else(|| site.url.clone());
        match key.code {
            KeyCode::Char('s') if running => {
                self.act(format!("{name} stopped"), Request::StopProject { name })
            }
            KeyCode::Char('s') => {
                self.act(format!("{name} started"), Request::StartProject { name })
            }
            KeyCode::Char('R') => self.act(
                format!("{name} restarted"),
                Request::RestartProject { name },
            ),
            KeyCode::Char('o') => match url {
                Some(url) => self.effects.push(Effect::Open(url)),
                None => self.toast(Err(format!("{name} has no address yet"))),
            },
            KeyCode::Enter | KeyCode::Char('l') => self.open_logs(
                format!("{name} · output"),
                LogSource::Site {
                    name,
                    role: ProcessRole::Main,
                    aux: None,
                },
            ),
            KeyCode::Char('d') if has_dev => self.open_logs(
                format!("{name} · helper output"),
                LogSource::Site {
                    name,
                    role: ProcessRole::Dev,
                    aux: None,
                },
            ),
            KeyCode::Char('D') if has_dev => {
                let dev_running = self
                    .selected_site()
                    .and_then(|s| s.dev_process.as_ref())
                    .is_some_and(|p| p.state == ProcessState::Running);
                if dev_running {
                    self.act(
                        format!("{name}'s helper stopped"),
                        Request::StopDevServer { name },
                    )
                } else {
                    self.act(
                        format!("{name}'s helper started"),
                        Request::StartDevServer { name },
                    )
                }
            }
            KeyCode::Char('i') => self.effects.push(Effect::Run(vec!["info".into(), name])),
            KeyCode::Char('x') => {
                self.overlay = Some(Overlay::Confirm {
                    message: format!("Remove {name} from LocDev? Its files are left untouched."),
                    cmd: Box::new(Cmd::Act {
                        done: format!("removed {name}"),
                        request: Box::new(Request::RemoveProject {
                            name,
                            delete_files: false,
                        }),
                    }),
                })
            }
            KeyCode::Char('a') => self.prefill("add "),
            _ => {}
        }
    }

    fn services_key(&mut self, key: KeyEvent) {
        let Some(instance) = self.selected_instance() else {
            return;
        };
        let name = instance.instance.name.clone();
        let running = instance.info.state == ProcessState::Running;
        let control = |name: String, action| Request::ControlInstance { name, action };
        match key.code {
            KeyCode::Char('s') if running => self.act(
                format!("{name} stopped"),
                control(name, ProcessAction::Stop),
            ),
            KeyCode::Char('s') => self.act(
                format!("{name} started"),
                control(name, ProcessAction::Start),
            ),
            KeyCode::Char('R') => self.act(
                format!("{name} restarted"),
                control(name, ProcessAction::Restart),
            ),
            KeyCode::Enter | KeyCode::Char('l') => {
                self.open_logs(format!("{name} · output"), LogSource::Instance(name))
            }
            KeyCode::Char('b') => self.act(
                format!("{name} backed up"),
                Request::BackupInstance { name },
            ),
            _ => {}
        }
    }

    fn processes_key(&mut self, key: KeyEvent) {
        let Some(p) = self.processes().get(self.row(Page::Processes)).copied() else {
            return;
        };
        let name = p.project.clone();
        let running = p.info.state == ProcessState::Running;
        let label = match (&p.aux, p.role) {
            (Some(aux), _) => format!("{name} · {aux}"),
            (None, ProcessRole::Main) => name.clone(),
            (None, ProcessRole::Dev) => format!("{name} · helper"),
        };
        let source = LogSource::Site {
            name: name.clone(),
            role: p.role,
            aux: p.aux.clone(),
        };
        let aux = p.aux.clone();
        let role = p.role;
        match key.code {
            KeyCode::Enter | KeyCode::Char('l') => {
                self.open_logs(format!("{label} · output"), source)
            }
            KeyCode::Char('s') => {
                let done = format!("{label} {}", if running { "stopped" } else { "started" });
                let request = match (aux, role, running) {
                    (Some(process), _, _) => Request::ControlProcess {
                        name,
                        process,
                        action: if running {
                            ProcessAction::Stop
                        } else {
                            ProcessAction::Start
                        },
                    },
                    (None, ProcessRole::Main, true) => Request::StopProject { name },
                    (None, ProcessRole::Main, false) => Request::StartProject { name },
                    (None, ProcessRole::Dev, true) => Request::StopDevServer { name },
                    (None, ProcessRole::Dev, false) => Request::StartDevServer { name },
                };
                self.act(done, request);
            }
            KeyCode::Char('R') => {
                let request = match (aux, role) {
                    (Some(process), _) => Request::ControlProcess {
                        name,
                        process,
                        action: ProcessAction::Restart,
                    },
                    (None, ProcessRole::Main) => Request::RestartProject { name },
                    (None, ProcessRole::Dev) => Request::RestartDevServer { name },
                };
                self.act(format!("{label} restarted"), request);
            }
            _ => {}
        }
    }

    fn mail_key(&mut self, key: KeyEvent) {
        let Some(m) = self.messages().get(self.row(Page::Mailbox)).copied() else {
            return;
        };
        let id = m.id.clone();
        let subject = m.subject.clone();
        match key.code {
            KeyCode::Enter => self.send(Cmd::MailGet(id)),
            KeyCode::Char('x') | KeyCode::Delete => {
                self.mail_open = None;
                self.overlay = Some(Overlay::Confirm {
                    message: format!("Delete “{subject}”?"),
                    cmd: Box::new(Cmd::Act {
                        done: "message deleted".into(),
                        request: Box::new(Request::MailDelete { ids: vec![id] }),
                    }),
                });
            }
            _ => {}
        }
    }

    fn activity_key(&mut self, key: KeyEvent) {
        let row = self.row(Page::Activity);
        let notes = self.notifications();
        if let Some(n) = notes.get(row) {
            let id = n.id;
            match key.code {
                KeyCode::Enter => self.act(
                    "marked as read",
                    Request::MarkNotifications {
                        ids: vec![id],
                        read: Some(true),
                        dismissed: None,
                    },
                ),
                KeyCode::Char('x') => self.act(
                    "dismissed",
                    Request::MarkNotifications {
                        ids: vec![id],
                        read: Some(true),
                        dismissed: Some(true),
                    },
                ),
                KeyCode::Char('a') => self.act(
                    "all marked as read",
                    Request::MarkNotifications {
                        ids: vec![],
                        read: Some(true),
                        dismissed: None,
                    },
                ),
                _ => {}
            }
            return;
        }
        let Some(job) = self.jobs().get(row - notes.len()).copied() else {
            return;
        };
        let id = job.id;
        let action = match (key.code, job.status) {
            (KeyCode::Char('c'), JobStatus::Pending | JobStatus::Running) => JobAction::Cancel,
            (KeyCode::Char('p'), JobStatus::Pending) => JobAction::Pause,
            (KeyCode::Char('p'), JobStatus::Paused) => JobAction::Resume,
            (KeyCode::Char('t'), JobStatus::Failed | JobStatus::Cancelled) => JobAction::Retry,
            (KeyCode::Enter, _) => {
                self.effects.push(Effect::Run(vec![
                    "jobs".into(),
                    "status".into(),
                    id.to_string(),
                ]));
                return;
            }
            _ => return,
        };
        self.act(
            format!("job {id}: {action:?}").to_lowercase(),
            Request::ControlJob { id, action },
        );
    }

    fn doctor_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('r') => {
                self.send(Cmd::Doctor);
                self.toast(Ok("checking again…".into()));
            }
            KeyCode::Enter => {
                let remedy = self
                    .checks
                    .as_ref()
                    .and_then(|c| c.get(self.row(Page::Doctor)))
                    .and_then(|c| c.remedy.clone());
                if let Some(rest) = remedy.as_deref().and_then(|r| r.strip_prefix("locdev ")) {
                    self.prefill(rest);
                }
            }
            _ => {}
        }
    }

    fn runtimes_key(&mut self, key: KeyEvent) {
        let Some(kind) = self
            .runtimes
            .as_ref()
            .and_then(|r| r.get(self.row(Page::Runtimes)))
            .map(|r| r.kind)
        else {
            return;
        };
        let kind = kind_arg(kind);
        match key.code {
            KeyCode::Char('i') | KeyCode::Enter => {
                self.prefill(&format!("runtime install {kind} "))
            }
            KeyCode::Char('d') => self.prefill(&format!("runtime default {kind} ")),
            KeyCode::Char('u') => {
                self.effects
                    .push(Effect::Run(vec!["runtime".into(), "update".into(), kind]))
            }
            KeyCode::Char('a') => self.effects.push(Effect::Run(vec![
                "runtime".into(),
                "available".into(),
                kind,
            ])),
            _ => {}
        }
    }

    fn prefill(&mut self, text: &str) {
        self.overlay = Some(Overlay::Prompt {
            input: text.to_string(),
        });
    }

    fn overlay_key(&mut self, key: KeyEvent) {
        let Some(overlay) = self.overlay.take() else {
            return;
        };
        match overlay {
            Overlay::Help => {}
            Overlay::Filter => match key.code {
                KeyCode::Esc => self.filter.clear(),
                KeyCode::Enter => {}
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.overlay = Some(Overlay::Filter);
                }
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    self.overlay = Some(Overlay::Filter);
                    self.clamp();
                }
                _ => self.overlay = Some(Overlay::Filter),
            },
            Overlay::Confirm { message, cmd } => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => self.send(*cmd),
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {}
                _ => self.overlay = Some(Overlay::Confirm { message, cmd }),
            },
            Overlay::Prompt { mut input } => match key.code {
                KeyCode::Esc => {}
                KeyCode::Enter => match split_args(&input) {
                    Ok(args) if args.is_empty() => {}
                    Ok(args) if matches!(args[0].as_str(), "tui" | "q" | "quit") => {
                        if args[0] != "tui" {
                            self.effects.push(Effect::Quit);
                        }
                    }
                    Ok(mut args) => {
                        if args[0] == "locdev" {
                            args.remove(0);
                        }
                        self.effects.push(Effect::Run(args));
                    }
                    Err(error) => {
                        self.toast(Err(error));
                        self.overlay = Some(Overlay::Prompt { input });
                    }
                },
                KeyCode::Backspace => {
                    input.pop();
                    self.overlay = Some(Overlay::Prompt { input });
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.overlay = Some(Overlay::Prompt {
                        input: String::new(),
                    });
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    self.overlay = Some(Overlay::Prompt { input });
                }
                _ => self.overlay = Some(Overlay::Prompt { input }),
            },
            Overlay::Palette {
                mut input,
                mut selected,
            } => {
                let len = self.palette(&input).len();
                match key.code {
                    KeyCode::Esc => return,
                    KeyCode::Enter => {
                        if let Some(item) = self.palette(&input).into_iter().nth(selected) {
                            self.run_action(item.action);
                        }
                        return;
                    }
                    KeyCode::Up => selected = step(selected, -1, len),
                    KeyCode::Down | KeyCode::Tab => selected = step(selected, 1, len),
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        selected = step(selected, 1, len)
                    }
                    KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        selected = step(selected, -1, len)
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        selected = 0;
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        selected = 0;
                    }
                    _ => {}
                }
                self.overlay = Some(Overlay::Palette { input, selected });
            }
        }
    }

    fn run_action(&mut self, action: Action) {
        match action {
            Action::Go(page) => {
                self.go(page);
                self.focus = Focus::Main;
            }
            Action::Site(name) => {
                self.open_site(&name);
                self.focus = Focus::Main;
            }
            Action::Service(name) => {
                self.open_service(&name);
                self.focus = Focus::Main;
            }
            Action::Send { done, request } => self.act(done, *request),
            Action::Logs { title, source } => self.open_logs(title, source),
            Action::Browse(url) => self.effects.push(Effect::Open(url)),
            Action::Prefill(text) => self.prefill(&text),
            Action::StartDaemon => self.send(Cmd::StartDaemon),
            Action::Doctor => {
                self.go(Page::Doctor);
                self.send(Cmd::Doctor);
            }
        }
    }

    /// "Go to or do": pages, sites, services, their actions and every CLI
    /// command (from the same catalog the app's Command line page shows).
    pub fn palette(&self, query: &str) -> Vec<PaletteItem> {
        let mut items = vec![];
        let item = |label: String, hint: &str, action| PaletteItem {
            label,
            hint: hint.to_string(),
            action,
        };
        for page in Page::ALL {
            items.push(item(
                format!("Go to {}", page.label()),
                "page",
                Action::Go(page),
            ));
        }
        if self.offline.is_some() {
            items.push(item(
                "Start the daemon".into(),
                "daemon",
                Action::StartDaemon,
            ));
        }
        items.push(item("Run the doctor".into(), "check", Action::Doctor));
        if let Some(s) = &self.snapshot {
            for p in &s.projects {
                let name = p.project.name.clone();
                items.push(item(name.clone(), "site", Action::Site(name.clone())));
                let running = p.process.state == ProcessState::Running;
                items.push(if running {
                    item(
                        format!("Stop {name}"),
                        "site",
                        Action::Send {
                            done: format!("{name} stopped"),
                            request: Box::new(Request::StopProject { name: name.clone() }),
                        },
                    )
                } else {
                    item(
                        format!("Start {name}"),
                        "site",
                        Action::Send {
                            done: format!("{name} started"),
                            request: Box::new(Request::StartProject { name: name.clone() }),
                        },
                    )
                });
                items.push(item(
                    format!("Restart {name}"),
                    "site",
                    Action::Send {
                        done: format!("{name} restarted"),
                        request: Box::new(Request::RestartProject { name: name.clone() }),
                    },
                ));
                items.push(item(
                    format!("Logs of {name}"),
                    "site",
                    Action::Logs {
                        title: format!("{name} · output"),
                        source: LogSource::Site {
                            name: name.clone(),
                            role: ProcessRole::Main,
                            aux: None,
                        },
                    },
                ));
                if let Some(url) = p.https_url.clone().or_else(|| p.url.clone()) {
                    items.push(item(
                        format!("Open {name} in the browser"),
                        "site",
                        Action::Browse(url),
                    ));
                }
            }
            for i in &s.instances {
                let name = i.instance.name.clone();
                items.push(item(name.clone(), "service", Action::Service(name.clone())));
                let running = i.info.state == ProcessState::Running;
                items.push(item(
                    format!("{} {name}", if running { "Stop" } else { "Start" }),
                    "service",
                    Action::Send {
                        done: format!("{name} {}", if running { "stopped" } else { "started" }),
                        request: Box::new(Request::ControlInstance {
                            name: name.clone(),
                            action: if running {
                                ProcessAction::Stop
                            } else {
                                ProcessAction::Start
                            },
                        }),
                    },
                ));
            }
        }
        for category in crate::catalog::catalog().categories {
            for entry in category.commands {
                let has_args = !entry.args.is_empty();
                let text = if has_args {
                    format!("{} ", entry.name)
                } else {
                    entry.name.clone()
                };
                items.push(item(
                    if entry.about.is_empty() {
                        format!("locdev {}", entry.name)
                    } else {
                        format!("locdev {} — {}", entry.name, entry.about)
                    },
                    "command",
                    Action::Prefill(text),
                ));
            }
        }

        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return items;
        }
        let mut scored: Vec<(i32, PaletteItem)> = items
            .into_iter()
            .filter_map(|i| fuzzy(&query, &i.label.to_lowercase()).map(|s| (s, i)))
            .collect();
        scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        scored.into_iter().map(|(_, i)| i).collect()
    }

    pub fn attention(&self) -> Vec<(CheckLevel, String)> {
        let mut out = vec![];
        if let Some(reason) = &self.offline {
            out.push((
                CheckLevel::Error,
                format!("Daemon not reachable ({reason}) — press S to start it"),
            ));
            return out;
        }
        if let Some(s) = &self.snapshot {
            for p in &s.projects {
                if p.process.state == ProcessState::Failed {
                    out.push((
                        CheckLevel::Error,
                        format!(
                            "{} failed{}",
                            p.project.name,
                            p.process
                                .message
                                .as_deref()
                                .map(|m| format!(": {m}"))
                                .unwrap_or_default()
                        ),
                    ));
                }
                if !p.path_exists {
                    out.push((
                        CheckLevel::Warn,
                        format!("{}: its folder is gone", p.project.name),
                    ));
                }
            }
            for i in &s.instances {
                if i.info.state == ProcessState::Failed {
                    out.push((CheckLevel::Error, format!("{} failed", i.instance.name)));
                }
            }
            // The proxy's own error is also a doctor check: shown once, below.
            if self.checks.is_none()
                && let Some(e) = &s.status.proxy.error
            {
                out.push((CheckLevel::Warn, format!("Proxy: {e}")));
            }
            let failed_jobs = s
                .jobs
                .iter()
                .filter(|j| j.status == JobStatus::Failed)
                .count();
            if failed_jobs > 0 {
                out.push((
                    CheckLevel::Warn,
                    format!("{failed_jobs} background job(s) failed"),
                ));
            }
            if s.status.unread_notifications > 0 {
                out.push((
                    CheckLevel::Info,
                    format!("{} unread notification(s)", s.status.unread_notifications),
                ));
            }
        }
        for c in self.checks.iter().flatten() {
            if matches!(c.level, CheckLevel::Warn | CheckLevel::Error) {
                out.push((c.level, format!("{}: {}", c.label, c.detail)));
            }
        }
        out
    }
}

fn step(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    current.saturating_add_signed(delta).min(len - 1)
}

/// The CLI's name for a runtime kind (`node`, `php`, `pnpm`…).
pub fn kind_arg(kind: locdev_core::model::RuntimeKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| kind.label().to_lowercase())
}

/// Subsequence match, preferring early and contiguous hits.
fn fuzzy(query: &str, text: &str) -> Option<i32> {
    if let Some(at) = text.find(query) {
        return Some(1000 - at as i32);
    }
    let mut score = 0;
    let mut last = 0usize;
    let mut chars = text.char_indices();
    for q in query.chars() {
        let (i, _) = chars.by_ref().find(|(_, c)| *c == q)?;
        score -= (i - last) as i32;
        last = i;
    }
    Some(score)
}

/// Splits a prompt line into arguments: whitespace separates, single and
/// double quotes group, backslash escapes. No shell is involved.
pub fn split_args(line: &str) -> Result<Vec<String>, String> {
    let mut args = vec![];
    let mut current = String::new();
    let mut in_arg = false;
    let mut quote: Option<char> = None;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') | (None, '\\') => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
                in_arg = true;
            }
            (Some(_), c) => current.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                in_arg = true;
            }
            (None, c) if c.is_whitespace() => {
                if in_arg {
                    args.push(std::mem::take(&mut current));
                    in_arg = false;
                }
            }
            (None, c) => {
                current.push(c);
                in_arg = true;
            }
        }
    }
    if quote.is_some() {
        return Err("unclosed quote".into());
    }
    if in_arg {
        args.push(current);
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_like_a_shell_without_one() {
        assert_eq!(
            split_args(r#"git commit "fix: a b" --yes"#).unwrap(),
            ["git", "commit", "fix: a b", "--yes"]
        );
        assert_eq!(
            split_args("env set 'A B' ''").unwrap(),
            ["env", "set", "A B", ""]
        );
        assert_eq!(split_args(r"a\ b c").unwrap(), ["a b", "c"]);
        assert!(split_args("say \"hi").is_err());
        assert!(split_args("   ").unwrap().is_empty());
    }

    #[test]
    fn fuzzy_prefers_substrings() {
        assert!(fuzzy("stop", "stop acme").unwrap() > fuzzy("stp", "stop acme").unwrap());
        assert!(fuzzy("xyz", "stop acme").is_none());
    }

    #[test]
    fn palette_lists_every_cli_command() {
        let app = App::new(
            Paths::under("/tmp/locdev-tui-test".into()),
            super::super::theme::DARK,
        );
        let commands = app
            .palette("")
            .into_iter()
            .filter(|i| i.hint == "command")
            .count();
        let total: usize = crate::catalog::catalog()
            .categories
            .iter()
            .map(|c| c.commands.len())
            .sum();
        assert_eq!(commands, total);
    }
}
