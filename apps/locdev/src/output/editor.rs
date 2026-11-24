//! Rendering for `locdev editor …`.

use locdev_core::editor::EditorsView;

use super::{Output, Style};

impl Output {
    pub fn editors(&self, view: &EditorsView) {
        if self.json {
            self.print_json(view);
            return;
        }
        if view.editors.is_empty() {
            println!(
                "No code editor found. Install one (VS Code, Cursor, Zed…) or put its command on PATH, then `locdev editor list --refresh`."
            );
        }
        for e in &view.editors {
            let mark = if view.default.as_deref() == Some(e.id.as_str()) {
                self.paint(Style::Green, "●")
            } else {
                " ".into()
            };
            println!(
                "{mark} {:<10} {:<20} {}",
                e.id,
                e.name,
                self.paint(Style::Dim, &e.program.display().to_string())
            );
        }
        match (&view.preferred, &view.default) {
            (Some(p), Some(d)) if p != d => println!(
                "{}",
                self.paint(Style::Yellow, &format!("{p} is chosen but not installed; {d} opens instead."))
            ),
            (None, Some(d)) => println!(
                "{}",
                self.paint(Style::Dim, &format!("None chosen: {d} (the first found) opens. `locdev editor set <id>` picks one."))
            ),
            _ => {}
        }
    }
}
