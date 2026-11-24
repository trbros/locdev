//! Workflow templates, filled in from what the project actually has.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use locdev_core::actions::{
    Concurrency, Filter, Job, Permissions, Step, TemplateInfo, Triggers, Workflow,
};
use locdev_core::detect;
use locdev_core::model::{ProjectKind, RuntimeKind};

use crate::versions::Versions;

/// What templates need to know about a project. Read from files only.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub kind: ProjectKind,
    pub node: bool,
    pub manager: Option<RuntimeKind>,
    pub yarn_berry: bool,
    pub scripts: BTreeSet<String>,
    /// `.nvmrc` or `.node-version`, for `node-version-file`.
    pub node_version_file: Option<String>,
    pub node_version: Option<String>,
    pub php: bool,
    pub php_version: Option<String>,
    pub laravel: bool,
    pub pest: bool,
    pub phpunit: bool,
    pub dockerfile: bool,
    /// The build output folder static hosts would serve.
    pub output: String,
    /// The branch workflows run on.
    pub branch: String,
}

impl Facts {
    pub fn read(path: &Path, branch: Option<&str>) -> Self {
        let stack = locdev_frameworks::analyze(
            locdev_frameworks::Catalog::bundled_ref(),
            path,
            Default::default(),
        );
        let package: Option<serde_json::Value> = std::fs::read_to_string(path.join("package.json"))
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok());
        let composer: Option<serde_json::Value> =
            std::fs::read_to_string(path.join("composer.json"))
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok());
        let requirements = detect::requirements(path);
        let requirement = |kind: RuntimeKind| requirements.iter().find(|r| r.kind == kind);
        let composer_dev = |name: &str| {
            composer.as_ref().is_some_and(|c| {
                ["require", "require-dev"]
                    .iter()
                    .any(|s| c[s].get(name).is_some())
            })
        };
        let node_version_file = [".nvmrc", ".node-version"]
            .into_iter()
            .find(|f| path.join(f).is_file())
            .map(str::to_string);
        Self {
            kind: stack.kind,
            node: package.is_some(),
            manager: package.as_ref().and(stack.package_manager),
            yarn_berry: path.join(".yarnrc.yml").is_file(),
            scripts: package
                .as_ref()
                .and_then(|p| p["scripts"].as_object())
                .map(|s| s.keys().cloned().collect())
                .unwrap_or_default(),
            node_version: requirement(RuntimeKind::Node).map(|r| version_line(&r.constraint)),
            node_version_file,
            php: composer.is_some(),
            php_version: requirement(RuntimeKind::Php).map(|r| version_line(&r.constraint)),
            laravel: stack.kind == ProjectKind::Laravel,
            pest: composer_dev("pestphp/pest"),
            phpunit: composer_dev("phpunit/phpunit"),
            dockerfile: path.join("Dockerfile").is_file(),
            output: match stack.kind {
                ProjectKind::Astro | ProjectKind::Vite => "dist",
                ProjectKind::Sveltekit => "build",
                ProjectKind::Next => "out",
                _ => "dist",
            }
            .into(),
            branch: branch
                .filter(|b| *b == "main" || *b == "master")
                .unwrap_or("main")
                .to_string(),
        }
    }
}

/// `^8.2` / `>=20 <23` / `20.11.1` → the first version line it names.
fn version_line(constraint: &str) -> String {
    let start = constraint.find(|c: char| c.is_ascii_digit()).unwrap_or(0);
    let digits: String = constraint[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let parts: Vec<&str> = digits.split('.').filter(|p| !p.is_empty()).collect();
    match parts.as_slice() {
        [] => constraint.to_string(),
        [major] => major.to_string(),
        [major, minor, ..] => format!("{major}.{minor}"),
    }
}

pub struct Template {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub file: &'static str,
    /// `(recommended, reason)`.
    pub fits: fn(&Facts) -> (bool, String),
    pub build: fn(&Facts, &Versions) -> Workflow,
}

pub const TEMPLATES: &[Template] = &[
    Template {
        id: "node-ci",
        title: "Node.js CI",
        description: "Install with the project's package manager, then lint, type-check, test and build (whichever scripts exist).",
        file: "ci.yml",
        fits: |f| match (f.node, f.php) {
            (true, false) => (true, format!("package.json with {}", scripts_found(f))),
            (true, true) => (false, "PHP CI covers the front-end build too".into()),
            _ => (false, "no package.json".into()),
        },
        build: node_ci,
    },
    Template {
        id: "php-ci",
        title: "PHP / Laravel CI",
        description: "Set up PHP and Composer, build the front-end if there is one, and run the test suite.",
        file: "ci.yml",
        fits: |f| {
            if f.php {
                (
                    true,
                    format!("composer.json{}", if f.laravel { " (Laravel)" } else { "" }),
                )
            } else {
                (false, "no composer.json".into())
            }
        },
        build: php_ci,
    },
    Template {
        id: "pages",
        title: "Deploy to GitHub Pages",
        description: "Build the site and publish it to GitHub Pages on every push to the main branch.",
        file: "pages.yml",
        fits: |f| {
            let fits = f.scripts.contains("build")
                && matches!(
                    f.kind,
                    ProjectKind::Vite
                        | ProjectKind::Astro
                        | ProjectKind::Static
                        | ProjectKind::Sveltekit
                );
            if fits {
                (true, format!("a static build (output in {}/)", f.output))
            } else if f.scripts.contains("build") {
                (
                    false,
                    "the project needs a server; check that it can export static files".into(),
                )
            } else {
                (false, "no build script".into())
            }
        },
        build: pages,
    },
    Template {
        id: "dependency-review",
        title: "Dependency review",
        description: "Fails pull requests that add dependencies with known vulnerabilities.",
        file: "dependency-review.yml",
        fits: |f| {
            if f.node || f.php {
                (
                    true,
                    "checks npm and Composer changes in pull requests".into(),
                )
            } else {
                (false, "no package manifests".into())
            }
        },
        build: dependency_review,
    },
    Template {
        id: "codeql",
        title: "CodeQL code scanning",
        description: "Static analysis for security bugs on pushes, pull requests and weekly.",
        file: "codeql.yml",
        fits: |f| {
            if f.node {
                (true, "JavaScript/TypeScript code".into())
            } else {
                (false, "CodeQL doesn't analyze PHP".into())
            }
        },
        build: codeql,
    },
    Template {
        id: "release",
        title: "Release on tag",
        description: "Create a GitHub release with generated notes when a v* tag is pushed.",
        file: "release.yml",
        fits: |_| (false, "for projects that publish versions".into()),
        build: release,
    },
    Template {
        id: "docker",
        title: "Docker image",
        description: "Build the Dockerfile on pull requests; build and push to GitHub Container Registry on the main branch and tags.",
        file: "docker.yml",
        fits: |f| {
            if f.dockerfile {
                (true, "Dockerfile found".into())
            } else {
                (false, "no Dockerfile".into())
            }
        },
        build: docker,
    },
];

pub fn list(facts: &Facts) -> Vec<TemplateInfo> {
    let mut list: Vec<TemplateInfo> = TEMPLATES
        .iter()
        .map(|t| {
            let (recommended, reason) = (t.fits)(facts);
            TemplateInfo {
                id: t.id.into(),
                title: t.title.into(),
                description: t.description.into(),
                file: t.file.into(),
                recommended,
                reason,
            }
        })
        .collect();
    list.sort_by_key(|t| !t.recommended);
    list
}

pub fn build(id: &str, facts: &Facts, versions: &Versions) -> Option<(Workflow, String)> {
    TEMPLATES
        .iter()
        .find(|t| t.id == id)
        .map(|t| ((t.build)(facts, versions), t.file.to_string()))
}

fn scripts_found(f: &Facts) -> String {
    let found: Vec<&str> = ["lint", "typecheck", "test", "build"]
        .into_iter()
        .filter(|s| f.scripts.contains(*s))
        .collect();
    if found.is_empty() {
        "no lint/test/build scripts yet".into()
    } else {
        format!("{} scripts", found.join(", "))
    }
}

// ── Building blocks ─────────────────────────────────────────────────────────

fn scopes(pairs: &[(&str, &str)]) -> Permissions {
    Permissions::Scopes(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}

fn with(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn uses(versions: &Versions, repo: &str, pairs: &[(&str, &str)]) -> Step {
    Step {
        uses: Some(versions.uses(repo)),
        with: with(pairs),
        ..Step::default()
    }
}

fn run(name: &str, command: &str) -> Step {
    Step {
        name: Some(name.into()),
        run: Some(command.into()),
        ..Step::default()
    }
}

/// Without the token left in .git/config: no template pushes with git.
fn checkout(versions: &Versions) -> Step {
    uses(
        versions,
        "actions/checkout",
        &[("persist-credentials", "false")],
    )
}

fn branch_triggers(f: &Facts) -> Triggers {
    Triggers {
        push: Some(Filter {
            branches: vec![f.branch.clone()],
            ..Filter::default()
        }),
        pull_request: Some(Filter::default()),
        ..Triggers::default()
    }
}

/// Cancel superseded runs of the same branch or pull request.
fn cancel_superseded() -> Option<Concurrency> {
    Some(Concurrency {
        group: "${{ github.workflow }}-${{ github.ref }}".into(),
        cancel_in_progress: true,
    })
}

/// Node.js (or Bun) set up, dependencies installed.
fn node_setup(f: &Facts, versions: &Versions) -> Vec<Step> {
    let manager = f.manager.unwrap_or(RuntimeKind::Npm);
    if manager == RuntimeKind::Bun {
        return vec![
            uses(versions, "oven-sh/setup-bun", &[]),
            run("Install dependencies", "bun install --frozen-lockfile"),
        ];
    }
    let mut steps = Vec::new();
    if manager == RuntimeKind::Pnpm {
        // Version from packageManager in package.json.
        steps.push(uses(versions, "pnpm/action-setup", &[]));
    }
    let cache = match manager {
        RuntimeKind::Pnpm => "pnpm",
        RuntimeKind::Yarn => "yarn",
        _ => "npm",
    };
    let mut setup = with(&[("cache", cache)]);
    match (&f.node_version_file, &f.node_version) {
        (Some(file), _) => setup.insert("node-version-file".into(), file.clone()),
        (None, Some(version)) => setup.insert("node-version".into(), version.clone()),
        (None, None) => setup.insert("node-version".into(), "lts/*".into()),
    };
    steps.push(Step {
        uses: Some(versions.uses("actions/setup-node")),
        with: setup,
        ..Step::default()
    });
    steps.push(run(
        "Install dependencies",
        match manager {
            RuntimeKind::Pnpm => "pnpm install --frozen-lockfile",
            RuntimeKind::Yarn if f.yarn_berry => "yarn install --immutable",
            RuntimeKind::Yarn => "yarn install --frozen-lockfile",
            _ => "npm ci",
        },
    ));
    steps
}

fn script(f: &Facts, name: &str) -> String {
    match f.manager.unwrap_or(RuntimeKind::Npm) {
        RuntimeKind::Pnpm => format!("pnpm run {name}"),
        RuntimeKind::Yarn => format!("yarn run {name}"),
        RuntimeKind::Bun => format!("bun run {name}"),
        _ => format!("npm run {name}"),
    }
}

// ── Templates ───────────────────────────────────────────────────────────────

fn node_ci(f: &Facts, v: &Versions) -> Workflow {
    let mut steps = vec![checkout(v)];
    steps.extend(node_setup(f, v));
    for (name, label) in [
        ("lint", "Lint"),
        ("typecheck", "Type-check"),
        ("test", "Test"),
        ("build", "Build"),
    ] {
        if f.scripts.contains(name) {
            steps.push(run(label, &script(f, name)));
        }
    }
    Workflow {
        name: "CI".into(),
        on: branch_triggers(f),
        permissions: Some(scopes(&[("contents", "read")])),
        concurrency: cancel_superseded(),
        jobs: vec![Job {
            id: "ci".into(),
            runs_on: "ubuntu-latest".into(),
            timeout_minutes: Some(15),
            steps,
            ..Job::default()
        }],
        ..Workflow::default()
    }
}

fn php_ci(f: &Facts, v: &Versions) -> Workflow {
    let php = f.php_version.clone().unwrap_or_else(|| "8.4".into());
    let mut extensions = vec!["mbstring", "intl"];
    if f.laravel {
        extensions.extend(["pdo_sqlite", "bcmath", "fileinfo"]);
    }
    let extensions = extensions.join(", ");
    let mut steps = vec![
        checkout(v),
        uses(
            v,
            "shivammathur/setup-php",
            &[
                ("php-version", &php),
                ("extensions", &extensions),
                ("coverage", "none"),
                ("tools", "composer:v2"),
            ],
        ),
        run(
            "Install Composer dependencies",
            "composer install --no-interaction --prefer-dist --no-progress",
        ),
    ];
    if f.node && f.scripts.contains("build") {
        // Laravel tests that render views need the Vite manifest.
        steps.extend(node_setup(f, v));
        steps.push(run("Build front-end", &script(f, "build")));
    }
    if f.laravel {
        steps.push(run(
            "Prepare the environment",
            "cp .env.example .env\nphp artisan key:generate\n",
        ));
        steps.push(run("Test", "php artisan test"));
    } else if f.pest {
        steps.push(run("Test", "vendor/bin/pest"));
    } else if f.phpunit {
        steps.push(run("Test", "vendor/bin/phpunit"));
    } else {
        steps.push(run("Validate composer.json", "composer validate --strict"));
    }
    Workflow {
        name: "CI".into(),
        on: branch_triggers(f),
        permissions: Some(scopes(&[("contents", "read")])),
        concurrency: cancel_superseded(),
        jobs: vec![Job {
            id: "tests".into(),
            runs_on: "ubuntu-latest".into(),
            timeout_minutes: Some(20),
            steps,
            ..Job::default()
        }],
        ..Workflow::default()
    }
}

fn pages(f: &Facts, v: &Versions) -> Workflow {
    let mut build = vec![checkout(v)];
    build.extend(node_setup(f, v));
    build.push(run("Build", &script(f, "build")));
    build.push(uses(v, "actions/configure-pages", &[]));
    build.push(uses(
        v,
        "actions/upload-pages-artifact",
        &[("path", &f.output)],
    ));
    Workflow {
        name: "Deploy to GitHub Pages".into(),
        on: Triggers {
            push: Some(Filter {
                branches: vec![f.branch.clone()],
                ..Filter::default()
            }),
            workflow_dispatch: true,
            ..Triggers::default()
        },
        permissions: Some(scopes(&[("contents", "read")])),
        // One deployment at a time, and never cancel one halfway.
        concurrency: Some(Concurrency {
            group: "pages".into(),
            cancel_in_progress: false,
        }),
        jobs: vec![
            Job {
                id: "build".into(),
                runs_on: "ubuntu-latest".into(),
                timeout_minutes: Some(15),
                steps: build,
                ..Job::default()
            },
            Job {
                id: "deploy".into(),
                timeout_minutes: Some(10),
                runs_on: "ubuntu-latest".into(),
                needs: vec!["build".into()],
                permissions: Some(scopes(&[("pages", "write"), ("id-token", "write")])),
                environment: Some("github-pages".into()),
                steps: vec![Step {
                    id: Some("deployment".into()),
                    ..uses(v, "actions/deploy-pages", &[])
                }],
                ..Job::default()
            },
        ],
        ..Workflow::default()
    }
}

fn dependency_review(_: &Facts, v: &Versions) -> Workflow {
    Workflow {
        name: "Dependency review".into(),
        on: Triggers {
            pull_request: Some(Filter::default()),
            ..Triggers::default()
        },
        permissions: Some(scopes(&[("contents", "read")])),
        concurrency: cancel_superseded(),
        jobs: vec![Job {
            id: "review".into(),
            timeout_minutes: Some(10),
            runs_on: "ubuntu-latest".into(),
            steps: vec![
                checkout(v),
                uses(
                    v,
                    "actions/dependency-review-action",
                    &[("fail-on-severity", "high")],
                ),
            ],
            ..Job::default()
        }],
        ..Workflow::default()
    }
}

fn codeql(f: &Facts, v: &Versions) -> Workflow {
    // github/codeql-action is a monorepo: init and analyze are subpaths.
    let version = v.get("github/codeql-action").unwrap_or("v4");
    let codeql = |action: &str| format!("github/codeql-action/{action}@{version}");
    let mut on = branch_triggers(f);
    on.schedule = vec!["27 4 * * 1".into()];
    Workflow {
        name: "CodeQL".into(),
        on,
        permissions: Some(scopes(&[("contents", "read")])),
        concurrency: cancel_superseded(),
        jobs: vec![Job {
            id: "analyze".into(),
            runs_on: "ubuntu-latest".into(),
            permissions: Some(scopes(&[
                ("actions", "read"),
                ("contents", "read"),
                ("security-events", "write"),
            ])),
            timeout_minutes: Some(30),
            steps: vec![
                checkout(v),
                Step {
                    name: Some("Initialize CodeQL".into()),
                    uses: Some(codeql("init")),
                    with: with(&[
                        ("languages", "javascript-typescript"),
                        ("build-mode", "none"),
                    ]),
                    ..Step::default()
                },
                Step {
                    name: Some("Analyze".into()),
                    uses: Some(codeql("analyze")),
                    ..Step::default()
                },
            ],
            ..Job::default()
        }],
        ..Workflow::default()
    }
}

fn release(_: &Facts, v: &Versions) -> Workflow {
    Workflow {
        name: "Release".into(),
        on: Triggers {
            push: Some(Filter {
                tags: vec!["v*".into()],
                ..Filter::default()
            }),
            ..Triggers::default()
        },
        permissions: Some(scopes(&[("contents", "read")])),
        jobs: vec![Job {
            id: "release".into(),
            timeout_minutes: Some(15),
            runs_on: "ubuntu-latest".into(),
            permissions: Some(scopes(&[("contents", "write")])),
            steps: vec![
                checkout(v),
                uses(
                    v,
                    "softprops/action-gh-release",
                    &[("generate_release_notes", "true")],
                ),
            ],
            ..Job::default()
        }],
        ..Workflow::default()
    }
}

fn docker(f: &Facts, v: &Versions) -> Workflow {
    let push = "${{ github.event_name != 'pull_request' }}";
    Workflow {
        name: "Docker".into(),
        on: Triggers {
            push: Some(Filter {
                branches: vec![f.branch.clone()],
                tags: vec!["v*".into()],
                ..Filter::default()
            }),
            pull_request: Some(Filter::default()),
            ..Triggers::default()
        },
        permissions: Some(scopes(&[("contents", "read")])),
        concurrency: cancel_superseded(),
        jobs: vec![Job {
            id: "image".into(),
            runs_on: "ubuntu-latest".into(),
            permissions: Some(scopes(&[("contents", "read"), ("packages", "write")])),
            timeout_minutes: Some(30),
            steps: vec![
                checkout(v),
                // Registry names must be lowercase; owners often aren't.
                run(
                    "Image name",
                    "echo \"IMAGE=ghcr.io/${GITHUB_REPOSITORY,,}\" >> \"$GITHUB_ENV\"",
                ),
                uses(v, "docker/setup-buildx-action", &[]),
                Step {
                    condition: Some(push.into()),
                    ..uses(
                        v,
                        "docker/login-action",
                        &[
                            ("registry", "ghcr.io"),
                            ("username", "${{ github.actor }}"),
                            ("password", "${{ secrets.GITHUB_TOKEN }}"),
                        ],
                    )
                },
                uses(
                    v,
                    "docker/build-push-action",
                    &[
                        ("context", "."),
                        ("push", push),
                        ("tags", "${{ env.IMAGE }}:${{ github.ref_name }}"),
                    ],
                ),
            ],
            ..Job::default()
        }],
        ..Workflow::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_lines() {
        assert_eq!(version_line("^8.2"), "8.2");
        assert_eq!(version_line(">=20 <23"), "20");
        assert_eq!(version_line("20.11.1"), "20.11");
        assert_eq!(version_line("lts/*"), "lts/*");
    }
}
