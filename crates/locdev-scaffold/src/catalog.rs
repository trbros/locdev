//! The starters. Flags come from each tool's own source or docs (checked
//! 2026-11-03): the Laravel installer's `NewCommand`, `create-next-app`,
//! `create-vite`, `create-astro`, `sv create`, `create nuxt`, `nest new`,
//! `rails new` (`app_base.rb`), `dotnet new` and the Symfony/Bedrock/Django
//! setup guides; `create-vue` (`index.ts`, `utils/resolveFeatures.ts`), the
//! `ng new` reference and `wp core download` (checked 2026-11-04). A flag
//! that isn't documented there doesn't go in.
//!
//! Frameworks without a generator (plain PHP, Flask, FastAPI, Axum, Actix
//! Web) start from LocDev's own minimal files (`templates/`), written before
//! the official tool (`cargo init`, `pip`) runs, shown in the plan first.

use locdev_core::scaffold::{
    OptionChoice, OptionWhen, PackageRegistry, Starter, StarterMethod, StarterOption,
};

fn method(id: &str, label: &str, tool: &str) -> StarterMethod {
    StarterMethod {
        id: id.into(),
        label: label.into(),
        tool: tool.into(),
        note: None,
    }
}

fn choice(id: &str, label: &str, choices: &[(&str, &str)], default: &str) -> StarterOption {
    StarterOption {
        id: id.into(),
        label: label.into(),
        help: None,
        choices: choices
            .iter()
            .map(|(value, label)| OptionChoice {
                value: (*value).into(),
                label: (*label).into(),
            })
            .collect(),
        default: default.into(),
        methods: Vec::new(),
        when: Vec::new(),
    }
}

fn toggle(id: &str, label: &str, default: bool) -> StarterOption {
    StarterOption {
        id: id.into(),
        label: label.into(),
        help: None,
        choices: Vec::new(),
        default: default.to_string(),
        methods: Vec::new(),
        when: Vec::new(),
    }
}

/// Asked only when `option` is one of `values`.
fn when(mut option: StarterOption, other: &str, values: &[&str]) -> StarterOption {
    option.when.push(OptionWhen {
        option: other.into(),
        values: values.iter().map(|v| (*v).into()).collect(),
    });
    option
}

fn only(mut option: StarterOption, methods: &[&str]) -> StarterOption {
    option.methods = methods.iter().map(|m| (*m).into()).collect();
    option
}

fn help(mut option: StarterOption, text: &str) -> StarterOption {
    option.help = Some(text.into());
    option
}

/// The JavaScript package managers a starter can run with.
fn js_methods() -> Vec<StarterMethod> {
    vec![
        method("npm", "npm", "npm"),
        method("pnpm", "pnpm", "pnpm"),
        method("bun", "Bun", "bun"),
    ]
}

pub fn starters() -> Vec<Starter> {
    vec![
        Starter {
            id: "laravel".into(),
            name: "Laravel".into(),
            family: "php".into(),
            framework: "laravel".into(),
            description: "PHP framework; starter kits with React, Vue, Svelte or Livewire and authentication.".into(),
            docs: "https://laravel.com/docs/installation".into(),
            methods: vec![
                StarterMethod {
                    note: Some("`laravel new`, the official installer.".into()),
                    ..method("installer", "Laravel installer", "laravel")
                },
                StarterMethod {
                    note: Some("`composer create-project` with the skeleton or a starter kit package.".into()),
                    ..method("composer", "Composer", "composer")
                },
            ],
            // The installer's own questions, in its order (laravel/installer
            // `NewCommand::interact`, checked 2026-11-23): a starter kit or not;
            // without one, a frontend stack (blank kits, no auth); with one, the
            // kit's stack, the auth provider, Livewire single-file components and
            // teams (both only with Laravel's own auth).
            options: vec![
                toggle("starter_kit", "Do you want to use a starter kit?", false),
                when(
                    choice(
                        "stack",
                        "Which frontend stack do you want to build on?",
                        &[
                            ("blade", "Blade (no additional frontend scaffolding)"),
                            ("react", "React (Inertia, Tailwind)"),
                            ("svelte", "Svelte (Inertia, Tailwind)"),
                            ("vue", "Vue (Inertia, Tailwind)"),
                            ("livewire", "Livewire (Tailwind)"),
                        ],
                        "blade",
                    ),
                    "starter_kit",
                    &["false"],
                ),
                when(
                    choice(
                        "kit",
                        "Which frontend stack should your starter kit use?",
                        &[("react", "React"), ("svelte", "Svelte"), ("vue", "Vue"), ("livewire", "Livewire")],
                        "react",
                    ),
                    "starter_kit",
                    &["true"],
                ),
                when(
                    choice(
                        "auth",
                        "Which authentication provider do you prefer?",
                        &[("laravel", "Laravel's built-in authentication"), ("workos", "WorkOS (requires a WorkOS account)")],
                        "laravel",
                    ),
                    "starter_kit",
                    &["true"],
                ),
                when(
                    when(
                        when(toggle("single_file", "Would you like to use single-file Livewire components?", true), "starter_kit", &["true"]),
                        "kit",
                        &["livewire"],
                    ),
                    "auth",
                    &["laravel"],
                ),
                when(
                    when(toggle("teams", "Would you like to add teams support to your application?", false), "starter_kit", &["true"]),
                    "auth",
                    &["laravel"],
                ),
                only(choice("testing", "Tests", &[("pest", "Pest"), ("phpunit", "PHPUnit")], "pest"), &["installer"]),
                only(
                    help(
                        choice(
                            "database",
                            "Database driver",
                            &[
                                ("sqlite", "SQLite"),
                                ("mysql", "MySQL"),
                                ("mariadb", "MariaDB"),
                                ("pgsql", "PostgreSQL"),
                            ],
                            "sqlite",
                        ),
                        "Choosing a LocDev database below sets this for you.",
                    ),
                    &["installer"],
                ),
                choice(
                    "node",
                    "Front-end dependencies",
                    &[("npm", "npm"), ("pnpm", "pnpm"), ("bun", "Bun"), ("none", "Don't install")],
                    "npm",
                ),
                only(toggle("boost", "Laravel Boost (AI guidelines and MCP)", true), &["installer"]),
                only(toggle("git", "Initialize a Git repository", true), &["installer"]),
            ],
            packages: Some(PackageRegistry::Packagist),
            database: true,
        },
        Starter {
            id: "symfony".into(),
            name: "Symfony".into(),
            family: "php".into(),
            framework: "php".into(),
            description: "PHP framework; a full web app or a minimal skeleton for APIs and microservices.".into(),
            docs: "https://symfony.com/doc/current/setup.html".into(),
            methods: vec![method("cli", "Symfony CLI", "symfony"), method("composer", "Composer", "composer")],
            options: vec![choice(
                "type",
                "Kind of project",
                &[("webapp", "Web application (--webapp)"), ("skeleton", "Skeleton (API, microservice, console)")],
                "webapp",
            )],
            packages: Some(PackageRegistry::Packagist),
            database: false,
        },
        Starter {
            id: "php".into(),
            name: "PHP".into(),
            family: "php".into(),
            framework: "php".into(),
            description: "Plain PHP: a public/index.php served by PHP's built-in server.".into(),
            docs: "https://www.php.net/manual/en/features.commandline.webserver.php".into(),
            methods: vec![StarterMethod {
                note: Some("LocDev's starter file (public/index.php); no installer.".into()),
                ..method("files", "LocDev starter", "php")
            }],
            options: vec![toggle("git", "Initialize a Git repository", true)],
            packages: None,
            database: false,
        },
        Starter {
            id: "wordpress".into(),
            name: "WordPress".into(),
            family: "php".into(),
            framework: "wordpress".into(),
            description: "WordPress itself, downloaded by WP-CLI; finish the setup in the browser.".into(),
            docs: "https://developer.wordpress.org/cli/commands/core/download/".into(),
            methods: vec![StarterMethod {
                note: Some("`wp core download` (WP-CLI checks the download's hash).".into()),
                ..method("wpcli", "WP-CLI", "wp")
            }],
            options: vec![choice(
                "locale",
                "Language",
                &[
                    ("en_US", "English (US)"),
                    ("pt_BR", "Português (Brasil)"),
                    ("es_ES", "Español"),
                    ("ja", "日本語"),
                ],
                "en_US",
            )],
            packages: None,
            database: false,
        },
        Starter {
            id: "bedrock".into(),
            name: "WordPress (Bedrock)".into(),
            family: "php".into(),
            framework: "wordpress".into(),
            description: "WordPress managed with Composer, configured through .env.".into(),
            docs: "https://roots.io/bedrock/docs/installation/".into(),
            methods: vec![method("composer", "Composer", "composer")],
            options: Vec::new(),
            packages: Some(PackageRegistry::Packagist),
            database: false,
        },
        Starter {
            id: "nextjs".into(),
            name: "Next.js".into(),
            family: "javascript".into(),
            framework: "nextjs".into(),
            description: "React framework with server rendering and the App Router.".into(),
            docs: "https://nextjs.org/docs/app/api-reference/cli/create-next-app".into(),
            methods: js_methods(),
            options: vec![
                choice("language", "Language", &[("ts", "TypeScript"), ("js", "JavaScript")], "ts"),
                toggle("tailwind", "Tailwind CSS", true),
                choice("linter", "Linter", &[("eslint", "ESLint"), ("biome", "Biome"), ("none", "None")], "eslint"),
                toggle("app", "App Router", true),
                toggle("src", "src/ folder", false),
                toggle("git", "Initialize a Git repository", true),
            ],
            packages: Some(PackageRegistry::Npm),
            database: false,
        },
        Starter {
            id: "vite".into(),
            name: "Vite".into(),
            family: "javascript".into(),
            framework: "vite".into(),
            description: "Front-end with Vite: React, Vue, Svelte, Solid, Preact, Lit or plain.".into(),
            docs: "https://vite.dev/guide/#scaffolding-your-first-vite-project".into(),
            methods: js_methods(),
            options: vec![
                choice(
                    "framework",
                    "Framework",
                    &[
                        ("react", "React"),
                        ("vue", "Vue"),
                        ("svelte", "Svelte"),
                        ("solid", "Solid"),
                        ("preact", "Preact"),
                        ("lit", "Lit"),
                        ("vanilla", "Vanilla"),
                    ],
                    "react",
                ),
                choice("language", "Language", &[("ts", "TypeScript"), ("js", "JavaScript")], "ts"),
            ],
            packages: Some(PackageRegistry::Npm),
            database: false,
        },
        Starter {
            id: "vue".into(),
            name: "Vue".into(),
            family: "javascript".into(),
            framework: "vue".into(),
            description: "Vue's official project setup (`create-vue`), with Vite.".into(),
            docs: "https://vuejs.org/guide/quick-start.html#creating-a-vue-application".into(),
            methods: js_methods(),
            options: vec![
                choice("language", "Language", &[("ts", "TypeScript"), ("js", "JavaScript")], "ts"),
                toggle("router", "Vue Router", true),
                toggle("pinia", "Pinia (state management)", false),
                toggle("vitest", "Vitest (unit tests)", false),
                toggle("eslint", "ESLint", true),
            ],
            packages: Some(PackageRegistry::Npm),
            database: false,
        },
        Starter {
            id: "angular".into(),
            name: "Angular".into(),
            family: "javascript".into(),
            framework: "angular".into(),
            description: "Angular workspace from the Angular CLI (`ng new`).".into(),
            docs: "https://angular.dev/cli/new".into(),
            methods: js_methods(),
            options: vec![
                choice(
                    "style",
                    "Styles",
                    &[
                        ("css", "CSS"),
                        ("scss", "SCSS"),
                        ("sass", "Sass"),
                        ("less", "Less"),
                        ("tailwind", "Tailwind CSS"),
                    ],
                    "css",
                ),
                toggle("routing", "Routing", true),
                toggle("ssr", "Server-side rendering", false),
                toggle("git", "Initialize a Git repository", true),
            ],
            packages: Some(PackageRegistry::Npm),
            database: false,
        },
        Starter {
            id: "nuxt".into(),
            name: "Nuxt".into(),
            family: "javascript".into(),
            framework: "nuxt".into(),
            description: "Vue framework with server rendering.".into(),
            docs: "https://nuxt.com/docs/4.x/api/commands/init".into(),
            methods: js_methods(),
            options: vec![toggle("git", "Initialize a Git repository", true)],
            packages: Some(PackageRegistry::Npm),
            database: false,
        },
        Starter {
            id: "astro".into(),
            name: "Astro".into(),
            family: "javascript".into(),
            framework: "astro".into(),
            description: "Content sites and islands.".into(),
            docs: "https://docs.astro.build/en/install-and-setup/".into(),
            methods: js_methods(),
            options: vec![
                choice(
                    "template",
                    "Template",
                    &[("basics", "Basics"), ("blog", "Blog"), ("portfolio", "Portfolio"), ("minimal", "Minimal")],
                    "basics",
                ),
                toggle("git", "Initialize a Git repository", true),
            ],
            packages: Some(PackageRegistry::Npm),
            database: false,
        },
        Starter {
            id: "sveltekit".into(),
            name: "SvelteKit".into(),
            family: "javascript".into(),
            framework: "sveltekit".into(),
            description: "Svelte's application framework (`sv create`).".into(),
            docs: "https://svelte.dev/docs/cli/sv-create".into(),
            methods: js_methods(),
            options: vec![
                choice("template", "Template", &[("minimal", "Minimal"), ("demo", "Demo")], "minimal"),
                choice("types", "Type checking", &[("ts", "TypeScript"), ("jsdoc", "JSDoc")], "ts"),
            ],
            packages: Some(PackageRegistry::Npm),
            database: false,
        },
        Starter {
            id: "nestjs".into(),
            name: "NestJS".into(),
            family: "javascript".into(),
            framework: "nestjs".into(),
            description: "Node.js server framework.".into(),
            docs: "https://docs.nestjs.com/first-steps".into(),
            methods: js_methods(),
            options: vec![
                choice("language", "Language", &[("ts", "TypeScript"), ("js", "JavaScript")], "ts"),
                toggle("git", "Initialize a Git repository", true),
            ],
            packages: Some(PackageRegistry::Npm),
            database: false,
        },
        Starter {
            id: "django".into(),
            name: "Django".into(),
            family: "python".into(),
            framework: "django".into(),
            description: "Python web framework, in a virtual environment inside the project.".into(),
            docs: "https://docs.djangoproject.com/en/stable/intro/tutorial01/".into(),
            methods: vec![StarterMethod {
                note: Some("A `.venv` in the project, Django installed with pip, then `django-admin startproject`.".into()),
                ..method("venv", "Python venv + pip", "python")
            }],
            options: Vec::new(),
            packages: None,
            database: false,
        },
        Starter {
            id: "flask".into(),
            name: "Flask".into(),
            family: "python".into(),
            framework: "flask".into(),
            description: "Python micro-framework: LocDev's app.py, Flask from pip in the project's .venv.".into(),
            docs: "https://flask.palletsprojects.com/en/stable/installation/".into(),
            methods: vec![StarterMethod {
                note: Some("LocDev's app.py and requirements.txt, a `.venv`, then `pip install -r requirements.txt`.".into()),
                ..method("venv", "Python venv + pip", "python")
            }],
            options: vec![toggle("git", "Initialize a Git repository", true)],
            packages: None,
            database: false,
        },
        Starter {
            id: "fastapi".into(),
            name: "FastAPI".into(),
            family: "python".into(),
            framework: "fastapi".into(),
            description: "Python APIs: LocDev's main.py, fastapi[standard] from pip in the project's .venv.".into(),
            docs: "https://fastapi.tiangolo.com/#installation".into(),
            methods: vec![StarterMethod {
                note: Some("LocDev's main.py and requirements.txt, a `.venv`, then `pip install -r requirements.txt`.".into()),
                ..method("venv", "Python venv + pip", "python")
            }],
            options: vec![toggle("git", "Initialize a Git repository", true)],
            packages: None,
            database: false,
        },
        Starter {
            id: "rust".into(),
            name: "Rust web server".into(),
            family: "rust".into(),
            framework: "rust".into(),
            description: "A Cargo project with Axum or Actix Web, reading its port from PORT.".into(),
            docs: "https://doc.rust-lang.org/cargo/commands/cargo-init.html".into(),
            methods: vec![StarterMethod {
                note: Some("LocDev's src/main.rs, then `cargo init` and `cargo add`.".into()),
                ..method("cargo", "Cargo", "cargo")
            }],
            options: vec![
                choice("framework", "Framework", &[("axum", "Axum"), ("actix", "Actix Web")], "axum"),
                toggle("git", "Initialize a Git repository", true),
            ],
            packages: None,
            database: false,
        },
        Starter {
            id: "rails".into(),
            name: "Ruby on Rails".into(),
            family: "ruby".into(),
            framework: "rails".into(),
            description: "Ruby web framework.".into(),
            docs: "https://guides.rubyonrails.org/getting_started.html".into(),
            methods: vec![method("rails", "rails new", "rails")],
            options: vec![
                choice(
                    "database",
                    "Database",
                    &[("sqlite3", "SQLite"), ("postgresql", "PostgreSQL"), ("mysql", "MySQL")],
                    "sqlite3",
                ),
                choice(
                    "css",
                    "CSS",
                    &[("none", "None"), ("tailwind", "Tailwind"), ("bootstrap", "Bootstrap"), ("sass", "Sass")],
                    "none",
                ),
                choice(
                    "javascript",
                    "JavaScript",
                    &[("importmap", "Import maps"), ("esbuild", "esbuild"), ("bun", "Bun")],
                    "importmap",
                ),
                toggle("git", "Initialize a Git repository", true),
            ],
            packages: None,
            database: false,
        },
        Starter {
            id: "dotnet".into(),
            name: "ASP.NET Core".into(),
            family: "dotnet".into(),
            framework: "dotnet".into(),
            description: ".NET web apps and APIs (`dotnet new`).".into(),
            docs: "https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-new-sdk-templates".into(),
            methods: vec![method("dotnet", ".NET SDK", "dotnet")],
            options: vec![choice(
                "template",
                "Template",
                &[
                    ("webapp", "Web App (Razor Pages)"),
                    ("mvc", "Web App (MVC)"),
                    ("webapi", "Web API"),
                    ("blazor", "Blazor Web App"),
                    ("web", "Empty"),
                ],
                "webapp",
            )],
            packages: None,
            database: false,
        },
    ]
}
