use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use locdev_core::mail::{MailEnvelope, MailWiring};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::smtp::{self, Sink};
use crate::store::{Filter, Store};
use crate::wiring::{Input, plan};

#[derive(Default)]
struct Captured(Mutex<Vec<(MailEnvelope, Vec<u8>)>>);

impl Sink for Captured {
    fn deliver(&self, envelope: MailEnvelope, raw: Vec<u8>) {
        self.0.lock().unwrap().push((envelope, raw));
    }
}

const MESSAGE: &str = "From: Shop <noreply@shop.test>\r\nTo: Ana <ana@example.com>\r\nSubject: Welcome\r\nDate: Tue, 03 Nov 2026 10:00:00 +0000\r\nMessage-ID: <1@shop.test>\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: multipart/alternative; boundary=\"a\"\r\n\r\n--a\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nHi Ana, confirm at https://shop.test/confirm?t=1\r\n.leading dot\r\n--a\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>Hi Ana, <a href=\"https://shop.test/confirm?t=1&amp;x=2\">confirm</a> <img src=\"cid:logo\"></p><script>alert(1)</script>\r\n--a--\r\n--b\r\nContent-Type: image/png\r\nContent-ID: <logo>\r\nContent-Disposition: inline\r\nContent-Transfer-Encoding: base64\r\n\r\niVBORw0KGgo=\r\n--b\r\nContent-Type: application/pdf; name=\"invoice.pdf\"\r\nContent-Disposition: attachment; filename=\"invoice.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQ=\r\n--b--\r\n";

#[tokio::test]
async fn smtp_session_captures_with_auth_and_dot_stuffing() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let sink = Arc::new(Captured::default());
    tokio::spawn(smtp::serve(listener, sink.clone()));

    crate::client::send(
        addr,
        Some("shop"),
        "noreply@shop.test",
        &["ana@example.com".into()],
        MESSAGE.as_bytes(),
    )
    .await
    .unwrap();
    let got = sink.0.lock().unwrap().clone();
    assert_eq!(got.len(), 1);
    let (env, raw) = &got[0];
    assert_eq!(env.auth_user.as_deref(), Some("shop"));
    assert_eq!(
        (env.from.as_str(), env.to.clone()),
        ("noreply@shop.test", vec!["ana@example.com".to_string()])
    );
    assert_eq!(env.protocol, "ESMTPA");
    assert_eq!(env.helo.as_deref(), Some("locdev"));
    // The client stuffed the leading dot; the server took it off again.
    assert_eq!(String::from_utf8_lossy(raw), MESSAGE);

    // AUTH LOGIN, a raw session, and refusals.
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let (r, mut w) = stream.into_split();
    let mut r = BufReader::new(r);
    let mut line = String::new();
    let mut say = async |cmd: &str| {
        w.write_all(cmd.as_bytes()).await.unwrap();
    };
    r.read_line(&mut line).await.unwrap();
    say("RCPT TO:<x@y>\r\n").await;
    line.clear();
    r.read_line(&mut line).await.unwrap();
    assert!(line.starts_with("503"), "{line}");
    say("AUTH LOGIN YmxvZw==\r\n").await;
    line.clear();
    r.read_line(&mut line).await.unwrap();
    assert!(line.starts_with("334"));
    say("c2VjcmV0\r\n").await;
    line.clear();
    r.read_line(&mut line).await.unwrap();
    assert!(line.starts_with("235"), "{line}");
    say("STARTTLS\r\nQUIT\r\n").await;
    line.clear();
    r.read_line(&mut line).await.unwrap();
    assert!(line.starts_with("502"));
}

#[test]
fn store_parses_lists_and_prunes() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let env = |t: u64| MailEnvelope {
        from: "a@b".into(),
        to: vec!["c@d".into()],
        received_at: t,
        protocol: "ESMTP".into(),
        ..Default::default()
    };
    let first = store
        .add(env(1000), Some("shop".into()), MESSAGE.as_bytes())
        .unwrap();
    store
        .add(
            env(1000),
            Some("blog".into()),
            b"Subject: plain\r\n\r\nhello there\r\n",
        )
        .unwrap();
    assert_eq!(first.subject, "Welcome");
    assert_eq!(first.from, "Shop <noreply@shop.test>");
    assert_eq!(first.attachments, 1, "the inline logo isn't an attachment");
    assert!(first.has_html);

    assert_eq!(
        store
            .list(&Filter {
                site: Some("shop".into()),
                ..Default::default()
            })
            .len(),
        1
    );
    assert_eq!(
        store.list(&Filter {
            query: Some("HELLO".into()),
            ..Default::default()
        })[0]
            .subject,
        "plain"
    );
    assert_eq!(
        store
            .list(&Filter {
                with_attachments: true,
                ..Default::default()
            })
            .len(),
        1
    );
    assert_eq!(store.counts(), (2, 2));

    let detail = store.get(&first.id).unwrap();
    assert_eq!(store.counts().1, 1, "opening marks it read");
    let html = detail.html.unwrap();
    assert!(
        html.contains("data:image/png;base64,iVBORw0KGgo="),
        "cid inlined"
    );
    assert_eq!(detail.links[0].url, "https://shop.test/confirm?t=1&x=2");
    assert_eq!(detail.links[0].text.as_deref(), Some("confirm"));
    let pdf = detail
        .attachments
        .iter()
        .find(|a| a.name.as_deref() == Some("invoice.pdf"))
        .unwrap();
    assert_eq!(pdf.content_type, "application/pdf");
    assert!(detail.problems.is_empty(), "{:?}", detail.problems);
    let (name, ctype, bytes) =
        crate::parse::attachment(&store.raw(&first.id).unwrap(), pdf.index).unwrap();
    assert_eq!(
        (name.as_deref(), ctype.as_str(), &bytes[..4]),
        (Some("invoice.pdf"), "application/pdf", &b"%PDF"[..])
    );

    // Survives a reopen; ids can't reach outside the folder.
    let again = Store::open(dir.path()).unwrap();
    assert_eq!(again.counts(), (2, 1));
    assert!(again.raw("../x").is_none());
    assert_eq!(again.clear(Some("blog")), 1);
    assert_eq!(again.delete(std::slice::from_ref(&first.id)), 1);
    assert_eq!(again.counts(), (0, 0));
}

#[test]
fn problems_are_what_the_message_shows() {
    let parsed = crate::parse::parse(b"To: x@y\r\nContent-Type: text/html\r\n\r\n<p>hi</p>\r\n");
    let all = parsed.problems.join("\n");
    for expected in [
        "no From header",
        "no Date header",
        "no Message-ID",
        "no Subject",
        "HTML only",
    ] {
        assert!(all.contains(expected), "{expected} missing in {all}");
    }
    let bad_cid = crate::parse::parse(b"From: a@b\r\nDate: Tue, 03 Nov 2026 10:00:00 +0000\r\nMessage-ID: <1@b>\r\nSubject: s\r\nMIME-Version: 1.0\r\nContent-Type: text/html\r\n\r\n<img src=\"cid:missing\">\r\n");
    assert!(bad_cid.problems.iter().any(|p| p.contains("cid:missing")));
}

fn wire(components: &[&str], files: &[(&str, &str)], env: &[(&str, &str)]) -> crate::wiring::Plan {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let p = dir.path().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let env: BTreeMap<String, String> = env
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let components: Vec<String> = components.iter().map(|c| c.to_string()).collect();
    let ini = dir.path().join("ini");
    let plan = plan(&Input {
        site: "shop",
        framework: "Test",
        components: &components,
        root: dir.path(),
        port: 1025,
        env: &env,
        sendmail: Some("'/opt/locdevd' sendmail --site shop".into()),
        ini_dir: &ini,
    });
    std::mem::forget(dir);
    plan
}

#[test]
fn laravel_wiring_follows_its_config_and_flags_real_smtp() {
    let modern = wire(
        &["laravel"],
        &[("config/mail.php", "'scheme' => env('MAIL_SCHEME'),")],
        &[("MAIL_MAILER", "log"), ("MAIL_HOST", "127.0.0.1")],
    );
    let names: Vec<&str> = modern.env.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "MAIL_MAILER",
            "MAIL_HOST",
            "MAIL_PORT",
            "MAIL_USERNAME",
            "MAIL_PASSWORD",
            "MAIL_SCHEME"
        ]
    );
    assert!(!modern.setup.real_smtp);
    let old = wire(
        &["laravel"],
        &[(
            "config/mail.php",
            "'encryption' => env('MAIL_ENCRYPTION', 'tls'),",
        )],
        &[
            ("MAIL_MAILER", "smtp"),
            ("MAIL_HOST", "smtp.mailgun.org"),
            ("MAIL_PASSWORD", "real"),
        ],
    );
    assert!(old.env.contains(&("MAIL_ENCRYPTION".into(), "null".into())));
    assert!(old.setup.real_smtp);
    assert!(
        old.setup
            .current
            .contains(&"MAIL_HOST=smtp.mailgun.org".to_string())
    );
    assert!(
        !old.setup.current.iter().any(|c| c.contains("real")),
        "passwords aren't shown"
    );
    assert!(
        wire(&["laravel"], &[], &[("MAIL_MAILER", "postmark")])
            .setup
            .real_smtp,
        "an API transport delivers for real"
    );
}

#[test]
fn other_stacks_use_their_names_or_a_snippet() {
    let node = wire(
        &["express"],
        &[(
            ".env.example",
            "SMTP_HOST=\nSMTP_PORT=\nSMTP_USER=\nSMTP_PASS=\nSMTP_SECURE=\n",
        )],
        &[("SMTP_HOST", "smtp.gmail.com")],
    );
    assert_eq!(node.setup.wiring, MailWiring::Automatic);
    assert!(
        node.env.contains(&("SMTP_PORT".into(), "1025".into()))
            && node.env.contains(&("SMTP_SECURE".into(), "false".into()))
    );
    assert!(node.setup.real_smtp);
    let django = wire(&["django"], &[], &[]);
    assert_eq!(django.setup.wiring, MailWiring::Manual);
    assert!(django.setup.snippet.unwrap().contains("EMAIL_PORT = 1025"));
    let wp = wire(&["wordpress", "php"], &[], &[]);
    let scan = wp
        .env
        .iter()
        .find(|(k, _)| k == "PHP_INI_SCAN_DIR")
        .unwrap();
    assert!(
        scan.1.starts_with(':') || cfg!(windows),
        "keeps PHP's default scan dir"
    );
    let ini = std::fs::read_to_string(
        // The list separator is `;` on Windows.
        std::path::Path::new(scan.1.trim_start_matches([':', ';'])).join("zz-locdev-mail.ini"),
    )
    .unwrap();
    assert!(
        ini.contains("sendmail_path = \"'/opt/locdevd' sendmail --site shop\"") || cfg!(windows)
    );
}
