// Released builds run detached (service, login item): no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod daemon;
mod dns;
mod doctor;
mod events;
mod fsutil;
mod jobs;
mod notify;
mod proxy;
mod registry;
mod runtimes;
mod security;
mod settings;
mod supervisor;

use std::io::{ErrorKind, IsTerminal};
use std::process::ExitCode;
use std::sync::Arc;

use locdev_core::paths::Paths;
use locdev_ipc::{Endpoint, Listener, serve};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::daemon::Daemon;

/// Rotate the log file once it grows past this.
const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;

#[tokio::main]
async fn main() -> ExitCode {
    // `locdevd sendmail --site <name> --port <n>`: PHP's mail() pipes the
    // message here (sendmail_path), and it goes to LocDev's Mailbox only.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("sendmail") {
        return sendmail(&args[2..]).await;
    }
    // `locdevd static --port <n> [--root <dir>]`: the default server of a
    // static site (index.html), so it needs neither Python nor Node.
    if args.get(1).map(String::as_str) == Some("static") {
        return serve_static(&args[2..]).await;
    }
    init_logging();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn sendmail(args: &[String]) -> ExitCode {
    use std::io::Read;
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let site = value("--site");
    let Some(port) = value("--port").and_then(|p| p.parse::<u16>().ok()) else {
        eprintln!("locdevd sendmail: --port is required");
        return ExitCode::from(64);
    };
    let mut raw = Vec::new();
    if let Err(error) = std::io::stdin().read_to_end(&mut raw) {
        eprintln!("locdevd sendmail: {error}");
        return ExitCode::from(74);
    }
    // Recipients from the headers (`-t`), plus any given as arguments.
    let (from, mut to) = locdev_mail::client::envelope_of(&raw);
    to.extend(
        args.iter()
            .filter(|a| a.contains('@') && !a.starts_with('-'))
            .cloned(),
    );
    let from = value("-f").unwrap_or(if from.is_empty() {
        "php@locdev.test".into()
    } else {
        from
    });
    let addr = std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port));
    match locdev_mail::client::send(addr, site.as_deref(), &from, &to, &raw).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("locdevd sendmail: {error}");
            ExitCode::from(75)
        }
    }
}

async fn serve_static(args: &[String]) -> ExitCode {
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let Some(port) = value("--port").and_then(|p| p.parse::<u16>().ok()) else {
        eprintln!("locdevd static: --port is required");
        return ExitCode::from(64);
    };
    let root = std::path::PathBuf::from(value("--root").unwrap_or_else(|| ".".into()));
    let root = match std::fs::canonicalize(&root) {
        Ok(root) if root.is_dir() => root,
        _ => {
            eprintln!("locdevd static: {} is not a folder", root.display());
            return ExitCode::from(66);
        }
    };
    // Loopback only, like every server LocDev runs.
    let listener = match tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await
    {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("locdevd static: port {port}: {error}");
            return ExitCode::from(69);
        }
    };
    println!("Serving {} on http://127.0.0.1:{port}", root.display());
    locdev_proxy::files::serve_dir(root, listener).await;
    ExitCode::SUCCESS
}

/// Logs to the terminal when there is one; otherwise (started by the
/// launcher, a login item or a service manager) to the daemon log file.
fn init_logging() {
    let filter =
        || EnvFilter::try_from_env("LOCDEV_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    if std::io::stdout().is_terminal() {
        tracing_subscriber::fmt().with_env_filter(filter()).init();
        return;
    }

    let file = Paths::resolve().ok().and_then(|paths| {
        let path = paths.daemon_log_file();
        std::fs::create_dir_all(path.parent()?).ok()?;
        if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
            let _ = std::fs::rename(&path, path.with_extension("log.1"));
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
    });

    match file {
        Some(file) => tracing_subscriber::fmt()
            .with_env_filter(filter())
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(file))
            .init(),
        None => tracing_subscriber::fmt().with_env_filter(filter()).init(),
    }
}

async fn run() -> anyhow::Result<()> {
    let paths = Paths::resolve()?;
    paths.ensure()?;

    let endpoint = Endpoint::for_paths(&paths);
    let mut listener = match Listener::bind(&endpoint).await {
        Ok(listener) => listener,
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            info!(%endpoint, "daemon already running");
            return Ok(());
        }
        Err(e) => return Err(anyhow::anyhow!("cannot listen on {endpoint}: {e}")),
    };

    let daemon = Arc::new(Daemon::load(paths.clone())?);
    daemon.keep_self();
    std::fs::write(paths.pid_file(), std::process::id().to_string())?;

    info!(version = locdev_core::VERSION, %endpoint, home = %paths.home.display(), "locdevd started");

    // The Debug Center listens before any site starts (it taps their output).
    let failures = daemon.start_debug();
    let (proxy_status, https, proxy_tasks) = proxy::start(
        daemon.proxy_config(),
        daemon.tls_config(),
        daemon.routes(),
        daemon.tls_acceptor(),
        Some(failures),
    )
    .await;
    daemon.set_proxy_status(proxy_status);
    daemon.set_https(https.port, https.error);

    let (dns_status, dns_server, dns_task) = dns::start(daemon.dns_config()).await;
    daemon.set_dns(dns_status, dns_server);
    let captured = daemon.start_mail().await;
    tokio::spawn(daemon.clone().mail_notifications(captured));

    // Probing goes through the OS resolver and can take a moment; keep it off
    // the startup path, then repeat now and then so the UI notices changes
    // made outside LocDev.
    let refresher = tokio::spawn({
        let daemon = daemon.clone();
        async move {
            let mut every = tokio::time::interval(std::time::Duration::from_secs(120));
            loop {
                every.tick().await;
                daemon.refresh_dns().await;
                daemon.refresh_tls().await;
            }
        }
    });

    // Runtime detection runs each candidate binary, so it (and the auto-start
    // that needs it to resolve pins) happens in the background instead of
    // delaying the IPC readiness clients wait for.
    tokio::spawn({
        let daemon = daemon.clone();
        async move {
            // A previous daemon that crashed may have left dev servers running;
            // stop them so auto-start brings them back under supervision.
            let reaped = daemon.supervisor.reap_orphans().await;
            if !reaped.is_empty() {
                warn!(?reaped, "stopped processes left by a previous daemon");
            }
            daemon.runtimes.refresh_system().await;
            daemon.autostart().await;
        }
    });

    // Support schedules and release catalogs (for "update available") are
    // refreshed in the background, never in offline mode.
    let updater = tokio::spawn({
        let daemon = daemon.clone();
        async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let mut every = tokio::time::interval(std::time::Duration::from_secs(6 * 3600));
            loop {
                every.tick().await;
                if daemon.settings.get().network.offline {
                    continue;
                }
                daemon.runtimes.refresh_lifecycle(7).await;
                daemon.runtimes.refresh_catalogs().await;
            }
        }
    });

    // Background work: the job queue (checks, updates, installs) and
    // failure notifications. Notifications that arrived while nothing could
    // show them are delivered now.
    let workers = [
        tokio::spawn(daemon.clone().run_jobs()),
        tokio::spawn(daemon.clone().watch_processes()),
        tokio::spawn(daemon.clone().watch_parked()),
    ];
    tokio::spawn({
        let daemon = daemon.clone();
        async move {
            let prefs = daemon.settings.get().notifications;
            daemon
                .notifications
                .deliver(&prefs, daemon.app_in_front())
                .await;
        }
    });

    let signal = shutdown_signal();
    tokio::pin!(signal);

    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok(stream) => {
                    let daemon = daemon.clone();
                    tokio::spawn(async move {
                        let result = serve(stream, |request| {
                            let daemon = daemon.clone();
                            async move { daemon.handle(request).await }
                        })
                        .await;
                        if let Err(error) = result {
                            debug!(%error, "connection closed with error");
                        }
                    });
                }
                Err(error) => warn!(%error, "failed to accept connection"),
            },
            () = daemon.shutdown_requested() => break,
            () = &mut signal => break,
        }
    }

    info!("shutting down");
    // Stop accepting first so clients see the daemon as gone while children stop.
    drop(listener);
    for task in proxy_tasks
        .into_iter()
        .chain(dns_task)
        .chain([refresher, updater])
        .chain(workers)
    {
        task.abort();
    }
    daemon.supervisor.stop_all().await;
    let _ = std::fs::remove_file(paths.pid_file());
    info!("bye");

    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
    let mut int = signal(SignalKind::interrupt()).expect("SIGINT handler");
    tokio::select! {
        _ = term.recv() => {}
        _ = int.recv() => {}
    }
}

#[cfg(windows)]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
