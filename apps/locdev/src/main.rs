mod actions;
mod catalog;
mod config;
mod db;
mod debug;
mod deploy;
mod dns;
mod editor;
mod email;
mod env;
mod mail;
mod observe;
mod output;
mod park;
mod php;
mod process;
mod runtime;
mod scaffold;
mod services;
mod site;
mod stack;
mod subdomain;
mod tls;
mod tui;
mod tunnel;
mod vcs;
mod work;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use locdev_core::model::{Check, CheckLevel, ProcessRole, ProjectView, RuntimeKind};
use locdev_core::paths::Paths;
use locdev_core::project_domain;
use locdev_core::protocol::{PROTOCOL_VERSION, Request, Response};
use locdev_ipc::{IpcError, launcher};

use crate::output::Output;

#[derive(Parser)]
#[command(
    name = "locdev",
    version,
    about = "Local development environment",
    long_about = "Local development environment.\n\nRun `locdev` alone in a terminal to open the terminal UI."
)]
pub(crate) struct Cli {
    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    json: bool,

    /// Without a command, opens the terminal UI (in an interactive terminal).
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// The terminal UI: dashboard, sites, services, logs and every command.
    ///
    /// Also what `locdev` with no command opens. Sites keep running when you
    /// quit; anything without a screen of its own runs from the `:` prompt.
    #[command(visible_alias = "ui")]
    Tui {
        /// The light ("paper") palette instead of the dark one.
        #[arg(long)]
        light: bool,
    },
    /// Show client and daemon versions.
    Version,
    /// Show daemon status.
    Status,
    /// Check the environment for problems.
    Doctor,
    /// Start the background daemon.
    Start,
    /// Stop the background daemon and every running project.
    Stop,
    /// Restart the background daemon.
    Restart,

    /// Register a project folder (defaults to the current directory).
    #[command(visible_alias = "link")]
    Add {
        path: Option<PathBuf>,
        /// Project name; becomes <name>.test. Defaults to the folder name.
        #[arg(long)]
        name: Option<String>,
        /// Dev server command. `{port}` is replaced with the project's port.
        #[arg(long)]
        command: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        /// Start the project whenever the daemon starts.
        #[arg(long)]
        auto_start: bool,
        /// Start the project right away.
        #[arg(long)]
        start: bool,
    },
    /// List registered projects.
    #[command(visible_alias = "ls")]
    List,
    /// Parked folders: every project inside one becomes a site, kept in step as projects come and go.
    Park(park::ParkArgs),
    /// PHP settings per version (memory_limit, upload size, timezone…), applied to the sites LocDev runs.
    #[command(subcommand)]
    Php(php::PhpCommand),
    /// Share a running site on the internet through a tunnel (Cloudflare Quick Tunnels).
    #[command(subcommand)]
    Tunnel(tunnel::TunnelCommand),
    /// The Debug Center: exceptions, dumps, log errors, HTTP 5xx and process failures of your sites.
    #[command(subcommand)]
    Debug(debug::DebugCommand),
    /// The code editor sites and files open in (VS Code, Cursor, Zed, JetBrains…).
    #[command(subcommand)]
    Editor(editor::EditorCommand),
    /// Request metrics of a site: counts by status, latency percentiles, busiest paths.
    Metrics {
        site: Option<String>,
        /// Start counting again.
        #[arg(long)]
        reset: bool,
    },
    /// Laravel Tinker with the site's PHP and environment (code, stdin or a prompt).
    Tinker {
        /// PHP to run, e.g. 'User::count()'. Without it: stdin, or a prompt in a terminal.
        #[arg(long, short = 'e')]
        execute: Option<String>,
        #[arg(long)]
        site: Option<String>,
    },
    /// Check for a newer LocDev release, and how to get it.
    Update {
        /// Ask GitHub now instead of using the last answer (kept 6 hours).
        #[arg(long)]
        refresh: bool,
    },
    /// Change a project's settings.
    Edit {
        name: String,
        /// New dev server command (pass "" to clear it).
        #[arg(long)]
        command: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        auto_start: Option<bool>,
        /// Node.js version for this project ("22", "20.11"); "" uses the default.
        #[arg(long)]
        node: Option<String>,
        /// PHP version for this project ("8.3"); "" uses the default.
        #[arg(long)]
        php: Option<String>,
        /// pnpm version for this project; "" uses the default.
        #[arg(long)]
        pnpm: Option<String>,
        /// Yarn version for this project; "" uses the default.
        #[arg(long)]
        yarn: Option<String>,
        /// Bun version for this project; "" uses the default.
        #[arg(long)]
        bun: Option<String>,
        /// Composer version for this project; "" uses the default.
        #[arg(long)]
        composer: Option<String>,
        /// Restart the dev server when it fails (backoff, at most 5 times in 2 minutes).
        #[arg(long)]
        restart_on_failure: Option<bool>,
    },
    /// Unregister a project. Its files are left untouched.
    #[command(visible_alias = "rm")]
    Remove { name: String },
    /// Start a site (its helper too, when set to start with it).
    Up {
        /// Defaults to the site in the current directory.
        name: Option<String>,
    },
    /// Stop a site and its helper dev server.
    Down { name: Option<String> },
    /// Restart a site (and its helper, if running).
    Reload { name: Option<String> },
    /// Show a site's output.
    Logs {
        name: Option<String>,
        /// Number of lines to show.
        #[arg(short = 'n', long, default_value_t = 100)]
        lines: usize,
        /// Keep printing new output.
        #[arg(short, long)]
        follow: bool,
        /// The helper dev server's output instead of the main process's.
        #[arg(long)]
        dev: bool,
    },
    /// Everything about a site: stack, addresses, runtimes, processes.
    Info { name: Option<String> },
    /// Open a site in the browser.
    Open {
        name: Option<String>,
        /// Open the HTTPS address even if HTTP is the default.
        #[arg(long)]
        https: bool,
    },
    /// Make a site HTTPS-only, checking and fixing what that needs.
    ///
    /// Checks DNS, the HTTPS listener, the site's certificate and whether the
    /// LocDev CA is trusted. Changes that need administrator rights (trusting
    /// the CA, routing *.test) are shown and confirmed first.
    Secure {
        name: Option<String>,
        /// Serve the site over plain HTTP again.
        #[arg(long)]
        off: bool,
        /// Do not ask before applying system changes.
        #[arg(short, long)]
        yes: bool,
    },
    /// The helper dev server next to a site (Vite beside Laravel).
    #[command(subcommand)]
    Dev(DevCommand),
    /// A site's subdomains: the wildcard and rules (`api`, `*.tenants`) to other ports or processes.
    #[command(subcommand, visible_alias = "sub")]
    Subdomain(subdomain::SubdomainCommand),
    /// How a hostname is resolved (DNS), routed (proxy) and served over HTTPS.
    Route {
        /// `api.shop.test`, `acme.tenants.shop.test:443`…
        host: String,
    },
    /// Every process LocDev runs, with CPU and memory.
    Ps,
    /// A site's dependencies: versions, updates, vulnerabilities, updating.
    #[command(subcommand)]
    Deps(work::DepsCommand),
    /// Background jobs: dependency checks and updates, runtime installs.
    #[command(subcommand)]
    Jobs(work::JobsCommand),
    /// Notifications LocDev keeps for you.
    #[command(subcommand, visible_alias = "notif")]
    Notifications(work::NotificationsCommand),
    /// Automation, notification and network settings.
    #[command(subcommand)]
    Settings(work::SettingsCommand),
    /// A site's Git repository: status, history, branches, stashes.
    #[command(subcommand)]
    Git(vcs::GitCommand),
    /// The GitHub account LocDev uses for issues, pull requests and workflows.
    #[command(subcommand)]
    Github(vcs::GithubCommand),
    /// GitHub Actions workflows: templates, checks, new files.
    #[command(subcommand)]
    Actions(actions::ActionsCommand),
    /// A site's auxiliary processes: queue workers, schedulers, WebSocket servers.
    #[command(subcommand, visible_alias = "proc")]
    Process(process::ProcessCommand),
    /// Databases sites use: connections, tables, rows and SQL.
    #[command(subcommand)]
    Db(db::DbCommand),
    /// A site's config files: options, analysis, history (comments are kept).
    #[command(subcommand)]
    Config(config::ConfigCommand),
    /// Create a project (`locdev new laravel shop`; alone, a guided setup) or things in a site (`locdev new email contact`).
    New(email::NewArgs),
    /// Tools new projects need (installers, package managers): status and install.
    #[command(subcommand)]
    Tools(scaffold::ToolsCommand),
    /// Email Studio: a site's email templates and components, previews and test sends.
    #[command(subcommand)]
    Email(email::EmailCommand),
    /// The Mailbox: e-mail your sites send, captured locally (nothing is delivered).
    #[command(subcommand)]
    Mail(mail::MailCommand),
    /// Deploy assistant: platforms that fit, what they need, readiness. Sends nothing.
    #[command(subcommand)]
    Deploy(deploy::DeployCommand),
    /// Frameworks: detection, manual stacks and the definitions in use.
    #[command(subcommand)]
    Stack(stack::StackCommand),
    /// Shared environment variables and secrets, injected into sites' processes.
    #[command(subcommand)]
    Env(env::EnvCommand),
    /// The password that guards destructive operations in LocDev.
    #[command(subcommand)]
    Security(vcs::SecurityCommand),
    /// Print the command catalog as JSON (used by the app and the docs).
    #[command(hide = true)]
    Catalog,

    /// Print the .test domain for a project name.
    Domain { name: String },

    /// Manage how the OS resolves *.test.
    #[command(subcommand)]
    Dns(DnsCommand),

    /// Manage HTTPS and the local certificate authority.
    #[command(subcommand)]
    Tls(TlsCommand),

    /// Start the daemon at login (per user, no administrator rights).
    #[command(subcommand)]
    Startup(ServiceCommand),

    /// Databases and caches: install, create instances, start/stop, back up.
    #[command(subcommand, visible_alias = "svc")]
    Service(services::ServiceCmd),

    /// Manage runtimes (Node.js, PHP) and package managers (pnpm, Yarn, Composer…).
    #[command(subcommand, visible_alias = "rt")]
    Runtime(RuntimeCommand),
}

#[derive(Subcommand)]
pub(crate) enum DevCommand {
    /// Show the helper's command, mode and state.
    Status { site: Option<String> },
    /// Start the helper now.
    Start { site: Option<String> },
    /// Stop the helper.
    Stop { site: Option<String> },
    /// Restart the helper.
    Restart { site: Option<String> },
    /// Decide how LocDev runs the detected helper: ask, manual, auto or never.
    Mode {
        mode: String,
        site: Option<String>,
        /// Use this command instead of the detected one ({port} is replaced).
        #[arg(long)]
        command: Option<String>,
        /// Start it right away.
        #[arg(long)]
        start: bool,
    },
    /// Show the helper's output.
    Logs {
        site: Option<String>,
        #[arg(short = 'n', long, default_value_t = 100)]
        lines: usize,
        #[arg(short, long)]
        follow: bool,
    },
}

#[derive(Subcommand)]
enum RuntimeCommand {
    /// Installed and detected versions, and the defaults.
    #[command(visible_alias = "ls")]
    List {
        /// Re-scan the machine for installed runtimes.
        #[arg(long)]
        refresh: bool,
    },
    /// Versions available to install.
    Available {
        kind: RuntimeKind,
        /// Every release instead of the newest of each major line.
        #[arg(long)]
        all: bool,
    },
    /// Download, verify and install a version ("22", "22.11.0", "lts", "latest").
    Install { kind: RuntimeKind, version: String },
    /// Install the newest release of an installed line ("22", "8.3").
    Update {
        kind: RuntimeKind,
        /// The line to update; every managed line when omitted.
        line: Option<String>,
    },
    /// Remove a LocDev-managed version.
    #[command(visible_alias = "rm")]
    Uninstall {
        kind: RuntimeKind,
        version: String,
        /// Remove it even if stopped sites or the default use it.
        #[arg(long)]
        force: bool,
    },
    /// The PHP extensions a site needs against what its PHP loads, and the
    /// ones its packages recommend.
    Extensions { site: Option<String> },
    /// Every extension a PHP on this machine loads (and the ones it has but
    /// doesn't load). The default PHP when no version is given.
    Modules {
        /// A detected PHP version ("8.4", "8.4.12").
        version: Option<String>,
    },
    /// Set the default version for projects without a pin ("" or --clear to unset).
    Default {
        kind: RuntimeKind,
        version: Option<String>,
        #[arg(long)]
        clear: bool,
    },
    /// Use the default versions in new terminals ("on"/"off"; shows the state without one).
    Terminal {
        #[arg(value_parser = ["on", "off"])]
        state: Option<String>,
    },
}

#[derive(Subcommand)]
enum ServiceCommand {
    /// Show whether the daemon starts at login.
    Status,
    /// Start the daemon at login.
    Install,
    /// Stop starting the daemon at login.
    Uninstall,
}

#[derive(Subcommand)]
enum TlsCommand {
    /// Show HTTPS and whether the local CA is trusted.
    Status,
    /// Trust the local CA in the system and browser stores (asks for administrator rights).
    Trust {
        #[arg(short, long)]
        yes: bool,
    },
    /// Remove the local CA from the system and browser stores.
    Untrust {
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum DnsCommand {
    /// Show the DNS server and how *.test is routed.
    Status,
    /// Route *.test to LocDev (asks for administrator rights).
    Install {
        /// Do not ask for confirmation.
        #[arg(short, long)]
        yes: bool,
    },
    /// Undo `locdev dns install` (asks for administrator rights).
    Uninstall {
        #[arg(short, long)]
        yes: bool,
    },
}

fn main() -> ExitCode {
    // Windows gives the main thread 1 MB of stack, which the command tree and
    // `run`'s future overflow in debug builds: run on a thread with more.
    std::thread::Builder::new()
        .name("locdev".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime")
                .block_on(cli_main())
        })
        .expect("main thread")
        .join()
        .unwrap_or(ExitCode::FAILURE)
}

async fn cli_main() -> ExitCode {
    quiet_on_closed_pipe();
    let cli = Cli::parse();
    let out = Output::new(cli.json);
    let command = match cli.command {
        Some(command) => command,
        None if !cli.json && tui::interactive() => Command::Tui { light: false },
        None => {
            use clap::CommandFactory;
            let _ = Cli::command().print_help();
            return ExitCode::SUCCESS;
        }
    };

    match run(command, &out).await {
        Ok(code) => code,
        Err(error) => {
            out.error(&error);
            ExitCode::FAILURE
        }
    }
}

async fn run(command: Command, out: &Output) -> anyhow::Result<ExitCode> {
    let paths = Paths::resolve()?;

    match command {
        Command::Tui { light } => return tui::run(paths, tui::Options { light }).await,
        Command::Version => {
            let daemon = match call(&paths, Request::Status).await {
                Ok(Response::Status(status)) => Some(status),
                _ => None,
            };
            out.version(daemon.as_ref());
        }

        Command::Status => match call(&paths, Request::Status).await {
            Ok(Response::Status(status)) => out.status(Some(&status)),
            Err(IpcError::Io(_)) => {
                out.status(None);
                return Ok(ExitCode::from(3));
            }
            other => unexpected(other)?,
        },

        Command::Doctor => {
            let mut checks = vec![];
            let daemon_checks = match call(&paths, Request::Status).await {
                Ok(Response::Status(status)) => {
                    checks.push(Check::new(
                        "daemon",
                        "Daemon",
                        CheckLevel::Ok,
                        format!("running (pid {}, v{})", status.pid, status.version),
                    ));
                    match call(&paths, Request::Doctor).await? {
                        Response::Doctor { checks } => checks,
                        other => unexpected(Ok(other))?,
                    }
                }
                Err(IpcError::IncompatibleProtocol { daemon, client }) => {
                    checks.push(
                        Check::new(
                            "daemon",
                            "Daemon",
                            CheckLevel::Error,
                            format!("daemon speaks protocol {daemon}, this CLI speaks {client}"),
                        )
                        .with_remedy("locdev restart"),
                    );
                    vec![]
                }
                Ok(other) => unexpected(Ok(other))?,
                Err(error) => {
                    checks.push(
                        Check::new(
                            "daemon",
                            "Daemon",
                            CheckLevel::Error,
                            format!("not reachable: {error}"),
                        )
                        .with_remedy("locdev start"),
                    );
                    vec![]
                }
            };
            checks.extend(daemon_checks);

            out.checks(&checks);
            if checks.iter().any(|c| c.level == CheckLevel::Error) {
                return Ok(ExitCode::FAILURE);
            }
        }

        Command::Start => {
            let started = launcher::ensure_running(&paths)
                .await
                .context("could not start the daemon")?;
            out.message(if started {
                "daemon started"
            } else {
                "daemon already running"
            });
        }

        Command::Stop => {
            let stopped = launcher::stop(&paths)
                .await
                .context("could not stop the daemon")?;
            out.message(if stopped {
                "daemon stopped"
            } else {
                "daemon was not running"
            });
        }

        Command::Restart => {
            launcher::stop(&paths)
                .await
                .context("could not stop the daemon")?;
            launcher::ensure_running(&paths)
                .await
                .context("could not start the daemon")?;
            out.message("daemon restarted");
        }

        Command::Add {
            path,
            name,
            command,
            port,
            auto_start,
            start,
        } => {
            let path = match path {
                Some(path) => path,
                None => std::env::current_dir()?,
            };
            let path = std::path::absolute(&path)
                .with_context(|| format!("invalid path {}", path.display()))?;

            let request = Request::AddProject {
                path,
                name,
                command,
                port,
                auto_start,
            };
            let mut project = expect_project(daemon_call(&paths, request).await?)?;

            if start {
                let request = Request::StartProject {
                    name: project.project.name.clone(),
                };
                project = expect_project(daemon_call(&paths, request).await?)?;
            }
            out.project_added(&project);
        }

        Command::List => match daemon_call(&paths, Request::ListProjects).await? {
            Response::Projects { projects } => out.projects(&projects),
            other => unexpected(Ok(other))?,
        },

        Command::Edit {
            name,
            command,
            port,
            auto_start,
            node,
            php,
            pnpm,
            yarn,
            bun,
            composer,
            restart_on_failure,
        } => {
            let tools = [
                (RuntimeKind::Pnpm, pnpm),
                (RuntimeKind::Yarn, yarn),
                (RuntimeKind::Bun, bun),
                (RuntimeKind::Composer, composer),
            ]
            .into_iter()
            .filter_map(|(kind, pin)| Some((kind, pin?)))
            .collect();
            let request = Request::UpdateProject {
                name,
                command,
                port,
                auto_start,
                node,
                php,
                tools,
                restart_on_failure,
            };
            out.project(&expect_project(daemon_call(&paths, request).await?)?);
        }

        Command::Remove { name } => {
            daemon_call(&paths, Request::RemoveProject { name: name.clone() }).await?;
            out.message(&format!("removed {name} (files were not touched)"));
        }

        Command::Up { name } => {
            let name = site::resolve(&paths, name).await?.project.name;
            let response = daemon_call(&paths, Request::StartProject { name }).await?;
            out.project(&expect_project(response)?);
        }

        Command::Down { name } => {
            let name = site::resolve(&paths, name).await?.project.name;
            let response = daemon_call(&paths, Request::StopProject { name }).await?;
            out.project(&expect_project(response)?);
        }

        Command::Reload { name } => {
            let name = site::resolve(&paths, name).await?.project.name;
            let response = daemon_call(&paths, Request::RestartProject { name }).await?;
            out.project(&expect_project(response)?);
        }

        Command::Logs {
            name,
            lines,
            follow,
            dev,
        } => {
            let name = site::resolve(&paths, name).await?.project.name;
            let process = if dev {
                ProcessRole::Dev
            } else {
                ProcessRole::Main
            };
            return site::logs(&paths, name, process, None, lines, follow, out).await;
        }

        Command::Info { name } => return site::info(&paths, name, out).await,
        Command::Open { name, https } => return site::open(&paths, name, https, out).await,
        Command::Secure { name, off, yes } => {
            return site::secure(&paths, name, off, yes, out).await;
        }
        Command::Dev(command) => return site::dev(command, &paths, out).await,
        Command::Subdomain(command) => return subdomain::run(command, &paths, out).await,
        Command::Park(args) => return park::run(args, &paths, out).await,
        Command::Php(command) => return php::run(command, &paths, out).await,
        Command::Tunnel(command) => return tunnel::run(command, &paths, out).await,
        Command::Editor(command) => return editor::run(command, &paths, out).await,
        Command::Metrics { site, reset } => {
            return observe::metrics(&paths, out, site, reset).await;
        }
        Command::Tinker { execute, site } => {
            return observe::tinker(&paths, out, site, execute).await;
        }
        Command::Debug(command) => return debug::run(command, &paths, out).await,
        Command::Update { refresh } => {
            let Response::Updates(status) =
                daemon_call(&paths, Request::UpdateCheck { refresh }).await?
            else {
                anyhow::bail!("unexpected response from daemon");
            };
            out.updates(&status);
            if status.error.is_some() {
                return Ok(ExitCode::FAILURE);
            }
        }
        Command::Route { host } => return subdomain::route(&paths, host, out).await,
        Command::Ps => return site::ps(&paths, out).await,
        Command::Deps(command) => return work::deps(command, &paths, out).await,
        Command::Jobs(command) => return work::jobs(command, &paths, out).await,
        Command::Notifications(command) => return work::notifications(command, &paths, out).await,
        Command::Settings(command) => return work::settings(command, &paths, out).await,
        Command::Git(command) => return vcs::git(command, &paths, out).await,
        Command::Github(command) => return vcs::github(command, &paths, out).await,
        Command::Security(command) => return vcs::security(command, &paths, out).await,
        Command::Actions(command) => return actions::run(command, &paths, out).await,
        Command::Env(command) => return env::run(command, &paths, out).await,
        Command::Stack(command) => return stack::run(command, &paths, out).await,
        Command::Db(command) => return db::run(command, &paths, out).await,
        Command::Config(command) => return config::run(command, &paths, out).await,
        Command::Deploy(command) => return deploy::run(command, &paths, out).await,
        Command::Mail(command) => return mail::run(command, &paths, out).await,
        Command::New(args) => return email::new(args, &paths, out).await,
        Command::Tools(command) => return scaffold::tools(command, &paths, out).await,
        Command::Email(command) => return email::run(command, &paths, out).await,
        Command::Process(command) => return process::run(command, &paths, out).await,
        Command::Catalog => {
            println!("{}", serde_json::to_string_pretty(&catalog::catalog())?);
        }

        Command::Dns(command) => return dns::run(command, &paths, out).await,
        Command::Tls(command) => return tls::run(command, &paths, out).await,
        Command::Runtime(command) => return runtime::run(command, &paths, out).await,

        Command::Service(command) => return services::run(command, &paths, out).await,

        Command::Startup(command) => {
            use locdev_platform::service;
            match command {
                ServiceCommand::Status => {}
                ServiceCommand::Install => {
                    let daemon = launcher::daemon_executable()
                        .context("could not find locdevd to register")?;
                    let home = std::env::var_os(locdev_core::paths::HOME_ENV).map(PathBuf::from);
                    service::install(&daemon, home.as_deref())
                        .context("could not register the login item")?;
                }
                ServiceCommand::Uninstall => {
                    service::uninstall().context("could not remove the login item")?;
                }
            }
            out.service(&service::status());
        }

        Command::Domain { name } => {
            out.message(&project_domain(&name)?);
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// `locdev … | head` closes the pipe before we're done printing, and
/// `println!` panics on that. Rust ignores SIGPIPE, and restoring the
/// default needs `unsafe` (forbidden here), so the panic hook does what the
/// signal would: stop quietly, with the shell's code for it (128 + SIGPIPE).
fn quiet_on_closed_pipe() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if info.payload_as_str().is_some_and(closed_output) {
            std::process::exit(if cfg!(unix) { 141 } else { 1 });
        }
        default(info);
    }));
}

/// The message `print!`/`eprint!` panic with when their stream is gone.
fn closed_output(message: &str) -> bool {
    (message.starts_with("failed printing to stdout")
        || message.starts_with("failed printing to stderr"))
        && (message.contains("Broken pipe")
            || message.contains("os error 32")
            // Windows: ERROR_NO_DATA / ERROR_BROKEN_PIPE.
            || message.contains("os error 232")
            || message.contains("os error 109"))
}

pub(crate) async fn call(paths: &Paths, request: Request) -> Result<Response, IpcError> {
    locdev_ipc::request(paths, request).await
}

/// Like [`call`], but explains how to recover when the daemon is down.
pub(crate) async fn daemon_call(paths: &Paths, request: Request) -> anyhow::Result<Response> {
    match call(paths, request).await {
        Ok(response) => Ok(response),
        Err(IpcError::Io(_)) => bail!("the daemon is not running; start it with `locdev start`"),
        Err(IpcError::IncompatibleProtocol { daemon, .. }) => bail!(
            "the running daemon speaks protocol {daemon}, this CLI speaks {PROTOCOL_VERSION}; run `locdev restart`"
        ),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn expect_project(response: Response) -> anyhow::Result<ProjectView> {
    match response {
        Response::Project { project } => Ok(project),
        other => unexpected(Ok(other)),
    }
}

fn unexpected<T>(response: Result<Response, IpcError>) -> anyhow::Result<T> {
    match response {
        Err(error) => Err(error.into()),
        Ok(other) => bail!("unexpected response from daemon: {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::closed_output;

    #[test]
    fn only_a_closed_output_is_quiet() {
        assert!(closed_output(
            "failed printing to stdout: Broken pipe (os error 32)"
        ));
        assert!(closed_output(
            "failed printing to stdout: The pipe is being closed. (os error 232)"
        ));
        assert!(!closed_output(
            "failed printing to stdout: No space left on device (os error 28)"
        ));
        assert!(!closed_output("index out of bounds"));
    }
}
