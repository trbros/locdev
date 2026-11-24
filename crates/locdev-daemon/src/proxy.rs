//! Brings the HTTP and HTTPS proxy listeners up on the configured ports,
//! falling back when a port is taken or needs privileges the daemon does not
//! have.

use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use locdev_core::model::ProxyStatus;
use locdev_proxy::{Proxy, Routes};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};

use crate::config::{ProxyConfig, TlsConfig};

/// Outcome of binding one listener.
#[derive(Debug, Default)]
pub struct Bound {
    pub port: Option<u16>,
    /// Why the preferred port could not be used (fallback or failure).
    pub error: Option<String>,
}

/// Starts the proxy. Only loopback addresses are bound, so sites never leak
/// onto the network. HTTPS is served only when `tls` is given.
pub async fn start(
    config: &ProxyConfig,
    tls_config: &TlsConfig,
    routes: Arc<Routes>,
    tls: Option<TlsAcceptor>,
    failures: Option<locdev_proxy::Failures>,
) -> (ProxyStatus, Bound, Vec<JoinHandle<()>>) {
    let mut status = ProxyStatus {
        enabled: config.enabled,
        requested_port: config.http_port,
        ..Default::default()
    };
    let mut tasks = Vec::new();
    if !config.enabled {
        return (status, Bound::default(), tasks);
    }

    let proxy = Proxy::with_failures(routes, failures);

    let (http, bound) = bind_with_fallback("http", config.http_port, config.fallback_port).await;
    status.port = bound.port;
    status.error = bound.error;
    for listener in http {
        tasks.push(tokio::spawn(proxy.clone().serve(listener)));
    }

    let mut https_bound = Bound::default();
    if let Some(tls) = tls {
        let (https, bound) =
            bind_with_fallback("https", tls_config.https_port, tls_config.fallback_port).await;
        for listener in https {
            tasks.push(tokio::spawn(proxy.clone().serve_tls(listener, tls.clone())));
        }
        https_bound = bound;
    }

    (status, https_bound, tasks)
}

/// Binds 127.0.0.1 on `port`, else on `fallback`, plus `[::1]` on the same
/// port when available (browsers may try it first for `*.localhost`).
async fn bind_with_fallback(label: &str, port: u16, fallback: u16) -> (Vec<TcpListener>, Bound) {
    let (listener, error) = match bind(port).await {
        Ok(listener) => (listener, None),
        Err(error) => {
            let reason = describe(port, &error);
            match bind(fallback).await {
                Ok(listener) => {
                    warn!(%reason, fallback, "{label} proxy using fallback port");
                    (listener, Some(reason))
                }
                Err(fallback_error) => {
                    let fallback = describe(fallback, &fallback_error);
                    warn!(%reason, %fallback, "{label} proxy disabled: no port available");
                    return (
                        Vec::new(),
                        Bound {
                            port: None,
                            error: Some(format!("{reason}; {fallback}")),
                        },
                    );
                }
            }
        }
    };

    // Port 0 in the config means "any free port" (used by tests).
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    info!(port, "{label} proxy listening");

    let mut listeners = vec![listener];
    if let Ok(v6) = TcpListener::bind(SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port)).await
    {
        listeners.push(v6);
    }

    (
        listeners,
        Bound {
            port: Some(port),
            error,
        },
    )
}

async fn bind(port: u16) -> io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)).await
}

fn describe(port: u16, error: &io::Error) -> String {
    match error.kind() {
        io::ErrorKind::PermissionDenied => format!("port {port} needs elevated privileges"),
        io::ErrorKind::AddrInUse => format!("port {port} is in use by another program"),
        _ => format!("port {port}: {error}"),
    }
}
