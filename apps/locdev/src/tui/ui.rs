//! Drawing. Layout follows the app: a chrome sidebar (brand, pages, sites,
//! services, system LEDs), the page in the middle and a tmux-style key bar.

use std::time::Duration;

use locdev_core::deps::{JobStatus, Severity};
use locdev_core::model::{
    CheckLevel, LogLevel, LogStream, ProcessRole, ProcessState, Routing, now_ms,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use super::app::{App, Focus, Overlay, Page, kind_arg};
use super::theme::Theme;

const LED: &str = "●";

pub fn draw(frame: &mut Frame, app: &mut App) {
    let t = app.theme;
    let area = frame.area();
    frame.render_widget(Block::new().style(Style::new().bg(t.bg).fg(t.fg)), area);

    let side_width = (area.width / 4).clamp(30, 40).min(area.width);
    let [side, main] =
        Layout::horizontal([Constraint::Length(side_width), Constraint::Fill(1)]).areas(area);
    sidebar(frame, app, side);

    let [header, body, keys] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(main.inner(ratatui::layout::Margin::new(3, 0)));
    page_header(frame, app, header);
    page(frame, app, body);
    key_bar(frame, app, keys);

    match &app.overlay {
        Some(Overlay::Help) => help(frame, &t, area),
        Some(Overlay::Palette { input, selected }) => {
            let (input, selected) = (input.clone(), *selected);
            palette(frame, app, area, &input, selected)
        }
        Some(Overlay::Prompt { input }) => prompt(frame, &t, area, input),
        Some(Overlay::Confirm { message, .. }) => confirm(frame, &t, area, message),
        Some(Overlay::Filter) | None => {}
    }
}

// ── Sidebar ─────────────────────────────────────────────────────────────────

fn state_color(t: &Theme, state: ProcessState) -> Color {
    match state {
        ProcessState::Running => t.phosphor,
        ProcessState::Stopping => t.amber,
        ProcessState::Failed => t.danger,
        ProcessState::Stopped | ProcessState::Exited => t.dim,
    }
}

fn state_label(state: ProcessState) -> &'static str {
    match state {
        ProcessState::Running => "running",
        ProcessState::Stopping => "stopping",
        ProcessState::Failed => "failed",
        ProcessState::Exited => "exited",
        ProcessState::Stopped => "stopped",
    }
}

/// Pads a line to `width` so its background covers the whole row.
fn full(mut line: Line<'_>, width: u16) -> Line<'_> {
    let used = line.width();
    if used < width as usize {
        line.spans
            .push(Span::raw(" ".repeat(width as usize - used)));
    }
    line
}

/// A row with text on the left and a quiet value flush right.
fn split_row<'a>(left: Vec<Span<'a>>, right: Span<'a>, width: u16) -> Line<'a> {
    let used: usize = left.iter().map(|s| s.width()).sum::<usize>() + right.width();
    let gap = (width as usize).saturating_sub(used).max(1);
    let mut spans = left;
    spans.push(Span::raw(" ".repeat(gap)));
    spans.push(right);
    Line::from(spans)
}

fn sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    frame.render_widget(Block::new().style(Style::new().bg(t.chrome)), area);
    let inner = area.inner(ratatui::layout::Margin::new(2, 1));
    let items = app.side_items();
    let focused = app.focus == Focus::Sidebar;
    let cursor = app.side_cursor;
    let row_style = |i: usize, current: bool| {
        if focused && i == cursor {
            Style::new().bg(t.accent)
        } else if current {
            Style::new().bg(t.card)
        } else {
            Style::new()
        }
    };

    let services = app.instances().len();
    let [brand, nav, sites_area, services_area, system] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(Page::ALL.len() as u16 + 1),
        Constraint::Fill(1),
        Constraint::Length(if services == 0 {
            0
        } else {
            (services as u16 + 2).min(9)
        }),
        Constraint::Length(5),
    ])
    .areas(inner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "locdev",
                Style::new().fg(t.ember).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {}", env!("CARGO_PKG_VERSION")), t.muted()),
        ])),
        brand,
    );

    // Pages: the current one gets the ember bar, like the app's rail.
    let mut lines = vec![];
    for (i, page) in Page::ALL.iter().enumerate() {
        let current = app.page == *page;
        let bar = if current {
            Span::styled("▌", Style::new().fg(t.ember))
        } else {
            Span::raw(" ")
        };
        let text = if current { t.bold() } else { t.text() };
        let mut line = Line::from(vec![
            bar,
            Span::styled(format!(" {} ", page.glyph()), t.muted()),
            Span::styled(format!(" {}", page.label()), text),
        ]);
        if *page == Page::Activity
            && let Some(s) = app.snapshot.as_ref()
            && s.status.unread_notifications > 0
        {
            line.spans.push(Span::styled(
                format!("  {}", s.status.unread_notifications),
                Style::new().fg(t.ember),
            ));
        }
        if *page == Page::Mailbox
            && let Some(s) = app.snapshot.as_ref()
            && s.status.mail.unread > 0
        {
            line.spans.push(Span::styled(
                format!("  {}", s.status.mail.unread),
                Style::new().fg(t.phosphor),
            ));
        }
        lines.push(full(line, nav.width).style(row_style(i, current)));
    }
    frame.render_widget(Paragraph::new(lines), nav);

    // Sites.
    let sites = app.sites();
    let running = sites
        .iter()
        .filter(|p| p.process.state == ProcessState::Running)
        .count();
    let width = sites_area.width;
    let mut lines = vec![split_row(
        vec![Span::styled("SITES", t.heading())],
        Span::styled(format!("{running}/{}", sites.len()), t.muted()),
        width,
    )];
    let first_site = Page::ALL.len();
    let visible = sites_area.height.saturating_sub(1) as usize;
    let offset = scroll_offset(
        cursor.checked_sub(first_site).filter(|i| *i < sites.len()),
        sites.len(),
        visible,
    );
    if sites.is_empty() {
        let hint = if app.filter.is_empty() {
            "no sites yet · a to add"
        } else {
            "no match"
        };
        lines.push(Line::styled(hint, t.dim()));
    }
    for (i, site) in sites.iter().enumerate().skip(offset).take(visible) {
        let current = app.page == Page::Sites
            && app
                .selected_site()
                .is_some_and(|s| s.project.name == site.project.name);
        let name = site.project.domain.clone();
        let framework = truncate(&site.stack.framework, 14);
        let name = truncate(
            &name,
            (width as usize).saturating_sub(framework.chars().count() + 4),
        );
        lines.push(
            split_row(
                vec![
                    Span::styled(LED, Style::new().fg(state_color(&t, site.process.state))),
                    Span::raw(" "),
                    Span::styled(name, t.text()),
                ],
                Span::styled(framework, t.muted()),
                width,
            )
            .style(row_style(first_site + i, current)),
        );
    }
    frame.render_widget(Paragraph::new(lines), sites_area);

    // Services.
    if services > 0 {
        let instances = app.instances();
        let ready = instances.iter().filter(|i| i.ready).count();
        let mut lines = vec![split_row(
            vec![Span::styled("SERVICES", t.heading())],
            Span::styled(format!("{ready}/{services}"), t.muted()),
            width,
        )];
        let first = first_site + sites.len();
        for (i, inst) in instances
            .iter()
            .enumerate()
            .take(services_area.height as usize - 2)
        {
            let color = match (inst.info.state, inst.ready) {
                (ProcessState::Running, true) => t.phosphor,
                (ProcessState::Running, false) => t.amber,
                (state, _) => state_color(&t, state),
            };
            let current = app.page == Page::Services
                && app
                    .selected_instance()
                    .is_some_and(|s| s.instance.name == inst.instance.name);
            lines.push(
                split_row(
                    vec![
                        Span::styled(LED, Style::new().fg(color)),
                        Span::raw(" "),
                        Span::styled(inst.instance.name.clone(), t.text()),
                    ],
                    Span::styled(inst.instance.version.clone(), t.muted()),
                    width,
                )
                .style(row_style(first + i, current)),
            );
        }
        frame.render_widget(Paragraph::new(lines), services_area);
    }

    // System LEDs at the bottom, like the app's status bar.
    let mut lines = vec![];
    for (name, color, value) in system_rows(app) {
        lines.push(split_row(
            vec![
                Span::styled(LED, Style::new().fg(color)),
                Span::raw(" "),
                Span::styled(name, t.text()),
            ],
            Span::styled(value, t.muted()),
            width,
        ));
    }
    frame.render_widget(Paragraph::new(lines), system);
    let _ = items;
}

fn system_rows(app: &App) -> Vec<(&'static str, Color, String)> {
    let t = app.theme;
    let Some(status) = app.status() else {
        return vec![(
            "daemon",
            t.danger,
            if app.offline.is_some() {
                "stopped".into()
            } else {
                "connecting".into()
            },
        )];
    };
    let proxy = match status.proxy.port {
        Some(port) => (t.phosphor, format!(":{port}")),
        None => (t.danger, "down".into()),
    };
    let dns = match (status.dns.port, status.dns.routing) {
        (None, _) => (t.danger, "down".into()),
        (Some(_), Routing::Locdev) => (t.phosphor, format!("resolving .{}", status.proxy.suffix)),
        (Some(_), _) => (t.amber, "not routed".into()),
    };
    let https = match (status.tls.port, status.tls.trusted) {
        (None, _) => (t.dim, "off".into()),
        (Some(_), true) => (t.phosphor, "trusted".into()),
        (Some(_), false) => (t.amber, "untrusted".into()),
    };
    let mail = match status.mail.port {
        Some(port) => (t.phosphor, format!(":{port}")),
        None => (t.dim, "off".into()),
    };
    vec![
        ("proxy", proxy.0, proxy.1),
        ("dns", dns.0, dns.1),
        ("https", https.0, https.1),
        ("mailbox", mail.0, mail.1),
    ]
}

// ── Page chrome ─────────────────────────────────────────────────────────────

fn page_header(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let [_, row, _] = Layout::vertical([Constraint::Length(1); 3]).areas(area);
    let title = match (&app.logs, &app.mail_open) {
        (Some(view), _) => view.title.clone(),
        (_, Some(_)) => "Mailbox · message".into(),
        _ => app.page.label().to_string(),
    };
    let mut left = vec![Span::styled(title, t.bold())];
    if !app.filter.is_empty() || matches!(app.overlay, Some(Overlay::Filter)) {
        left.push(Span::styled("   / ", t.key()));
        left.push(Span::styled(app.filter.clone(), t.text()));
        if matches!(app.overlay, Some(Overlay::Filter)) {
            left.push(Span::styled("▏", Style::new().fg(t.ember)));
        }
    }
    let m = &app.machine;
    let right = Line::from(vec![
        Span::styled("cpu ", t.muted()),
        Span::styled(format!("{:.1}%", m.cpu), t.text()),
        Span::styled("    mem ", t.muted()),
        Span::styled(bytes(m.used), t.text()),
    ]);
    frame.render_widget(Paragraph::new(Line::from(left)), row);
    frame.render_widget(Paragraph::new(right).alignment(Alignment::Right), row);
}

fn key_bar(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let mut spans = vec![];
    match &app.toast {
        Some((message, at)) if at.elapsed() < Duration::from_secs(5) => {
            let (color, text) = match message {
                Ok(m) => (t.phosphor, m.clone()),
                Err(e) => (t.danger, e.clone()),
            };
            spans.push(Span::styled(format!("{LED} "), Style::new().fg(color)));
            spans.push(Span::styled(truncate(&text, 60), t.text()));
        }
        _ => {
            let running = app.running_count();
            let color = if app.offline.is_some() {
                t.danger
            } else if running > 0 {
                t.phosphor
            } else {
                t.dim
            };
            spans.push(Span::styled(format!("{LED} "), Style::new().fg(color)));
            spans.push(Span::styled(
                if app.offline.is_some() {
                    "daemon stopped".to_string()
                } else {
                    format!("{running} running")
                },
                t.muted(),
            ));
        }
    }
    let hints: &[(&str, &str)] = if app.logs.is_some() || app.mail_open.is_some() {
        &[
            ("↑↓", "scroll"),
            ("G", "follow"),
            ("esc", "back"),
            (":", "commands"),
            ("?", "help"),
            ("q", "quit"),
        ]
    } else {
        &[
            ("↑↓", "move"),
            ("enter", "open"),
            ("ctrl+p", "go to or do"),
            ("/", "filter"),
            (
                "tab",
                if app.focus == Focus::Sidebar {
                    "main"
                } else {
                    "sidebar"
                },
            ),
            (":", "commands"),
            ("?", "help"),
            ("q", "quit"),
        ]
    };
    for (key, label) in hints {
        spans.push(Span::raw("   "));
        spans.push(Span::styled(*key, t.key()));
        spans.push(Span::styled(format!(" {label}"), t.muted()));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// A card like the app's sections: raised background, heading, padding.
/// Returns the content area.
fn card(frame: &mut Frame, t: &Theme, area: Rect, title: &str) -> Rect {
    frame.render_widget(Block::new().style(Style::new().bg(t.card)), area);
    if area.height < 3 {
        return Rect::default();
    }
    let title_area = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1);
    frame.render_widget(
        Paragraph::new(Span::styled(title.to_uppercase(), t.heading())),
        title_area,
    );
    Rect::new(
        area.x + 2,
        area.y + 3,
        area.width.saturating_sub(4),
        area.height.saturating_sub(4),
    )
}

fn heading(frame: &mut Frame, t: &Theme, area: Rect, title: &str) {
    frame.render_widget(
        Paragraph::new(Span::styled(title.to_uppercase(), t.heading())),
        area,
    );
}

fn fact<'a>(t: &Theme, label: &'a str, value: impl Into<String>, width: usize) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<width$}"), t.muted()),
        Span::styled(value.into(), t.text()),
    ])
}

fn page(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.logs.is_some() {
        return logs(frame, app, area);
    }
    if let Some((_, text)) = &app.mail_open {
        let t = app.theme;
        let inner = card(frame, &t, area, "message");
        frame.render_widget(
            Paragraph::new(text.clone())
                .style(t.text())
                .wrap(Wrap { trim: false })
                .scroll((app.mail_scroll, 0)),
            inner,
        );
        return;
    }
    match app.page {
        Page::Dashboard => dashboard(frame, app, area),
        Page::Sites => sites(frame, app, area),
        Page::Services => services(frame, app, area),
        Page::Runtimes => runtimes(frame, app, area),
        Page::Processes => processes(frame, app, area),
        Page::Mailbox => mailbox(frame, app, area),
        Page::Activity => activity(frame, app, area),
        Page::Doctor => doctor(frame, app, area),
        Page::About => about(frame, app, area),
    }
}

// ── Pages ───────────────────────────────────────────────────────────────────

fn level_color(t: &Theme, level: CheckLevel) -> Color {
    match level {
        CheckLevel::Ok => t.phosphor,
        CheckLevel::Info => t.signal,
        CheckLevel::Warn => t.amber,
        CheckLevel::Error => t.danger,
    }
}

fn dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let attention = app.attention();
    let rows = attention.len().clamp(1, 6) as u16;
    let [title, needs, _, cards] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(rows),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    heading(frame, &t, title, "Needs attention");
    let lines: Vec<Line> = if attention.is_empty() {
        let text = if app.snapshot.is_none() {
            "Connecting to the daemon…"
        } else {
            "Everything is running"
        };
        vec![Line::from(vec![
            Span::styled(format!(" {LED}  "), Style::new().fg(t.phosphor)),
            Span::styled(text, t.text()),
        ])]
    } else {
        attention
            .iter()
            .take(6)
            .map(|(level, text)| {
                Line::from(vec![
                    Span::styled(
                        format!(" {LED}  "),
                        Style::new().fg(level_color(&t, *level)),
                    ),
                    Span::styled(text.clone(), t.text()),
                ])
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines).style(Style::new().bg(t.card)), needs);

    let [left, right] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
        .spacing(2)
        .areas(cards);
    let [resources, sites_card] = Layout::vertical([Constraint::Length(9), Constraint::Fill(1)])
        .spacing(1)
        .areas(left);
    let [system, recent] = Layout::vertical([Constraint::Length(13), Constraint::Fill(1)])
        .spacing(1)
        .areas(right);

    // Resources: the machine, and what LocDev's processes take of it.
    let inner = card(frame, &t, resources, "Resources");
    let m = &app.machine;
    let (cpu, mem) = app.snapshot.as_ref().map_or((0.0, 0), |s| {
        (
            s.processes.iter().filter_map(|p| p.cpu).sum::<f32>(),
            s.processes.iter().filter_map(|p| p.memory).sum::<u64>(),
        )
    });
    let bar_width = (inner.width as usize).saturating_sub(30).clamp(6, 40);
    let lines = vec![
        fact(&t, "cpu", format!("{:.1}%", m.cpu), 10),
        Line::from(vec![
            Span::styled(format!("{:<10}", "memory"), t.muted()),
            bar(&t, m.used as f64 / m.total.max(1) as f64, bar_width),
            Span::styled(
                format!("  {} / {}", bytes(m.used), bytes(m.total)),
                t.text(),
            ),
        ]),
        Line::raw(""),
        fact(
            &t,
            "locdev",
            if mem == 0 {
                "nothing running".to_string()
            } else {
                format!("{:.1}% cpu · {}", cpu.max(0.0), bytes(mem))
            },
            10,
        ),
    ];
    frame.render_widget(Paragraph::new(lines), inner);

    // Sites at a glance.
    let inner = card(frame, &t, sites_card, "Sites");
    let width = inner.width;
    let lines: Vec<Line> = app
        .snapshot
        .iter()
        .flat_map(|s| &s.projects)
        .take(inner.height as usize)
        .map(|p| {
            split_row(
                vec![
                    Span::styled(LED, Style::new().fg(state_color(&t, p.process.state))),
                    Span::raw(" "),
                    Span::styled(p.project.domain.clone(), t.text()),
                ],
                Span::styled(
                    p.https_url
                        .clone()
                        .or(p.url.clone())
                        .unwrap_or_else(|| state_label(p.process.state).into()),
                    t.muted(),
                ),
                width,
            )
        })
        .collect();
    let lines = if lines.is_empty() {
        vec![Line::styled(
            "No sites yet. Add one with `: add <folder>`.",
            t.dim(),
        )]
    } else {
        lines
    };
    frame.render_widget(Paragraph::new(lines), inner);

    // System.
    let inner = card(frame, &t, system, "System");
    let w = 11;
    let mut lines = vec![];
    match app.status() {
        Some(s) => {
            for (name, color, value) in system_rows(app) {
                lines.push(Line::from(vec![
                    Span::styled(format!("{name:<w$}"), t.muted()),
                    Span::styled(format!("{LED} "), Style::new().fg(color)),
                    Span::styled(value, t.text()),
                ]));
            }
            let workers = s.running;
            lines.push(fact(
                &t,
                "sites",
                format!("{workers} running of {}", s.projects),
                w,
            ));
            if let Some(snap) = &app.snapshot {
                let ready = snap.instances.iter().filter(|i| i.ready).count();
                lines.push(fact(
                    &t,
                    "services",
                    format!("{ready} ready of {}", snap.instances.len()),
                    w,
                ));
            }
            lines.push(fact(
                &t,
                "autostart",
                match app.startup {
                    Some(true) => "on",
                    Some(false) => "off",
                    None => "unsupported",
                },
                w,
            ));
            lines.push(fact(
                &t,
                "daemon",
                format!("{} · pid {}", s.version, s.pid),
                w,
            ));
            lines.push(fact(
                &t,
                "platform",
                format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
                w,
            ));
        }
        None => lines.push(Line::styled(
            app.offline.clone().unwrap_or_else(|| "connecting…".into()),
            Style::new().fg(t.danger),
        )),
    }
    frame.render_widget(Paragraph::new(lines), inner);

    // Recent: notifications, newest first.
    let inner = card(frame, &t, recent, "Recent");
    let notes = app.notifications();
    let lines: Vec<Line> = if notes.is_empty() {
        vec![Line::styled("Nothing new since the TUI opened", t.dim())]
    } else {
        notes
            .iter()
            .take(inner.height as usize)
            .map(|n| {
                split_row(
                    vec![
                        Span::styled(LED, Style::new().fg(severity_color(&t, n.severity, n.read))),
                        Span::raw(" "),
                        Span::styled(
                            truncate(
                                &n.title,
                                (inner.width as usize).saturating_sub(ago(n.created_at).len() + 4),
                            ),
                            t.text(),
                        ),
                    ],
                    Span::styled(ago(n.created_at), t.muted()),
                    inner.width,
                )
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines), inner);
}

fn severity_color(t: &Theme, severity: Severity, read: bool) -> Color {
    if read {
        return t.dim;
    }
    match severity {
        Severity::Info => t.signal,
        Severity::Warn => t.amber,
        Severity::Error => t.danger,
    }
}

/// A table-ish list: header + rows, the selected one highlighted.
fn table(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    header: &[(&str, usize)],
    rows: Vec<Vec<Span<'static>>>,
    selected: usize,
    empty: &str,
) {
    let t = app.theme;
    let focused = app.focus == Focus::Main;
    let mut lines = vec![];
    let mut head = vec![Span::raw("   ")];
    for (title, width) in header {
        head.push(Span::styled(
            format!("{:<width$}", title.to_uppercase()),
            t.heading(),
        ));
    }
    lines.push(Line::from(head));
    if rows.is_empty() {
        lines.push(Line::styled(format!("   {empty}"), t.dim()));
    }
    let visible = area.height.saturating_sub(1) as usize;
    let offset = scroll_offset(Some(selected), rows.len(), visible);
    for (i, row) in rows.into_iter().enumerate().skip(offset).take(visible) {
        let mut spans = vec![if i == selected {
            Span::styled(
                " ▌ ",
                Style::new().fg(if focused { t.ember } else { t.dim }),
            )
        } else {
            Span::raw("   ")
        }];
        for (span, (_, width)) in row
            .into_iter()
            .zip(header.iter().chain(std::iter::repeat(&("", 0))))
        {
            let text = span.content.to_string();
            let fitted = if *width == 0 {
                text
            } else {
                format!("{:<width$}", truncate(&text, width.saturating_sub(1)))
            };
            spans.push(Span::styled(fitted, span.style));
        }
        let mut line = Line::from(spans);
        if i == selected {
            line = full(line, area.width).style(Style::new().bg(if focused {
                t.accent
            } else {
                t.card
            }));
        }
        lines.push(line);
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn led_span(t: &Theme, color: Color, label: &str) -> Span<'static> {
    let _ = t;
    Span::styled(format!("{LED} {label}"), Style::new().fg(color))
}

fn sites(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let list = app.sites();
    let height = (list.len() as u16 + 2).clamp(3, area.height / 2);
    let [top, _, bottom] = Layout::vertical([
        Constraint::Length(height),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    let rows = list
        .iter()
        .map(|p| {
            vec![
                Span::styled(p.project.domain.clone(), t.text()),
                led_span(
                    &t,
                    state_color(&t, p.process.state),
                    state_label(p.process.state),
                ),
                Span::styled(p.stack.framework.clone(), t.muted()),
                Span::styled(
                    p.runtimes
                        .iter()
                        .map(|r| {
                            format!(
                                "{} {}",
                                kind_arg(r.kind),
                                r.version.as_ref().map_or("?", |v| v.version.as_str())
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" · "),
                    t.muted(),
                ),
                Span::styled(
                    p.https_url.clone().or(p.url.clone()).unwrap_or_default(),
                    t.muted(),
                ),
            ]
        })
        .collect();
    table(
        frame,
        app,
        top,
        &[
            ("site", 24),
            ("state", 12),
            ("stack", 18),
            ("runtimes", 26),
            ("address", 0),
        ],
        rows,
        app.row(Page::Sites),
        "No sites yet. Press a (or `: add <folder>`) to add one.",
    );

    let Some(site) = app.selected_site() else {
        return;
    };
    let [info, out] = Layout::horizontal([Constraint::Length(48), Constraint::Fill(1)])
        .spacing(2)
        .areas(bottom);
    let inner = card(frame, &t, info, &site.project.name);
    let w = 10;
    let mut lines = vec![
        fact(&t, "state", process_line(&site.process), w),
        fact(
            &t,
            "address",
            site.https_url
                .clone()
                .or(site.url.clone())
                .unwrap_or("—".into()),
            w,
        ),
        fact(
            &t,
            "direct",
            site.direct_url.clone().unwrap_or("—".into()),
            w,
        ),
        fact(&t, "folder", site.project.path.display().to_string(), w),
        fact(&t, "stack", site.stack.framework.clone(), w),
        fact(
            &t,
            "command",
            site.project
                .command
                .clone()
                .or(site.stack.command.clone())
                .unwrap_or("—".into()),
            w,
        ),
    ];
    if let Some(dev) = &site.project.dev_server {
        let state = site
            .dev_process
            .as_ref()
            .map(process_line)
            .unwrap_or_else(|| "stopped".into());
        lines.push(fact(
            &t,
            "helper",
            format!("{} · {:?} · {state}", dev.command, dev.mode).to_lowercase(),
            w,
        ));
    }
    for r in &site.runtimes {
        lines.push(fact(
            &t,
            "runtime",
            format!(
                "{} {}",
                r.kind.label(),
                r.version
                    .as_ref()
                    .map_or("not found", |v| v.version.as_str())
            ),
            w,
        ));
    }
    lines.push(Line::raw(""));
    let mut keys = vec![
        ("s", "start/stop"),
        ("R", "restart"),
        ("o", "browser"),
        ("l", "logs"),
        ("i", "info"),
    ];
    if site.project.dev_server.is_some() {
        keys.push(("D", "helper"));
        keys.push(("d", "helper logs"));
    }
    keys.push(("x", "remove"));
    lines.extend(key_lines(&t, &keys, inner.width));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    let inner = card(frame, &t, out, "Output");
    let source = super::LogSource::Site {
        name: site.project.name.clone(),
        role: ProcessRole::Main,
        aux: None,
    };
    log_tail(
        frame,
        app,
        inner,
        app.log_lines
            .get(&source)
            .map(Vec::as_slice)
            .unwrap_or_default(),
        0,
    );
}

fn key_lines<'a>(t: &Theme, keys: &[(&'a str, &'a str)], width: u16) -> Vec<Line<'a>> {
    let mut lines = vec![];
    let mut spans: Vec<Span> = vec![];
    let mut used = 0;
    for (key, label) in keys {
        let w = key.len() + label.len() + 3;
        if used + w > width as usize && !spans.is_empty() {
            lines.push(Line::from(std::mem::take(&mut spans)));
            used = 0;
        }
        spans.push(Span::styled(*key, t.key()));
        spans.push(Span::styled(format!(" {label}   "), t.muted()));
        used += w + 2;
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}

fn process_line(info: &locdev_core::model::ProcessInfo) -> String {
    let mut text = state_label(info.state).to_string();
    if info.state == ProcessState::Running {
        if let Some(pid) = info.pid {
            text.push_str(&format!(" · pid {pid}"));
        }
        if let Some(at) = info.started_at {
            text.push_str(&format!(" · up {}", duration(now_ms().saturating_sub(at))));
        }
    } else if let Some(code) = info.exit_code {
        text.push_str(&format!(" · exit {code}"));
    }
    if let Some(m) = &info.message
        && info.state != ProcessState::Running
    {
        text.push_str(&format!(" · {m}"));
    }
    text
}

fn services(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let list = app.instances();
    let height = (list.len() as u16 + 2).clamp(3, area.height / 2);
    let [top, _, bottom] = Layout::vertical([
        Constraint::Length(height),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    let rows = list
        .iter()
        .map(|i| {
            let (color, label) = match (i.info.state, i.ready) {
                (ProcessState::Running, true) => (t.phosphor, "ready"),
                (ProcessState::Running, false) => (t.amber, "starting"),
                (state, _) => (state_color(&t, state), state_label(state)),
            };
            vec![
                Span::styled(i.instance.name.clone(), t.text()),
                led_span(&t, color, label),
                Span::styled(i.instance.kind.label().to_string(), t.muted()),
                Span::styled(i.instance.version.clone(), t.muted()),
                Span::styled(format!("127.0.0.1:{}", i.instance.port), t.muted()),
                Span::styled(
                    if i.instance.autostart {
                        "autostart"
                    } else {
                        ""
                    }
                    .to_string(),
                    t.muted(),
                ),
            ]
        })
        .collect();
    table(
        frame,
        app,
        top,
        &[
            ("instance", 20),
            ("state", 12),
            ("kind", 14),
            ("version", 10),
            ("address", 18),
            ("", 0),
        ],
        rows,
        app.row(Page::Services),
        "No instances. Create one with `: service create <kind>`.",
    );
    let Some(inst) = app.selected_instance() else {
        return;
    };
    let [info, out] = Layout::horizontal([Constraint::Length(56), Constraint::Fill(1)])
        .spacing(2)
        .areas(bottom);
    let inner = card(frame, &t, info, &inst.instance.name);
    let mut lines = vec![
        fact(&t, "state", process_line(&inst.info), 10),
        fact(&t, "data", inst.data_dir.display().to_string(), 10),
        Line::raw(""),
    ];
    for (key, value) in inst.connection.iter().take(6) {
        lines.push(Line::from(vec![
            Span::styled(format!("{key} "), t.muted()),
            Span::styled(value.clone(), Style::new().fg(t.signal)),
        ]));
    }
    lines.push(Line::raw(""));
    lines.extend(key_lines(
        &t,
        &[
            ("s", "start/stop"),
            ("R", "restart"),
            ("l", "logs"),
            ("b", "back up"),
        ],
        inner.width,
    ));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    let inner = card(frame, &t, out, "Output");
    let source = super::LogSource::Instance(inst.instance.name.clone());
    log_tail(
        frame,
        app,
        inner,
        app.log_lines
            .get(&source)
            .map(Vec::as_slice)
            .unwrap_or_default(),
        0,
    );
}

fn runtimes(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let Some(list) = &app.runtimes else {
        frame.render_widget(
            Paragraph::new(Span::styled("Detecting runtimes…", t.dim())),
            area,
        );
        return;
    };
    let selected = app.row(Page::Runtimes);
    let focused = app.focus == Focus::Main;
    let mut lines = vec![];
    for (i, r) in list.iter().enumerate() {
        let mut head = vec![
            if i == selected {
                Span::styled("▌ ", Style::new().fg(if focused { t.ember } else { t.dim }))
            } else {
                Span::raw("  ")
            },
            Span::styled(format!("{:<12}", r.kind.label()), t.bold()),
            Span::styled("default ", t.muted()),
            Span::styled(
                r.default.clone().unwrap_or_else(|| "system".into()),
                t.text(),
            ),
        ];
        if let Some(job) = &r.job {
            head.push(Span::styled(
                format!("   installing {} …", job.version),
                Style::new().fg(t.amber),
            ));
        }
        if !r.updates.is_empty() {
            head.push(Span::styled(
                format!("   {} update(s)", r.updates.len()),
                Style::new().fg(t.ember),
            ));
        }
        let mut line = Line::from(head);
        if i == selected {
            line = full(line, area.width).style(Style::new().bg(if focused {
                t.accent
            } else {
                t.card
            }));
        }
        lines.push(line);
        if r.versions.is_empty() {
            lines.push(Line::styled("    none found", t.dim()));
        }
        for v in &r.versions {
            let users = r.usage.get(&v.version).map(Vec::len).unwrap_or(0);
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(format!("{:<14}", v.version), t.text()),
                Span::styled(format!("{:<10}", v.origin), t.muted()),
                Span::styled(
                    v.support
                        .as_ref()
                        .map(|s| format!("{:<14}", format!("{:?}", s.state).to_lowercase()))
                        .unwrap_or_else(|| " ".repeat(14)),
                    t.muted(),
                ),
                Span::styled(
                    if users > 0 {
                        format!("{users} site(s)")
                    } else {
                        String::new()
                    },
                    t.dim(),
                ),
            ]));
        }
        lines.push(Line::raw(""));
    }
    lines.extend(key_lines(
        &t,
        &[
            ("enter", "install a version"),
            ("d", "set default"),
            ("u", "update"),
            ("a", "available"),
        ],
        area.width,
    ));
    // Keep the selected runtime in view.
    let before: usize = list
        .iter()
        .take(selected)
        .map(|r| r.versions.len().max(1) + 2)
        .sum();
    let scroll = before.saturating_sub(area.height as usize / 3) as u16;
    frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
}

fn processes(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let rows = app
        .processes()
        .iter()
        .map(|p| {
            let name = match (&p.aux, p.role) {
                (Some(aux), _) => format!("{} · {aux}", p.project),
                (None, ProcessRole::Main) => p.project.clone(),
                (None, ProcessRole::Dev) => format!("{} · helper", p.project),
            };
            let running = p.info.state == ProcessState::Running;
            vec![
                Span::styled(name, t.text()),
                led_span(&t, state_color(&t, p.info.state), state_label(p.info.state)),
                Span::styled(p.label.clone(), t.muted()),
                Span::styled(
                    p.info
                        .pid
                        .filter(|_| running)
                        .map(|v| v.to_string())
                        .unwrap_or("-".into()),
                    t.muted(),
                ),
                Span::styled(
                    p.port.map(|v| v.to_string()).unwrap_or("-".into()),
                    t.muted(),
                ),
                Span::styled(
                    p.cpu.map(|v| format!("{v:.1}%")).unwrap_or("-".into()),
                    t.text(),
                ),
                Span::styled(p.memory.map(bytes).unwrap_or("-".into()), t.text()),
                Span::styled(
                    match (running, p.info.started_at) {
                        (true, Some(at)) => duration(now_ms().saturating_sub(at)),
                        _ => String::new(),
                    },
                    t.muted(),
                ),
            ]
        })
        .collect();
    let [list, keys] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    table(
        frame,
        app,
        list,
        &[
            ("process", 26),
            ("state", 12),
            ("program", 12),
            ("pid", 9),
            ("port", 7),
            ("cpu", 8),
            ("memory", 10),
            ("uptime", 0),
        ],
        rows,
        app.row(Page::Processes),
        "Nothing is running.",
    );
    frame.render_widget(
        Paragraph::new(key_lines(
            &t,
            &[("enter", "logs"), ("s", "start/stop"), ("R", "restart")],
            keys.width,
        )),
        keys,
    );
}

fn mailbox(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    if app.mail.is_none() {
        frame.render_widget(Paragraph::new(Span::styled("Loading…", t.dim())), area);
        return;
    }
    let rows = app
        .messages()
        .iter()
        .map(|m| {
            vec![
                Span::styled(
                    if m.read { " " } else { LED }.to_string(),
                    Style::new().fg(t.phosphor),
                ),
                Span::styled(m.subject.clone(), if m.read { t.text() } else { t.bold() }),
                Span::styled(m.from.clone(), t.muted()),
                Span::styled(m.site.clone().unwrap_or_default(), t.muted()),
                Span::styled(ago(m.received_at), t.muted()),
            ]
        })
        .collect();
    let [list, keys] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    let port = app
        .status()
        .and_then(|s| s.mail.port)
        .map(|p| format!("Captured on 127.0.0.1:{p}; nothing is delivered."))
        .unwrap_or_else(|| "The capture server is off.".into());
    table(
        frame,
        app,
        list,
        &[
            ("", 2),
            ("subject", 40),
            ("from", 28),
            ("site", 16),
            ("received", 0),
        ],
        rows,
        app.row(Page::Mailbox),
        &format!("No messages. {port}"),
    );
    frame.render_widget(
        Paragraph::new(key_lines(
            &t,
            &[("enter", "read"), ("x", "delete"), ("r", "refresh")],
            keys.width,
        )),
        keys,
    );
}

fn activity(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let notes = app.notifications();
    let jobs = app.jobs();
    let selected = app.row(Page::Activity);
    let split = (notes.len() as u16 + 3).clamp(4, area.height / 2);
    let [top, bottom, keys] = Layout::vertical([
        Constraint::Length(split),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);
    let rows = notes
        .iter()
        .map(|n| {
            vec![
                Span::styled(
                    LED.to_string(),
                    Style::new().fg(severity_color(&t, n.severity, n.read)),
                ),
                Span::styled(n.title.clone(), if n.read { t.text() } else { t.bold() }),
                Span::styled(n.project.clone().unwrap_or_default(), t.muted()),
                Span::styled(ago(n.created_at), t.muted()),
            ]
        })
        .collect();
    table(
        frame,
        app,
        top,
        &[("", 2), ("notification", 56), ("site", 18), ("when", 0)],
        rows,
        if selected < notes.len() {
            selected
        } else {
            usize::MAX
        },
        "No notifications.",
    );
    let rows = jobs
        .iter()
        .map(|j| {
            let color = match j.status {
                JobStatus::Running => t.signal,
                JobStatus::Completed => t.phosphor,
                JobStatus::Failed => t.danger,
                JobStatus::Pending | JobStatus::Paused => t.amber,
                JobStatus::Cancelled => t.dim,
            };
            let progress = j
                .progress
                .as_ref()
                .map(|p| match p.total {
                    Some(total) => format!("{} {}/{total}", p.label, p.done),
                    None => p.label.clone(),
                })
                .or(j.error.clone())
                .or(j.result.clone())
                .unwrap_or_default();
            vec![
                Span::styled(format!("#{}", j.id), t.muted()),
                Span::styled(j.kind.label().to_string(), t.text()),
                led_span(&t, color, &format!("{:?}", j.status).to_lowercase()),
                Span::styled(j.project.clone().unwrap_or_default(), t.muted()),
                Span::styled(progress, t.muted()),
            ]
        })
        .collect();
    table(
        frame,
        app,
        bottom,
        &[
            ("job", 7),
            ("kind", 28),
            ("status", 14),
            ("site", 18),
            ("", 0),
        ],
        rows,
        selected.checked_sub(notes.len()).unwrap_or(usize::MAX),
        "No background jobs.",
    );
    let hints: &[(&str, &str)] = if selected < notes.len() {
        &[
            ("enter", "mark read"),
            ("x", "dismiss"),
            ("a", "mark all read"),
        ]
    } else {
        &[
            ("enter", "details"),
            ("c", "cancel"),
            ("p", "pause/resume"),
            ("t", "retry"),
        ]
    };
    frame.render_widget(Paragraph::new(key_lines(&t, hints, keys.width)), keys);
}

fn doctor(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let Some(checks) = &app.checks else {
        frame.render_widget(Paragraph::new(Span::styled("Checking…", t.dim())), area);
        return;
    };
    let selected = app.row(Page::Doctor);
    let focused = app.focus == Focus::Main;
    let mut lines = vec![];
    for (i, c) in checks.iter().enumerate() {
        let mut line = Line::from(vec![
            if i == selected {
                Span::styled("▌ ", Style::new().fg(if focused { t.ember } else { t.dim }))
            } else {
                Span::raw("  ")
            },
            Span::styled(format!("{LED} "), Style::new().fg(level_color(&t, c.level))),
            Span::styled(format!("{:<22}", c.label), t.text()),
            Span::styled(c.detail.clone(), t.muted()),
        ]);
        if i == selected {
            line = full(line, area.width).style(Style::new().bg(if focused {
                t.accent
            } else {
                t.card
            }));
        }
        lines.push(line);
        if let Some(remedy) = &c.remedy {
            lines.push(Line::from(vec![
                Span::raw("                           "),
                Span::styled("→ ", Style::new().fg(t.ember)),
                Span::styled(remedy.clone(), Style::new().fg(t.signal)),
            ]));
        }
    }
    lines.push(Line::raw(""));
    lines.extend(key_lines(
        &t,
        &[("enter", "run the fix"), ("r", "check again")],
        area.width,
    ));
    let before: usize = checks
        .iter()
        .take(selected)
        .map(|c| 1 + c.remedy.is_some() as usize)
        .sum();
    let scroll = before.saturating_sub(area.height as usize / 2) as u16;
    frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
}

fn about(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let [left, right] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
        .spacing(2)
        .areas(area);
    let [versions, links] = Layout::vertical([Constraint::Length(12), Constraint::Fill(1)])
        .spacing(1)
        .areas(left);
    let inner = card(frame, &t, versions, "Versions");
    let w = 14;
    let protocol = locdev_core::protocol::PROTOCOL_VERSION;
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("{:<w$}", "CLI"), t.muted()),
            Span::styled(
                env!("CARGO_PKG_VERSION"),
                Style::new().fg(t.ember).add_modifier(Modifier::BOLD),
            ),
        ]),
        fact(
            &t,
            "daemon",
            app.status()
                .map_or("not running".into(), |s| s.version.clone()),
            w,
        ),
        fact(
            &t,
            "IPC protocol",
            match app.status() {
                Some(s) if s.protocol != protocol => format!("{protocol} (daemon: {})", s.protocol),
                _ => protocol.to_string(),
            },
            w,
        ),
        fact(
            &t,
            "platform",
            format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
            w,
        ),
        fact(
            &t,
            "built for",
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            },
            w,
        ),
    ];
    if let Some(s) = app.status()
        && s.version != env!("CARGO_PKG_VERSION")
    {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "The CLI and the daemon differ: `locdev restart` runs the daemon next to this CLI.",
            Style::new().fg(t.amber),
        ));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    let inner = card(frame, &t, links, "LocDev");
    let lines = vec![
        Line::styled(
            "Local development environment: sites, runtimes, services,",
            t.text(),
        ),
        Line::styled(
            "HTTPS and mail on your machine. No account required.",
            t.text(),
        ),
        Line::raw(""),
        fact(&t, "homepage", "https://locdev.app", 10),
        fact(&t, "license", "Apache-2.0", 10),
        Line::raw(""),
        Line::styled(
            "The app and this TUI talk to the same daemon; every",
            t.muted(),
        ),
        Line::styled(
            "screen of the app has a `locdev` command (: or ctrl+p).",
            t.muted(),
        ),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    let inner = card(frame, &t, right, "Files");
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or("?".into());
    let daemon = locdev_ipc::launcher::daemon_executable()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "not found".into());
    let p = &app.paths;
    let w = 10;
    let lines = vec![
        fact(&t, "cli", exe, w),
        fact(&t, "daemon", daemon, w),
        Line::raw(""),
        fact(&t, "config", p.config_dir.display().to_string(), w),
        fact(&t, "data", p.data_dir.display().to_string(), w),
        fact(&t, "logs", p.log_dir.display().to_string(), w),
        fact(&t, "runtime", p.runtime_dir.display().to_string(), w),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ── Logs ────────────────────────────────────────────────────────────────────

fn logs(frame: &mut Frame, app: &App, area: Rect) {
    let Some(view) = &app.logs else {
        return;
    };
    let t = app.theme;
    frame.render_widget(Block::new().style(Style::new().bg(t.card)), area);
    let inner = area.inner(ratatui::layout::Margin::new(2, 1));
    let lines = app
        .log_lines
        .get(&view.source)
        .map(Vec::as_slice)
        .unwrap_or_default();
    log_tail(frame, app, inner, lines, view.scroll);
}

fn log_tail(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    lines: &[locdev_core::model::LogLine],
    scroll: usize,
) {
    let t = app.theme;
    if lines.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No output yet.", t.dim())),
            area,
        );
        return;
    }
    let height = area.height as usize;
    let end = lines
        .len()
        .saturating_sub(scroll.min(lines.len().saturating_sub(1)));
    let start = end.saturating_sub(height);
    let text: Vec<Line> = lines[start..end]
        .iter()
        .map(|l| {
            let color = match (l.level, l.stream) {
                (Some(LogLevel::Error), _) => t.danger,
                (Some(LogLevel::Warn), _) => t.amber,
                (_, LogStream::System) => t.signal,
                _ => t.fg,
            };
            Line::from(Span::styled(strip_ansi(&l.text), Style::new().fg(color)))
        })
        .collect();
    frame.render_widget(Paragraph::new(Text::from(text)), area);
}

/// Dev servers color their output; the escapes would garble the screen.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if c == '\t' {
            out.push_str("    ");
        } else if !c.is_control() {
            out.push(c);
        }
    }
    out
}

// ── Overlays ────────────────────────────────────────────────────────────────

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(4));
    let height = height.min(area.height.saturating_sub(2));
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 3,
        width,
        height,
    )
}

fn panel(frame: &mut Frame, t: &Theme, area: Rect, title: &str) -> Rect {
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::bordered()
            .border_style(Style::new().fg(t.border))
            .style(Style::new().bg(t.card).fg(t.fg))
            .title(Span::styled(format!(" {title} "), t.heading())),
        area,
    );
    area.inner(ratatui::layout::Margin::new(2, 1))
}

fn help(frame: &mut Frame, t: &Theme, area: Rect) {
    let rows: &[(&str, &str)] = &[
        ("↑↓ j k", "move · scroll output"),
        ("tab", "switch between the sidebar and the page"),
        ("enter", "open the page, the site, the log, the message"),
        ("1 … 9", "jump to a page"),
        (
            "ctrl+p",
            "go to or do: pages, sites, actions, every command",
        ),
        (":", "run any locdev command here (git, db, env, deploy…)"),
        ("/", "filter lists; esc clears"),
        ("r", "refresh"),
        ("s · R", "start/stop · restart the selected site or service"),
        ("o", "open the selected site in the browser"),
        ("l · d", "output of the site · of its helper dev server"),
        ("S", "start the daemon when it is stopped"),
        ("esc", "back"),
        ("q", "quit (the daemon and your sites keep running)"),
    ];
    let inner = panel(frame, t, centered(area, 72, rows.len() as u16 + 6), "keys");
    let mut lines: Vec<Line> = rows
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("{k:<10}"), t.key()),
                Span::styled(*v, t.text()),
            ])
        })
        .collect();
    lines.push(Line::raw(""));
    lines.push(Line::styled("Any key closes this.", t.dim()));
    frame.render_widget(Paragraph::new(lines), inner);
}

fn palette(frame: &mut Frame, app: &App, area: Rect, input: &str, selected: usize) {
    let t = app.theme;
    let items = app.palette(input);
    let inner = panel(frame, &t, centered(area, 90, 22), "go to or do");
    let [field, _, list] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(inner);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("› ", t.key()),
            Span::styled(input.to_string(), t.text()),
            Span::styled("▏", Style::new().fg(t.ember)),
        ])),
        field,
    );
    let visible = list.height as usize;
    let offset = scroll_offset(Some(selected), items.len(), visible);
    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, item)| {
            let line = split_row(
                vec![Span::styled(
                    truncate(&item.label, list.width as usize - 12),
                    t.text(),
                )],
                Span::styled(item.hint.clone(), t.dim()),
                list.width,
            );
            if i == selected {
                line.style(Style::new().bg(t.accent))
            } else {
                line
            }
        })
        .collect();
    let lines = if lines.is_empty() {
        vec![Line::styled(
            "Nothing matches. Try `:` to type a command.",
            t.dim(),
        )]
    } else {
        lines
    };
    frame.render_widget(Paragraph::new(lines), list);
}

fn prompt(frame: &mut Frame, t: &Theme, area: Rect, input: &str) {
    let inner = panel(frame, t, centered(area, 90, 6), "command");
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("locdev ", t.muted()),
                Span::styled(input.to_string(), t.text()),
                Span::styled("▏", Style::new().fg(t.ember)),
            ]),
            Line::raw(""),
            Line::styled(
                "enter runs it in this terminal (confirmations included) · esc cancels · ctrl+p lists commands",
                t.dim(),
            ),
        ]),
        inner,
    );
}

fn confirm(frame: &mut Frame, t: &Theme, area: Rect, message: &str) {
    let inner = panel(frame, t, centered(area, 64, 7), "confirm");
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(message.to_string(), t.text()),
            Line::raw(""),
            Line::from(vec![
                Span::styled("y", t.key()),
                Span::styled(" yes   ", t.muted()),
                Span::styled("n", t.key()),
                Span::styled(" no", t.muted()),
            ]),
        ])
        .wrap(Wrap { trim: false }),
        inner,
    );
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn bar(t: &Theme, ratio: f64, width: usize) -> Span<'static> {
    let filled = ((ratio.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    let color = if ratio > 0.9 {
        t.danger
    } else if ratio > 0.75 {
        t.amber
    } else {
        t.phosphor
    };
    let _ = color;
    Span::styled(
        format!("{}{}", "━".repeat(filled), "─".repeat(width - filled)),
        Style::new().fg(color),
    )
}

fn scroll_offset(selected: Option<usize>, len: usize, visible: usize) -> usize {
    match selected {
        Some(i) if visible > 0 && i < len && i >= visible => i + 1 - visible,
        _ => 0,
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn bytes(n: u64) -> String {
    let n = n as f64;
    if n >= 1e9 {
        format!("{:.1}GB", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.0}MB", n / 1e6)
    } else {
        format!("{:.0}KB", n / 1e3)
    }
}

fn duration(ms: u64) -> String {
    let s = ms / 1000;
    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m", s / 60),
        3600..86400 => format!("{}h{:02}m", s / 3600, s % 3600 / 60),
        _ => format!("{}d{}h", s / 86400, s % 86400 / 3600),
    }
}

fn ago(at: u64) -> String {
    format!("{} ago", duration(now_ms().saturating_sub(at)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_terminal_colors() {
        assert_eq!(
            strip_ansi("\x1b[32m  VITE\x1b[0m ready\tin"),
            "  VITE ready    in"
        );
    }

    #[test]
    fn keeps_the_selection_visible() {
        assert_eq!(scroll_offset(Some(2), 10, 5), 0);
        assert_eq!(scroll_offset(Some(7), 10, 5), 3);
        assert_eq!(scroll_offset(None, 10, 5), 0);
    }
}
