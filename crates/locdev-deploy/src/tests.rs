use locdev_core::db::DbEngine;
use locdev_core::deploy::{CheckLevel, Rendering, Verdict};
use locdev_core::model::{Component, ComponentCategory, ProcessKind, RuntimeKind, Stack};

use crate::{Inputs, analyze, config, write};

fn component(id: &str, name: &str, category: ComponentCategory, primary: bool) -> Component {
    Component {
        id: id.into(),
        name: name.into(),
        category,
        version: None,
        version_source: None,
        evidence: vec![],
        manual: false,
        primary,
    }
}

fn stack(components: &[(&str, &str, bool)]) -> Stack {
    Stack {
        package_manager: Some(RuntimeKind::Npm),
        components: components
            .iter()
            .map(|(id, name, primary)| {
                let category = if *primary {
                    ComponentCategory::Framework
                } else {
                    ComponentCategory::Tool
                };
                component(id, name, category, *primary)
            })
            .collect(),
        ..Default::default()
    }
}

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

fn verdict(report: &locdev_core::deploy::DeployReport, id: &str) -> Verdict {
    report
        .providers
        .iter()
        .find(|p| p.id == id)
        .unwrap()
        .verdict
}

#[test]
fn laravel_with_reverb_queues_and_scheduler() {
    let dir = project(&[
        ("artisan", ""),
        (
            "composer.json",
            r#"{"require":{"php":"^8.3","laravel/framework":"^12.0","laravel/reverb":"^1.0"}}"#,
        ),
        (
            "composer.lock",
            r#"{"packages":[{"name":"laravel/reverb","version":"v1.4.0"}]}"#,
        ),
        (
            "routes/console.php",
            "<?php\nSchedule::command('reports:send')->daily();\n",
        ),
        (
            ".env",
            "APP_KEY=base64:abc\nAPP_DEBUG=true\nDB_CONNECTION=mysql\nQUEUE_CONNECTION=redis\nMAIL_PASSWORD=supersecret\n",
        ),
        (
            ".env.example",
            "APP_KEY=\nAPP_DEBUG=true\nDB_CONNECTION=mysql\nQUEUE_CONNECTION=redis\n",
        ),
    ]);
    let st = stack(&[("laravel", "Laravel", true)]);
    let inputs = Inputs {
        root: dir.path(),
        name: "shop",
        stack: &st,
        databases: vec![(DbEngine::Mysql, false), (DbEngine::Redis, false)],
        processes: vec![],
    };
    let report = analyze(&inputs);
    let p = &report.profile;
    assert_eq!(
        (p.runtime.as_str(), p.rendering),
        ("php", Rendering::Server)
    );
    assert_eq!(p.runtime_version.as_deref(), Some("^8.3"));
    let needs: Vec<&str> = p.needs.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        needs,
        vec!["websockets", "workers", "scheduler", "mysql", "redis"]
    );

    for id in ["vercel", "netlify", "cloudflare", "github-pages"] {
        assert_eq!(verdict(&report, id), Verdict::NotSuitable, "{id}");
    }
    for id in ["forge", "railway", "vps"] {
        assert_eq!(verdict(&report, id), Verdict::Fits, "{id}");
    }
    // PHP runs on Render and Fly only from a Dockerfile.
    assert_eq!(verdict(&report, "render"), Verdict::NeedsChanges);
    assert_eq!(verdict(&report, "fly"), Verdict::NeedsChanges);
    // Ranked by fit: every Fits before any NeedsChanges before NotSuitable.
    let order: Vec<Verdict> = report.providers.iter().map(|p| p.verdict).collect();
    assert!(order.windows(2).all(|w| w[0] <= w[1]), "{order:?}");

    let env: Vec<(&str, bool, &str)> = p
        .env
        .iter()
        .map(|e| (e.name.as_str(), e.secret, e.note.as_str()))
        .collect();
    assert!(env.contains(&(
        "APP_KEY",
        true,
        "a new key for production (`php artisan key:generate --show`)"
    )));
    assert!(env.contains(&(
        "MAIL_PASSWORD",
        true,
        "only in .env: add it to .env.example if production needs it"
    )));
    assert!(env.contains(&("APP_DEBUG", false, "false in production")));
    // Values never leave the analysis.
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("supersecret") && !json.contains("base64:abc"));
    let checks: Vec<(&str, CheckLevel)> = report
        .checks
        .iter()
        .map(|c| (c.id.as_str(), c.level))
        .collect();
    assert!(checks.contains(&("env-example", CheckLevel::Warn)));
    assert!(checks.contains(&("app-debug", CheckLevel::Warn)));
    assert!(checks.contains(&("lockfile", CheckLevel::Ok)));
    assert!(
        config(&inputs, "render")
            .unwrap_err()
            .contains("Dockerfile")
    );
}

const NEXT_PACKAGE: &str = r#"{"name":"web","scripts":{"build":"next build","start":"next start"},"engines":{"node":">=22"},"dependencies":{"next":"16.0.0"}}"#;

#[test]
fn next_server_rendering_with_postgres() {
    let dir = project(&[
        ("package.json", NEXT_PACKAGE),
        ("package-lock.json", "{}"),
        (
            "next.config.ts",
            "const config = { poweredByHeader: false };\nexport default config;\n",
        ),
        (
            ".env",
            "DATABASE_URL=postgres://u:hunter2@127.0.0.1/web\nAUTH_SECRET=hunter3\nNEXT_PUBLIC_SITE_URL=http://web.test\n",
        ),
        (
            ".env.example",
            "DATABASE_URL=\nAUTH_SECRET=\nNEXT_PUBLIC_SITE_URL=\n",
        ),
    ]);
    let st = stack(&[("nextjs", "Next.js", true), ("react", "React", false)]);
    let inputs = Inputs {
        root: dir.path(),
        name: "web",
        stack: &st,
        databases: vec![(DbEngine::Postgres, false)],
        processes: vec![],
    };
    let report = analyze(&inputs);
    let p = &report.profile;
    assert_eq!(p.rendering, Rendering::Ssr);
    assert_eq!(p.build_command.as_deref(), Some("npm ci && npm run build"));
    assert_eq!(p.start_command.as_deref(), Some("npm run start"));
    assert_eq!(p.runtime_version.as_deref(), Some(">=22"));
    for id in [
        "vercel",
        "netlify",
        "cloudflare",
        "railway",
        "render",
        "vps",
    ] {
        assert_eq!(verdict(&report, id), Verdict::Fits, "{id}");
    }
    assert_eq!(verdict(&report, "github-pages"), Verdict::NotSuitable);
    assert_eq!(verdict(&report, "forge"), Verdict::NotSuitable);
    let secret = |n: &str| p.env.iter().find(|e| e.name == n).unwrap().secret;
    assert!(secret("AUTH_SECRET") && !secret("NEXT_PUBLIC_SITE_URL"));

    let render = config(&inputs, "render").unwrap();
    assert!(render.content.contains("runtime: node"));
    assert!(
        render.content.contains(
            "fromDatabase:\n          name: web-db\n          property: connectionString"
        )
    );
    assert!(
        render
            .content
            .contains("- key: AUTH_SECRET\n        sync: false")
    );
    assert!(render.content.contains("databases:\n  - name: web-db"));
    assert!(!render.content.contains("hunter"), "no value from .env");
    assert!(!render.exists);

    // Written only as reviewed, and never over an existing file.
    assert!(
        write(&inputs, "render", "0")
            .unwrap_err()
            .contains("review it again")
    );
    assert_eq!(
        write(&inputs, "render", &render.hash).unwrap(),
        "render.yaml"
    );
    assert!(
        write(&inputs, "render", &render.hash)
            .unwrap_err()
            .contains("doesn't overwrite")
    );
    assert!(
        config(&inputs, "netlify")
            .unwrap_err()
            .contains("detected by Netlify itself")
    );
}

#[test]
fn static_export_and_spa() {
    let dir = project(&[
        ("package.json", NEXT_PACKAGE),
        ("next.config.mjs", "export default { output: 'export' }\n"),
    ]);
    let st = stack(&[("nextjs", "Next.js", true)]);
    let inputs = Inputs {
        root: dir.path(),
        name: "web",
        stack: &st,
        databases: vec![],
        processes: vec![],
    };
    let report = analyze(&inputs);
    assert_eq!(report.profile.rendering, Rendering::Static);
    assert_eq!(report.profile.output_dir.as_deref(), Some("out"));
    assert_eq!(verdict(&report, "github-pages"), Verdict::Fits);

    let dir = project(&[
        (
            "package.json",
            r#"{"scripts":{"build":"vite build"},"devDependencies":{"vite":"7.0.0"}}"#,
        ),
        (
            "vite.config.ts",
            "export default { build: { outDir: 'public_html' } }\n",
        ),
    ]);
    let st = stack(&[("vite", "Vite", true), ("react", "React", false)]);
    let inputs = Inputs {
        root: dir.path(),
        name: "spa",
        stack: &st,
        databases: vec![],
        processes: vec![],
    };
    let report = analyze(&inputs);
    assert_eq!(report.profile.rendering, Rendering::Spa);
    assert_eq!(report.profile.output_dir.as_deref(), Some("public_html"));
    let netlify = config(&inputs, "netlify").unwrap();
    assert!(netlify.content.contains("publish = \"public_html\""));
    assert!(netlify.content.contains("to = \"/index.html\""));
    let render = config(&inputs, "render").unwrap();
    assert!(
        render.content.contains("runtime: static")
            && render.content.contains("destination: /index.html")
    );
    // No lockfile, no engines: both flagged.
    let levels: Vec<(&str, CheckLevel)> = report
        .checks
        .iter()
        .map(|c| (c.id.as_str(), c.level))
        .collect();
    assert!(
        levels.contains(&("lockfile", CheckLevel::Warn))
            && levels.contains(&("runtime", CheckLevel::Warn))
    );
}

#[test]
fn websockets_and_workers_change_serverless_verdicts() {
    let dir = project(&[(
        "package.json",
        r#"{"scripts":{"build":"next build","start":"next start"},"dependencies":{"next":"16.0.0","socket.io":"4.8.0","bullmq":"5.0.0"}}"#,
    )]);
    let st = stack(&[("nextjs", "Next.js", true)]);
    let inputs = Inputs {
        root: dir.path(),
        name: "rt",
        stack: &st,
        databases: vec![(DbEngine::Sqlite, false)],
        processes: vec![ProcessKind::Worker],
    };
    let report = analyze(&inputs);
    let needs: Vec<&str> = report.profile.needs.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(needs, vec!["websockets", "workers", "sqlite"]);
    // Vercel serves WebSockets (Fluid compute) but has no long-running workers nor disk.
    let vercel = report.providers.iter().find(|p| p.id == "vercel").unwrap();
    assert_eq!(vercel.verdict, Verdict::NeedsChanges);
    assert!(
        vercel
            .requirements
            .iter()
            .any(|r| r.text.starts_with("WebSockets:") && r.source.is_some())
    );
    assert!(
        vercel
            .requirements
            .iter()
            .any(|r| r.text.contains("queue workers somewhere else"))
    );
    assert!(
        vercel
            .requirements
            .iter()
            .any(|r| r.text.contains("SQLite needs a disk"))
    );
    assert_eq!(verdict(&report, "railway"), Verdict::Fits);
    assert!(report.checks.iter().any(|c| c.id == "sqlite"));
}

#[test]
fn committed_env_fails_the_checks() {
    let dir = project(&[
        ("package.json", r#"{"scripts":{"start":"node server.js"}}"#),
        (".env", "TOKEN=x\n"),
    ]);
    let git = |args: &[&str]| {
        let ok = std::process::Command::new("git")
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(args)
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "init"]);
    let st = stack(&[("express", "Express", true)]);
    let inputs = Inputs {
        root: dir.path(),
        name: "api",
        stack: &st,
        databases: vec![],
        processes: vec![],
    };
    let report = analyze(&inputs);
    let env = report
        .checks
        .iter()
        .find(|c| c.id == "env-committed")
        .unwrap();
    assert_eq!(env.level, CheckLevel::Fail);
    // Express runs as functions on Vercel: possible, with changes.
    assert_eq!(verdict(&report, "vercel"), Verdict::NeedsChanges);
    assert_eq!(verdict(&report, "netlify"), Verdict::NotSuitable);
}

#[test]
fn config_offers_only_when_it_would_be_written() {
    let dir = project(&[("package.json", NEXT_PACKAGE)]);
    let st = stack(&[("nextjs", "Next.js", true)]);
    let inputs = Inputs {
        root: dir.path(),
        name: "web",
        stack: &st,
        databases: vec![],
        processes: vec![],
    };
    let report = analyze(&inputs);
    let file = |id: &str| {
        report
            .providers
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .config_file
            .clone()
    };
    assert_eq!(file("render").as_deref(), Some("render.yaml"));
    assert_eq!(
        file("netlify"),
        None,
        "Next.js on Netlify needs no netlify.toml"
    );
}
