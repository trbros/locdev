//! Static sites: serves a folder over HTTP on loopback, so a plain
//! `index.html` project runs without Python or Node (`locdevd static`).
//! Clean URLs (`/about` → `about.html`, `about/index.html`) and a `404.html`
//! like most static hosts; nothing outside the folder is ever served.

use std::convert::Infallible;
use std::path::{Component, Path, PathBuf};

use bytes::Bytes;
use http_body_util::Full;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode, header};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use tokio::net::TcpListener;

/// Serves `root` until the task ends.
pub async fn serve_dir(root: PathBuf, listener: TcpListener) {
    let root = std::sync::Arc::new(root);
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let root = root.clone();
        tokio::spawn(async move {
            let service = service_fn(move |req| {
                let root = root.clone();
                async move { Ok::<_, Infallible>(respond(&root, &req).await) }
            });
            let _ = auto::Builder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

async fn respond<B>(root: &Path, req: &Request<B>) -> Response<Full<Bytes>> {
    let head = req.method() == Method::HEAD;
    if req.method() != Method::GET && !head {
        return text(StatusCode::METHOD_NOT_ALLOWED, "method not allowed");
    }
    let Some(relative) = safe_path(req.uri().path()) else {
        return text(StatusCode::BAD_REQUEST, "bad path");
    };
    let mut found = None;
    for candidate in candidates(&relative) {
        let path = root.join(&candidate);
        if tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_file()) {
            found = Some(path);
            break;
        }
    }
    let (status, path) = match found {
        Some(path) => (StatusCode::OK, path),
        None => (StatusCode::NOT_FOUND, root.join("404.html")),
    };
    let Ok(body) = tokio::fs::read(&path).await else {
        return text(StatusCode::NOT_FOUND, "not found");
    };
    let length = body.len();
    let mut response = Response::new(Full::new(if head {
        Bytes::new()
    } else {
        Bytes::from(body)
    }));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, mime(&path).parse().unwrap());
    headers.insert(header::CONTENT_LENGTH, length.into());
    // Edits show on reload.
    headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    response
}

fn text(status: StatusCode, message: &'static str) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::from_static(message.as_bytes())));
    *response.status_mut() = status;
    response
}

/// The request path as a relative path inside the folder, or `None` when it
/// tries to leave it (`..`, a drive or root, NUL, bad escapes).
fn safe_path(path: &str) -> Option<PathBuf> {
    let decoded = percent_decode(path)?;
    if decoded.contains('\0') || decoded.contains('\\') {
        return None;
    }
    let mut out = PathBuf::new();
    for part in Path::new(decoded.trim_start_matches('/')).components() {
        match part {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(out)
}

/// What a path can mean: the file, its `index.html`, or `<path>.html`.
fn candidates(relative: &Path) -> Vec<PathBuf> {
    if relative.as_os_str().is_empty() {
        return vec![PathBuf::from("index.html")];
    }
    let mut html = relative.as_os_str().to_owned();
    html.push(".html");
    vec![
        relative.to_path_buf(),
        relative.join("index.html"),
        PathBuf::from(html),
    ]
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn mime(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "txt" | "md" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wasm" => "application/wasm",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_stay_inside_the_folder() {
        assert_eq!(safe_path("/a%20b/c.css"), Some(PathBuf::from("a b/c.css")));
        assert_eq!(safe_path("/"), Some(PathBuf::new()));
        for bad in [
            "/../etc/passwd",
            "/a/%2e%2e/%2e%2e/x",
            "/%00",
            "/a%5c..%5cb",
            "/%zz",
        ] {
            assert_eq!(safe_path(bad), None, "{bad}");
        }
    }

    #[tokio::test]
    async fn serves_files_clean_urls_and_404() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "home").unwrap();
        std::fs::write(dir.path().join("about.html"), "about").unwrap();
        std::fs::write(dir.path().join("404.html"), "missing").unwrap();
        let get = |uri: &str| Request::get(uri).body(()).unwrap();
        let body = |r: Response<Full<Bytes>>| async move {
            use http_body_util::BodyExt;
            let status = r.status();
            let bytes = r.into_body().collect().await.unwrap().to_bytes();
            (status, String::from_utf8(bytes.to_vec()).unwrap())
        };
        assert_eq!(
            body(respond(dir.path(), &get("/")).await).await,
            (StatusCode::OK, "home".into())
        );
        assert_eq!(
            body(respond(dir.path(), &get("/about")).await).await,
            (StatusCode::OK, "about".into())
        );
        assert_eq!(
            body(respond(dir.path(), &get("/nope")).await).await,
            (StatusCode::NOT_FOUND, "missing".into())
        );
        assert_eq!(
            respond(dir.path(), &get("/../x")).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
}
