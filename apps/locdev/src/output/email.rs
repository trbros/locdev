//! Rendering for `locdev email …` and `locdev new email …`.

use locdev_core::email::{EmailCatalog, EmailCreated, EmailDoc, Segment};

use super::{Output, Style};

impl Output {
    pub fn email_catalog(&self, c: &EmailCatalog) {
        if self.json {
            self.print_json(c);
            return;
        }
        println!(
            "{}",
            self.paint(Style::Bold, &format!("Templates ({})", c.templates.len()))
        );
        if c.templates.is_empty() {
            println!("  none yet: `locdev new email <name>` creates one");
        }
        for t in &c.templates {
            let mailables: Vec<&str> = t
                .mailables
                .iter()
                .map(|m| m.class.rsplit('\\').next().unwrap_or(&m.class))
                .collect();
            println!(
                "  {:<52} {:<9} {}{}",
                t.path,
                format!("{:?}", t.format).to_lowercase(),
                if mailables.is_empty() {
                    String::new()
                } else {
                    self.paint(Style::Dim, &mailables.join(", "))
                },
                if t.has_blocks {
                    self.paint(Style::Cyan, "  blocks")
                } else {
                    String::new()
                }
            );
        }
        let own: Vec<_> = c.components.iter().filter(|x| !x.builtin).collect();
        println!(
            "\n{}",
            self.paint(Style::Bold, &format!("Components ({})", own.len()))
        );
        for x in own {
            let props: Vec<String> = x.props.iter().map(|p| p.name.clone()).collect();
            println!(
                "  <{}>  {}",
                x.usage,
                self.paint(Style::Dim, &props.join(", "))
            );
        }
        println!();
        for line in &c.conventions {
            println!("{}", self.paint(Style::Dim, line));
        }
        if c.framework_preview {
            println!(
                "{}",
                self.paint(
                    Style::Dim,
                    "Framework preview available (`locdev email preview <path> --framework`)."
                )
            );
        }
    }

    pub fn email_created(&self, c: &EmailCreated) {
        if self.json {
            self.print_json(c);
            return;
        }
        if let Some(ran) = &c.ran {
            println!("ran      {ran}");
        }
        for f in &c.files {
            println!("created  {f}");
        }
        for n in &c.notes {
            println!("{}", self.paint(Style::Dim, &format!("note     {n}")));
        }
    }

    pub fn email_doc(&self, doc: &EmailDoc) {
        if self.json {
            self.print_json(doc);
            return;
        }
        println!(
            "{}  {}",
            self.paint(Style::Bold, &doc.path),
            self.paint(Style::Dim, &format!("{:?}", doc.format).to_lowercase())
        );
        for segment in &doc.segments {
            match segment {
                Segment::Block {
                    id,
                    block,
                    props,
                    edited,
                } => {
                    let summary = props
                        .as_object()
                        .map(|m| {
                            m.iter()
                                .filter(|(_, v)| !v.is_object() && !v.is_array())
                                .take(4)
                                .map(|(k, v)| {
                                    let v = v.as_str().map(str::to_string).unwrap_or(v.to_string());
                                    format!("{k}={}", super::truncate(&v, 30))
                                })
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default();
                    println!(
                        "  {} {:<10} {}{}",
                        self.paint(Style::Cyan, id),
                        block,
                        self.paint(Style::Dim, &summary),
                        if *edited {
                            self.paint(Style::Yellow, "  (edited outside LocDev)")
                        } else {
                            String::new()
                        }
                    );
                }
                Segment::Code { code, .. } => {
                    let lines = code.lines().count();
                    println!(
                        "  {}",
                        self.paint(
                            Style::Dim,
                            &format!("code · {lines} line(s), kept as written")
                        )
                    );
                }
            }
        }
        if !doc.variables.is_empty() {
            println!("\nvariables: {}", doc.variables.join(", "));
        }
        if doc.sample.as_object().is_some_and(|m| !m.is_empty()) {
            println!("sample: {}", doc.sample);
        }
        for note in &doc.notes {
            self.note(note);
        }
    }
}
