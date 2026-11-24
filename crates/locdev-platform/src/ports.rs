//! Who is on a port, and what a process is: used to explain a "port in use"
//! failure and to recognise processes a previous daemon left behind. Read
//! only; nothing here signals or changes a process.

use std::collections::HashSet;
use std::process::Stdio;

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortOwner {
    pub pid: u32,
    pub name: String,
}

/// The process listening on `127.0.0.1:port` (or any address), if it can be
/// found. Other users' processes may be invisible without privileges.
pub fn owner(port: u16) -> Option<PortOwner> {
    let pid = listening_pid(port)?;
    let Ok(mut system) = System::new() else {
        return Some(PortOwner {
            pid,
            name: "an unknown process".into(),
        });
    };
    let pid_ = Pid::from_u32(pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid_]),
        true,
        ProcessRefreshKind::nothing(),
    );
    let name = system
        .process(pid_)
        .map(|p| p.name().to_string_lossy().into_owned())
        .unwrap_or_else(|| "an unknown process".into());
    Some(PortOwner { pid, name })
}

/// Whether `pid` is alive and its command line contains `command` (the
/// shell LocDev started it with carries the command as an argument).
pub fn process_matches(pid: u32, command: &str) -> bool {
    let Ok(mut system) = System::new() else {
        return false;
    };
    let pid = Pid::from_u32(pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
    );
    system.process(pid).is_some_and(|p| {
        let line: Vec<String> = p
            .cmd()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        line.join(" ").contains(command)
    })
}

/// CPU (percent of one core) and resident memory (bytes) of process trees,
/// summed from each root down. Sampled twice, `interval` apart, since CPU use
/// is a rate.
pub fn tree_usage(roots: &[u32], interval: std::time::Duration) -> Vec<Option<(f32, u64)>> {
    let Ok(mut system) = System::new() else {
        return vec![None; roots.len()];
    };
    let kind = ProcessRefreshKind::nothing().with_cpu().with_memory();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
    std::thread::sleep(interval.max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL));
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);

    roots
        .iter()
        .map(|&root| {
            let root = Pid::from_u32(root);
            system.process(root)?;
            let (mut cpu, mut memory) = (0.0, 0);
            // On Linux every thread is listed too, with its process's memory;
            // count processes only.
            for (pid, process) in system
                .processes()
                .iter()
                .filter(|(_, p)| p.thread_kind().is_none())
            {
                // Walk up from every process: is `root` an ancestor?
                if has_ancestor(*pid, root, |current| {
                    system.process(current).and_then(|p| p.parent())
                }) {
                    cpu += process.cpu_usage();
                    memory += process.memory();
                }
            }
            Some((cpu, memory))
        })
        .collect()
}

fn has_ancestor(pid: Pid, root: Pid, parent: impl Fn(Pid) -> Option<Pid>) -> bool {
    // Windows can reuse a dead parent's PID, making a snapshot look cyclic.
    let mut visited = HashSet::new();
    let mut at = Some(pid);
    while let Some(current) = at {
        if current == root {
            return true;
        }
        if !visited.insert(current) {
            break;
        }
        at = parent(current);
    }
    false
}

#[cfg(target_os = "linux")]
fn listening_pid(port: u16) -> Option<u32> {
    // /proc/net/tcp{,6}: `sl local rem st ... inode`; LISTEN is state 0A.
    let mut inodes = Vec::new();
    for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let Ok(text) = std::fs::read_to_string(table) else {
            continue;
        };
        for line in text.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let (Some(local), Some(state), Some(inode)) =
                (fields.get(1), fields.get(3), fields.get(9))
            else {
                continue;
            };
            let local_port = local
                .rsplit(':')
                .next()
                .and_then(|p| u16::from_str_radix(p, 16).ok());
            if *state == "0A" && local_port == Some(port) {
                inodes.push(format!("socket:[{inode}]"));
            }
        }
    }
    if inodes.is_empty() {
        return None;
    }
    for entry in std::fs::read_dir("/proc").ok()?.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(fds) = std::fs::read_dir(entry.path().join("fd")) else {
            continue;
        };
        for fd in fds.flatten() {
            if let Ok(target) = std::fs::read_link(fd.path())
                && inodes.iter().any(|i| target.as_os_str() == i.as_str())
            {
                return Some(pid);
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn listening_pid(port: u16) -> Option<u32> {
    let output = run(
        "lsof",
        &["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-Fp"],
    )?;
    output
        .lines()
        .find_map(|l| l.strip_prefix('p')?.parse().ok())
}

#[cfg(windows)]
fn listening_pid(port: u16) -> Option<u32> {
    let output = run("netstat", &["-ano", "-p", "TCP"])?;
    output.lines().find_map(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let local_port = fields.get(1)?.rsplit(':').next()?.parse::<u16>().ok()?;
        (fields.get(3) == Some(&"LISTENING") && local_port == port)
            .then(|| fields.get(4)?.parse().ok())
            .flatten()
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn listening_pid(_port: u16) -> Option<u32> {
    None
}

#[allow(dead_code)]
fn run(program: &str, args: &[&str]) -> Option<String> {
    let output = locdev_core::process::command(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_our_own_listener() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        // Linux, macOS and Windows all find this process; elsewhere it's None.
        if let Some(owner) = owner(port) {
            assert_eq!(owner.pid, std::process::id());
        }
        assert!(owner(1).is_none() || cfg!(not(target_os = "linux")));
    }

    #[test]
    fn matches_command_lines() {
        let me = std::process::id();
        let exe = std::env::current_exe().unwrap();
        let name = exe.file_name().unwrap().to_string_lossy().into_owned();
        assert!(process_matches(me, &name));
        assert!(!process_matches(me, "definitely-not-in-the-command-line"));
    }

    #[test]
    fn ancestry_handles_cycles_and_missing_parents() {
        let pid = Pid::from_u32;
        let parent = |current: Pid| match current.as_u32() {
            10 => Some(pid(20)),
            20 => Some(pid(30)),
            30 => Some(pid(20)),
            40 => Some(pid(40)),
            _ => None,
        };
        assert!(has_ancestor(pid(10), pid(10), parent));
        assert!(has_ancestor(pid(10), pid(30), parent));
        assert!(!has_ancestor(pid(10), pid(99), parent));
        assert!(!has_ancestor(pid(40), pid(99), parent));
        assert!(!has_ancestor(pid(50), pid(99), parent));
    }

    #[test]
    fn measures_a_tree() {
        let usage = tree_usage(
            &[std::process::id(), u32::MAX - 7],
            std::time::Duration::from_millis(10),
        );
        let (_, memory) = usage[0].expect("own process");
        assert!(memory > 0);
        assert!(usage[1].is_none());
    }
}
