//! How well each platform fits a profile, requirement by requirement. The
//! verdict comes only from what the project needs; ties keep fewer changes first.

use locdev_core::deploy::{DeployProfile, Point, ProviderFit, Rendering, Verdict};

use crate::providers::{Offer, Provider};

fn point(text: impl Into<String>, source: Option<&str>) -> Point {
    Point {
        text: text.into(),
        source: source.map(str::to_string),
    }
}

struct Fit {
    verdict: Verdict,
    requirements: Vec<Point>,
    blockers: Vec<Point>,
    /// Changes the project would need here (the tie-breaker).
    changes: usize,
}

impl Fit {
    fn change(&mut self, text: impl Into<String>, source: Option<&str>) {
        if self.verdict == Verdict::Fits {
            self.verdict = Verdict::NeedsChanges;
        }
        self.changes += 1;
        self.requirements.push(point(text, source));
    }

    fn block(&mut self, text: impl Into<String>, source: Option<&str>) {
        self.verdict = Verdict::NotSuitable;
        self.blockers.push(point(text, source));
    }

    fn require(&mut self, text: impl Into<String>, source: Option<&str>) {
        self.requirements.push(point(text, source));
    }
}

fn runs(p: &Provider, profile: &DeployProfile, framework_id: Option<&str>, fit: &mut Fit) {
    let native = p.runtimes.contains(&profile.runtime.as_str());
    let id = framework_id.unwrap_or("");
    match profile.rendering {
        Rendering::Static | Rendering::Spa => {
            if p.static_sites {
                if let Some(dir) = &profile.output_dir {
                    fit.require(format!("Publish `{dir}` after the build"), None);
                }
            } else if p.docker {
                fit.change(
                    "Serve the built files from a small web server image (a Dockerfile)",
                    Some(p.docs),
                );
            } else {
                fit.block("It doesn't host static sites", Some(p.docs));
            }
        }
        Rendering::Ssr => {
            if p.ssr_frameworks.contains(&id) {
                fit.require(
                    format!(
                        "{} is deployed directly (framework support or official adapter)",
                        profile.framework
                    ),
                    None,
                );
            } else if p.servers && native {
                if let Some(start) = &profile.start_command {
                    fit.require(format!("Run `{start}` as a long-running service"), None);
                } else {
                    fit.change("Add a start script that runs the production server", None);
                }
            } else if p.docker {
                fit.change("Build it from a Dockerfile", Some(p.docs));
            } else {
                fit.block(
                    format!("{} isn't supported here", profile.framework),
                    Some(p.docs),
                );
            }
        }
        Rendering::Server => {
            if p.servers && native {
                fit.require(
                    format!("Runs as a long-running {} service", profile.runtime),
                    None,
                );
            } else if p.servers && p.docker {
                fit.change(
                    format!(
                        "Build it from a Dockerfile ({} isn't built from source here)",
                        profile.runtime
                    ),
                    Some(p.docs),
                );
            } else if p.function_backends.contains(&id) {
                fit.change(
                    format!("Runs as functions: no in-memory state between requests and nothing running in the background ({})", profile.framework),
                    Some(p.docs),
                );
            } else {
                fit.block(
                    format!("It doesn't run {} application servers", profile.runtime),
                    Some(p.docs),
                );
            }
        }
    }
}

fn offer(fit: &mut Fit, offer: &Offer, what: &str, missing: &str) {
    match offer {
        Some((how, source)) => fit.require(format!("{what}: {how}"), Some(source)),
        None => fit.change(missing.to_string(), None),
    }
}

fn database(fit: &mut Fit, offer: &Offer, what: &str) {
    match offer {
        Some((how, source)) => fit.require(format!("{what}: {how}"), Some(source)),
        // A database elsewhere is normal; it doesn't change the verdict.
        None => fit.require(format!("{what}: an external managed provider"), None),
    }
}

/// The fit and how many changes it takes.
pub fn fit(
    p: &Provider,
    profile: &DeployProfile,
    framework_id: Option<&str>,
) -> (ProviderFit, usize) {
    let mut fit = Fit {
        verdict: Verdict::Fits,
        requirements: Vec::new(),
        blockers: Vec::new(),
        changes: 0,
    };
    runs(p, profile, framework_id, &mut fit);
    if fit.verdict != Verdict::NotSuitable {
        for need in &profile.needs {
            match need.id.as_str() {
                "websockets" => offer(
                    &mut fit,
                    &p.websockets,
                    "WebSockets",
                    "Move WebSockets to a hosted realtime service (or another platform)",
                ),
                "workers" => offer(
                    &mut fit,
                    &p.workers,
                    "Queue workers",
                    "Run the queue workers somewhere else, or process jobs synchronously",
                ),
                "scheduler" => offer(
                    &mut fit,
                    &p.scheduler,
                    "Scheduler",
                    "Trigger scheduled tasks from an external cron",
                ),
                "postgres" => database(&mut fit, &p.postgres, "PostgreSQL"),
                "mysql" => database(&mut fit, &p.mysql, "MySQL/MariaDB"),
                "mongo" => database(&mut fit, &p.mongo, "MongoDB"),
                "redis" => database(&mut fit, &p.redis, "Redis"),
                "sqlite" => match &p.disk {
                    Some((how, source)) => fit.require(
                        format!("Keep the SQLite file on persistent storage: {how}"),
                        Some(source),
                    ),
                    None => fit.change(
                        "SQLite needs a disk that survives deploys; move to a managed database",
                        None,
                    ),
                },
                "storage" => match &p.disk {
                    Some((how, source)) => {
                        fit.require(format!("Uploaded files: {how}"), Some(source))
                    }
                    None => fit.change(
                        "Store uploaded files in object storage (S3-compatible)",
                        None,
                    ),
                },
                _ => {}
            }
        }
        let secrets = profile.env.iter().filter(|e| e.secret).count();
        if !profile.env.is_empty() {
            fit.require(
                format!(
                    "Set {} environment variables ({secrets} secret) in its settings",
                    profile.env.len()
                ),
                None,
            );
        }
    }
    let mut limitations: Vec<Point> = fit.blockers;
    limitations.extend(p.limits.iter().map(|(t, s)| point(*t, *s)));
    let changes = fit.changes;
    let out = ProviderFit {
        id: p.id.into(),
        name: p.name.into(),
        model: p.model.into(),
        verdict: fit.verdict,
        summary: p.summary.into(),
        strengths: p.strengths.iter().map(|(t, s)| point(*t, *s)).collect(),
        limitations,
        requirements: fit.requirements,
        docs: p.docs.into(),
        // Offered only when LocDev would really write one for this project.
        config_file: p
            .config_file
            .filter(|_| crate::generate::supports(p.id, profile))
            .map(str::to_string),
        setup: p.setup.map(str::to_string),
    };
    (out, changes)
}

/// Fits first, then those needing changes (fewest first), then the rest.
/// The catalog order breaks remaining ties, which carries no preference.
pub fn rank(mut fits: Vec<(ProviderFit, usize)>) -> Vec<ProviderFit> {
    fits.sort_by_key(|(f, changes)| (f.verdict, *changes));
    fits.into_iter().map(|(f, _)| f).collect()
}
