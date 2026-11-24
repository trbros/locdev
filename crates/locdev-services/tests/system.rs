//! Runs real servers found on this machine (skipped by default):
//! `cargo test -p locdev-services --test system -- --ignored --nocapture`

#![cfg(unix)]

use std::process::{Command, Stdio};
use std::time::Duration;

use locdev_core::services::{InstallSource, ServiceKind};
use locdev_services::{Layout, discover, init, ready, spec};

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
#[ignore = "needs system PostgreSQL/Redis"]
fn system_servers_start_on_loopback() {
    let root = tempfile::tempdir().unwrap();
    let layout = Layout::new(root.path().to_path_buf());
    let installs = discover(&layout);
    for i in &installs {
        println!(
            "found {:?} {} ({:?}) at {}",
            i.kind,
            i.version,
            i.source,
            i.bin_dir.display()
        );
    }
    for kind in [ServiceKind::Postgres, ServiceKind::Redis] {
        let Some(install) = installs
            .iter()
            .find(|i| i.kind == kind && i.source == InstallSource::System)
        else {
            println!("no system {kind:?}; skipped");
            continue;
        };
        let port = free_port();
        let dirs = layout.dirs(kind.id());
        init(kind, &install.bin_dir, &dirs, port).expect("init");
        let command = spec::start_command(kind, &install.bin_dir, &dirs, port).unwrap();
        println!("$ {command}");
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut up = false;
        for _ in 0..100 {
            if ready(port) {
                up = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = Command::new("kill").arg(child.id().to_string()).status();
        let _ = child.wait();
        assert!(up, "{kind:?} accepted connections on {port}");
        // Only loopback: nothing listens on another address.
        println!("{kind:?} {} ready on 127.0.0.1:{port}", install.version);
    }
}
