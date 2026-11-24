//! Rendering for `locdev update`.

use locdev_core::update::UpdateStatus;

use super::{Output, Style};

impl Output {
    pub fn updates(&self, status: &UpdateStatus) {
        if self.json {
            self.print_json(status);
            return;
        }
        println!("locdev/locdevd {}", status.daemon);
        if let Some(error) = &status.error {
            println!(
                "{}",
                self.paint(Style::Red, &format!("Couldn't check: {error}"))
            );
        }
        let Some(latest) = &status.latest else {
            if status.error.is_none() {
                println!("{} has no published release yet.", status.repo);
            }
            return;
        };
        let date = latest
            .published_at
            .as_deref()
            .and_then(|d| d.get(..10))
            .map(|d| format!(" ({d})"))
            .unwrap_or_default();
        println!(
            "Latest release: {}{date}  {}",
            self.paint(Style::Bold, &latest.tag),
            latest.url
        );
        match (&latest.app, &latest.daemon) {
            (Some(app), Some(daemon)) => println!("  app {app} · daemon/CLI {daemon}"),
            _ => println!(
                "  {}",
                self.paint(Style::Dim, "the release doesn't list its versions")
            ),
        }
        if status.daemon_update {
            println!(
                "{}",
                self.paint(Style::Yellow, "A newer daemon and CLI are available.")
            );
            println!("  With the app: it updates them (Settings → Updates).");
            let base = format!(
                "https://github.com/{}/releases/latest/download",
                status.repo
            );
            if cfg!(windows) {
                println!("  Standalone CLI: irm {base}/install.ps1 | iex");
            } else {
                println!("  Standalone CLI: curl -fsSL {base}/install.sh | sh");
            }
            println!("  The install script checks the download against the release's SHA256SUMS.");
        } else if latest.daemon.is_some() {
            println!(
                "{}",
                self.paint(Style::Green, "The daemon and CLI are up to date.")
            );
        }
    }
}
