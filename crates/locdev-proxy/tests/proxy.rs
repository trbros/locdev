//! Drives the proxy over real sockets with raw HTTP/1.1, against tiny upstreams.

use std::sync::Arc;

use locdev_proxy::{Proxy, Route, Routes};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn start_proxy(routes: Vec<Route>) -> u16 {
    let table = Arc::new(Routes::default());
    table.replace(routes);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(Proxy::new(table).serve(listener));
    port
}

/// Upstream that answers every request with the request head it received.
async fn echo_upstream() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let head = read_head(&mut stream).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{head}",
                    head.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            });
        }
    });
    port
}

async fn read_head(stream: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while !buf.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).await.unwrap() == 0 {
            break;
        }
        buf.push(byte[0]);
    }
    String::from_utf8(buf).unwrap()
}

async fn get(port: u16, host: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!("GET /hello?x=1 HTTP/1.1\r\nhost: {host}\r\nconnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response
}

#[tokio::test]
async fn forwards_by_host_with_forwarded_headers() {
    let upstream = echo_upstream().await;
    let proxy = start_proxy(vec![Route {
        name: "blog".into(),
        port: Some(upstream),
        ..Default::default()
    }])
    .await;

    for host in [
        format!("blog.test:{proxy}"),
        format!("api.blog.localhost:{proxy}"),
    ] {
        let response = get(proxy, &host).await.to_ascii_lowercase();
        assert!(response.starts_with("http/1.1 200"), "{response}");
        assert!(response.contains("get /hello?x=1 http/1.1"), "{response}");
        // The original Host is preserved; apps build URLs from it.
        assert!(response.contains(&format!("host: {host}")), "{response}");
        assert!(
            response.contains(&format!("x-forwarded-host: {host}")),
            "{response}"
        );
        assert!(
            response.contains("x-forwarded-for: 127.0.0.1"),
            "{response}"
        );
    }
}

#[tokio::test]
async fn unknown_and_stopped_sites_get_pages() {
    // Bind and drop to get a port nothing listens on.
    let dead_port = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let proxy = start_proxy(vec![Route {
        name: "shop".into(),
        port: Some(dead_port),
        ..Default::default()
    }])
    .await;

    let unknown = get(proxy, "nope.test").await;
    assert!(unknown.starts_with("HTTP/1.1 404"), "{unknown}");
    assert!(unknown.contains("shop.test"), "lists registered sites");

    let stopped = get(proxy, "shop.test").await;
    assert!(stopped.starts_with("HTTP/1.1 502"), "{stopped}");
    assert!(stopped.contains("locdev up shop"));

    let index = get(proxy, "localhost").await;
    assert!(index.starts_with("HTTP/1.1 200"), "{index}");
    assert!(index.contains("shop.localhost"));
}

#[tokio::test]
async fn tunnels_websocket_upgrades() {
    // Upstream accepts the upgrade, then echoes raw bytes back.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let head = read_head(&mut stream).await.to_ascii_lowercase();
        assert!(head.contains("upgrade: websocket"), "{head}");
        stream
            .write_all(b"HTTP/1.1 101 Switching Protocols\r\nconnection: upgrade\r\nupgrade: websocket\r\n\r\n")
            .await
            .unwrap();
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).await.unwrap();
        stream.write_all(&buf).await.unwrap();
    });

    let proxy = start_proxy(vec![Route {
        name: "hmr".into(),
        port: Some(upstream),
        ..Default::default()
    }])
    .await;

    let mut client = TcpStream::connect(("127.0.0.1", proxy)).await.unwrap();
    client
        .write_all(
            b"GET /ws HTTP/1.1\r\nhost: hmr.test\r\nconnection: Upgrade\r\nupgrade: websocket\r\nsec-websocket-version: 13\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
        )
        .await
        .unwrap();
    let head = read_head(&mut client).await;
    assert!(head.starts_with("HTTP/1.1 101"), "{head}");

    client.write_all(b"ping").await.unwrap();
    let mut echoed = [0u8; 4];
    client.read_exact(&mut echoed).await.unwrap();
    assert_eq!(&echoed, b"ping");
}

#[tokio::test]
async fn serves_https_with_per_site_certificates() {
    use locdev_tls::{Authority, SiteCerts, server_config};
    use rustls::pki_types::{CertificateDer, ServerName};

    let upstream = echo_upstream().await;
    let table = Arc::new(Routes::default());
    table.replace(vec![Route {
        name: "blog".into(),
        port: Some(upstream),
        ..Default::default()
    }]);

    let dir = tempfile::tempdir().unwrap();
    let ca = Arc::new(Authority::load_or_create(dir.path()).unwrap());
    let acceptor =
        tokio_rustls::TlsAcceptor::from(server_config(SiteCerts::new(ca.clone())).unwrap());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(Proxy::new(table).serve_tls(listener, acceptor));

    // A client that trusts only the LocDev CA.
    let mut roots = rustls::RootCertStore::empty();
    roots.add(CertificateDer::from(ca.der().to_vec())).unwrap();
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));

    let tcp = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let mut tls = connector
        .connect(ServerName::try_from("blog.test").unwrap(), tcp)
        .await
        .expect("handshake with a certificate for blog.test");

    tls.write_all(b"GET / HTTP/1.1\r\nhost: blog.test\r\nconnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut response = Vec::new();
    let _ = tls.read_to_end(&mut response).await;
    let response = String::from_utf8_lossy(&response).to_ascii_lowercase();

    assert!(response.starts_with("http/1.1 200"), "{response}");
    assert!(response.contains("x-forwarded-proto: https"), "{response}");
    assert!(response.contains("x-forwarded-port: 443"), "{response}");
}

#[tokio::test]
async fn secure_sites_redirect_plain_http() {
    let upstream = echo_upstream().await;
    let table = Arc::new(Routes::default());
    table.replace(vec![
        Route {
            name: "shop".into(),
            port: Some(upstream),
            secure: true,
            ..Default::default()
        },
        Route {
            name: "blog".into(),
            port: Some(upstream),
            secure: false,
            ..Default::default()
        },
    ]);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(Proxy::new(table.clone()).serve(listener));

    // Without HTTPS being served there is nowhere to redirect to.
    assert!(get(port, "shop.test").await.starts_with("HTTP/1.1 200"));

    table.set_https_port(Some(7443));
    let response = get(port, &format!("shop.test:{port}"))
        .await
        .to_ascii_lowercase();
    assert!(response.starts_with("http/1.1 307"), "{response}");
    assert!(
        response.contains("location: https://shop.test:7443/hello?x=1"),
        "{response}"
    );
    // Other sites keep plain HTTP.
    assert!(get(port, "blog.test").await.starts_with("HTTP/1.1 200"));
}

/// Answers every request with a fixed body, to tell upstreams apart.
async fn named_upstream(name: &'static str) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                read_head(&mut stream).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{name}",
                    name.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            });
        }
    });
    port
}

#[tokio::test]
async fn routes_subdomains_by_the_most_specific_rule() {
    use locdev_proxy::SubRoute;
    let site = named_upstream("site").await;
    let api = named_upstream("api").await;
    let tenants = named_upstream("tenants").await;
    let rule = |pattern: &str, port: Option<u16>| SubRoute {
        pattern: pattern.into(),
        port,
        label: format!("rule {pattern}"),
    };
    let route = Route {
        name: "shop".into(),
        port: Some(site),
        subdomains: vec![
            rule("api", Some(api)),
            rule("*.tenants", Some(tenants)),
            rule("ws", None),
        ],
        ..Default::default()
    };
    let proxy = start_proxy(vec![
        route.clone(),
        Route {
            name: "only".into(),
            port: Some(site),
            exclusive: true,
            subdomains: vec![rule("api", Some(api))],
            ..Default::default()
        },
    ])
    .await;

    let body = |r: String| r.rsplit("\r\n\r\n").next().unwrap_or("").to_string();
    assert_eq!(body(get(proxy, "shop.test").await), "site");
    assert_eq!(body(get(proxy, "api.shop.test").await), "api");
    assert_eq!(body(get(proxy, "acme.tenants.shop.test").await), "tenants");
    assert_eq!(
        body(get(proxy, "a.b.tenants.shop.test:80").await),
        "tenants"
    );
    // Wildcard on: anything else is the site.
    assert_eq!(body(get(proxy, "cliente1.shop.test").await), "site");
    assert!(get(proxy, "ws.shop.test").await.starts_with("HTTP/1.1 502"));

    // Wildcard off: only the site and listed subdomains.
    assert_eq!(body(get(proxy, "api.only.test").await), "api");
    assert_eq!(body(get(proxy, "only.test").await), "site");
    let off = get(proxy, "other.only.test").await;
    assert!(
        off.starts_with("HTTP/1.1 404") && off.contains("subdomain not configured"),
        "{off}"
    );
}
