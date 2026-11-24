//! From a [`NewProject`] to the exact commands. The daemon recomputes this
//! when the user confirms, so what runs is what was shown.

use std::path::{Path, PathBuf};

use locdev_core::scaffold::{NewProject, PackageRegistry, PlannedFile, ProjectPlan, Starter, Step};

use crate::catalog::starters;

/// A folder name that is also a site name: lowercase, digits, `-` and `_`.
pub fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit())
        && name.len() <= 63
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// npm (`name`, `@scope/name`) and Composer (`vendor/name`) package names,
/// with an optional version constraint after `@` (npm) or `:` (Composer).
pub fn valid_package(registry: PackageRegistry, spec: &str) -> bool {
    let ok = |s: &str| {
        !s.is_empty()
            && !s.starts_with(['.', '-'])
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    let constraint = |s: &str| {
        !s.is_empty()
            && s.len() <= 40
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "^~*.<>=|- ".contains(c))
            && !s.starts_with('-')
    };
    match registry {
        PackageRegistry::Npm => {
            let (scope, rest) = match spec.strip_prefix('@') {
                Some(rest) => match rest.split_once('/') {
                    Some((scope, rest)) => (Some(scope), rest),
                    None => return false,
                },
                None => (None, spec),
            };
            let (name, version) = match rest.split_once('@') {
                Some((name, version)) => (name, Some(version)),
                None => (rest, None),
            };
            scope.is_none_or(ok)
                && ok(name)
                && name.chars().all(|c| !c.is_ascii_uppercase())
                && version.is_none_or(constraint)
        }
        PackageRegistry::Packagist => {
            let (name, version) = match spec.split_once(':') {
                Some((name, version)) => (name, Some(version)),
                None => (spec, None),
            };
            matches!(name.split_once('/'), Some((vendor, package)) if ok(vendor) && ok(package))
                && name.chars().all(|c| !c.is_ascii_uppercase())
                && version.is_none_or(constraint)
        }
    }
}

fn opt<'a>(project: &'a NewProject, starter: &'a Starter, id: &str) -> &'a str {
    if let Some(value) = project.options.get(id) {
        return value;
    }
    starter
        .options
        .iter()
        .find(|o| o.id == id)
        .map(|o| o.default.as_str())
        .unwrap_or("")
}

fn on(project: &NewProject, starter: &Starter, id: &str) -> bool {
    opt(project, starter, id) == "true"
}

fn step(label: &str, cwd: &Path, program: &str, args: &[&str]) -> Step {
    Step {
        label: label.into(),
        program: program.into(),
        args: args.iter().map(|a| (*a).to_string()).collect(),
        cwd: cwd.to_path_buf(),
        env: Vec::new(),
    }
}

/// `npx` / `pnpm dlx` / `bunx` for a package's binary.
fn runner(method: &str) -> (&'static str, Vec<&'static str>) {
    match method {
        "pnpm" => ("pnpm", vec!["dlx"]),
        "bun" => ("bunx", vec![]),
        _ => ("npx", vec!["--yes"]),
    }
}

/// `npm create x@latest dir -- flags` (the form each tool documents; npm
/// needs `--` before the flags, and `--yes` to skip its download question). pnpm and Bun run the same `create-x`
/// package directly, which takes the flags as they are.
fn create(method: &str, package: &str, name: &str, flags: &[&str]) -> (String, Vec<String>) {
    let mut args = match method {
        "pnpm" => vec![
            "dlx".to_string(),
            format!("create-{package}@latest"),
            name.to_string(),
        ],
        "bun" => vec![format!("create-{package}@latest"), name.to_string()],
        // `--yes`: npm would otherwise stop to ask before downloading
        // create-x (and nothing answers it when run with `locdev new … -y`).
        _ => vec![
            "create".to_string(),
            "--yes".to_string(),
            format!("{package}@latest"),
            name.to_string(),
            "--".to_string(),
        ],
    };
    args.extend(flags.iter().map(|f| (*f).to_string()));
    let program = match method {
        "pnpm" => "pnpm",
        "bun" => "bunx",
        _ => "npm",
    };
    (program.into(), args)
}

pub fn plan(project: &NewProject) -> Result<ProjectPlan, String> {
    let all = starters();
    let starter = all
        .iter()
        .find(|s| s.id == project.starter)
        .ok_or_else(|| format!("unknown starter '{}'", project.starter))?;
    let method = starter
        .methods
        .iter()
        .find(|m| m.id == project.method)
        .ok_or_else(|| {
            format!(
                "{} can't be created with '{}'",
                starter.name, project.method
            )
        })?;
    for (id, value) in &project.options {
        let Some(option) = starter.options.iter().find(|o| &o.id == id) else {
            return Err(format!("{} has no option '{id}'", starter.name));
        };
        let valid = if option.choices.is_empty() {
            value == "true" || value == "false"
        } else {
            option.choices.iter().any(|c| &c.value == value)
        };
        if !valid {
            return Err(format!("'{value}' isn't a value for {}", option.label));
        }
    }
    if !valid_name(&project.name) {
        return Err("the name may only have lowercase letters, digits, '-' and '_', starting with a letter or digit".into());
    }
    if !project.parent.is_absolute() || !project.parent.is_dir() {
        return Err(format!("{} is not a folder", project.parent.display()));
    }
    let dir = project.parent.join(&project.name);
    if dir.exists() {
        return Err(format!(
            "{} already exists; LocDev never creates over a folder",
            dir.display()
        ));
    }
    let registry = starter.packages;
    if !project.packages.is_empty() {
        let Some(registry) = registry else {
            return Err(format!(
                "{} has no package registry to add packages from",
                starter.name
            ));
        };
        if let Some(bad) = project
            .packages
            .iter()
            .find(|p| !valid_package(registry, p))
        {
            return Err(format!("'{bad}' is not a package name"));
        }
    }
    if project.database.is_some() && !starter.database {
        return Err(format!(
            "{} can't be pointed at a LocDev database yet",
            starter.name
        ));
    }
    if let Some(db) = &project.database
        && !["postgres", "mysql", "mariadb"].contains(&db.service.as_str())
    {
        return Err(format!("'{}' is not a database LocDev runs", db.service));
    }

    let parent = project.parent.as_path();
    let name = project.name.as_str();
    let m = method.id.as_str();
    let mut steps = Vec::new();
    let mut notes = Vec::new();
    let mut node_pm: Option<&str> = None;
    // Starters without a generator: LocDev makes the folder and writes its
    // own files first, then the official tools run inside it.
    let mut make_dir = false;
    let mut files: Vec<PlannedFile> = Vec::new();
    let git = || step("Initialize a Git repository", &dir, "git", &["init"]);

    match starter.id.as_str() {
        "php" => {
            make_dir = true;
            files.push(template("public/index.php", PHP_INDEX));
            if on(project, starter, "git") {
                steps.push(git());
            }
        }
        "wordpress" => {
            make_dir = true;
            let locale = format!("--locale={}", opt(project, starter, "locale"));
            steps.push(step(
                "Download WordPress",
                &dir,
                "wp",
                &["core", "download", &locale],
            ));
            notes.push(
                "WordPress asks for its database and admin account in the browser on the first visit (or run `wp config create` and `wp core install`)."
                    .into(),
            );
        }
        "vue" => {
            let mut flags = Vec::new();
            for (option, flag) in [
                ("router", "--router"),
                ("pinia", "--pinia"),
                ("vitest", "--vitest"),
                ("eslint", "--eslint"),
            ] {
                if on(project, starter, option) {
                    flags.push(flag);
                }
            }
            if opt(project, starter, "language") == "ts" {
                flags.insert(0, "--ts");
            }
            if flags.is_empty() {
                // create-vue only skips its questions when a feature flag is given.
                notes.push(
                    "With no feature chosen, create-vue asks its questions in the terminal.".into(),
                );
            }
            let (program, args) = create(m, "vue", name, &flags);
            steps.push(Step {
                label: "Create the project".into(),
                program,
                args,
                cwd: parent.into(),
                env: Vec::new(),
            });
            node_pm = Some(m);
        }
        "angular" => {
            let (program, base) = runner(m);
            let mut args: Vec<String> = base.iter().map(|s| (*s).to_string()).collect();
            args.extend([
                "@angular/cli@latest".to_string(),
                "new".into(),
                name.into(),
                "--defaults".into(),
                format!("--style={}", opt(project, starter, "style")),
                format!("--routing={}", on(project, starter, "routing")),
                format!("--ssr={}", on(project, starter, "ssr")),
                format!("--package-manager={m}"),
                "--ai-config=none".into(),
            ]);
            if !on(project, starter, "git") {
                args.push("--skip-git".into());
            }
            steps.push(Step {
                label: "Create the workspace".into(),
                program: program.into(),
                args,
                cwd: parent.into(),
                // The CLI's documented override: no usage data, and no
                // question about it in the middle of the run.
                env: vec![("NG_CLI_ANALYTICS".into(), "false".into())],
            });
        }
        "flask" | "fastapi" => {
            make_dir = true;
            let (file, contents, requirements) = if starter.id == "flask" {
                ("app.py", FLASK_APP, FLASK_REQUIREMENTS)
            } else {
                ("main.py", FASTAPI_MAIN, FASTAPI_REQUIREMENTS)
            };
            files.push(template(file, contents));
            files.push(template("requirements.txt", requirements));
            files.push(template(".gitignore", PYTHON_GITIGNORE));
            let python = if cfg!(windows) { "python" } else { "python3" };
            steps.push(step(
                "Create the virtual environment",
                &dir,
                python,
                &["-m", "venv", ".venv"],
            ));
            steps.push(Step {
                label: format!("Install {}", starter.name),
                program: venv_python(&dir).to_string_lossy().into_owned(),
                args: ["-m", "pip", "install", "-r", "requirements.txt"]
                    .map(String::from)
                    .to_vec(),
                cwd: dir.clone(),
                env: Vec::new(),
            });
            if on(project, starter, "git") {
                steps.push(git());
            }
            notes.push("LocDev runs it with the project's .venv.".into());
        }
        "rust" => {
            if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
                return Err("a Rust package name must start with a letter".into());
            }
            make_dir = true;
            let axum = opt(project, starter, "framework") == "axum";
            files.push(template(
                "src/main.rs",
                if axum { AXUM_MAIN } else { ACTIX_MAIN },
            ));
            let vcs = if on(project, starter, "git") {
                "git"
            } else {
                "none"
            };
            // `cargo init` keeps an existing src/main.rs as the binary.
            steps.push(step(
                "Create the Cargo package",
                &dir,
                "cargo",
                &["init", "--name", name, "--vcs", vcs],
            ));
            if axum {
                steps.push(step("Add Axum", &dir, "cargo", &["add", "axum"]));
                steps.push(step(
                    "Add Tokio",
                    &dir,
                    "cargo",
                    &["add", "tokio", "--features", "full"],
                ));
            } else {
                steps.push(step("Add Actix Web", &dir, "cargo", &["add", "actix-web"]));
            }
            notes.push("The server reads its port from PORT, which LocDev sets.".into());
        }
        "laravel" => {
            let starter_kit = on(project, starter, "starter_kit");
            // Without a starter kit, a frontend stack other than Blade is the
            // blank kit (no auth), as `laravel new` does.
            let kit = if starter_kit {
                opt(project, starter, "kit")
            } else {
                opt(project, starter, "stack")
            };
            let workos = starter_kit && opt(project, starter, "auth") == "workos";
            let class_components =
                starter_kit && kit == "livewire" && !workos && !on(project, starter, "single_file");
            // The installer doesn't ask for teams with class components.
            let teams =
                starter_kit && !workos && !class_components && on(project, starter, "teams");
            let node = opt(project, starter, "node");
            // A LocDev database decides the driver.
            let driver = match project.database.as_ref().map(|d| d.service.as_str()) {
                Some("postgres") => "pgsql",
                Some("mysql") => "mysql",
                Some("mariadb") => "mariadb",
                _ => opt(project, starter, "database"),
            };
            if m == "installer" {
                let mut args: Vec<String> =
                    vec!["new".into(), name.into(), "--no-interaction".into()];
                if kit != "blade" {
                    args.push(format!("--{kit}"));
                    if !starter_kit {
                        args.push("--no-authentication".into());
                    }
                }
                if workos {
                    args.push("--workos".into());
                }
                if class_components {
                    args.push("--livewire-class-components".into());
                }
                if teams {
                    args.push("--teams".into());
                }
                args.push(format!("--{}", opt(project, starter, "testing")));
                args.push(format!("--database={driver}"));
                args.push(if node == "none" {
                    "--no-node".into()
                } else {
                    format!("--{node}")
                });
                args.push(if on(project, starter, "boost") {
                    "--boost".into()
                } else {
                    "--no-boost".into()
                });
                if on(project, starter, "git") {
                    args.push("--git".into());
                }
                steps.push(Step {
                    label: "Create the project".into(),
                    program: "laravel".into(),
                    args,
                    cwd: parent.into(),
                    env: Vec::new(),
                });
            } else {
                // The installer's `getStarterKit` plus the branch it appends.
                let package = if kit == "blade" {
                    "laravel/laravel".to_string()
                } else if !starter_kit {
                    format!("laravel/blank-{kit}-starter-kit")
                } else {
                    let branch = match (workos, teams, class_components) {
                        (true, _, _) => ":dev-workos",
                        (_, true, _) => ":dev-teams",
                        (_, _, true) => ":dev-components",
                        _ => "",
                    };
                    format!("laravel/{kit}-starter-kit{branch}")
                };
                let mut args = vec!["create-project", &package, name, "--no-interaction"];
                if kit != "blade" {
                    args.push("--stability=dev");
                }
                steps.push(step("Create the project", parent, "composer", &args));
                if kit != "blade" && node != "none" {
                    node_pm = Some(node);
                }
                if project.database.is_none() && driver != "sqlite" {
                    notes.push(
                        "Composer creates it with SQLite; set DB_* in .env for another database."
                            .into(),
                    );
                }
            }
        }
        "symfony" => {
            let webapp = opt(project, starter, "type") == "webapp";
            if m == "cli" {
                let mut args = vec!["new", name];
                if webapp {
                    args.push("--webapp");
                }
                steps.push(step("Create the project", parent, "symfony", &args));
            } else {
                steps.push(step(
                    "Create the project",
                    parent,
                    "composer",
                    &[
                        "create-project",
                        "symfony/skeleton",
                        name,
                        "--no-interaction",
                    ],
                ));
                if webapp {
                    steps.push(step(
                        "Add the web app pack",
                        &dir,
                        "composer",
                        &["require", "webapp", "--no-interaction"],
                    ));
                }
            }
        }
        "bedrock" => {
            steps.push(step(
                "Create the project",
                parent,
                "composer",
                &["create-project", "roots/bedrock", name, "--no-interaction"],
            ));
            notes.push("Bedrock reads the database and URLs from .env (DB_NAME, DB_USER, WP_HOME…); fill it in before installing WordPress.".into());
        }
        "nextjs" => {
            let (program, base) = runner(m);
            let mut args: Vec<String> = base.iter().map(|s| (*s).to_string()).collect();
            args.extend(["create-next-app@latest".into(), name.into(), "--yes".into()]);
            args.push(if opt(project, starter, "language") == "js" {
                "--js".into()
            } else {
                "--ts".into()
            });
            args.push(if on(project, starter, "tailwind") {
                "--tailwind".into()
            } else {
                "--no-tailwind".into()
            });
            args.push(match opt(project, starter, "linter") {
                "biome" => "--biome".into(),
                "none" => "--no-linter".into(),
                _ => "--eslint".into(),
            });
            args.push(if on(project, starter, "app") {
                "--app".into()
            } else {
                "--no-app".into()
            });
            args.push(if on(project, starter, "src") {
                "--src-dir".into()
            } else {
                "--no-src-dir".into()
            });
            args.push(format!("--use-{}", if m == "bun" { "bun" } else { m }));
            if !on(project, starter, "git") {
                args.push("--disable-git".into());
            }
            steps.push(Step {
                label: "Create the project".into(),
                program: program.into(),
                args,
                cwd: parent.into(),
                env: Vec::new(),
            });
        }
        "vite" => {
            let framework = opt(project, starter, "framework");
            let template = if opt(project, starter, "language") == "ts" {
                format!("{framework}-ts")
            } else {
                framework.to_string()
            };
            let (program, args) = create(
                m,
                "vite",
                name,
                &["--template", &template, "--no-interactive"],
            );
            steps.push(Step {
                label: "Create the project".into(),
                program,
                args,
                cwd: parent.into(),
                env: Vec::new(),
            });
            node_pm = Some(m);
        }
        "nuxt" => {
            let git = if on(project, starter, "git") {
                "--gitInit"
            } else {
                "--no-gitInit"
            };
            let (program, args) = create(
                m,
                "nuxt",
                name,
                &[
                    "--template",
                    "minimal",
                    "--packageManager",
                    m,
                    git,
                    "--install",
                    "--no-modules",
                ],
            );
            steps.push(Step {
                label: "Create the project".into(),
                program,
                args,
                cwd: parent.into(),
                env: Vec::new(),
            });
        }
        "astro" => {
            let git = if on(project, starter, "git") {
                "--git"
            } else {
                "--no-git"
            };
            let template = opt(project, starter, "template");
            let (program, args) = create(
                m,
                "astro",
                name,
                &[
                    "--template",
                    template,
                    "--install",
                    git,
                    "--yes",
                    "--skip-houston",
                ],
            );
            steps.push(Step {
                label: "Create the project".into(),
                program,
                args,
                cwd: parent.into(),
                env: Vec::new(),
            });
        }
        "sveltekit" => {
            let (program, base) = runner(m);
            let mut args: Vec<String> = base.iter().map(|s| (*s).to_string()).collect();
            args.extend(
                [
                    "sv@latest",
                    "create",
                    name,
                    "--template",
                    opt(project, starter, "template"),
                    "--types",
                    opt(project, starter, "types"),
                    "--no-add-ons",
                    "--install",
                    m,
                ]
                .map(String::from),
            );
            steps.push(Step {
                label: "Create the project".into(),
                program: program.into(),
                args,
                cwd: parent.into(),
                env: Vec::new(),
            });
        }
        "nestjs" => {
            let (program, base) = runner(m);
            let mut args: Vec<String> = base.iter().map(|s| (*s).to_string()).collect();
            args.extend(
                [
                    "@nestjs/cli@latest",
                    "new",
                    name,
                    "--package-manager",
                    m,
                    "--language",
                ]
                .map(String::from),
            );
            args.push(opt(project, starter, "language").into());
            if !on(project, starter, "git") {
                args.push("--skip-git".into());
            }
            steps.push(Step {
                label: "Create the project".into(),
                program: program.into(),
                args,
                cwd: parent.into(),
                env: Vec::new(),
            });
        }
        "django" => {
            let python = if cfg!(windows) { "python" } else { "python3" };
            let venv_python = venv_python(&dir);
            steps.push(step(
                "Create the virtual environment",
                parent,
                python,
                &["-m", "venv", &format!("{name}/.venv")],
            ));
            steps.push(Step {
                label: "Install Django".into(),
                program: venv_python.to_string_lossy().into_owned(),
                args: ["-m", "pip", "install", "django"]
                    .map(String::from)
                    .to_vec(),
                cwd: dir.clone(),
                env: Vec::new(),
            });
            steps.push(Step {
                label: "Create the Django project".into(),
                program: venv_python.to_string_lossy().into_owned(),
                args: ["-m", "django", "startproject", "config", "."]
                    .map(String::from)
                    .to_vec(),
                cwd: dir.clone(),
                env: Vec::new(),
            });
            notes.push(
                "The settings package is `config/`; LocDev runs it with the project's .venv."
                    .into(),
            );
        }
        "rails" => {
            let mut args: Vec<String> = vec![
                "new".into(),
                name.into(),
                format!("--database={}", opt(project, starter, "database")),
                format!("--javascript={}", opt(project, starter, "javascript")),
            ];
            let css = opt(project, starter, "css");
            if css != "none" {
                args.push(format!("--css={css}"));
            }
            if !on(project, starter, "git") {
                args.push("--skip-git".into());
            }
            steps.push(Step {
                label: "Create the project".into(),
                program: "rails".into(),
                args,
                cwd: parent.into(),
                env: Vec::new(),
            });
        }
        "dotnet" => {
            let template = opt(project, starter, "template");
            steps.push(step(
                "Create the project",
                parent,
                "dotnet",
                &["new", template, "-o", name],
            ));
        }
        other => return Err(format!("no plan for '{other}'")),
    }

    if let Some(pm) = node_pm {
        steps.push(step(
            "Install the front-end dependencies",
            &dir,
            pm,
            &["install"],
        ));
        if starter.id == "laravel" {
            steps.push(step("Build the front-end", &dir, pm, &["run", "build"]));
        }
    }

    if !project.packages.is_empty() {
        let packages: Vec<&str> = project.packages.iter().map(String::as_str).collect();
        let (program, mut args): (&str, Vec<&str>) = match registry {
            Some(PackageRegistry::Packagist) => ("composer", vec!["require"]),
            _ => match m {
                "pnpm" => ("pnpm", vec!["add"]),
                "bun" => ("bun", vec!["add"]),
                _ => ("npm", vec!["install"]),
            },
        };
        args.extend(packages);
        steps.push(step("Add the chosen packages", &dir, program, &args));
    }

    let mut actions = Vec::new();
    let mut finish = Vec::new();
    if let Some(db) = &project.database {
        let database = project.name.replace('-', "_");
        actions.push(format!(
            "Create the {} instance '{}' if it doesn't exist, and start it",
            db.service, db.instance
        ));
        actions.push(format!("Create the database '{database}' in it"));
        actions.push("Point DB_CONNECTION, DB_HOST, DB_PORT, DB_DATABASE, DB_USERNAME and DB_PASSWORD in the new project's .env at it".into());
        finish.push(step(
            "Run the migrations",
            &dir,
            "php",
            &["artisan", "migrate", "--no-interaction"],
        ));
    }
    if project.register {
        actions.push(format!("Register the folder as the site {name}"));
    }

    for s in steps.iter().chain(&finish) {
        check_program(s, &dir)?;
    }
    Ok(ProjectPlan {
        dir,
        make_dir,
        files,
        steps,
        actions,
        finish,
        notes,
        missing: Vec::new(),
    })
}

const PHP_INDEX: &str = include_str!("../templates/php/public/index.php");
const FLASK_APP: &str = include_str!("../templates/flask/app.py");
const FLASK_REQUIREMENTS: &str = include_str!("../templates/flask/requirements.txt");
const FASTAPI_MAIN: &str = include_str!("../templates/fastapi/main.py");
const FASTAPI_REQUIREMENTS: &str = include_str!("../templates/fastapi/requirements.txt");
const AXUM_MAIN: &str = include_str!("../templates/axum/main.rs");
const ACTIX_MAIN: &str = include_str!("../templates/actix/main.rs");
const PYTHON_GITIGNORE: &str = ".venv/\n__pycache__/\n*.pyc\n.env\n";

fn template(path: &str, contents: &str) -> PlannedFile {
    PlannedFile {
        path: path.into(),
        contents: contents.into(),
    }
}

/// The project's own Python, for the Python starters' steps.
pub fn venv_python(dir: &Path) -> PathBuf {
    if cfg!(windows) {
        dir.join(".venv").join("Scripts").join("python.exe")
    } else {
        dir.join(".venv").join("bin").join("python")
    }
}

/// Programs come from [`crate::PROGRAMS`], or are the new project's venv.
fn check_program(step: &Step, dir: &Path) -> Result<(), String> {
    if crate::PROGRAMS.contains(&step.program.as_str())
        || Path::new(&step.program) == venv_python(dir)
    {
        Ok(())
    } else {
        Err(format!("'{}' is not a program LocDev runs", step.program))
    }
}

/// Which tool each step needs (for the "missing" list).
pub fn tool_of(program: &str) -> Option<&'static str> {
    Some(match program {
        "laravel" => "laravel",
        "composer" => "composer",
        "php" => "php",
        "symfony" => "symfony",
        "npm" | "npx" => "npm",
        "pnpm" => "pnpm",
        "bun" | "bunx" => "bun",
        "rails" => "rails",
        "python3" | "python" => "python",
        "dotnet" => "dotnet",
        "cargo" => "cargo",
        "wp" => "wp",
        "git" => "git",
        _ => return None,
    })
}

/// Sets `pairs` in a freshly created project's `.env`: an existing
/// `KEY=` line (or Laravel's commented `# KEY=`) is replaced in place, a
/// missing key is appended. Everything else stays as it was. Only used on a
/// project LocDev just created, as the plan says.
pub fn set_env(content: &str, pairs: &[(String, String)]) -> String {
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    for (key, value) in pairs {
        let line = format!("{key}={value}");
        let matches = |l: &str| {
            let l = l.trim_start();
            let l = l.strip_prefix('#').map(str::trim_start).unwrap_or(l);
            l.strip_prefix(key.as_str())
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        };
        match lines.iter().position(|l| matches(l)) {
            Some(i) => lines[i] = line,
            None => lines.push(line),
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}
