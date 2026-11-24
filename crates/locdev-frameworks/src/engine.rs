//! What a project is made of, from its files alone, by evaluating the
//! framework definitions. This is the only stack detection in LocDev.

use std::collections::BTreeSet;
use std::path::Path;

use locdev_core::model::{
    Component, ComponentCategory, DevServer, DevServerMode, DevTool, Integration, ProcessOffer,
    ProjectKind, RuntimeKind, Stack, WorkspaceMember,
};
use locdev_core::semver;
use locdev_core::slugify;

use crate::catalog::{Catalog, Definition, DevVariant, EnvSpec, PortMode};
use crate::project::ProjectFiles;

/// The user's choices about a project's stack (`Project.adapter`,
/// `stacks`, `hidden_stacks`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Overrides<'a> {
    pub adapter: Option<&'a str>,
    pub add: &'a [String],
    pub hide: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    pub kind: ProjectKind,
    pub name: String,
    /// Suggested command; `{port}` is substituted when the project starts.
    pub command: Option<String>,
    pub stack: Stack,
}

/// For `locdev add`: the folder's name as a site name, and its stack.
pub fn detect(catalog: &Catalog, path: &Path) -> Detection {
    let name = path
        .file_name()
        .map(|n| slugify(&n.to_string_lossy()))
        .unwrap_or_default();
    let stack = analyze(catalog, path, Overrides::default());
    Detection {
        kind: stack.kind,
        name,
        command: stack.command.clone(),
        stack,
    }
}

/// Vite ignores `PORT`; it takes the port on the command line. `--strictPort`
/// makes a taken port an error instead of a silent move to another one.
const VITE_ARGS: &str = "--port {port} --strictPort --host 127.0.0.1";

/// UI libraries named in labels ("React + Vite"), in order of preference.
const UI_ORDER: [&str; 5] = ["react", "vue", "svelte", "solid", "preact"];

struct Matched<'a> {
    def: &'a Definition,
    evidence: Vec<String>,
    manual: bool,
}

pub fn analyze(catalog: &Catalog, path: &Path, overrides: Overrides) -> Stack {
    analyze_inner(catalog, path, overrides, true)
}

fn analyze_inner(catalog: &Catalog, path: &Path, overrides: Overrides, members: bool) -> Stack {
    let files = ProjectFiles::new(path);
    let pm = files.package().map(|_| PackageManager::detect(path));

    let mut matched: Vec<Matched> = catalog
        .frameworks
        .iter()
        .filter(|d| !overrides.hide.contains(&d.id) || overrides.adapter == Some(d.id.as_str()))
        .filter_map(|def| {
            matches(&files, def).map(|evidence| Matched {
                def,
                evidence,
                manual: false,
            })
        })
        .collect();
    for id in overrides
        .add
        .iter()
        .map(String::as_str)
        .chain(overrides.adapter)
    {
        if let Some(def) = catalog.get(id)
            && !matched.iter().any(|m| m.def.id == id)
        {
            matched.push(Matched {
                def,
                evidence: vec!["added by hand".into()],
                manual: true,
            });
        }
    }

    // The primary component: a forced adapter, else the highest priority
    // that knows how to start this project, else the highest that matched.
    let mut by_priority: Vec<&Matched> = matched.iter().filter(|m| m.def.priority > 0).collect();
    by_priority.sort_by_key(|m| -m.def.priority);
    let forced = overrides
        .adapter
        .and_then(|id| matched.iter().find(|m| m.def.id == id));
    let runnable = |m: &&Matched| dev_variant(&files, m.def, pm).is_some();
    let primary = forced
        .or_else(|| by_priority.iter().copied().find(runnable))
        .or_else(|| by_priority.first().copied());

    let mut stack = Stack {
        package_manager: pm.map(PackageManager::kind),
        ..Stack::default()
    };
    let has = |id: &str| matched.iter().any(|m| m.def.id == id);

    if let Some(primary) = primary {
        let def = primary.def;
        stack.adapter = Some(def.id.clone());
        stack.kind = family(def.family.as_deref());
        stack.framework = label(def, &matched);
        stack.dev_tool = def.dev_tool.as_deref().and_then(dev_tool);
        stack.requires = def.requires.clone();
        stack.notes = def.notes.clone();
        match dev_variant(&files, def, pm) {
            Some((variant, command)) => {
                stack.command = Some(command);
                stack.integration = if variant.script.is_empty() {
                    Integration::Native
                } else {
                    Integration::Script
                };
                stack.port_env = variant.port_env.clone();
                if let Some(note) = &variant.note {
                    stack.notes.push(note.clone());
                }
                if variant.port == PortMode::Env
                    && variant.note.is_none()
                    && variant.port_env.is_empty()
                {
                    stack
                        .notes
                        .push("The app must take its port from the PORT variable.".into());
                }
            }
            None => {
                stack.integration = Integration::Custom;
                if !def.dev.is_empty() {
                    stack.notes.push(format!(
                        "{} was recognized, but none of its usual start commands fits this project; set one with `locdev edit <site> --command`.",
                        def.name
                    ));
                }
            }
        }
        if def.venv {
            stack.venv = [".venv", "venv"]
                .into_iter()
                .find(|d| path.join(d).join("pyvenv.cfg").is_file())
                .map(str::to_string);
        }
        if let Some(note) = support_note(&files, def) {
            stack.notes.push(note);
        }
        // Laravel's Vite helper, only through a script that starts Vite.
        if def.helper.as_deref() == Some("vite")
            && has("vite")
            && let (Some(pm), Some(name)) = (pm, vite_script(&files))
        {
            stack.dev_server = Some(DevServer {
                tool: DevTool::Vite,
                command: pm.run(&name, VITE_ARGS),
                port: None,
                mode: DevServerMode::Ask,
            });
        }
    } else {
        stack.framework = "Unknown".into();
    }

    // Components: primary first, then frameworks, runtimes, tools, UI, languages.
    let primary_id = primary.map(|m| m.def.id.clone());
    let mut components: Vec<Component> = matched
        .iter()
        .map(|m| {
            let version = m.def.version.iter().find_map(|rule| files.version(rule));
            Component {
                id: m.def.id.clone(),
                name: m.def.name.clone(),
                category: m.def.category,
                version_source: version.as_ref().map(|(_, s)| s.clone()),
                version: version.map(|(v, _)| v),
                evidence: m.evidence.clone(),
                manual: m.manual,
                primary: primary_id.as_deref() == Some(m.def.id.as_str()),
            }
        })
        .collect();
    // What the primary implies, when its files didn't show it already.
    if let Some(primary) = primary {
        for id in &primary.def.implies {
            if components.iter().any(|c| &c.id == id) || overrides.hide.contains(id) {
                continue;
            }
            if let Some(def) = catalog.get(id) {
                components.push(Component {
                    id: def.id.clone(),
                    name: def.name.clone(),
                    category: def.category,
                    version: None,
                    version_source: None,
                    evidence: vec![format!("used by {}", primary.def.name)],
                    manual: false,
                    primary: false,
                });
            }
        }
    }
    let rank = |c: &Component| {
        let category = match c.category {
            ComponentCategory::Framework => 1,
            ComponentCategory::Runtime => 2,
            ComponentCategory::Tool => 3,
            ComponentCategory::Ui => 4,
            ComponentCategory::Language => 5,
        };
        (!c.primary, category, c.name.to_lowercase())
    };
    components.sort_by_key(rank);

    // Auxiliary processes of every component present (hidden ones excluded).
    for m in &matched {
        for process in &m.def.processes {
            if !process.requires_any.is_empty()
                && !process
                    .requires_any
                    .iter()
                    .any(|r| files.check(r).is_some())
            {
                continue;
            }
            let Some((_, mut command)) = pick_variant(&files, &process.run, pm) else {
                continue;
            };
            if command.contains("{django_module}") {
                let Some(module) = files.django_module() else {
                    continue;
                };
                command = command.replace("{django_module}", &module);
            }
            if stack.processes.iter().any(|p| p.id == process.id) {
                continue;
            }
            stack.processes.push(ProcessOffer {
                id: process.id.clone(),
                name: process.name.clone(),
                kind: process.kind,
                command,
                port: process.port,
                conflicts: process.conflicts.clone(),
                note: process.note.clone(),
                source: m.def.id.clone(),
            });
        }
    }
    stack.components = components;

    let mut config: BTreeSet<String> = BTreeSet::new();
    for m in &matched {
        for file in &m.def.config_files {
            if files.has(file) {
                config.insert(file.clone());
            }
        }
    }
    stack.config_files = config.into_iter().collect();

    if members {
        stack.workspaces = crate::workspace::members(&files)
            .into_iter()
            .map(|rel| {
                let member = analyze_inner(catalog, &path.join(&rel), Overrides::default(), false);
                WorkspaceMember {
                    framework: if member.adapter.is_some() {
                        member.framework
                    } else {
                        rel.rsplit('/').next().unwrap_or(&rel).to_string()
                    },
                    adapter: member.adapter,
                    path: rel,
                }
            })
            .collect();
    }
    stack
}

/// The evidence for `def`, when every rule holds.
fn matches(files: &ProjectFiles, def: &Definition) -> Option<Vec<String>> {
    if def.detect.is_empty() {
        return None;
    }
    let mut evidence = Vec::new();
    for rule in &def.detect.all {
        evidence.push(files.check(rule)?);
    }
    if !def.detect.any.is_empty() {
        let found: Vec<String> = def
            .detect
            .any
            .iter()
            .filter_map(|r| files.check(r))
            .collect();
        if found.is_empty() {
            return None;
        }
        evidence.extend(found);
    }
    if def.detect.none.iter().any(|r| files.check(r).is_some()) {
        return None;
    }
    evidence.dedup();
    Some(evidence)
}

/// The first dev variant that applies, with its command.
fn dev_variant<'a>(
    files: &ProjectFiles,
    def: &'a Definition,
    pm: Option<PackageManager>,
) -> Option<(&'a DevVariant, String)> {
    pick_variant(files, &def.dev, pm)
}

/// The first variant whose rules hold, with its command.
fn pick_variant<'a>(
    files: &ProjectFiles,
    variants: &'a [DevVariant],
    pm: Option<PackageManager>,
) -> Option<(&'a DevVariant, String)> {
    variants.iter().find_map(|variant| {
        if variant.when.iter().any(|r| files.check(r).is_none()) {
            return None;
        }
        if !variant.script.is_empty() {
            let pm = pm?;
            let name = variant.script.iter().find(|name| {
                files.script(name).is_some_and(|body| {
                    variant
                        .script_contains
                        .as_deref()
                        .is_none_or(|needle| body.contains(needle))
                })
            })?;
            return Some((variant, pm.run(name, variant.args.as_deref().unwrap_or(""))));
        }
        let command = match (&variant.command_windows, cfg!(windows)) {
            (Some(windows), true) => windows,
            _ => variant.command.as_ref()?,
        };
        Some((variant, command.clone()))
    })
}

fn family(family: Option<&str>) -> ProjectKind {
    match family {
        Some("laravel") => ProjectKind::Laravel,
        Some("wordpress") => ProjectKind::Wordpress,
        Some("php") => ProjectKind::Php,
        Some("next") => ProjectKind::Next,
        Some("nuxt") => ProjectKind::Nuxt,
        Some("astro") => ProjectKind::Astro,
        Some("sveltekit") => ProjectKind::Sveltekit,
        Some("vite") => ProjectKind::Vite,
        Some("node") => ProjectKind::Node,
        Some("static") => ProjectKind::Static,
        _ => ProjectKind::Unknown,
    }
}

fn dev_tool(tool: &str) -> Option<DevTool> {
    match tool {
        "vite" => Some(DevTool::Vite),
        "next" => Some(DevTool::Next),
        "nuxt" => Some(DevTool::Nuxt),
        "astro" => Some(DevTool::Astro),
        _ => None,
    }
}

/// "Laravel + Vue + Vite", "React + Vite", "Vue (Node.js)", "Django".
fn label(def: &Definition, matched: &[Matched]) -> String {
    let ui = UI_ORDER
        .iter()
        .find_map(|id| matched.iter().find(|m| m.def.id == *id))
        .map(|m| m.def.name.as_str());
    let has_vite = matched.iter().any(|m| m.def.id == "vite");
    match (def.id.as_str(), ui) {
        ("laravel", ui) => {
            let mut label = "Laravel".to_string();
            if let Some(ui) = ui {
                label.push_str(&format!(" + {ui}"));
            }
            if has_vite {
                label.push_str(" + Vite");
            }
            label
        }
        ("vite", Some(ui)) => format!("{ui} + Vite"),
        ("nodejs", Some(ui)) => format!("{ui} (Node.js)"),
        _ => def.name.clone(),
    }
}

/// A note when the installed version is outside what the adapter was
/// written for (the command may still work).
fn support_note(files: &ProjectFiles, def: &Definition) -> Option<String> {
    let supported = def.supported.as_deref()?;
    let range = semver::parse_npm(supported).ok()?;
    let (version, source) = def.version.iter().find_map(|r| files.version(r))?;
    let exact = semver::Version::parse(&version).or_else(|| {
        // A declared range ("^11.0"): judge by its lowest major.
        let major = semver::parse_npm(&version)
            .or_else(|_| semver::parse_composer(&version))
            .ok()?
            .lowest_major()?;
        Some(semver::Version::new(major, u64::MAX / 2, 0))
    })?;
    (!range.matches(&exact)).then(|| {
        format!(
            "{} {version} ({source}) is older than the versions this adapter was written for ({supported}); the command may still work.",
            def.name
        )
    })
}

/// The script that starts Vite: `dev` when it does, else the first one that
/// runs `vite` without building or previewing.
fn vite_script(files: &ProjectFiles) -> Option<String> {
    let starts_vite = |body: &str| {
        let body = body.trim();
        (body == "vite" || body.starts_with("vite ") || body.starts_with("vite dev"))
            && !body.contains("build")
            && !body.contains("preview")
    };
    if files.script("dev").is_some_and(|b| starts_vite(&b)) {
        return Some("dev".into());
    }
    ["start", "serve", "watch", "hot"]
        .into_iter()
        .find(|name| files.script(name).is_some_and(|b| starts_vite(&b)))
        .map(str::to_string)
}

/// How the framework loads `.env` files, when its definition says so; the
/// condition (`if_any`) is evaluated against the project.
pub fn env_spec(catalog: &Catalog, stack: &Stack, path: &Path) -> Option<(String, EnvSpec)> {
    let def = catalog.get(stack.adapter.as_deref()?)?;
    let mut spec = def.env.clone()?;
    if !spec.if_any.is_empty() {
        let files = ProjectFiles::new(path);
        if !spec.if_any.iter().any(|r| files.check(r).is_some()) {
            spec.files.clear();
            spec.loader = "none".into();
        }
    }
    Some((def.name.clone(), spec))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl PackageManager {
    pub fn kind(self) -> RuntimeKind {
        match self {
            Self::Npm => RuntimeKind::Npm,
            Self::Pnpm => RuntimeKind::Pnpm,
            Self::Yarn => RuntimeKind::Yarn,
            Self::Bun => RuntimeKind::Bun,
        }
    }

    pub fn program(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
        }
    }

    pub fn detect(path: &Path) -> Self {
        let has = |file: &str| path.join(file).exists();
        if has("pnpm-lock.yaml") {
            Self::Pnpm
        } else if has("yarn.lock") {
            Self::Yarn
        } else if has("bun.lock") || has("bun.lockb") {
            Self::Bun
        } else {
            Self::Npm
        }
    }

    pub fn run(self, script: &str, args: &str) -> String {
        let base = format!("{} run {script}", self.program());
        match (self, args.is_empty()) {
            (_, true) => base,
            // npm needs `--` to hand arguments to the script.
            (Self::Npm, false) => format!("{base} -- {args}"),
            (_, false) => format!("{base} {args}"),
        }
    }
}
