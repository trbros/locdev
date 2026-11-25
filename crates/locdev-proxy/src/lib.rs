//! HTTP reverse proxy: routes `<name>.test` / `<name>.localhost` to the
//! project's dev server on `127.0.0.1:<port>`, including WebSocket upgrades
//! (needed for Vite/Next.js hot reload).

pub mod files;
pub mod metrics;
mod pages;

pub use locdev_core::host;

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use bytes::Bytes;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::body::Incoming;
use hyper::header::{self, HeaderMap, HeaderName, HeaderValue};
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, Uri, Version};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, warn};

use locdev_core::host::Target;

type Body = BoxBody<Bytes, hyper::Error>;

/// What a connection came in on; used to build links and forwarded headers.
#[derive(Debug, Clone, Copy)]
struct Listen {
    port: u16,
    https: bool,
}

impl Listen {
    /// `scheme://` prefix plus `:port` suffix, omitted for default ports.
    fn origin(self) -> Origin {
        let default = if self.https { 443 } else { 80 };
        Origin {
            scheme: if self.https { "https" } else { "http" },
            port: if self.port == default {
                String::new()
            } else {
                format!(":{}", self.port)
            },
        }
    }
}

pub(crate) struct Origin {
    pub scheme: &'static str,
    pub port: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Route {
    pub name: String,
    pub port: Option<u16>,
    /// HTTPS only: plain HTTP is redirected (when HTTPS is being served).
    pub secure: bool,
    /// Subdomains that go somewhere else than the site's port.
    pub subdomains: Vec<SubRoute>,
    /// Only the site itself and listed subdomains answer (the wildcard is off).
    pub exclusive: bool,
}

/// A subdomain pattern (see `locdev_core::subdomains`) and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubRoute {
    pub pattern: String,
    /// `None`: the target has no port yet (an auxiliary process never started).
    pub port: Option<u16>,
    /// What the page says when it can't be reached (`process reverb`, `port 9000`).
    pub label: String,
}

/// Where a request for `sub` under `route` goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination<'a> {
    Site,
    Sub(&'a SubRoute),
    /// The wildcard is off and no rule matches.
    NotConfigured,
}

impl Route {
    pub fn destination(&self, sub: &str) -> Destination<'_> {
        if sub.is_empty() {
            return Destination::Site;
        }
        let best = self
            .subdomains
            .iter()
            .filter(|r| locdev_core::subdomains::matches(&r.pattern, sub))
            .max_by_key(|r| (!r.pattern.starts_with("*."), r.pattern.split('.').count()));
        match best {
            Some(rule) => Destination::Sub(rule),
            None if self.exclusive => Destination::NotConfigured,
            None => Destination::Site,
        }
    }
}

/// Routing table shared between the daemon (writer) and the proxy (reader).
#[derive(Debug, Default)]
pub struct Routes {
    inner: RwLock<HashMap<String, Route>>,
    /// The HTTPS port, once it's listening; redirects need it.
    https_port: RwLock<Option<u16>>,
    /// What went through each site (shared like the table itself).
    pub metrics: metrics::Metrics,
}

impl Routes {
    pub fn set_https_port(&self, port: Option<u16>) {
        *self.https_port.write().unwrap() = port;
    }

    fn https_port(&self) -> Option<u16> {
        *self.https_port.read().unwrap()
    }

    pub fn replace(&self, routes: impl IntoIterator<Item = Route>) {
        let map = routes.into_iter().map(|r| (r.name.clone(), r)).collect();
        *self.inner.write().unwrap() = map;
    }

    fn get(&self, name: &str) -> Option<Route> {
        self.inner.read().unwrap().get(name).cloned()
    }

    fn all(&self) -> Vec<Route> {
        let mut routes: Vec<_> = self.inner.read().unwrap().values().cloned().collect();
        routes.sort_by(|a, b| a.name.cmp(&b.name));
        routes
    }
}

/// A request a site answered with a server error (or couldn't answer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpFailure {
    pub site: String,
    pub host: String,
    pub method: String,
    /// Path and query.
    pub path: String,
    pub status: u16,
    pub duration_ms: u64,
    /// The site's server didn't answer at all.
    pub upstream_down: bool,
}

pub type Failures = tokio::sync::mpsc::UnboundedSender<HttpFailure>;

pub struct Proxy {
    routes: Arc<Routes>,
    client: Client<HttpConnector, Incoming>,
    failures: Option<Failures>,
}

impl Proxy {
    pub fn new(routes: Arc<Routes>) -> Arc<Self> {
        Self::with_failures(routes, None)
    }

    /// Also reports 5xx answers (and unreachable sites) to `failures`.
    pub fn with_failures(routes: Arc<Routes>, failures: Option<Failures>) -> Arc<Self> {
        let client = Client::builder(TokioExecutor::new()).build_http();
        Arc::new(Self {
            routes,
            client,
            failures,
        })
    }

    fn report(&self, failure: HttpFailure) {
        if let Some(failures) = &self.failures {
            let _ = failures.send(failure);
        }
    }

    /// Serves plain HTTP until the task is dropped/aborted.
    pub async fn serve(self: Arc<Self>, listener: TcpListener) {
        self.accept_loop(listener, None).await;
    }

    /// Serves HTTPS until the task is dropped/aborted. Certificates are
    /// chosen per connection from the SNI by the acceptor's config.
    pub async fn serve_tls(self: Arc<Self>, listener: TcpListener, tls: TlsAcceptor) {
        self.accept_loop(listener, Some(tls)).await;
    }

    async fn accept_loop(self: Arc<Self>, listener: TcpListener, tls: Option<TlsAcceptor>) {
        let https = tls.is_some();
        let listen = Listen {
            port: listener
                .local_addr()
                .map(|a| a.port())
                .unwrap_or(if https { 443 } else { 80 }),
            https,
        };

        loop {
            let (stream, peer) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(error) => {
                    warn!(%error, "proxy accept failed");
                    continue;
                }
            };

            let proxy = self.clone();
            let tls = tls.clone();
            tokio::spawn(async move {
                match tls {
                    None => proxy.serve_connection(stream, peer, listen).await,
                    Some(tls) => match tls.accept(stream).await {
                        Ok(stream) => proxy.serve_connection(stream, peer, listen).await,
                        Err(error) => debug!(%error, "tls handshake failed"),
                    },
                }
            });
        }
    }

    async fn serve_connection<S>(self: Arc<Self>, stream: S, peer: SocketAddr, listen: Listen)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let service = service_fn(move |req| {
            let proxy = self.clone();
            async move { Ok::<_, Infallible>(proxy.handle(req, peer, listen).await) }
        });
        let result = auto::Builder::new(TokioExecutor::new())
            .serve_connection_with_upgrades(TokioIo::new(stream), service)
            .await;
        if let Err(error) = result {
            debug!(%error, "proxy connection ended with error");
        }
    }

    async fn handle(
        &self,
        req: Request<Incoming>,
        peer: SocketAddr,
        listen: Listen,
    ) -> Response<Body> {
        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .or_else(|| req.uri().authority().map(|a| a.as_str()))
            .unwrap_or("")
            .to_ascii_lowercase();
        let origin = listen.origin();

        match host::target(&host) {
            Target::Index => html(StatusCode::OK, pages::index(&self.routes.all(), &origin)),
            Target::Unknown => html(
                StatusCode::NOT_FOUND,
                pages::unknown(&host, &self.routes.all(), "test", &origin),
            ),
            Target::Site { name, suffix, sub } => {
                let Some(route) = self.routes.get(name) else {
                    return html(
                        StatusCode::NOT_FOUND,
                        pages::unknown(
                            host::strip_port(&host),
                            &self.routes.all(),
                            suffix,
                            &origin,
                        ),
                    );
                };
                if route.secure && !listen.https && self.routes.https_port().is_some() {
                    let port = self.routes.https_port().unwrap_or(443);
                    let path = req.uri().path_and_query().map_or("/", |p| p.as_str());
                    return redirect(&https_location(host::strip_port(&host), port, path));
                }
                let (port, label) = match route.destination(sub) {
                    Destination::Site => (route.port, None),
                    Destination::Sub(rule) => (rule.port, Some(rule.label.clone())),
                    Destination::NotConfigured => {
                        return html(
                            StatusCode::NOT_FOUND,
                            pages::subdomain_off(host::strip_port(&host), &route),
                        );
                    }
                };
                match (port, label) {
                    (None, None) => html(StatusCode::BAD_GATEWAY, pages::no_port(&route)),
                    (None, Some(label)) => html(
                        StatusCode::BAD_GATEWAY,
                        pages::sub_no_port(host::strip_port(&host), &label),
                    ),
                    (Some(port), label) => {
                        self.forward(req, &route, port, label.as_deref(), &host, peer, listen)
                            .await
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn forward(
        &self,
        mut req: Request<Incoming>,
        route: &Route,
        port: u16,
        label: Option<&str>,
        host: &str,
        peer: SocketAddr,
        listen: Listen,
    ) -> Response<Body> {
        let started = std::time::Instant::now();
        let path = req.uri().path_and_query().map_or("/", |p| p.as_str());
        let shown_path = path.to_string();
        let method = req.method().to_string();
        let record = |status: u16, down: bool| {
            self.routes.metrics.record(metrics::Seen {
                site: &route.name,
                host,
                method: &method,
                path: &shown_path,
                status,
                ms: started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
                upstream_down: down,
                at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_millis() as u64),
            });
        };
        let failure = |status: u16, down: bool| HttpFailure {
            site: route.name.clone(),
            host: host.to_string(),
            method: method.clone(),
            path: shown_path.clone(),
            status,
            duration_ms: started.elapsed().as_millis() as u64,
            upstream_down: down,
        };
        let Ok(uri) = format!("http://127.0.0.1:{port}{path}").parse::<Uri>() else {
            return html(StatusCode::BAD_REQUEST, "bad request".into());
        };

        let upgrade = is_upgrade(req.headers());
        let downstream = upgrade.then(|| hyper::upgrade::on(&mut req));

        *req.uri_mut() = uri;
        *req.version_mut() = Version::HTTP_11;
        let headers = req.headers_mut();
        strip_hop_by_hop(headers, upgrade);
        add_forwarded(headers, host, peer, listen.https);
        relabel_next_dev_origin(headers, host, &shown_path);
        relabel_nuxt_dev_host(headers, host, &shown_path);

        let mut response = match self.client.request(req).await {
            Ok(response) => response,
            Err(error) => {
                debug!(site = %route.name, %error, "upstream unavailable");
                record(502, true);
                self.report(failure(502, true));
                return match label {
                    None => html(StatusCode::BAD_GATEWAY, pages::not_running(route)),
                    Some(label) => html(
                        StatusCode::BAD_GATEWAY,
                        pages::sub_not_running(host::strip_port(host), label),
                    ),
                };
            }
        };

        record(response.status().as_u16(), false);
        if let Some(downstream) = downstream
            && response.status() == StatusCode::SWITCHING_PROTOCOLS
        {
            let upstream = hyper::upgrade::on(&mut response);
            tokio::spawn(async move {
                match tokio::try_join!(downstream, upstream) {
                    Ok((down, up)) => {
                        let mut down = TokioIo::new(down);
                        let mut up = TokioIo::new(up);
                        let _ = tokio::io::copy_bidirectional(&mut down, &mut up).await;
                    }
                    Err(error) => debug!(%error, "upgrade failed"),
                }
            });
        } else {
            strip_hop_by_hop(response.headers_mut(), false);
        }
        if listen.https {
            keep_redirect_on_https(response.headers_mut(), host);
        }
        if response.status().is_server_error() {
            self.report(failure(response.status().as_u16(), false));
        }

        response.map(|body| body.boxed())
    }
}

fn is_upgrade(headers: &HeaderMap) -> bool {
    headers.contains_key(header::UPGRADE)
        && headers
            .get_all(header::CONNECTION)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .any(|v| {
                v.split(',')
                    .any(|t| t.trim().eq_ignore_ascii_case("upgrade"))
            })
}

/// Connection-scoped headers must not be forwarded (RFC 9110 §7.6.1), except
/// `Connection`/`Upgrade` themselves when tunnelling an upgrade.
fn strip_hop_by_hop(headers: &mut HeaderMap, keep_upgrade: bool) {
    let listed: Vec<HeaderName> = headers
        .get_all(header::CONNECTION)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|t| HeaderName::from_bytes(t.trim().as_bytes()).ok())
        .filter(|name| !(keep_upgrade && name == header::UPGRADE))
        .collect();
    for name in listed {
        headers.remove(name);
    }

    for name in [
        header::PROXY_AUTHENTICATE,
        header::PROXY_AUTHORIZATION,
        header::TE,
        header::TRAILER,
        header::TRANSFER_ENCODING,
    ] {
        headers.remove(name);
    }
    headers.remove("keep-alive");
    headers.remove("proxy-connection");

    if !keep_upgrade {
        headers.remove(header::CONNECTION);
        headers.remove(header::UPGRADE);
    }
}

fn add_forwarded(headers: &mut HeaderMap, host: &str, peer: SocketAddr, https: bool) {
    let ip = peer.ip().to_string();
    let forwarded_for = match headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        Some(prev) => format!("{prev}, {ip}"),
        None => ip,
    };
    let (_, port) = host
        .rsplit_once(':')
        .filter(|(_, p)| p.bytes().all(|b| b.is_ascii_digit()))
        .unwrap_or(("", if https { "443" } else { "80" }));

    for (name, value) in [
        ("x-forwarded-for", forwarded_for.as_str()),
        ("x-forwarded-host", host),
        ("x-forwarded-proto", if https { "https" } else { "http" }),
        ("x-forwarded-port", port),
    ] {
        if let Ok(value) = HeaderValue::from_str(value) {
            headers.insert(name, value);
        }
    }
}

/// The Next.js dev server (15.2+, blocking by default since 16) refuses its
/// own resources, the hot-reload WebSocket above all, to any `Origin` outside
/// `allowedDevOrigins`, which allows `**.localhost` but not `.test`; the page
/// then reloads after a few failed reconnects, forever. A same-origin request
/// (its `Origin` is the very host it was sent to) gets that host's
/// `.localhost` twin as `Origin`, which Next allows. Cross-site requests and
/// every other path keep their `Origin`, so Next's protection and the apps'
/// own checks (Server Actions compare `Origin` with the host) are unchanged.
fn relabel_next_dev_origin(headers: &mut HeaderMap, host: &str, path: &str) {
    let pathname = path.split('?').next().unwrap_or(path);
    if !(pathname.contains("/_next") || pathname.contains("/__nextjs")) {
        return;
    }
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return;
    };
    let Some((scheme, authority)) = origin.split_once("://") else {
        return;
    };
    if !authority.eq_ignore_ascii_case(host) {
        return;
    }
    let name = host::strip_port(host);
    let Some(base) = name.strip_suffix(&format!(".{}", locdev_core::TLD)) else {
        return;
    };
    let port = host.strip_prefix(name).unwrap_or("");
    if let Ok(value) = HeaderValue::from_str(&format!("{scheme}://{base}.localhost{port}")) {
        headers.insert(header::ORIGIN, value);
    }
}

/// The Nuxt CLI (4.1+) answers its own endpoints (`/__nuxt_dev__/…`, the
/// error badge every page loads) only for a `Host` it was told about in
/// `nuxt.config` (`devServer.allowedHosts`) or on the command line, besides
/// `localhost`, `*.localhost` and IP addresses; `<site>.test` gets a 403. Those
/// requests carry the `.localhost` twin as `Host`; everything else, Vite's
/// assets and the app's own routes included, keeps the site's name.
fn relabel_nuxt_dev_host(headers: &mut HeaderMap, host: &str, path: &str) {
    if !path.starts_with("/__nuxt_dev__/") {
        return;
    }
    let name = host::strip_port(host);
    let Some(base) = name.strip_suffix(&format!(".{}", locdev_core::TLD)) else {
        return;
    };
    let port = host.strip_prefix(name).unwrap_or("");
    if let Ok(value) = HeaderValue::from_str(&format!("{base}.localhost{port}")) {
        headers.insert(header::HOST, value);
    }
}

/// A redirect to `http://` on the very host an HTTPS request came in for
/// points back to HTTPS. Dev servers behind the proxy speak plain HTTP and,
/// unless told to trust `X-Forwarded-Proto`, build absolute redirects with
/// `http://` (WordPress with an `http://` `WP_HOME`, a framework's own URL
/// setting): the browser would leave HTTPS, or, on an HTTPS-only site, bounce
/// between the two. The port the app saw (its own, or none) gives way to the
/// one the browser used. Other hosts and relative locations are untouched.
fn keep_redirect_on_https(headers: &mut HeaderMap, host: &str) {
    let Some(location) = headers.get(header::LOCATION).and_then(|v| v.to_str().ok()) else {
        return;
    };
    let Some(rest) = location
        .get(..7)
        .filter(|scheme| scheme.eq_ignore_ascii_case("http://"))
        .map(|_| &location[7..])
    else {
        return;
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, path) = rest.split_at(end);
    if !host::strip_port(authority).eq_ignore_ascii_case(host::strip_port(host)) {
        return;
    }
    if let Ok(value) = HeaderValue::from_str(&format!("https://{host}{path}")) {
        headers.insert(header::LOCATION, value);
    }
}

/// `https://host[:port]/path`, the port omitted when it's 443.
fn https_location(host: &str, port: u16, path: &str) -> String {
    let port = if port == 443 {
        String::new()
    } else {
        format!(":{port}")
    };
    format!("https://{host}{port}{path}")
}

/// 307 keeps the method and body (a form POST stays a POST).
fn redirect(location: &str) -> Response<Body> {
    let mut response = html(StatusCode::TEMPORARY_REDIRECT, String::new());
    if let Ok(value) = HeaderValue::from_str(location) {
        response.headers_mut().insert(header::LOCATION, value);
    }
    response
}

fn html(status: StatusCode, body: String) -> Response<Body> {
    let mut response = Response::new(
        Full::new(Bytes::from(body))
            .map_err(|never| match never {})
            .boxed(),
    );
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin_after(origin: &str, host: &str, path: &str) -> String {
        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, HeaderValue::from_str(origin).unwrap());
        relabel_next_dev_origin(&mut headers, host, path);
        headers[header::ORIGIN].to_str().unwrap().to_string()
    }

    fn nuxt_host_after(host: &str, path: &str) -> String {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_str(host).unwrap());
        relabel_nuxt_dev_host(&mut headers, host, path);
        headers[header::HOST].to_str().unwrap().to_string()
    }

    #[test]
    fn nuxt_dev_endpoints_see_a_localhost_host() {
        assert_eq!(
            nuxt_host_after("shop.test", "/__nuxt_dev__/error/badge.js?v=1"),
            "shop.localhost"
        );
        assert_eq!(
            nuxt_host_after("api.shop.test:7443", "/__nuxt_dev__/x"),
            "api.shop.localhost:7443"
        );
        for (host, path) in [
            ("shop.test", "/"),
            ("shop.test", "/_nuxt/app.js"),
            ("shop.test", "/a/__nuxt_dev__/x"),
            ("shop.localhost", "/__nuxt_dev__/x"),
        ] {
            assert_eq!(nuxt_host_after(host, path), host);
        }
    }

    fn location_after(location: &str, host: &str) -> String {
        let mut headers = HeaderMap::new();
        headers.insert(header::LOCATION, HeaderValue::from_str(location).unwrap());
        keep_redirect_on_https(&mut headers, host);
        headers[header::LOCATION].to_str().unwrap().to_string()
    }

    #[test]
    fn redirects_to_the_same_host_stay_on_https() {
        assert_eq!(
            location_after("http://shop.test/wp-admin/install.php", "shop.test"),
            "https://shop.test/wp-admin/install.php"
        );
        // The app's own port, or none, becomes the one the browser used.
        assert_eq!(
            location_after("http://shop.test:8100/a?b=1", "shop.test:7443"),
            "https://shop.test:7443/a?b=1"
        );
        assert_eq!(
            location_after("HTTP://Shop.Test", "shop.test"),
            "https://shop.test"
        );
        assert_eq!(
            location_after("http://api.shop.test/x", "api.shop.test"),
            "https://api.shop.test/x"
        );
        // Other hosts, relative and HTTPS locations are left alone.
        for kept in [
            "http://other.test/x",
            "http://shop.test.evil.com/",
            "/login",
            "//shop.test/x",
            "https://shop.test/x",
            "http://127.0.0.1:8100/x",
        ] {
            assert_eq!(location_after(kept, "shop.test"), kept);
        }
    }

    #[test]
    fn next_dev_resources_see_a_localhost_origin() {
        assert_eq!(
            origin_after("https://shop.test", "shop.test", "/_next/hmr?id=1"),
            "https://shop.localhost"
        );
        assert_eq!(
            origin_after(
                "http://api.shop.test:8080",
                "api.shop.test:8080",
                "/_next/webpack-hmr"
            ),
            "http://api.shop.localhost:8080"
        );
        assert_eq!(
            origin_after(
                "https://shop.test",
                "shop.test",
                "/__nextjs_original-stack-frames"
            ),
            "https://shop.localhost"
        );
    }

    #[test]
    fn other_origins_and_paths_are_left_alone() {
        // Cross-site: Next must still block it.
        assert_eq!(
            origin_after("https://evil.test", "shop.test", "/_next/hmr"),
            "https://evil.test"
        );
        // App routes: Server Actions compare Origin with the host.
        assert_eq!(
            origin_after("https://shop.test", "shop.test", "/checkout"),
            "https://shop.test"
        );
        // Already allowed by Next.
        assert_eq!(
            origin_after("https://shop.localhost", "shop.localhost", "/_next/hmr"),
            "https://shop.localhost"
        );
        assert_eq!(origin_after("null", "shop.test", "/_next/hmr"), "null");
    }
}
