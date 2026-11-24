//! `locdev new project …`, `locdev new list` and `locdev tools …`: creating a
//! project with the framework's own installer, from the terminal. The plan
//! is shown first; the run is the daemon's terminal, mirrored here (what you
//! type goes to the installer).

use std::collections::BTreeMap;
use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, bail};
use clap::{Args, Subcommand};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};
use locdev_core::scaffold::{DatabaseChoice, NewProject, ProjectPlan, Starter, Tool};

use crate::daemon_call;
use crate::output::Output;
use crate::work::confirm;

#[derive(Args)]
pub(crate) struct NewProjectArgs {
    /// The starter (`locdev new list`): laravel, symfony, php, wordpress, bedrock, nextjs, vite, vue, angular, nuxt, astro, sveltekit, nestjs, django, flask, fastapi, rust, rails, dotnet.
    starter: String,
    /// Folder (and site) name: lowercase letters, digits, - and _.
    name: String,
    /// How to create it (installer, composer, npm, pnpm, bun…); the first one by default.
    #[arg(long, visible_alias = "package-manager")]
    method: Option<String>,
    /// TypeScript, for starters that offer it (same as `-o language=ts`).
    #[arg(long)]
    typescript: bool,
    /// PHP version to create it with and pin the site to (8.4).
    #[arg(long)]
    php: Option<String>,
    /// Node.js version to create it with and pin the site to (22).
    #[arg(long)]
    node: Option<String>,
    /// A starter option, KEY=VALUE (see `locdev new list`). Repeatable.
    #[arg(long = "option", short = 'o', value_name = "KEY=VALUE")]
    options: Vec<String>,
    /// A package to add after creating it (npm or Composer, by the starter). Repeatable.
    #[arg(long = "package", short = 'p')]
    packages: Vec<String>,
    /// A LocDev database for it, SERVICE:INSTANCE (postgres:db, mysql:main…); the instance is created if needed.
    #[arg(long)]
    database: Option<String>,
    /// The folder to create it in (the current one by default).
    #[arg(long = "in", value_name = "DIR")]
    parent: Option<PathBuf>,
    /// Don't register it as a site at the end.
    #[arg(long)]
    no_site: bool,
    /// Run without asking (still shows the plan).
    #[arg(long, short = 'y')]
    yes: bool,
}

#[derive(Subcommand)]
pub(crate) enum ToolsCommand {
    /// The tools starters need, whether they're installed and their version.
    #[command(visible_alias = "ls")]
    List,
    /// Search npm or Packagist for packages to add to a new project.
    Search {
        #[arg(value_parser = ["npm", "packagist"])]
        registry: String,
        query: String,
    },
    /// The daemon's terminals (installs, new projects), running or finished.
    Terminals,
    /// Follow a terminal's output; lines you type go to it.
    Attach { id: u64 },
    /// Stop what a terminal is running.
    Kill { id: u64 },
    /// Forget a finished terminal.
    Close { id: u64 },
    /// Install a tool with its documented command (shown first), in a terminal.
    Install {
        id: String,
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

pub async fn list(paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    let Response::Starters { starters } = daemon_call(paths, Request::ListStarters).await? else {
        bail!("unexpected response from daemon");
    };
    if out.is_json() {
        println!("{}", serde_json::to_string_pretty(&starters)?);
        return Ok(ExitCode::SUCCESS);
    }
    for s in &starters {
        print_starter(s);
    }
    Ok(ExitCode::SUCCESS)
}

fn print_starter(s: &Starter) {
    let methods: Vec<&str> = s.methods.iter().map(|m| m.id.as_str()).collect();
    println!("{:<11} {} — {}", s.id, s.name, s.description);
    println!("            methods: {}", methods.join(", "));
    for o in &s.options {
        let values = if o.choices.is_empty() {
            "true|false".to_string()
        } else {
            o.choices
                .iter()
                .map(|c| c.value.as_str())
                .collect::<Vec<_>>()
                .join("|")
        };
        let only = if o.methods.is_empty() {
            String::new()
        } else {
            format!(" ({} only)", o.methods.join(", "))
        };
        println!(
            "            -o {}={values}  default {}{only}",
            o.id, o.default
        );
    }
    println!();
}

fn print_plan(plan: &ProjectPlan) {
    println!("Creates {}", plan.dir.display());
    for file in &plan.files {
        println!(
            "  + {} ({} lines, LocDev's starter file)",
            file.path,
            file.contents.lines().count()
        );
    }
    for (i, step) in plan.steps.iter().enumerate() {
        println!("  {}. {}\n     $ {}", i + 1, step.label, step.display());
    }
    for action in &plan.actions {
        println!("  · {action}");
    }
    for (i, step) in plan.finish.iter().enumerate() {
        println!(
            "  {}. {}\n     $ {}",
            plan.steps.len() + i + 1,
            step.label,
            step.display()
        );
    }
    for note in &plan.notes {
        println!("  note: {note}");
    }
}

/// `locdev new <starter> <name> …`: the same as `locdev new project`.
#[derive(clap::Parser)]
#[command(name = "locdev new", no_binary_name = true)]
struct Shortcut {
    #[command(flatten)]
    args: NewProjectArgs,
}

pub async fn shortcut(
    mut argv: Vec<String>,
    paths: &Paths,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    use clap::Parser;
    // Everything after an external subcommand arrives unparsed, the global
    // `--json` included.
    let json = argv.iter().any(|a| a == "--json");
    argv.retain(|a| a != "--json");
    let json_out = Output::new(true);
    let out = if json { &json_out } else { out };
    match Shortcut::try_parse_from(argv) {
        Ok(Shortcut { args }) => new_project(args, paths, out).await,
        Err(error) => error.exit(),
    }
}

async fn starters(paths: &Paths) -> anyhow::Result<Vec<Starter>> {
    let Response::Starters { starters } = daemon_call(paths, Request::ListStarters).await? else {
        bail!("unexpected response from daemon");
    };
    Ok(starters)
}

pub async fn new_project(
    args: NewProjectArgs,
    paths: &Paths,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    let starters = starters(paths).await?;
    let starter = starters
        .iter()
        .find(|s| s.id == args.starter)
        .with_context(|| format!("no starter '{}'; see `locdev new list`", args.starter))?;
    let method = match args.method {
        Some(m) => m,
        None => starter.methods[0].id.clone(),
    };
    let mut options = BTreeMap::new();
    for raw in &args.options {
        let (k, v) = raw
            .split_once('=')
            .with_context(|| format!("'{raw}' is not KEY=VALUE"))?;
        options.insert(k.to_string(), v.to_string());
    }
    if args.typescript {
        // `language` (most starters) or `types` (SvelteKit), whichever offers ts.
        let option = starter
            .options
            .iter()
            .find(|o| o.choices.iter().any(|c| c.value == "ts"))
            .with_context(|| format!("{} has no TypeScript option", starter.name))?;
        options.insert(option.id.clone(), "ts".into());
    }
    let database = match args.database {
        Some(raw) => {
            let (service, instance) = raw
                .split_once(':')
                .with_context(|| format!("'{raw}' is not SERVICE:INSTANCE"))?;
            Some(DatabaseChoice {
                service: service.into(),
                instance: instance.into(),
            })
        }
        None => None,
    };
    let parent = match args.parent {
        Some(p) => std::path::absolute(p)?,
        None => std::env::current_dir()?,
    };
    let project = NewProject {
        starter: starter.id.clone(),
        method,
        name: args.name,
        parent,
        options,
        packages: args.packages,
        database,
        register: !args.no_site,
        node: args.node,
        php: args.php,
    };
    create(project, args.yes, paths, out).await
}

/// Shows the plan, asks, and follows the run in this terminal.
async fn create(
    project: NewProject,
    yes: bool,
    paths: &Paths,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    let Response::ProjectPlan(plan) = daemon_call(
        paths,
        Request::PlanProject {
            project: project.clone(),
        },
    )
    .await?
    else {
        bail!("unexpected response from daemon");
    };
    if out.is_json() && !yes {
        println!("{}", serde_json::to_string_pretty(&plan)?);
        return Ok(ExitCode::SUCCESS);
    }
    print_plan(&plan);
    if !plan.missing.is_empty() {
        bail!(
            "install {} first (`locdev tools list`)",
            plan.missing.join(", ")
        );
    }
    if !yes {
        confirm("Run this?")?;
    }
    let Response::Terminal(info) = daemon_call(paths, Request::CreateProject { project }).await?
    else {
        bail!("unexpected response from daemon");
    };
    follow(paths, info.id).await
}

/// One answer from the terminal; Enter keeps `default`.
fn ask(question: &str, default: &str) -> anyhow::Result<String> {
    if default.is_empty() {
        eprint!("{question}: ");
    } else {
        eprint!("{question} [{default}]: ");
    }
    std::io::stderr().flush()?;
    let mut answer = String::new();
    if std::io::stdin().lock().read_line(&mut answer)? == 0 {
        bail!("cancelled; nothing was changed");
    }
    let answer = answer.trim();
    Ok(if answer.is_empty() { default } else { answer }.to_string())
}

/// `locdev new` alone: the same choices as the app's wizard, as questions.
pub async fn wizard(paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    if !std::io::stdin().is_terminal() {
        bail!(
            "`locdev new` alone asks questions in a terminal; use `locdev new <starter> <name>` (see `locdev new list`)"
        );
    }
    let starters = starters(paths).await?;
    eprintln!("New project\n");
    for (i, s) in starters.iter().enumerate() {
        eprintln!("  {:>2}. {:<20} {}", i + 1, s.name, s.description);
    }
    let starter = loop {
        let answer = ask("\nFramework (number or id)", "")?;
        let found = answer
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|n| starters.get(n))
            .or_else(|| starters.iter().find(|s| s.id == answer));
        match found {
            Some(s) => break s,
            None => eprintln!("No starter '{answer}'."),
        }
    };
    let name = ask("Name (folder and site: lowercase, digits, - and _)", "")?;
    let parent = std::path::absolute(ask(
        "Create it in",
        &std::env::current_dir()?.to_string_lossy(),
    )?)?;
    let method = if starter.methods.len() == 1 {
        starter.methods[0].id.clone()
    } else {
        let ids: Vec<&str> = starter.methods.iter().map(|m| m.id.as_str()).collect();
        loop {
            let answer = ask(&format!("Create with ({})", ids.join(", ")), ids[0])?;
            if ids.contains(&answer.as_str()) {
                break answer;
            }
            eprintln!("Pick one of: {}", ids.join(", "));
        }
    };
    let mut options = BTreeMap::new();
    // In order, like the installers: an answer decides what is asked next.
    for option in &starter.options {
        if !starter.asks(option, &method, &options) {
            continue;
        }
        let values: Vec<&str> = if option.choices.is_empty() {
            vec!["true", "false"]
        } else {
            option.choices.iter().map(|c| c.value.as_str()).collect()
        };
        loop {
            let answer = ask(
                &format!("{} ({})", option.label, values.join(", ")),
                &option.default,
            )?;
            if values.contains(&answer.as_str()) {
                options.insert(option.id.clone(), answer);
                break;
            }
            eprintln!("Pick one of: {}", values.join(", "));
        }
    }
    let version = |label: &str| -> anyhow::Result<Option<String>> {
        let answer = ask(&format!("{label} version (Enter: LocDev's default)"), "")?;
        Ok((!answer.is_empty()).then_some(answer))
    };
    let php = if starter.family == "php" {
        version("PHP")?
    } else {
        None
    };
    let node = if starter.family == "javascript" {
        version("Node.js")?
    } else {
        None
    };
    let project = NewProject {
        starter: starter.id.clone(),
        method,
        name,
        parent,
        options,
        packages: Vec::new(),
        database: None,
        register: true,
        node,
        php,
    };
    eprintln!();
    create(project, false, paths, out).await
}

pub async fn tools(command: ToolsCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        ToolsCommand::Search { registry, query } => {
            let registry = if registry == "npm" {
                locdev_core::scaffold::PackageRegistry::Npm
            } else {
                locdev_core::scaffold::PackageRegistry::Packagist
            };
            let Response::Packages { packages } =
                daemon_call(paths, Request::SearchPackages { registry, query }).await?
            else {
                bail!("unexpected response from daemon");
            };
            if out.is_json() {
                out.print_json(&packages);
            } else if packages.is_empty() {
                println!("nothing found");
            } else {
                for p in &packages {
                    println!(
                        "{:<36} {:<10} {}",
                        p.name,
                        p.version.as_deref().unwrap_or(""),
                        p.description.as_deref().unwrap_or("")
                    );
                }
            }
            return Ok(ExitCode::SUCCESS);
        }
        ToolsCommand::Terminals => {
            let Response::Terminals { terminals } =
                daemon_call(paths, Request::ListTerminals).await?
            else {
                bail!("unexpected response from daemon");
            };
            if out.is_json() {
                out.print_json(&terminals);
            } else if terminals.is_empty() {
                println!("no terminals");
            } else {
                for t in &terminals {
                    let state = match (t.running, t.code) {
                        (true, _) => format!("running (step {}/{})", t.step, t.steps),
                        (false, Some(0)) => "done".into(),
                        (false, Some(code)) => format!("failed (exit {code})"),
                        (false, None) => "stopped".into(),
                    };
                    println!("{:<5} {:<40} {state}", t.id, t.title);
                }
            }
            return Ok(ExitCode::SUCCESS);
        }
        ToolsCommand::Attach { id } => return follow(paths, id).await,
        ToolsCommand::Kill { id } => {
            daemon_call(paths, Request::TerminalKill { id }).await?;
            out.message(&format!("terminal {id} stopped"));
            return Ok(ExitCode::SUCCESS);
        }
        ToolsCommand::Close { id } => {
            daemon_call(paths, Request::TerminalClose { id }).await?;
            out.message(&format!("terminal {id} closed"));
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }
    let Response::Tools { tools } = daemon_call(paths, Request::ListTools).await? else {
        bail!("unexpected response from daemon");
    };
    match command {
        ToolsCommand::List => {
            if out.is_json() {
                println!("{}", serde_json::to_string_pretty(&tools)?);
            } else {
                for t in &tools {
                    print_tool(t);
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        ToolsCommand::Install { id, yes } => {
            let tool = tools
                .iter()
                .find(|t| t.id == id)
                .with_context(|| format!("no tool '{id}'"))?;
            if let Some(locdev_core::scaffold::ToolInstall::Command { steps }) = &tool.install {
                for s in steps {
                    println!("$ {}", s.display());
                }
            }
            if !yes {
                confirm(&format!("Install {}?", tool.name))?;
            }
            let Response::Terminal(info) = daemon_call(paths, Request::InstallTool { id }).await?
            else {
                bail!("unexpected response from daemon");
            };
            follow(paths, info.id).await
        }
        _ => unreachable!("handled above"),
    }
}

fn print_tool(t: &Tool) {
    let state = match (&t.installed, &t.version) {
        (true, Some(v)) => v.clone(),
        (true, None) => "installed".into(),
        (false, _) => "not installed".into(),
    };
    let how = match &t.install {
        Some(locdev_core::scaffold::ToolInstall::Command { steps }) => {
            format!(
                "locdev tools install {}  ({})",
                t.id,
                steps
                    .iter()
                    .map(|s| s.display())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        }
        Some(locdev_core::scaffold::ToolInstall::Runtime { runtime }) => {
            format!("locdev runtime install {runtime}")
        }
        None => t.docs.clone(),
    };
    println!("{:<9} {:<18} {state}", t.id, t.name);
    if !t.installed {
        println!("          install: {how}");
    }
}

/// Prints the terminal's output until it ends; lines typed here go to it.
pub(crate) async fn follow(paths: &Paths, id: u64) -> anyhow::Result<ExitCode> {
    if std::io::stdin().is_terminal() {
        let paths = paths.clone();
        let handle = tokio::runtime::Handle::current();
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                let data = format!("{line}\r");
                let _ = handle.block_on(daemon_call(&paths, Request::TerminalWrite { id, data }));
            }
        });
    }
    let mut from = 0;
    loop {
        let Response::TerminalOutput(out) =
            daemon_call(paths, Request::TerminalRead { id, from }).await?
        else {
            bail!("unexpected response from daemon");
        };
        // ConPTY asks where the cursor is (`ESC[6n`) and waits for the
        // terminal's answer; the app's xterm gives it, here we do.
        if out.data.contains("\x1b[6n") {
            let data = "\x1b[1;1R".to_string();
            daemon_call(paths, Request::TerminalWrite { id, data }).await?;
        }
        print!("{}", out.data.replace("\x1b[6n", ""));
        std::io::stdout().flush()?;
        from = out.next;
        if !out.info.running {
            println!();
            if let Some(site) = out.info.site {
                println!("Site {site} registered: `locdev up {site}`.");
            }
            return Ok(if out.info.code == Some(0) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            });
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}
