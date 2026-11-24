//! Runs the `*.test` DNS server and checks how the OS routes the TLD.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use locdev_core::TLD;
use locdev_core::model::{DnsStatus, Routing, now_ms};
use locdev_dns::DnsServer;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::config::DnsConfig;

pub async fn start(
    config: &DnsConfig,
) -> (DnsStatus, Option<Arc<DnsServer>>, Option<JoinHandle<()>>) {
    let mut status = DnsStatus {
        enabled: config.enabled,
        ..Default::default()
    };
    if !config.enabled {
        return (status, None, None);
    }

    // Loopback only: the server must never answer the network.
    let mut errors = Vec::new();
    for ip in config.addresses.iter().filter(|ip| ip.is_loopback()) {
        let wanted = SocketAddr::from((*ip, config.port));
        match UdpSocket::bind(wanted).await {
            Ok(socket) => {
                let addr = socket.local_addr().unwrap_or(wanted);
                info!(%addr, "dns server listening");
                status.port = Some(addr.port());
                status.address = Some(addr.ip().to_string());
                let server = DnsServer::new(TLD);
                let task = tokio::spawn(server.clone().serve(socket));
                return (status, Some(server), Some(task));
            }
            Err(error) => {
                let owner = locdev_platform::ports::owner(config.port)
                    .map(|o| format!(" ({}, pid {})", o.name, o.pid))
                    .unwrap_or_default();
                use std::io::ErrorKind::{AddrInUse, PermissionDenied};
                let kind = error.kind();
                // Windows needs no privileges for port 53; WSAEACCES there
                // means another program (another account's, or one with
                // SO_EXCLUSIVEADDRUSE) already holds the port, and elevating
                // LocDev would not help.
                errors.push(
                    if kind == AddrInUse || (cfg!(windows) && kind == PermissionDenied) {
                        format!("{wanted} is in use by another program{owner}")
                    } else if kind == PermissionDenied {
                        format!("{wanted} needs elevated privileges")
                    } else {
                        format!("{wanted}: {error}")
                    },
                );
            }
        }
    }
    let message = if errors.is_empty() {
        "no loopback address configured in [dns] addresses".to_string()
    } else {
        errors.join("; ")
    };
    warn!(%message, "dns server disabled");
    status.error = Some(message);
    (status, None, None)
}

/// Resolves a never-before-used name through the OS and checks whether our
/// server was the one asked. Distinguishes "works through LocDev" from
/// "works through some other tool" (another `.test` resolver on the machine).
pub async fn probe(server: Option<&DnsServer>) -> Routing {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let name = format!(
        "locdev-probe-{}-{}.{TLD}",
        now_ms(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );

    let lookup = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::lookup_host((name.as_str(), 80)),
    )
    .await;
    let loopback = match lookup {
        Ok(Ok(mut addrs)) => addrs.any(|a| a.ip().is_loopback()),
        _ => false,
    };

    match (loopback, server.is_some_and(|s| s.saw(&name))) {
        (true, true) => Routing::Locdev,
        (true, false) => Routing::Other,
        _ => Routing::None,
    }
}
