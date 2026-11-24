//! Config files for platforms whose format is stable and documented
//! (checked against the specs on `providers::REVIEWED`). Values are never
//! copied from .env: secrets are declared for the platform to ask for.

use locdev_core::deploy::{DeployProfile, Rendering};

use crate::profile::secret_name;

pub struct Generated {
    pub file: &'static str,
    pub content: String,
    pub notes: Vec<String>,
}

fn toml_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn yaml_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    s.trim_matches('-').to_string()
}

/// Whether LocDev writes a config file for `provider` and this profile.
pub fn supports(provider: &str, profile: &DeployProfile) -> bool {
    match provider {
        "netlify" => netlify(profile).is_ok(),
        "render" => render(profile, "app").is_ok(),
        _ => false,
    }
}

/// netlify.toml for a static or client-side build.
pub fn netlify(profile: &DeployProfile) -> Result<Generated, String> {
    if !matches!(profile.rendering, Rendering::Static | Rendering::Spa) {
        return Err(format!(
            "{} is detected by Netlify itself (framework adapter); a netlify.toml isn't needed",
            profile.framework
        ));
    }
    let command = profile
        .build_command
        .as_deref()
        .ok_or("there's no build command to put in it")?;
    let publish = profile
        .output_dir
        .as_deref()
        .ok_or("the build's output folder isn't known")?;
    let mut out = format!(
        "# Written by LocDev; review before committing.\n# https://docs.netlify.com/build/configure-builds/file-based-configuration/\n\n[build]\n  command = {}\n  publish = {}\n",
        toml_string(command),
        toml_string(publish)
    );
    let mut notes = vec!["Environment variables go in the Netlify UI (Site configuration → Environment variables), not in this file.".to_string()];
    if profile.rendering == Rendering::Spa {
        out.push_str("\n# Client-side routes: every path serves the app.\n[[redirects]]\n  from = \"/*\"\n  to = \"/index.html\"\n  status = 200\n");
        notes.push(
            "The redirect sends every path to index.html, as client-side routing needs.".into(),
        );
    }
    Ok(Generated {
        file: "netlify.toml",
        content: out,
        notes,
    })
}

/// render.yaml (Blueprint) for a Node.js app, with its database and cache.
pub fn render(profile: &DeployProfile, name: &str) -> Result<Generated, String> {
    if profile.runtime != "node" {
        return Err(format!(
            "on Render, {} runs from a Dockerfile; LocDev only writes render.yaml for Node.js apps",
            profile.runtime
        ));
    }
    let app = slug(name);
    let build = profile
        .build_command
        .as_deref()
        .ok_or("there's no build script in package.json")?;
    let mut out = String::from(
        "# Written by LocDev; review before committing.\n# https://render.com/docs/blueprint-spec\n\nservices:\n",
    );
    let mut notes = Vec::new();
    let needs = |id: &str| profile.needs.iter().any(|n| n.id == id);
    match profile.rendering {
        Rendering::Static | Rendering::Spa => {
            let dir = profile
                .output_dir
                .as_deref()
                .ok_or("the build's output folder isn't known")?;
            out.push_str(&format!(
                "  - type: web\n    name: {app}\n    runtime: static\n    buildCommand: {}\n    staticPublishPath: {}\n",
                yaml_string(build),
                yaml_string(dir)
            ));
            if profile.rendering == Rendering::Spa {
                out.push_str("    routes:\n      - type: rewrite\n        source: /*\n        destination: /index.html\n");
            }
        }
        Rendering::Ssr | Rendering::Server => {
            let start = profile
                .start_command
                .as_deref()
                .ok_or("there's no start script in package.json")?;
            out.push_str(&format!(
                "  - type: web\n    name: {app}\n    runtime: node\n    buildCommand: {}\n    startCommand: {}\n",
                yaml_string(build),
                yaml_string(start)
            ));
            let mut vars = Vec::new();
            for env in &profile.env {
                let line = match env.name.as_str() {
                    "DATABASE_URL" if needs("postgres") => format!(
                        "      - key: DATABASE_URL\n        fromDatabase:\n          name: {app}-db\n          property: connectionString\n"
                    ),
                    "REDIS_URL" if needs("redis") => format!(
                        "      - key: REDIS_URL\n        fromService:\n          type: keyvalue\n          name: {app}-cache\n          property: connectionString\n"
                    ),
                    "NODE_ENV" => "      - key: NODE_ENV\n        value: production\n".into(),
                    // Values never come from .env: Render asks for each at creation.
                    n => format!("      - key: {n}\n        sync: false\n"),
                };
                vars.push(line);
            }
            if !vars.is_empty() {
                out.push_str("    envVars:\n");
                for v in vars {
                    out.push_str(&v);
                }
                let secrets = profile.env.iter().filter(|e| secret_name(&e.name)).count();
                notes.push(format!(
                    "`sync: false` variables ({} of them, {secrets} secret) are asked for in the dashboard when the Blueprint is created; no value is written here.",
                    profile.env.len()
                ));
            }
            if needs("redis") {
                out.push_str(&format!(
                    "  - type: keyvalue\n    name: {app}-cache\n    ipAllowList: [] # only services in this account\n"
                ));
            }
            if needs("workers") {
                notes.push("Queue workers: add a `type: worker` service with your worker's start command (LocDev doesn't know it for Node.js).".into());
            }
            if needs("scheduler") {
                notes.push("Scheduled tasks: add a `type: cron` service with a `schedule` and the command to run.".into());
            }
        }
    }
    if needs("postgres") && matches!(profile.rendering, Rendering::Ssr | Rendering::Server) {
        out.push_str(&format!("\ndatabases:\n  - name: {app}-db\n"));
    }
    if needs("mysql") || needs("mongo") {
        notes.push(
            "Render has no managed MySQL or MongoDB; point the app at an external provider.".into(),
        );
    }
    if needs("sqlite") {
        notes.push("SQLite needs a persistent disk on Render (paid instances): add a `disk` to the web service, or move to Postgres.".into());
    }
    Ok(Generated {
        file: "render.yaml",
        content: out,
        notes,
    })
}
