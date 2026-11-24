//! Rendering for the `locdev git` reads and changes and `locdev github show`.

use locdev_core::git::{CommitDetail, DestructivePreview, GitResult, GithubData, Remote};
use serde_json::json;

use super::{Output, Style, truncate};

impl Output {
    /// Plain text from the daemon (a diff, a file).
    pub fn text(&self, text: &str, truncated: bool) {
        if self.json {
            self.print_json(json!({ "text": text, "truncated": truncated }));
            return;
        }
        self.diff(text);
        if truncated {
            self.note("(truncated)");
        }
    }

    fn diff(&self, text: &str) {
        for line in text.lines() {
            let style = if line.starts_with("+++") || line.starts_with("---") {
                Some(Style::Bold)
            } else if line.starts_with('+') {
                Some(Style::Green)
            } else if line.starts_with('-') {
                Some(Style::Red)
            } else if line.starts_with("@@") {
                Some(Style::Cyan)
            } else {
                None
            };
            match style {
                Some(style) => println!("{}", self.paint(style, line)),
                None => println!("{line}"),
            }
        }
    }

    pub fn commit_detail(&self, detail: &CommitDetail) {
        if self.json {
            self.print_json(detail);
            return;
        }
        let c = &detail.commit;
        println!(
            "{} {}",
            self.paint(Style::Yellow, &c.sha),
            self.paint(Style::Bold, &c.subject)
        );
        println!(
            "{}",
            self.paint(Style::Dim, &format!("{} <{}>", c.author, c.email))
        );
        if !detail.body.trim().is_empty() {
            println!("\n{}", detail.body.trim_end());
        }
        println!();
        for f in &detail.files {
            let counts = match (f.added, f.removed) {
                (Some(a), Some(r)) => format!("+{a} -{r}"),
                _ => "binary".into(),
            };
            println!("  {}  {}", f.path, self.paint(Style::Dim, &counts));
        }
        println!();
        self.diff(&detail.diff);
        if detail.truncated {
            self.note("(diff truncated)");
        }
    }

    pub fn remotes(&self, remotes: &[Remote]) {
        if self.json {
            self.print_json(remotes);
            return;
        }
        if remotes.is_empty() {
            println!("no remotes");
        }
        for r in remotes {
            println!("{}  {}", self.paint(Style::Bold, &r.name), r.fetch_url);
            if r.push_url != r.fetch_url {
                println!(
                    "  {}",
                    self.paint(Style::Dim, &format!("push: {}", r.push_url))
                );
            }
        }
    }

    /// What a destructive change loses; on stderr so `--json` stays clean.
    pub fn destructive_preview(&self, p: &DestructivePreview) {
        eprintln!("{}", self.paint(Style::Yellow, &p.summary));
        for lost in &p.lost {
            eprintln!("  - {lost}");
        }
        if let Some(backup) = &p.backup {
            eprintln!(
                "{}",
                self.paint(Style::Dim, &format!("backup first: {backup}"))
            );
        }
        if let Some(blocked) = &p.blocked {
            eprintln!("{}", self.paint(Style::Red, &format!("blocked: {blocked}")));
        }
    }

    pub fn git_result(&self, r: &GitResult) {
        if self.json {
            self.print_json(r);
            return;
        }
        println!("{}", r.message);
        if !r.output.trim().is_empty() {
            println!("{}", self.paint(Style::Dim, r.output.trim_end()));
        }
        if let Some(backup) = &r.backup {
            println!("{}", self.paint(Style::Dim, &format!("backup: {backup}")));
        }
    }

    pub fn github_data(&self, data: &GithubData) {
        if self.json {
            self.print_json(data);
            return;
        }
        match data {
            GithubData::Overview(o) => {
                println!(
                    "{}{}",
                    self.paint(Style::Bold, &o.repo.slug()),
                    if o.private { "  (private)" } else { "" }
                );
                if let Some(d) = &o.description {
                    println!("{d}");
                }
                println!(
                    "{}",
                    self.paint(
                        Style::Dim,
                        &format!(
                            "default branch {} · {} open issues · {} stars · {}",
                            o.default_branch, o.open_issues, o.stars, o.html_url
                        )
                    )
                );
            }
            GithubData::Issues { items } | GithubData::Pulls { items } => {
                if items.is_empty() {
                    println!("nothing open");
                }
                for i in items {
                    let mut extra = vec![i.author.clone()];
                    if let Some(b) = &i.branch {
                        extra.push(b.clone());
                    }
                    if i.draft {
                        extra.push("draft".into());
                    }
                    extra.extend(i.labels.iter().cloned());
                    println!(
                        "{} {}  {}",
                        self.paint(Style::Yellow, &format!("#{}", i.number)),
                        truncate(&i.title, 70),
                        self.paint(Style::Dim, &extra.join(" · "))
                    );
                }
            }
            GithubData::Workflows { workflows } => {
                for w in workflows {
                    println!(
                        "{:<12} {}  {}",
                        w.id,
                        self.paint(Style::Bold, &w.name),
                        self.paint(Style::Dim, &format!("{} · {}", w.path, w.state))
                    );
                }
            }
            GithubData::Runs { runs } => {
                for r in runs {
                    let result = r.conclusion.as_deref().unwrap_or(&r.status);
                    let style = match result {
                        "success" => Style::Green,
                        "failure" | "timed_out" | "startup_failure" => Style::Red,
                        "cancelled" | "skipped" => Style::Dim,
                        _ => Style::Yellow,
                    };
                    println!(
                        "{:<12} {:<11} {}  {}",
                        r.id,
                        self.paint(style, result),
                        truncate(&r.name, 40),
                        self.paint(
                            Style::Dim,
                            &format!("{} · {} · {}", r.branch, r.event, r.created_at)
                        )
                    );
                }
            }
            GithubData::Releases { releases } => {
                for r in releases {
                    let kind = if r.draft {
                        " (draft)"
                    } else if r.prerelease {
                        " (pre-release)"
                    } else {
                        ""
                    };
                    println!(
                        "{}{kind}  {}  {}",
                        self.paint(Style::Bold, &r.tag),
                        r.name,
                        self.paint(Style::Dim, r.published_at.as_deref().unwrap_or(""))
                    );
                }
            }
            GithubData::Branches { names } => {
                for n in names {
                    println!("{n}");
                }
            }
            GithubData::Commits { commits } => self.commits(commits),
            GithubData::Checks { sha, checks } => {
                println!("{}", self.paint(Style::Dim, sha));
                for c in checks {
                    let result = c.conclusion.as_deref().unwrap_or(&c.status);
                    println!("  {:<11} {}", result, c.name);
                }
            }
        }
    }
}
