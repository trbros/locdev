//! Rendering for `locdev mail …`.

use locdev_core::mail::{MailDetail, MailSetup, MailSummary, MailWiring, MailboxStatus};

use super::{Output, Style, truncate};

fn when(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970 (Howard Hinnant's algorithm), UTC.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60
    )
}

fn size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1048576.0)
    }
}

impl Output {
    pub fn mailbox_status(&self, s: &MailboxStatus) {
        if self.json {
            self.print_json(s);
            return;
        }
        match (s.port, &s.error) {
            (Some(port), _) => println!(
                "SMTP 127.0.0.1:{port} (capture only)  {} messages, {} unread",
                s.messages, s.unread
            ),
            (None, Some(error)) => println!(
                "{}",
                self.paint(Style::Red, &format!("not running: {error}"))
            ),
            (None, None) => println!("off ([mail] enabled = false in config.toml)"),
        }
    }

    pub fn mail_list(&self, list: &[MailSummary]) {
        if self.json {
            self.print_json(list);
            return;
        }
        if list.is_empty() {
            println!(
                "No messages. `locdev mail enable <site>` points a site's mail here; `locdev mail test` sends one."
            );
        }
        for m in list {
            let unread = if m.read { " " } else { "●" };
            let clip = if m.attachments > 0 {
                format!(" 📎{}", m.attachments)
            } else {
                String::new()
            };
            println!(
                "{} {:<14} {:<12} {:<28} {}{}",
                self.paint(Style::Cyan, unread),
                m.id,
                truncate(m.site.as_deref().unwrap_or("—"), 12),
                truncate(&m.to.join(", "), 28),
                self.paint(
                    Style::Bold,
                    &truncate(
                        if m.subject.is_empty() {
                            "(no subject)"
                        } else {
                            &m.subject
                        },
                        60
                    )
                ),
                clip
            );
        }
    }

    pub fn mail_detail(&self, d: &MailDetail, html: bool, headers: bool) {
        if self.json {
            self.print_json(d);
            return;
        }
        if html {
            print!("{}", d.html.as_deref().unwrap_or(""));
            return;
        }
        let s = &d.summary;
        println!(
            "{}",
            self.paint(
                Style::Bold,
                if s.subject.is_empty() {
                    "(no subject)"
                } else {
                    &s.subject
                }
            )
        );
        println!("From     {}", s.from);
        println!("To       {}", s.to.join(", "));
        if !d.cc.is_empty() {
            println!("Cc       {}", d.cc.join(", "));
        }
        if let Some(date) = s.date {
            println!("Date     {}", when(date));
        }
        println!("Received {}  {}", when(s.received_at), size(s.size));
        let e = &d.envelope;
        println!(
            "{}",
            self.paint(
                Style::Dim,
                &format!(
                    "Envelope MAIL FROM <{}> RCPT TO {} · {} from {} (HELO {}){}",
                    e.from,
                    e.to.iter()
                        .map(|t| format!("<{t}>"))
                        .collect::<Vec<_>>()
                        .join(" "),
                    e.protocol,
                    e.peer,
                    e.helo.as_deref().unwrap_or("—"),
                    e.auth_user
                        .as_deref()
                        .map(|u| format!(", AUTH {u}"))
                        .unwrap_or_default()
                )
            )
        );
        if let (Some(site), framework) = (&s.site, &d.framework) {
            println!(
                "Site     {site}{}",
                framework
                    .as_deref()
                    .map(|f| format!(" ({f})"))
                    .unwrap_or_default()
            );
        }
        if headers {
            println!();
            for h in &d.headers {
                println!("{}: {}", self.paint(Style::Cyan, &h.name), h.value);
            }
        }
        println!();
        match &d.text {
            Some(text) => {
                if d.text_generated {
                    println!(
                        "{}",
                        self.paint(
                            Style::Dim,
                            "(text generated from the HTML; the message has no text part)"
                        )
                    );
                }
                println!("{}", text.trim_end());
            }
            None => println!("{}", self.paint(Style::Dim, "(no text)")),
        }
        if !d.links.is_empty() {
            println!("\n{}", self.paint(Style::Bold, "Links"));
            for l in &d.links {
                println!(
                    "  {}{}",
                    l.url,
                    l.text
                        .as_deref()
                        .map(|t| format!("  ({t})"))
                        .unwrap_or_default()
                );
            }
        }
        if !d.attachments.is_empty() {
            println!("\n{}", self.paint(Style::Bold, "Attachments"));
            for a in &d.attachments {
                println!(
                    "  {}  {}  {}  {}{}",
                    a.index,
                    a.name.as_deref().unwrap_or("(unnamed)"),
                    a.content_type,
                    size(a.size),
                    if a.inline { "  inline" } else { "" }
                );
            }
        }
        if !d.problems.is_empty() {
            println!("\n{}", self.paint(Style::Yellow, "Problems"));
            for p in &d.problems {
                println!("  {p}");
            }
        }
    }

    pub fn mail_setup(&self, s: &MailSetup) {
        if self.json {
            self.print_json(s);
            return;
        }
        println!(
            "{}  {}  {}",
            self.paint(Style::Bold, &s.site),
            s.framework,
            if s.enabled {
                self.paint(Style::Green, "mailbox on")
            } else {
                self.paint(Style::Dim, "mailbox off")
            }
        );
        if !s.current.is_empty() {
            println!("now      {}", s.current.join("  "));
        }
        if s.real_smtp {
            println!(
                "{}",
                self.paint(
                    Style::Yellow,
                    "         a real mail server: enabling needs --override-real"
                )
            );
        }
        match s.wiring {
            MailWiring::Automatic => {
                for h in &s.injects {
                    println!("sets     {}={}", h.name, h.value);
                }
            }
            MailWiring::Manual => println!("manual   add this to the project:"),
        }
        if let (MailWiring::Manual, Some(snippet)) = (s.wiring, &s.snippet) {
            for line in snippet.lines() {
                println!("         {line}");
            }
        }
        for note in &s.notes {
            println!("{}", self.paint(Style::Dim, &format!("note     {note}")));
        }
    }
}
