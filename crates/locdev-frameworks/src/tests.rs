use std::fs;
use std::path::Path;

use locdev_core::model::{DevServerMode, DevTool, Integration, ProjectKind, RuntimeKind, Stack};

use crate::{Catalog, Overrides, analyze, detect};

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::Builder::new().prefix("My_App").tempdir().unwrap();
    for (file, contents) in files {
        let path = dir.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    dir
}

fn stack(path: &Path) -> Stack {
    analyze(&Catalog::bundled(), path, Overrides::default())
}

fn ids(stack: &Stack) -> Vec<&str> {
    stack.components.iter().map(|c| c.id.as_str()).collect()
}

// ── What detect.rs did before, unchanged ─────────────────────────────────────

#[test]
fn detects_laravel() {
    let dir = project(&[("artisan", ""), ("composer.json", "{}")]);
    let detection = detect(&Catalog::bundled(), dir.path());
    assert_eq!(detection.kind, ProjectKind::Laravel);
    assert!(detection.command.unwrap().contains("artisan serve"));
    assert!(detection.name.starts_with("my-app"));
    assert_eq!(detection.stack.integration, Integration::Native);
}

#[test]
fn detects_next_with_pnpm() {
    let dir = project(&[
        (
            "package.json",
            r#"{"scripts":{"dev":"next dev"},"dependencies":{"next":"15"}}"#,
        ),
        ("pnpm-lock.yaml", ""),
    ]);
    let s = stack(dir.path());
    assert_eq!(s.kind, ProjectKind::Next);
    assert_eq!(
        s.command.as_deref(),
        Some("pnpm run dev --hostname localhost --port {port}")
    );
    // localhost, kept on IPv4: see the definition's note.
    assert!(
        s.port_env
            .iter()
            .any(|e| e == "NODE_OPTIONS=--dns-result-order=ipv4first")
    );
    assert_eq!(s.package_manager, Some(RuntimeKind::Pnpm));
    assert_eq!(s.adapter.as_deref(), Some("nextjs"));
    assert_eq!(s.integration, Integration::Script);
}

#[test]
fn detects_a_vite_spa_and_passes_the_port() {
    let dir = project(&[
        (
            "package.json",
            r#"{"scripts":{"dev":"vite","build":"vite build"},"dependencies":{"react":"19"},"devDependencies":{"vite":"8"}}"#,
        ),
        ("vite.config.ts", ""),
    ]);
    let s = stack(dir.path());
    assert_eq!(s.kind, ProjectKind::Vite);
    assert_eq!(s.framework, "React + Vite");
    assert_eq!(s.dev_tool, Some(DevTool::Vite));
    assert_eq!(
        s.command.as_deref(),
        Some("npm run dev -- --port {port} --strictPort --host 127.0.0.1")
    );
    assert!(s.dev_server.is_none());
    assert_eq!(s.config_files, vec!["vite.config.ts".to_string()]);
    assert!(ids(&s).contains(&"react"));
}

#[test]
fn laravel_with_vite_gets_a_helper_waiting_for_consent() {
    let dir = project(&[
        ("artisan", ""),
        ("composer.json", "{}"),
        (
            "package.json",
            r#"{"scripts":{"dev":"vite","build":"vite build"},"devDependencies":{"vite":"7","laravel-vite-plugin":"3"}}"#,
        ),
        ("vite.config.js", ""),
    ]);
    let s = stack(dir.path());
    assert_eq!(s.kind, ProjectKind::Laravel);
    assert_eq!(s.framework, "Laravel + Vite");
    let helper = s.dev_server.unwrap();
    assert_eq!(helper.tool, DevTool::Vite);
    assert_eq!(helper.mode, DevServerMode::Ask);
    assert_eq!(
        helper.command,
        "npm run dev -- --port {port} --strictPort --host 127.0.0.1"
    );
}

#[test]
fn never_guesses_a_vite_script() {
    let dir = project(&[
        ("artisan", ""),
        ("composer.json", "{}"),
        (
            "package.json",
            r#"{"scripts":{"dev":"concurrently \"vite\" \"php artisan queue\""},"devDependencies":{"vite":"7"}}"#,
        ),
    ]);
    assert!(stack(dir.path()).dev_server.is_none());
}

#[test]
fn detects_nuxt_and_astro_ports() {
    let nuxt = project(&[(
        "package.json",
        r#"{"scripts":{"dev":"nuxt dev"},"dependencies":{"nuxt":"4"}}"#,
    )]);
    let s = stack(nuxt.path());
    assert_eq!(s.kind, ProjectKind::Nuxt);
    assert_eq!(
        s.command.as_deref(),
        Some("npm run dev -- --port {port} --host 127.0.0.1")
    );
    let astro = project(&[
        (
            "package.json",
            r#"{"scripts":{"dev":"astro dev"},"dependencies":{"astro":"5"}}"#,
        ),
        ("bun.lock", ""),
    ]);
    assert_eq!(
        stack(astro.path()).command.as_deref(),
        Some("bun run dev --port {port} --host 127.0.0.1")
    );
}

#[test]
fn static_sites_run_locdevs_static_server() {
    let dir = project(&[("index.html", "<h1>hi</h1>")]);
    let s = stack(dir.path());
    assert_eq!(s.kind, ProjectKind::Static);
    assert_eq!(s.command.as_deref(), Some("locdevd static --port {port}"));
    let public = project(&[("public/index.html", "<h1>hi</h1>")]);
    assert_eq!(
        stack(public.path()).command.as_deref(),
        Some("locdevd static --port {port} --root public")
    );
}

#[test]
fn detects_node_with_npm() {
    let dir = project(&[("package.json", r#"{"scripts":{"start":"node server.js"}}"#)]);
    let d = detect(&Catalog::bundled(), dir.path());
    assert_eq!(d.kind, ProjectKind::Node);
    assert_eq!(d.command.as_deref(), Some("npm run start"));
    assert!(d.stack.components.iter().any(|c| c.id == "javascript"));
}

#[test]
fn detects_php_public_docroot() {
    let dir = project(&[("public/index.php", "<?php")]);
    let s = stack(dir.path());
    assert_eq!(s.kind, ProjectKind::Php);
    assert_eq!(
        s.command.as_deref(),
        Some("php -S 127.0.0.1:{port} -t public")
    );
}

#[test]
fn unknown_has_no_command() {
    let dir = project(&[("README.md", "")]);
    let s = stack(dir.path());
    assert_eq!(s.kind, ProjectKind::Unknown);
    assert_eq!(s.command, None);
    assert_eq!(s.integration, Integration::None);
}

// ── New adapters ─────────────────────────────────────────────────────────────

#[test]
fn bedrock_is_wordpress_served_from_web() {
    let bedrock = project(&[
        ("composer.json", r#"{"name":"roots/bedrock"}"#),
        ("config/application.php", "<?php"),
        ("web/wp-config.php", "<?php"),
        ("web/index.php", "<?php"),
    ]);
    let s = stack(bedrock.path());
    assert_eq!(s.kind, ProjectKind::Wordpress);
    assert_eq!(s.command.as_deref(), Some("php -S 127.0.0.1:{port} -t web"));

    let classic = project(&[("wp-config.php", "<?php"), ("wp-load.php", "<?php")]);
    let s = stack(classic.path());
    assert_eq!(s.kind, ProjectKind::Wordpress);
    assert_eq!(s.command.as_deref(), Some("php -S 127.0.0.1:{port} -t ."));
}

#[test]
fn django_uses_the_venv_and_runserver() {
    let dir = project(&[
        ("manage.py", ""),
        ("requirements.txt", "Django==5.1.2\npython-dotenv>=1\n"),
        (".venv/pyvenv.cfg", "home = /usr/bin\n"),
    ]);
    let s = stack(dir.path());
    assert_eq!(s.adapter.as_deref(), Some("django"));
    assert_eq!(s.framework, "Django");
    if !cfg!(windows) {
        assert_eq!(
            s.command.as_deref(),
            Some("python3 manage.py runserver 127.0.0.1:{port}")
        );
    }
    assert_eq!(s.venv.as_deref(), Some(".venv"));
    let django = s.components.iter().find(|c| c.id == "django").unwrap();
    assert_eq!(django.version.as_deref(), Some("5.1.2"));
    assert!(django.primary);
    let (_, env) = crate::env_spec(&Catalog::bundled(), &s, dir.path()).unwrap();
    assert_eq!(env.files, vec![".env".to_string()]);
}

#[test]
fn flask_needs_an_entry_file() {
    let with = project(&[("requirements.txt", "flask\n"), ("wsgi.py", "")]);
    assert_eq!(
        stack(with.path()).command.as_deref(),
        Some("flask --app wsgi run --host 127.0.0.1 --port {port}")
    );
    let without = project(&[("requirements.txt", "flask\n")]);
    let s = stack(without.path());
    assert_eq!(s.adapter.as_deref(), Some("flask"));
    assert_eq!(s.integration, Integration::Custom);
    assert!(s.command.is_none());
}

#[test]
fn fastapi_from_pyproject() {
    let dir = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"api\"\ndependencies = [\"fastapi[standard]>=0.115\"]\n",
        ),
        ("app/main.py", ""),
    ]);
    let s = stack(dir.path());
    assert_eq!(
        s.command.as_deref(),
        Some("fastapi dev app/main.py --host 127.0.0.1 --port {port}")
    );
}

#[test]
fn rails_with_version_and_dotenv_rules() {
    let dir = project(&[
        (
            "Gemfile",
            "source 'https://rubygems.org'\ngem \"rails\", \"~> 8.0\"\n",
        ),
        ("Gemfile.lock", "GEM\n  specs:\n    rails (8.0.1)\n"),
        ("config/application.rb", ""),
    ]);
    let s = stack(dir.path());
    assert_eq!(s.adapter.as_deref(), Some("rails"));
    if !cfg!(windows) {
        assert_eq!(
            s.command.as_deref(),
            Some("bin/rails server -b 127.0.0.1 -p {port}")
        );
    }
    assert_eq!(s.components[0].version.as_deref(), Some("8.0.1"));
    // No dotenv gem: Rails reads no .env file.
    let (_, env) = crate::env_spec(&Catalog::bundled(), &s, dir.path()).unwrap();
    assert!(env.files.is_empty());
    assert_eq!(env.loader, "none");
}

#[test]
fn spring_boot_passes_the_port_by_env() {
    let dir = project(&[
        (
            "pom.xml",
            "<project><parent><groupId>org.springframework.boot</groupId><artifactId>spring-boot-starter-parent</artifactId><version>3.4.1</version></parent></project>",
        ),
        ("mvnw", ""),
    ]);
    let s = stack(dir.path());
    assert_eq!(s.adapter.as_deref(), Some("spring"));
    assert_eq!(
        s.port_env,
        vec![
            "SERVER_PORT".to_string(),
            "SERVER_ADDRESS=127.0.0.1".to_string()
        ]
    );
    assert_eq!(s.components[0].version.as_deref(), Some("3.4.1"));
    let java = s.components.iter().find(|c| c.id == "java").unwrap();
    assert!(!java.primary);
}

#[test]
fn dotnet_go_rust_hugo() {
    let dotnet = project(&[(
        "Api.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk.Web\"><PropertyGroup><TargetFramework>net9.0</TargetFramework></PropertyGroup></Project>",
    )]);
    let s = stack(dotnet.path());
    assert_eq!(s.command.as_deref(), Some("dotnet run --no-launch-profile"));
    assert_eq!(
        s.port_env,
        vec![
            "ASPNETCORE_URLS=http://127.0.0.1:{port}".to_string(),
            "ASPNETCORE_ENVIRONMENT=Development".to_string()
        ]
    );
    assert_eq!(s.components[0].version.as_deref(), Some("9.0"));

    let go = project(&[("go.mod", "module example.com/app\n\ngo 1.23\n")]);
    let s = stack(go.path());
    assert_eq!(s.command.as_deref(), Some("go run ."));
    assert!(s.notes.iter().any(|n| n.contains("PORT")));

    let rust = project(&[("Cargo.toml", "[package]\nname = \"app\"\n")]);
    assert_eq!(stack(rust.path()).command.as_deref(), Some("cargo run"));
    let workspace = project(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
        ("crates/api/Cargo.toml", "[package]\nname = \"api\"\n"),
    ]);
    let s = stack(workspace.path());
    assert_eq!(s.command, None);
    assert_eq!(s.workspaces[0].path, "crates/api");
    assert_eq!(s.workspaces[0].adapter.as_deref(), Some("rust"));

    let hugo = project(&[("hugo.toml", "baseURL = '/'\n")]);
    assert_eq!(
        stack(hugo.path()).command.as_deref(),
        Some("hugo server --bind 127.0.0.1 --port {port}")
    );
}

#[test]
fn node_frameworks_keep_scripts_and_note_the_port() {
    let nest = project(&[(
        "package.json",
        r#"{"scripts":{"start:dev":"nest start --watch"},"dependencies":{"@nestjs/core":"11","express":"5"}}"#,
    )]);
    let s = stack(nest.path());
    // NestJS (80) wins over Express (65), which it brings along.
    assert_eq!(s.adapter.as_deref(), Some("nestjs"));
    assert_eq!(s.command.as_deref(), Some("npm run start:dev"));
    assert!(ids(&s).contains(&"express"));
    assert!(s.notes.iter().any(|n| n.contains("PORT")));

    let angular = project(&[
        ("angular.json", "{}"),
        (
            "package.json",
            r#"{"scripts":{"start":"ng serve"},"dependencies":{"@angular/core":"19"}}"#,
        ),
    ]);
    assert_eq!(
        stack(angular.path()).command.as_deref(),
        Some("npm run start -- --host 127.0.0.1 --port {port}")
    );

    let gatsby = project(&[(
        "package.json",
        r#"{"dependencies":{"gatsby":"5","react":"18"}}"#,
    )]);
    assert_eq!(
        stack(gatsby.path()).command.as_deref(),
        Some("npx gatsby develop -H 127.0.0.1 -p {port}")
    );
}

#[test]
fn remix_variants_follow_the_script() {
    let vite = project(&[(
        "package.json",
        r#"{"scripts":{"dev":"react-router dev"},"dependencies":{"@react-router/dev":"7","react":"19"}}"#,
    )]);
    let s = stack(vite.path());
    assert_eq!(s.adapter.as_deref(), Some("remix"));
    assert_eq!(
        s.command.as_deref(),
        Some("npm run dev -- --port {port} --strictPort --host 127.0.0.1")
    );
}

#[test]
fn a_vite_dep_without_a_vite_script_falls_back_to_node() {
    let dir = project(&[(
        "package.json",
        r#"{"scripts":{"dev":"node server.js"},"devDependencies":{"vite":"8"}}"#,
    )]);
    let s = stack(dir.path());
    assert_eq!(s.adapter.as_deref(), Some("nodejs"));
    assert_eq!(s.command.as_deref(), Some("npm run dev"));
}

#[test]
fn tailwind_typescript_and_versions_from_node_modules() {
    let dir = project(&[
        (
            "package.json",
            r#"{"scripts":{"dev":"vite"},"devDependencies":{"vite":"^8","tailwindcss":"^4","typescript":"^5"}}"#,
        ),
        ("node_modules/vite/package.json", r#"{"version":"8.3.5"}"#),
        ("tsconfig.json", "{}"),
    ]);
    let s = stack(dir.path());
    let vite = s.components.iter().find(|c| c.id == "vite").unwrap();
    assert_eq!(
        (vite.version.as_deref(), vite.version_source.as_deref()),
        (Some("8.3.5"), Some("node_modules"))
    );
    let tw = s.components.iter().find(|c| c.id == "tailwind").unwrap();
    assert_eq!(tw.version_source.as_deref(), Some("package.json (range)"));
    assert!(ids(&s).contains(&"typescript"));
    assert!(!ids(&s).contains(&"javascript"));
}

#[test]
fn old_versions_get_a_note() {
    let dir = project(&[
        ("artisan", ""),
        (
            "composer.json",
            r#"{"require":{"laravel/framework":"^9.0"}}"#,
        ),
    ]);
    assert!(
        stack(dir.path())
            .notes
            .iter()
            .any(|n| n.contains("older than"))
    );
    let new = project(&[
        ("artisan", ""),
        (
            "composer.json",
            r#"{"require":{"laravel/framework":"^12.0"}}"#,
        ),
    ]);
    assert!(
        !stack(new.path())
            .notes
            .iter()
            .any(|n| n.contains("older than"))
    );
}

#[test]
fn manual_choices_win() {
    let dir = project(&[("requirements.txt", "flask\n"), ("app.py", "")]);
    let catalog = Catalog::bundled();
    let hide = vec!["flask".to_string()];
    let s = analyze(
        &catalog,
        dir.path(),
        Overrides {
            hide: &hide,
            ..Default::default()
        },
    );
    assert_eq!(s.adapter.as_deref(), Some("python"));

    let add = vec!["tailwind".to_string()];
    let s = analyze(
        &catalog,
        dir.path(),
        Overrides {
            add: &add,
            ..Default::default()
        },
    );
    let tw = s.components.iter().find(|c| c.id == "tailwind").unwrap();
    assert!(tw.manual);

    let s = analyze(
        &catalog,
        dir.path(),
        Overrides {
            adapter: Some("fastapi"),
            ..Default::default()
        },
    );
    assert_eq!(s.adapter.as_deref(), Some("fastapi"));
    assert_eq!(
        s.integration,
        Integration::Custom,
        "no main.py: FastAPI can't be started"
    );
}

#[test]
fn monorepo_members_are_listed() {
    let dir = project(&[
        (
            "package.json",
            r#"{"private":true,"workspaces":["apps/*","packages/ui"]}"#,
        ),
        (
            "apps/web/package.json",
            r#"{"scripts":{"dev":"next dev"},"dependencies":{"next":"15"}}"#,
        ),
        (
            "apps/api/package.json",
            r#"{"scripts":{"start:dev":"nest start"},"dependencies":{"@nestjs/core":"11"}}"#,
        ),
        (
            "packages/ui/package.json",
            r#"{"dependencies":{"react":"19"}}"#,
        ),
    ]);
    let s = stack(dir.path());
    let members: Vec<(&str, Option<&str>)> = s
        .workspaces
        .iter()
        .map(|m| (m.path.as_str(), m.adapter.as_deref()))
        .collect();
    assert_eq!(
        members,
        vec![
            ("apps/api", Some("nestjs")),
            ("apps/web", Some("nextjs")),
            ("packages/ui", Some("nodejs"))
        ]
    );
    // Not runnable at the root: no dev script.
    assert!(s.command.is_none());
}

#[test]
fn every_prompt_stack_has_a_definition() {
    let catalog = Catalog::bundled();
    for id in [
        "laravel",
        "nextjs",
        "vue",
        "rails",
        "rust",
        "react",
        "nuxt",
        "svelte",
        "astro",
        "angular",
        "preact",
        "solid",
        "qwik",
        "remix",
        "gatsby",
        "ember",
        "redwood",
        "wordpress",
        "php",
        "nodejs",
        "express",
        "fastify",
        "nestjs",
        "hono",
        "elysia",
        "bun",
        "deno",
        "python",
        "django",
        "flask",
        "fastapi",
        "ruby",
        "go",
        "hugo",
        "java",
        "spring",
        "dotnet",
        "vite",
        "html",
        "javascript",
        "typescript",
    ] {
        assert!(catalog.get(id).is_some(), "{id}");
    }
}

/// The app and the docs render the definitions from this file; it must
/// match the bundled set (`make cli-catalog` rewrites it).
#[test]
fn committed_definitions_summary_is_current() {
    let catalog = Catalog::bundled();
    let summary = serde_json::json!({
        "revision": catalog.revision,
        "definitions": catalog.frameworks.iter().map(crate::info::info).collect::<Vec<_>>(),
    });
    let fresh = serde_json::to_string_pretty(&summary).unwrap() + "\n";
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let update = std::env::var_os("LOCDEV_UPDATE_CATALOG").is_some();
    for copy in [
        "apps/desktop/src/generated/frameworks.json",
        "apps/docs/generated/frameworks.json",
    ] {
        let path = root.join(copy);
        if update {
            std::fs::write(&path, &fresh).unwrap();
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            committed == fresh,
            "{copy} is out of date; run `make cli-catalog`"
        );
    }
}

#[test]
fn laravel_offers_processes_by_what_is_installed() {
    let dir = project(&[
        ("artisan", ""),
        (
            "composer.json",
            r#"{"require":{"laravel/framework":"^12","laravel/horizon":"^5","laravel/reverb":"^1"}}"#,
        ),
    ]);
    let s = stack(dir.path());
    let ids: Vec<&str> = s.processes.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["queue", "horizon", "scheduler", "reverb"]);
    let horizon = s.processes.iter().find(|p| p.id == "horizon").unwrap();
    assert_eq!(horizon.command, "php artisan horizon");
    assert_eq!(horizon.conflicts, vec!["queue".to_string()]);

    let plain = project(&[
        ("artisan", ""),
        (
            "composer.json",
            r#"{"require":{"laravel/framework":"^12"}}"#,
        ),
    ]);
    let ids: Vec<String> = stack(plain.path())
        .processes
        .into_iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(ids, vec!["queue".to_string(), "scheduler".to_string()]);
}

#[test]
fn celery_needs_the_django_module() {
    let dir = project(&[
        (
            "manage.py",
            "os.environ.setdefault('DJANGO_SETTINGS_MODULE', 'shop.settings')\n",
        ),
        ("requirements.txt", "django\ncelery[redis]==5.4\n"),
    ]);
    let s = stack(dir.path());
    let celery = s.processes.iter().find(|p| p.id == "celery").unwrap();
    assert_eq!(celery.command, "celery -A shop worker -l info");
    assert!(s.processes.iter().any(|p| p.id == "beat"));

    let unknown = project(&[("manage.py", ""), ("requirements.txt", "django\ncelery\n")]);
    assert!(stack(unknown.path()).processes.is_empty());
}

#[test]
fn rails_and_storybook_processes() {
    let rails = project(&[
        ("Gemfile", "gem \"rails\"\ngem \"solid_queue\"\n"),
        ("config/application.rb", ""),
        ("bin/jobs", ""),
    ]);
    let s = stack(rails.path());
    assert_eq!(s.processes[0].command, "bin/jobs");
    assert!(s.processes.iter().all(|p| p.id != "sidekiq"));

    let sb = project(&[(
        "package.json",
        r#"{"scripts":{"dev":"vite","storybook":"storybook dev"},"devDependencies":{"vite":"8","storybook":"9"}}"#,
    )]);
    let s = stack(sb.path());
    let p = s.processes.iter().find(|p| p.id == "storybook").unwrap();
    assert!(p.port);
    assert_eq!(
        p.command,
        "npm run storybook -- --port {port} --host 127.0.0.1 --no-open"
    );
}
