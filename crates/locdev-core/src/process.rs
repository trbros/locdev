//! Helper programs without a console window on Windows.
//!
//! The daemon and the app are GUI-subsystem programs there, so every console
//! program they start (`git`, `gh`, `node -v`, `netstat`…) would get a
//! console window of its own and flash on screen. Everything we run is
//! headless (output is captured or logged), so ask for no window.

/// `CREATE_NO_WINDOW`: a console with no window; children inherit it.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub trait NoWindow {
    /// No console window on Windows; nothing elsewhere.
    fn no_window(&mut self) -> &mut Self;
}

impl NoWindow for std::process::Command {
    fn no_window(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(CREATE_NO_WINDOW);
        }
        self
    }
}

/// A command to start without a console window (see [`NoWindow`]).
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    command.no_window();
    command
}
