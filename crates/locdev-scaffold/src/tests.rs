use std::collections::BTreeMap;

use locdev_core::scaffold::{DatabaseChoice, NewProject, PackageRegistry};

use crate::plan::{valid_name, valid_package};
use crate::{PROGRAMS, plan, starters};

fn project(
    dir: &std::path::Path,
    starter: &str,
    method: &str,
    options: &[(&str, &str)],
) -> NewProject {
    NewProject {
        starter: starter.into(),
        method: method.into(),
        name: "shop".into(),
        parent: dir.to_path_buf(),
        options: options
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect::<BTreeMap<_, _>>(),
        packages: Vec::new(),
        database: None,
        register: true,
        node: None,
        php: None,
    }
}

fn commands(p: &locdev_core::scaffold::ProjectPlan) -> Vec<String> {
    p.steps
        .iter()
        .chain(&p.finish)
        .map(|s| s.display())
        .collect()
}

#[test]
fn every_starter_and_method_plans_with_defaults() {
    let dir = tempfile::tempdir().unwrap();
    for starter in starters() {
        assert!(!starter.methods.is_empty(), "{}", starter.id);
        for option in &starter.options {
            assert!(
                option.choices.is_empty()
                    || option.choices.iter().any(|c| c.value == option.default),
                "{}: default of {}",
                starter.id,
                option.id
            );
        }
        for method in &starter.methods {
            let plan = plan(&project(dir.path(), &starter.id, &method.id, &[]))
                .unwrap_or_else(|e| panic!("{}/{}: {e}", starter.id, method.id));
            assert!(!plan.steps.is_empty());
            assert_eq!(plan.dir, dir.path().join("shop"));
            for step in &plan.steps {
                assert!(
                    PROGRAMS.contains(&step.program.as_str()) || step.program.contains(".venv"),
                    "{}",
                    step.program
                );
            }
        }
    }
}

#[test]
fn laravel_installer_uses_the_documented_flags() {
    let dir = tempfile::tempdir().unwrap();
    let p = plan(&project(
        dir.path(),
        "laravel",
        "installer",
        &[
            ("starter_kit", "true"),
            ("kit", "react"),
            ("auth", "workos"),
            ("testing", "pest"),
            ("node", "pnpm"),
            ("boost", "false"),
            ("git", "false"),
        ],
    ))
    .unwrap();
    assert_eq!(
        commands(&p),
        [
            "laravel new shop --no-interaction --react --workos --pest --database=sqlite --pnpm --no-boost"
        ]
    );
    // No starter kit: a stack other than Blade is the blank kit, no auth.
    let p = plan(&project(
        dir.path(),
        "laravel",
        "installer",
        &[("stack", "vue"), ("git", "false")],
    ))
    .unwrap();
    assert_eq!(
        commands(&p),
        [
            "laravel new shop --no-interaction --vue --no-authentication --pest --database=sqlite --npm --boost"
        ]
    );
    // Livewire kit with class components: teams isn't asked then.
    let kit = [
        ("starter_kit", "true"),
        ("kit", "livewire"),
        ("single_file", "false"),
        ("teams", "true"),
        ("git", "false"),
    ];
    let p = plan(&project(dir.path(), "laravel", "installer", &kit)).unwrap();
    assert_eq!(
        commands(&p),
        [
            "laravel new shop --no-interaction --livewire --livewire-class-components --pest --database=sqlite --npm --boost"
        ]
    );
}

#[test]
fn laravel_asks_like_the_installer() {
    let starter = starters().into_iter().find(|s| s.id == "laravel").unwrap();
    let asked = |chosen: &[(&str, &str)]| -> Vec<String> {
        let chosen: BTreeMap<String, String> = chosen
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        starter
            .options
            .iter()
            .filter(|o| starter.asks(o, "installer", &chosen))
            .map(|o| o.id.clone())
            .collect()
    };
    let none = asked(&[]);
    assert!(none.contains(&"stack".to_string()) && !none.contains(&"auth".to_string()));
    let kit = asked(&[("starter_kit", "true"), ("kit", "livewire")]);
    for id in ["kit", "auth", "single_file", "teams"] {
        assert!(kit.contains(&id.to_string()), "{id} in {kit:?}");
    }
    assert!(!kit.contains(&"stack".to_string()));
    let workos = asked(&[("starter_kit", "true"), ("auth", "workos")]);
    assert!(!workos.contains(&"teams".to_string()) && !workos.contains(&"single_file".to_string()));
}

#[test]
fn laravel_with_composer_picks_the_kit_package_and_builds() {
    let dir = tempfile::tempdir().unwrap();
    let p = plan(&project(
        dir.path(),
        "laravel",
        "composer",
        &[("stack", "vue")],
    ))
    .unwrap();
    assert_eq!(
        commands(&p),
        [
            "composer create-project laravel/blank-vue-starter-kit shop --no-interaction --stability=dev",
            "npm install",
            "npm run build"
        ]
    );
    let p = plan(&project(dir.path(), "laravel", "composer", &[])).unwrap();
    assert_eq!(
        commands(&p),
        ["composer create-project laravel/laravel shop --no-interaction"]
    );
    let p = plan(&project(
        dir.path(),
        "laravel",
        "composer",
        &[
            ("starter_kit", "true"),
            ("kit", "react"),
            ("teams", "true"),
            ("node", "none"),
        ],
    ))
    .unwrap();
    assert_eq!(
        commands(&p),
        [
            "composer create-project laravel/react-starter-kit:dev-teams shop --no-interaction --stability=dev"
        ]
    );
}

#[test]
fn a_locdev_database_sets_the_driver_and_migrates() {
    let dir = tempfile::tempdir().unwrap();
    let mut new = project(
        dir.path(),
        "laravel",
        "installer",
        &[("database", "sqlite")],
    );
    new.database = Some(DatabaseChoice {
        service: "postgres".into(),
        instance: "db".into(),
    });
    let p = plan(&new).unwrap();
    assert!(p.steps[0].args.contains(&"--database=pgsql".to_string()));
    assert_eq!(
        commands(&p).last().unwrap(),
        "php artisan migrate --no-interaction"
    );
    assert!(p.actions.iter().any(|a| a.contains("database 'shop'")));
    // Only starters that support it.
    let mut vite = project(dir.path(), "vite", "npm", &[]);
    vite.database = new.database.clone();
    assert!(plan(&vite).is_err());
}

#[test]
fn javascript_starters_follow_each_tools_cli() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = |starter: &str, method: &str, options: &[(&str, &str)]| {
        commands(&plan(&project(dir.path(), starter, method, options)).unwrap())
    };
    assert_eq!(
        cmd("vite", "npm", &[("framework", "vue")]),
        [
            "npm create --yes vite@latest shop -- --template vue-ts --no-interactive",
            "npm install"
        ]
    );
    assert_eq!(
        cmd("vite", "pnpm", &[("language", "js")]),
        [
            "pnpm dlx create-vite@latest shop --template react --no-interactive",
            "pnpm install"
        ]
    );
    assert_eq!(
        cmd("nextjs", "pnpm", &[("linter", "biome"), ("git", "false")]),
        [
            "pnpm dlx create-next-app@latest shop --yes --ts --tailwind --biome --app --no-src-dir --use-pnpm --disable-git"
        ]
    );
    assert_eq!(
        cmd("nuxt", "npm", &[]),
        [
            "npm create --yes nuxt@latest shop -- --template minimal --packageManager npm --gitInit --install --no-modules"
        ]
    );
    assert_eq!(
        cmd("astro", "bun", &[("git", "false")]),
        ["bunx create-astro@latest shop --template basics --install --no-git --yes --skip-houston"]
    );
    assert_eq!(
        cmd("sveltekit", "npm", &[]),
        [
            "npx --yes sv@latest create shop --template minimal --types ts --no-add-ons --install npm"
        ]
    );
    assert_eq!(
        cmd("nestjs", "npm", &[("git", "false")]),
        ["npx --yes @nestjs/cli@latest new shop --package-manager npm --language ts --skip-git"]
    );
}

#[test]
fn other_stacks() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = |starter: &str, method: &str, options: &[(&str, &str)]| {
        commands(&plan(&project(dir.path(), starter, method, options)).unwrap())
    };
    assert_eq!(cmd("symfony", "cli", &[]), ["symfony new shop --webapp"]);
    assert_eq!(
        cmd("symfony", "composer", &[]),
        [
            "composer create-project symfony/skeleton shop --no-interaction",
            "composer require webapp --no-interaction"
        ]
    );
    assert_eq!(
        cmd(
            "rails",
            "rails",
            &[("database", "postgresql"), ("css", "tailwind")]
        ),
        ["rails new shop --database=postgresql --javascript=importmap --css=tailwind"]
    );
    assert_eq!(
        cmd("dotnet", "dotnet", &[("template", "webapi")]),
        ["dotnet new webapi -o shop"]
    );
    let django = cmd("django", "venv", &[]);
    assert_eq!(django.len(), 3);
    assert!(django[0].ends_with("-m venv shop/.venv"));
    assert!(django[2].ends_with("-m django startproject config ."));
}

#[test]
fn packages_are_added_with_the_projects_manager() {
    let dir = tempfile::tempdir().unwrap();
    let mut new = project(dir.path(), "laravel", "installer", &[]);
    new.packages = vec![
        "laravel/horizon".into(),
        "spatie/laravel-permission:^6".into(),
    ];
    assert_eq!(
        commands(&plan(&new).unwrap()).last().unwrap(),
        "composer require laravel/horizon spatie/laravel-permission:^6"
    );
    let mut next = project(dir.path(), "nextjs", "pnpm", &[]);
    next.packages = vec!["zod".into(), "@tanstack/react-query@5".into()];
    assert_eq!(
        commands(&plan(&next).unwrap()).last().unwrap(),
        "pnpm add zod @tanstack/react-query@5"
    );
    next.packages = vec!["zod; rm -rf ~".into()];
    assert!(plan(&next).is_err());
}

#[test]
fn names_folders_and_values_are_checked() {
    assert!(valid_name("my-shop_2"));
    for bad in ["", "Shop", "-x", "../x", "a b", "x/y", &"a".repeat(64)] {
        assert!(!valid_name(bad), "{bad}");
    }
    assert!(valid_package(PackageRegistry::Npm, "@scope/pkg@^1.2"));
    assert!(!valid_package(PackageRegistry::Npm, "--registry=evil"));
    assert!(!valid_package(PackageRegistry::Packagist, "nope"));
    assert!(valid_package(PackageRegistry::Packagist, "vendor/pkg:^2.0"));

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("shop")).unwrap();
    assert!(
        plan(&project(dir.path(), "vite", "npm", &[]))
            .unwrap_err()
            .contains("already exists")
    );
    let mut relative = project(dir.path(), "vite", "npm", &[]);
    relative.parent = "code".into();
    assert!(plan(&relative).is_err());
    assert!(
        plan(&project(
            dir.path(),
            "vite",
            "npm",
            &[("framework", "angular")]
        ))
        .is_err()
    );
    assert!(plan(&project(dir.path(), "vite", "yarn", &[])).is_err());
}

#[test]
fn search_results_parse() {
    let npm: serde_json::Value = serde_json::json!({"objects": [{"package": {"name": "zod", "version": "4.1.0",
        "description": "TypeScript-first schema validation", "links": {"npm": "https://www.npmjs.com/package/zod"}},
        "downloads": {"monthly": 1000}}]});
    let hits = crate::search::parse_npm(&npm);
    assert_eq!(hits[0].name, "zod");
    assert_eq!(hits[0].downloads, Some(1000));
    let packagist = serde_json::json!({"results": [{"name": "laravel/horizon", "description": "Dashboard",
        "url": "https://packagist.org/packages/laravel/horizon", "downloads": 5}, {"name": "x/y", "virtual": true}]});
    let hits = crate::search::parse_packagist(&packagist);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].name, "laravel/horizon");
}

#[test]
fn env_lines_are_replaced_uncommented_or_appended() {
    let env = "APP_NAME=Shop\nDB_CONNECTION=sqlite\n# DB_HOST=127.0.0.1\n# DB_PORT=3306\nDB_HOSTNAME_X=keep\n";
    let pairs = [
        ("DB_CONNECTION".to_string(), "pgsql".to_string()),
        ("DB_HOST".to_string(), "127.0.0.1".to_string()),
        ("DB_PORT".to_string(), "5433".to_string()),
        ("DB_DATABASE".to_string(), "shop".to_string()),
    ];
    assert_eq!(
        crate::plan::set_env(env, &pairs),
        "APP_NAME=Shop\nDB_CONNECTION=pgsql\nDB_HOST=127.0.0.1\nDB_PORT=5433\nDB_HOSTNAME_X=keep\nDB_DATABASE=shop\n"
    );
}

/// The catalog the app's mock and the docs read; `make cli-catalog` updates it.
#[test]
fn committed_starters_are_current() {
    let fresh = serde_json::to_string_pretty(&starters()).unwrap() + "\n";
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let update = std::env::var_os("LOCDEV_UPDATE_CATALOG").is_some();
    for copy in [
        "apps/desktop/src/generated/starters.json",
        "apps/docs/generated/starters.json",
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
fn vue_and_angular_follow_their_clis() {
    let dir = tempfile::tempdir().unwrap();
    let vue = plan(&project(dir.path(), "vue", "pnpm", &[("pinia", "true")])).unwrap();
    assert_eq!(
        commands(&vue),
        [
            "pnpm dlx create-vue@latest shop --ts --router --pinia --eslint",
            "pnpm install",
        ]
    );
    let plain = plan(&project(
        dir.path(),
        "vue",
        "npm",
        &[("language", "js"), ("router", "false"), ("eslint", "false")],
    ))
    .unwrap();
    assert_eq!(commands(&plain)[0], "npm create --yes vue@latest shop --");
    assert!(plain.notes.iter().any(|n| n.contains("asks its questions")));

    let angular = plan(&project(
        dir.path(),
        "angular",
        "npm",
        &[("style", "scss"), ("git", "false")],
    ))
    .unwrap();
    assert_eq!(
        commands(&angular),
        [
            "NG_CLI_ANALYTICS=false npx --yes @angular/cli@latest new shop --defaults --style=scss --routing=true --ssr=false --package-manager=npm --ai-config=none --skip-git"
        ]
    );
    let bun = plan(&project(dir.path(), "angular", "bun", &[])).unwrap();
    assert!(
        commands(&bun)[0].starts_with("NG_CLI_ANALYTICS=false bunx @angular/cli@latest new shop")
    );
}

#[test]
fn starters_without_a_generator_write_locdevs_files_first() {
    let dir = tempfile::tempdir().unwrap();
    let shop = dir.path().join("shop");

    let php = plan(&project(dir.path(), "php", "files", &[])).unwrap();
    assert!(php.make_dir);
    assert_eq!(php.files[0].path, "public/index.php");
    assert!(php.files[0].contents.contains("PHP_VERSION"));
    assert_eq!(commands(&php), ["git init"]);
    assert!(php.steps.iter().all(|s| s.cwd == shop));

    let flask = plan(&project(dir.path(), "flask", "venv", &[("git", "false")])).unwrap();
    let paths: Vec<&str> = flask.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["app.py", "requirements.txt", ".gitignore"]);
    assert_eq!(flask.files[1].contents, "flask\n");
    let python = if cfg!(windows) { "python" } else { "python3" };
    assert_eq!(flask.steps[0].display(), format!("{python} -m venv .venv"));
    assert!(flask.steps[1].program.contains(".venv"));
    assert_eq!(
        flask.steps[1].args,
        ["-m", "pip", "install", "-r", "requirements.txt"]
    );
    assert_eq!(flask.steps.len(), 2);

    let fastapi = plan(&project(dir.path(), "fastapi", "venv", &[])).unwrap();
    assert_eq!(fastapi.files[0].path, "main.py");
    assert_eq!(fastapi.files[1].contents, "fastapi[standard]\n");
    assert_eq!(commands(&fastapi).last().unwrap(), "git init");

    let axum = plan(&project(dir.path(), "rust", "cargo", &[])).unwrap();
    assert_eq!(axum.files[0].path, "src/main.rs");
    assert!(axum.files[0].contents.contains("axum::serve"));
    assert_eq!(
        commands(&axum),
        [
            "cargo init --name shop --vcs git",
            "cargo add axum",
            "cargo add tokio --features full",
        ]
    );
    let actix = plan(&project(
        dir.path(),
        "rust",
        "cargo",
        &[("framework", "actix"), ("git", "false")],
    ))
    .unwrap();
    assert!(actix.files[0].contents.contains("HttpServer"));
    assert_eq!(
        commands(&actix),
        ["cargo init --name shop --vcs none", "cargo add actix-web"]
    );
    let mut digits = project(dir.path(), "rust", "cargo", &[]);
    digits.name = "1shop".into();
    assert!(plan(&digits).unwrap_err().contains("letter"));

    let wordpress = plan(&project(
        dir.path(),
        "wordpress",
        "wpcli",
        &[("locale", "pt_BR")],
    ))
    .unwrap();
    assert!(wordpress.make_dir && wordpress.files.is_empty());
    assert_eq!(commands(&wordpress), ["wp core download --locale=pt_BR"]);

    // Generators make their own folder; nothing is written by LocDev.
    let next = plan(&project(dir.path(), "nextjs", "npm", &[])).unwrap();
    assert!(!next.make_dir && next.files.is_empty());
}

#[test]
fn starter_files_are_found_by_detection() {
    // What LocDev writes must be what the framework definitions recognize,
    // so the new site runs with the right command.
    let catalog = locdev_frameworks::Catalog::bundled();
    for (starter, option, adapter) in [
        ("php", None, "php"),
        ("flask", None, "flask"),
        ("fastapi", None, "fastapi"),
        ("rust", Some(("framework", "axum")), "rust"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let options: Vec<(&str, &str)> = option.into_iter().collect();
        let method = &starters()
            .into_iter()
            .find(|s| s.id == starter)
            .unwrap()
            .methods[0]
            .id
            .clone();
        let p = plan(&project(dir.path(), starter, method, &options)).unwrap();
        std::fs::create_dir(&p.dir).unwrap();
        for file in &p.files {
            let path = p.dir.join(&file.path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, &file.contents).unwrap();
        }
        if starter == "rust" {
            // What `cargo init` adds.
            std::fs::write(p.dir.join("Cargo.toml"), "[package]\nname = \"shop\"\n").unwrap();
        }
        let stack = locdev_frameworks::analyze(&catalog, &p.dir, Default::default());
        assert_eq!(stack.adapter.as_deref(), Some(adapter), "{starter}");
        assert!(stack.command.is_some(), "{starter} has a dev command");
    }
}
