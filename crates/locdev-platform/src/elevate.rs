//! Runs a [`Script`] with administrator rights through the platform's own
//! authorization prompt. Callers must have shown the script to the user and
//! obtained their consent first.

use std::io;
use std::process::{Command, Stdio};

use crate::Script;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// From a terminal: `sudo` asks for the password inline.
    Terminal,
    /// From a GUI app: a graphical prompt (polkit, macOS auth dialog, UAC).
    Gui,
}

/// Blocks until the script finishes. Errors when the user declines, the
/// prompt is unavailable or the script fails.
pub fn run(script: &Script, mode: Mode) -> io::Result<()> {
    let mut command = command(script, mode)?;
    let status = match mode {
        Mode::Terminal => command.status()?,
        Mode::Gui => command.stdin(Stdio::null()).status()?,
    };

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(match status.code() {
            // pkexec: 126 = dismissed, 127 = not authorized.
            Some(126 | 127) if cfg!(target_os = "linux") && mode == Mode::Gui => {
                "authorization was cancelled".to_string()
            }
            // ERROR_CANCELLED, from the launcher when UAC is declined.
            Some(1223) if cfg!(windows) => "authorization was cancelled".to_string(),
            Some(code) => format!("the privileged step failed (exit code {code})"),
            None => "the privileged step was interrupted".to_string(),
        }))
    }
}

/// Runs a script without elevation, as the current user.
pub fn run_as_user(script: &Script) -> io::Result<()> {
    let status = match script {
        Script::Sh(source) => Command::new("/bin/sh").args(["-c", source]).status()?,
        Script::PowerShell(source) => locdev_core::process::command("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", source])
            .status()?,
    };
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "the user-level step failed ({status})"
        )))
    }
}

fn command(script: &Script, mode: Mode) -> io::Result<Command> {
    match script {
        Script::Sh(source) => sh_command(source, mode),
        Script::PowerShell(source) => Ok(powershell_command(source)),
    }
}

fn sh_command(source: &str, mode: Mode) -> io::Result<Command> {
    if cfg!(windows) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "sh scripts do not run on Windows",
        ));
    }

    let mut command = match mode {
        Mode::Terminal => Command::new("sudo"),
        Mode::Gui if cfg!(target_os = "macos") => {
            // AppleScript strings cannot hold the script verbatim; pass it
            // base64-encoded and decode it inside the privileged shell.
            let mut command = Command::new("osascript");
            let inner = format!(
                "echo {} | /usr/bin/base64 -D | /bin/sh",
                base64(source.as_bytes())
            );
            command.arg("-e").arg(format!(
                "do shell script \"{inner}\" with administrator privileges with prompt \"LocDev needs to change your DNS settings.\""
            ));
            return Ok(command);
        }
        Mode::Gui => {
            if !on_path("pkexec") {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "pkexec is not installed; run `locdev dns install` from a terminal instead",
                ));
            }
            Command::new("pkexec")
        }
    };
    command.args(["/bin/sh", "-c", source]);
    Ok(command)
}

/// Starts an elevated PowerShell (UAC prompt) and propagates its exit code.
/// A declined prompt makes Start-Process fail without stopping the script,
/// which would otherwise `exit $null` = 0, a false success; it exits 1223
/// (ERROR_CANCELLED) instead.
fn powershell_command(source: &str) -> Command {
    let utf16: Vec<u8> = source.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = base64(&utf16);
    let launcher = format!(
        "try {{ $p = Start-Process powershell -Verb RunAs -Wait -PassThru -WindowStyle Hidden -ErrorAction Stop \
         -ArgumentList '-NoProfile','-NonInteractive','-EncodedCommand','{encoded}' }} catch {{ exit 1223 }}; \
         if ($null -eq $p.ExitCode) {{ exit 1 }}; exit $p.ExitCode"
    );

    let mut command = locdev_core::process::command("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &launcher]);
    command
}

fn on_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(bin).is_file()))
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[(n >> (18 - 6 * i) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), expected);
        }
    }

    #[test]
    fn powershell_is_utf16_encoded() {
        let command = powershell_command("Clear-DnsClientCache");
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        // "C\0l\0..." in base64 starts with "QwBsA".
        assert!(args.last().unwrap().contains("'QwBsA"), "{args:?}");
    }
}
