//! Locating, starting and stopping the daemon process. Shared by every client
//! so the CLI and the desktop app behave identically.

use std::fs::OpenOptions;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use locdev_core::paths::Paths;
use locdev_core::protocol::Request;

use crate::{IpcError, connect, request};

/// Overrides the daemon executable used by clients.
pub const DAEMON_ENV: &str = "LOCDEV_DAEMON";

pub const DAEMON_BIN: &str = if cfg!(windows) {
    "locdevd.exe"
} else {
    "locdevd"
};

const READY_TIMEOUT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(50);

/// Looks for the daemon in `$LOCDEV_DAEMON`, next to the running executable
/// (installed bundles and `target/` builds) and finally on `PATH`.
pub fn daemon_executable() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(DAEMON_ENV).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(path));
    }

    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(DAEMON_BIN)))
        .filter(|path| path.is_file());

    beside_exe.or_else(|| {
        std::env::var_os("PATH").and_then(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join(DAEMON_BIN))
                .find(|candidate| candidate.is_file())
        })
    })
}

pub async fn is_running(paths: &Paths) -> bool {
    connect(paths).await.is_ok()
}

/// The daemon binds its IPC endpoint first (to claim it) and only answers
/// requests once the proxy and DNS server are up, so an answered `Status`
/// means it is fully started.
async fn is_ready(paths: &Paths) -> bool {
    let probe = async { connect(paths).await?.call(Request::Status).await };
    matches!(tokio::time::timeout(POLL * 4, probe).await, Ok(Ok(_)))
}

/// Starts the daemon unless it is already running. Returns `true` when this
/// call started it.
pub async fn ensure_running(paths: &Paths) -> Result<bool, IpcError> {
    if is_running(paths).await {
        return Ok(false);
    }

    spawn_detached(paths)?;

    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    while tokio::time::Instant::now() < deadline {
        if is_ready(paths).await {
            return Ok(true);
        }
        tokio::time::sleep(POLL).await;
    }

    Err(IpcError::Timeout)
}

/// Asks the daemon to shut down and waits until its endpoint goes away.
/// Returns `false` if it was not running.
pub async fn stop(paths: &Paths) -> Result<bool, IpcError> {
    if !is_running(paths).await {
        return Ok(false);
    }

    request(paths, Request::Shutdown).await?;

    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    while tokio::time::Instant::now() < deadline {
        if !is_running(paths).await {
            return Ok(true);
        }
        tokio::time::sleep(POLL).await;
    }

    Err(IpcError::Timeout)
}

fn spawn_detached(paths: &Paths) -> Result<(), IpcError> {
    let exe = daemon_executable().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("could not find {DAEMON_BIN}; install it next to this program, on PATH, or set {DAEMON_ENV}"),
        )
    })?;

    paths.ensure()?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.daemon_log_file())?;

    let mut command = Command::new(exe);
    command
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);

    // Detach from the client's terminal/console so closing it (or Ctrl+C)
    // does not take the daemon down.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    // On Windows a console of its own with no window (not DETACHED_PROCESS):
    // helpers the daemon starts inherit it instead of each opening a console
    // window that flashes on screen.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(locdev_core::process::CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }

    let mut child = command.spawn()?;

    // Reap the process when it exits so long-lived clients (the desktop app)
    // don't accumulate zombies.
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}
