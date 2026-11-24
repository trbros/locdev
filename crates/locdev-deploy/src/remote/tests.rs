//! Each client against a fake API on loopback that records what it got.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use locdev_core::remote::RemoteState;

use super::*;

#[derive(Debug, Clone)]
struct Seen {
    method: String,
    path: String,
    auth: String,
    body: String,
}

/// Serves `routes` ("METHOD /path" → (status, body)); unknown → 404.
fn fake(routes: HashMap<&'static str, (u16, &'static str)>) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut parts = line.split_whitespace();
            let (method, path) = (
                parts.next().unwrap_or("").to_string(),
                parts.next().unwrap_or("").to_string(),
            );
            let (mut auth, mut length) = (String::new(), 0usize);
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let header = header.trim_end();
                if header.is_empty() {
                    break;
                }
                let (k, v) = header.split_once(':').unwrap_or((header, ""));
                match k.to_ascii_lowercase().as_str() {
                    "authorization" => auth = v.trim().to_string(),
                    "content-length" => length = v.trim().parse().unwrap_or(0),
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            log.lock().unwrap().push(Seen {
                method: method.clone(),
                path: path.clone(),
                auth,
                body: String::from_utf8_lossy(&body).into_owned(),
            });
            let key = format!("{method} {}", path.split('?').next().unwrap());
            let (status, reply) = routes
                .iter()
                .find(|(k, _)| **k == key)
                .map(|(_, v)| *v)
                .unwrap_or((404, "{}"));
            let mut stream = stream;
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    (base, seen)
}

const VERCEL_PROJECT: &str = r#"{"id":"prj_1","name":"shop","framework":"nextjs","link":{"type":"github","org":"acme","repo":"shop","productionBranch":"main"},"targets":{"production":{"id":"dpl_2","alias":["shop.acme.com"]}}}"#;

#[test]
fn vercel_reads_deploys_and_rolls_back() {
    let (base, seen) = fake(HashMap::from([
        (
            "GET /v2/user",
            (200, r#"{"user":{"username":"ana","email":"ana@acme.com"}}"#),
        ),
        (
            "GET /v10/projects",
            (
                200,
                r#"{"projects":[{"id":"prj_1","name":"shop","link":{"org":"acme","repo":"shop","productionBranch":"main"}}]}"#,
            ),
        ),
        ("GET /v9/projects/prj_1", (200, VERCEL_PROJECT)),
        (
            "GET /v7/deployments",
            (
                200,
                r#"{"deployments":[
            {"uid":"dpl_3","name":"shop","url":"shop-3.vercel.app","readyState":"BUILDING","created":1793800000000,"target":"production","meta":{"githubCommitRef":"main","githubCommitSha":"abc123","githubCommitMessage":"Fix cart"},"inspectorUrl":"https://vercel.com/acme/shop/3"},
            {"uid":"dpl_2","name":"shop","url":"shop-2.vercel.app","state":"READY","created":1793700000000,"target":"production","meta":{}}]}"#,
            ),
        ),
        (
            "GET /v9/projects/prj_1/domains",
            (
                200,
                r#"{"domains":[{"name":"shop.acme.com"},{"name":"shop.vercel.app"}]}"#,
            ),
        ),
        (
            "GET /v10/projects/prj_1/env",
            (
                200,
                r#"{"envs":[{"key":"DATABASE_URL","target":["production","preview"],"value":"enc:xyz"}]}"#,
            ),
        ),
        (
            "POST /v13/deployments",
            (
                200,
                r#"{"id":"dpl_4","name":"shop","url":"shop-4.vercel.app","readyState":"QUEUED"}"#,
            ),
        ),
        ("PATCH /v12/deployments/dpl_3/cancel", (200, "{}")),
        ("POST /v1/projects/prj_1/rollback/dpl_2", (201, "")),
        (
            "GET /v3/deployments/dpl_3/events",
            (
                200,
                r#"[{"type":"stdout","payload":{"text":"Installing dependencies…"}},{"type":"stdout","text":"Build completed"}]"#,
            ),
        ),
    ]));
    let client = Client::new(
        "vercel",
        "tok_secret".into(),
        Some("acme".into()),
        Some(base),
    )
    .unwrap();

    let account = client.account().unwrap();
    assert_eq!(account.user, "ana");
    let projects = client.projects("acme").unwrap();
    assert_eq!(projects[0].repo.as_deref(), Some("acme/shop"));
    let project = client.project("prj_1", "acme").unwrap();
    assert_eq!(project.url.as_deref(), Some("https://shop.acme.com"));
    assert_eq!(project.dashboard, "https://vercel.com/acme/shop");

    let deployments = client.deployments("prj_1", "acme").unwrap();
    assert_eq!(deployments[0].state, RemoteState::Building);
    assert_eq!(deployments[0].commit.as_deref(), Some("abc123"));
    assert!(deployments[1].current && deployments[1].production);
    assert_eq!(client.domains("prj_1").unwrap().len(), 2);
    let env = client.env("prj_1").unwrap();
    assert_eq!(env[0].key, "DATABASE_URL");
    assert_eq!(env[0].targets, ["production", "preview"]);

    let git = GitRef {
        owner: "acme".into(),
        repo: "shop".into(),
        branch: "main".into(),
        sha: "abc123".into(),
    };
    let created = client.deploy(&project, &git, true, "acme").unwrap();
    assert_eq!(created.id, "dpl_4");
    client.cancel("dpl_3").unwrap();
    client.rollback("prj_1", "dpl_2").unwrap();
    let logs = client.logs(&project, "dpl_3").unwrap();
    assert_eq!(logs.lines, ["Installing dependencies…", "Build completed"]);

    let seen = seen.lock().unwrap();
    assert!(seen.iter().all(|s| s.auth == "Bearer tok_secret"));
    assert!(
        seen.iter().all(|s| s.path.contains("slug=acme")),
        "every call acts for the team"
    );
    let create = seen
        .iter()
        .find(|s| s.method == "POST" && s.path.starts_with("/v13/"))
        .unwrap();
    let body: Value = serde_json::from_str(&create.body).unwrap();
    assert_eq!(body["gitSource"]["type"], "github");
    assert_eq!(body["gitSource"]["sha"], "abc123");
    assert_eq!(body["target"], "production");
    assert!(
        !seen.iter().any(|s| s.path.contains("decrypt")),
        "values are never asked for"
    );
}

#[test]
fn netlify_builds_restores_and_links_logs() {
    let site = r#"{"id":"site_1","name":"blog","account_id":"acc_1","ssl_url":"https://blog.netlify.app","admin_url":"https://app.netlify.com/sites/blog","custom_domain":"blog.acme.com","domain_aliases":["www.blog.acme.com"],"published_deploy":{"id":"dep_1","framework":"astro"},"build_settings":{"repo_url":"https://github.com/acme/blog","repo_branch":"main"}}"#;
    let (base, seen) = fake(HashMap::from([
        (
            "GET /user",
            (200, r#"{"full_name":"Ana","email":"ana@acme.com"}"#),
        ),
        (
            "GET /sites",
            (200, Box::leak(format!("[{site}]").into_boxed_str()) as &str),
        ),
        ("GET /sites/site_1", (200, site)),
        (
            "GET /sites/site_1/deploys",
            (
                200,
                r#"[{"id":"dep_2","state":"error","context":"production","branch":"main","commit_ref":"def456","title":"Break it","error_message":"Build script returned non-zero exit code: 2","created_at":"2026-11-04T12:00:00.000Z","admin_url":"https://app.netlify.com/sites/blog"},{"id":"dep_1","state":"ready","context":"production","deploy_ssl_url":"https://dep_1--blog.netlify.app","created_at":"2026-11-03T10:00:00Z"}]"#,
            ),
        ),
        (
            "GET /accounts/acc_1/env",
            (
                200,
                r#"[{"key":"API_URL","scopes":["builds"],"values":[{"context":"production","value":"x"},{"context":"deploy-preview","value":"y"}]}]"#,
            ),
        ),
        (
            "POST /sites/site_1/builds",
            (
                200,
                r#"{"id":"build_9","deploy_id":"dep_3","created_at":"2026-11-04T13:00:00Z"}"#,
            ),
        ),
        ("POST /deploys/dep_2/cancel", (200, "{}")),
        ("POST /sites/site_1/deploys/dep_1/restore", (200, "{}")),
    ]));
    let client = Client::new("netlify", "nfp_secret".into(), None, Some(base)).unwrap();
    assert_eq!(client.account().unwrap().user, "Ana");
    let project = client.project("site_1", "").unwrap();
    assert_eq!(project.repo.as_deref(), Some("acme/blog"));
    assert_eq!(client.projects("").unwrap().len(), 1);

    let deploys = client.deployments("site_1", "").unwrap();
    assert_eq!(deploys[0].state, RemoteState::Error);
    assert!(deploys[0].error.as_deref().unwrap().contains("non-zero"));
    assert!(deploys[1].current);
    assert_eq!(deploys[1].created_at, Some(1_793_700_000_000));
    assert_eq!(
        client.domains("site_1").unwrap(),
        ["blog.acme.com", "www.blog.acme.com", "blog.netlify.app"]
    );
    assert_eq!(
        client.env("site_1").unwrap()[0].targets,
        ["production", "deploy-preview"]
    );

    let git = GitRef {
        owner: "acme".into(),
        repo: "blog".into(),
        branch: "feature".into(),
        sha: "def456".into(),
    };
    let started = client.deploy(&project, &git, false, "").unwrap();
    assert_eq!(started.id, "dep_3");
    assert_eq!(
        started.dashboard,
        "https://app.netlify.com/sites/blog/deploys/dep_3"
    );
    client.cancel("dep_2").unwrap();
    client.rollback("site_1", "dep_1").unwrap();
    let logs = client.logs(&project, "dep_2").unwrap();
    assert!(logs.lines.is_empty());
    assert_eq!(
        logs.link.as_deref(),
        Some("https://app.netlify.com/sites/blog/deploys/dep_2")
    );

    let seen = seen.lock().unwrap();
    assert!(seen.iter().all(|s| s.auth == "Bearer nfp_secret"));
    assert!(
        seen.iter()
            .any(|s| s.path == "/sites/site_1/builds?branch=feature")
    );
}

#[test]
fn errors_are_explained_without_the_token() {
    let (base, _) = fake(HashMap::from([(
        "GET /v2/user",
        (401, r#"{"error":{"message":"bad"}}"#),
    )]));
    let client = Client::new("vercel", "tok_secret".into(), None, Some(base)).unwrap();
    let error = client.account().unwrap_err();
    assert!(error.contains("refused the token"), "{error}");
    assert!(!error.contains("tok_secret"));
    assert!(Client::new("render", "x".into(), None, None).is_err());
}

#[test]
fn timestamps() {
    assert_eq!(rfc3339_ms("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(rfc3339_ms("2026-11-03T10:00:00Z"), Some(1_793_700_000_000));
    assert_eq!(rfc3339_ms("not a date"), None);
}
