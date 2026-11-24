//! Terminals the daemon runs for the user: creating a project, installing a
//! tool. Each is a pseudo-terminal (ConPTY on Windows), so installers that
//! ask questions can be answered from the app, and the steps of a plan run
//! one after another, all writing to one output buffer. Programs run without
//! a shell (`Step` is a program and its arguments). The app polls
//! `TerminalRead` from an offset.
//!
//! Each step gets a fresh pseudo-terminal: on macOS (and the BSDs) the kernel
//! revokes a controlling terminal when its session leader exits, so a slave
//! kept open for the next step only answers `EBADF`.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use locdev_core::scaffold::{Step, TerminalInfo, TerminalOutput};
use locdev_core::{AppError, ErrorCode};
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize};
use tracing::{info, warn};

/// Output kept per terminal; older output is dropped (and the client told).
const KEEP: usize = 2 * 1024 * 1024;
/// Finished terminals kept for the app to read.
const FINISHED: usize = 20;
/// How long a finished step's output may keep draining before the next step
/// starts (a background process it left behind can hold the terminal open).
const DRAIN: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct Terminals {
    next: AtomicU64,
    sessions: Mutex<HashMap<u64, Arc<Session>>>,
}

pub struct Session {
    info: Mutex<TerminalInfo>,
    /// Bytes from `start` on.
    buffer: Mutex<(u64, Vec<u8>)>,
    /// The running step's terminal; none between steps.
    pty: Mutex<Option<Pty>>,
    size: Mutex<PtySize>,
    killer: Mutex<Option<Box<dyn ChildKiller + Send + Sync>>>,
    killed: AtomicBool,
}

struct Pty {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
}

impl Terminals {
    /// Creates a terminal; each step opens its own pseudo-terminal.
    pub fn open(&self, title: String, steps: usize) -> Result<Arc<Session>, AppError> {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        let session = Arc::new(Session {
            info: Mutex::new(TerminalInfo {
                id,
                title,
                running: true,
                code: None,
                step: 0,
                steps,
                site: None,
            }),
            buffer: Mutex::new((0, Vec::new())),
            pty: Mutex::new(None),
            size: Mutex::new(PtySize {
                rows: 30,
                cols: 110,
                pixel_width: 0,
                pixel_height: 0,
            }),
            killer: Mutex::new(None),
            killed: AtomicBool::new(false),
        });
        let mut sessions = self.sessions.lock().unwrap();
        // Forget the oldest finished ones.
        let mut finished: Vec<u64> = sessions
            .iter()
            .filter(|(_, s)| !s.info.lock().unwrap().running)
            .map(|(id, _)| *id)
            .collect();
        finished.sort_unstable();
        while finished.len() >= FINISHED {
            sessions.remove(&finished.remove(0));
        }
        sessions.insert(id, session.clone());
        Ok(session)
    }

    fn get(&self, id: u64) -> Result<Arc<Session>, AppError> {
        self.sessions
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("no terminal {id}")))
    }

    pub fn list(&self) -> Vec<TerminalInfo> {
        let mut out: Vec<TerminalInfo> = self
            .sessions
            .lock()
            .unwrap()
            .values()
            .map(|s| s.info())
            .collect();
        out.sort_by_key(|t| std::cmp::Reverse(t.id));
        out
    }

    pub fn read(&self, id: u64, from: u64) -> Result<TerminalOutput, AppError> {
        let session = self.get(id)?;
        let (start, bytes) = &*session.buffer.lock().unwrap();
        let end = start + bytes.len() as u64;
        let from = from.min(end);
        let truncated = from < *start;
        let skip = from.saturating_sub(*start) as usize;
        Ok(TerminalOutput {
            info: session.info(),
            data: String::from_utf8_lossy(&bytes[skip..]).into_owned(),
            next: end,
            truncated,
        })
    }

    pub fn write(&self, id: u64, data: &str) -> Result<(), AppError> {
        let session = self.get(id)?;
        if data.len() > 64 * 1024 {
            return Err(AppError::invalid("too much input at once"));
        }
        if !session.info().running {
            return Err(AppError::invalid("the terminal is closed"));
        }
        // Between steps nobody reads; the input is dropped.
        if let Some(pty) = session.pty.lock().unwrap().as_mut() {
            pty.writer.write_all(data.as_bytes())?;
            pty.writer.flush()?;
        }
        Ok(())
    }

    pub fn resize(&self, id: u64, cols: u16, rows: u16) -> Result<(), AppError> {
        let session = self.get(id)?;
        let size = PtySize {
            rows: rows.clamp(5, 500),
            cols: cols.clamp(20, 1000),
            pixel_width: 0,
            pixel_height: 0,
        };
        *session.size.lock().unwrap() = size;
        if let Some(pty) = session.pty.lock().unwrap().as_ref() {
            pty.master
                .resize(size)
                .map_err(|e| AppError::internal(e.to_string()))?;
        }
        Ok(())
    }

    pub fn kill(&self, id: u64) -> Result<(), AppError> {
        self.get(id)?.kill();
        Ok(())
    }

    pub fn close(&self, id: u64) -> Result<(), AppError> {
        let session = self.get(id)?;
        if session.info().running {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "the terminal is still running; stop it first",
            ));
        }
        self.sessions.lock().unwrap().remove(&id);
        Ok(())
    }
}

impl Session {
    pub fn info(&self) -> TerminalInfo {
        self.info.lock().unwrap().clone()
    }

    pub fn id(&self) -> u64 {
        self.info.lock().unwrap().id
    }

    fn push(&self, bytes: &[u8]) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.1.extend_from_slice(bytes);
        if buffer.1.len() > KEEP {
            let drop = buffer.1.len() - KEEP;
            buffer.1.drain(..drop);
            buffer.0 += drop as u64;
        }
    }

    /// A line from LocDev itself (dimmed), between the programs' output.
    pub fn note(&self, text: &str) {
        self.push(format!("\r\n\x1b[2m› {text}\x1b[0m\r\n").as_bytes());
    }

    pub fn killed(&self) -> bool {
        self.killed.load(Ordering::Relaxed)
    }

    pub fn kill(&self) {
        self.killed.store(true, Ordering::Relaxed);
        if let Some(killer) = self.killer.lock().unwrap().as_mut() {
            let _ = killer.kill();
        }
    }

    pub fn set_site(&self, site: String) {
        self.info.lock().unwrap().site = Some(site);
    }

    /// Runs one step on the terminal and waits for it; `n` is 1-based.
    pub async fn run(
        self: &Arc<Self>,
        n: usize,
        step: &Step,
        env: &[(String, String)],
    ) -> Result<u32, AppError> {
        if self.killed() {
            return Err(AppError::invalid("stopped"));
        }
        self.info.lock().unwrap().step = n;
        self.push(
            format!(
                "\r\n\x1b[1;38;5;208m▸ {}\x1b[0m\r\n\x1b[2m$ {}\x1b[0m\r\n",
                step.label,
                step.display()
            )
            .as_bytes(),
        );
        let mut cmd = CommandBuilder::new(program_for(&step.program, env, &step.env));
        cmd.args(&step.args);
        cmd.cwd(&step.cwd);
        for (key, value) in env.iter().chain(&step.env) {
            cmd.env(key, value);
        }
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        let size = *self.size.lock().unwrap();
        let pair = portable_pty::native_pty_system()
            .openpty(size)
            .map_err(|e| AppError::internal(format!("could not open a terminal: {e}")))?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| AppError::internal(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| AppError::internal(e.to_string()))?;
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AppError::new(ErrorCode::NotRunnable, format!("{}: {e}", step.program)))?;
        // Only the child holds the slave now, so the reader sees the end
        // once it (and whatever it left running) is gone.
        drop(pair.slave);
        *self.killer.lock().unwrap() = Some(child.clone_killer());
        *self.pty.lock().unwrap() = Some(Pty {
            master: pair.master,
            writer,
        });
        let (drained, wait_drained) = mpsc::channel::<()>();
        let copy = self.clone();
        std::thread::spawn(move || {
            let mut chunk = [0u8; 8192];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => copy.push(&chunk[..n]),
                }
            }
            let _ = drained.send(());
        });
        info!(terminal = self.id(), program = %step.program, "terminal step started");
        let status = tokio::task::spawn_blocking(move || child.wait())
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        *self.killer.lock().unwrap() = None;
        // Closing the master ends a ConPTY's output; on Unix the reader's own
        // descriptor keeps what is still buffered readable.
        *self.pty.lock().unwrap() = None;
        let _ = tokio::task::spawn_blocking(move || wait_drained.recv_timeout(DRAIN)).await;
        let status = status.map_err(|e| AppError::internal(e.to_string()))?;
        Ok(status.exit_code())
    }

    /// Ends the run: closes the terminal (so the reader stops) and records
    /// the result.
    pub fn finish(&self, code: Option<i32>) {
        {
            let mut info = self.info.lock().unwrap();
            info.running = false;
            info.code = code;
        }
        match code {
            Some(0) => self.note("Done."),
            Some(code) => self.note(&format!("Stopped: exit code {code}.")),
            None => self.note("Stopped."),
        }
        if self.pty.lock().unwrap().take().is_some() {
            warn!(terminal = self.id(), "terminal finished while a step ran");
        }
    }
}

/// The program to start. On Windows a bare name is looked up on the step's
/// PATH with `.exe`/`.cmd`/`.bat` first: Composer's and npm's bin folders hold
/// an extensionless Unix script next to the `.bat`, and a pseudo-terminal
/// would pick that one ("not a valid Win32 application").
fn program_for(program: &str, env: &[(String, String)], step: &[(String, String)]) -> String {
    if !cfg!(windows) || program.contains(['/', '\\']) || program.contains('.') {
        return program.to_string();
    }
    let path = step
        .iter()
        .chain(env)
        .rev()
        .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
        .map(|(_, v)| v.clone())
        .or_else(|| std::env::var("PATH").ok())
        .unwrap_or_default();
    locdev_scaffold::tools::find(program, &path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| program.to_string())
}
