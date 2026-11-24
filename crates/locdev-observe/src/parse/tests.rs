use super::*;

fn feed(runtime: Option<&str>, text: &str) -> Vec<Event> {
    let mut a = Assembler::new(runtime.map(str::to_string), None, EventSeverity::Warn);
    let mut out = Vec::new();
    for line in text.lines() {
        out.extend(a.push(Line {
            text: line,
            at: 1,
            level: None,
        }));
    }
    out.extend(a.flush());
    out
}

#[test]
fn php_uncaught_exception_with_stack() {
    let out = feed(
        Some("php"),
        "PHP Fatal error:  Uncaught RuntimeException: Boom in /app/routes/web.php:7
Stack trace:
#0 /app/vendor/laravel/framework/src/Illuminate/Routing/Route.php(205): Illuminate\\Routing\\RouteFileRegistrar->{closure}()
#1 /app/public/index.php(17): Illuminate\\Foundation\\Application->handleRequest()
#2 {main}
  thrown in /app/routes/web.php on line 7
Server running",
    );
    assert_eq!(out.len(), 1, "{out:#?}");
    let e = &out[0];
    assert_eq!(e.kind, EventKind::Exception);
    assert_eq!(e.code.as_deref(), Some("RuntimeException"));
    assert_eq!(e.message, "Boom");
    assert_eq!(
        (e.file.as_deref(), e.line),
        (Some("/app/routes/web.php"), Some(7))
    );
    assert_eq!(e.stack.len(), 2);
    assert!(e.stack[0].vendor && !e.stack[1].vendor);
    assert_eq!(e.stack[1].line, Some(17));
    assert!(e.raw.contains("thrown in"));
}

#[test]
fn php_warnings_and_server_errors() {
    let out = feed(
        Some("php"),
        "PHP Warning:  Undefined variable $x in /app/index.php on line 3
[Thu Nov  5 10:00:00 2026] 127.0.0.1:54321 [200]: GET /ok
[Thu Nov  5 10:00:01 2026] 127.0.0.1:54322 [500]: POST /oops",
    );
    assert_eq!(out.len(), 2, "{out:#?}");
    assert_eq!(out[0].severity, EventSeverity::Warn);
    assert_eq!(out[0].message, "Undefined variable $x");
    assert_eq!(out[0].line, Some(3));
    assert_eq!(out[1].kind, EventKind::Http);
    let req = out[1].request.as_ref().unwrap();
    assert_eq!(
        (req.method.as_deref(), req.url.as_deref(), req.status),
        (Some("POST"), Some("/oops"), Some(500))
    );
}

#[test]
fn node_errors_keep_their_frames() {
    let out = feed(
        Some("node"),
        "TypeError: Cannot read properties of undefined (reading 'id')
    at handler (/app/src/server.js:12:20)
    at Layer.handle [as handle_request] (/app/node_modules/express/lib/router/layer.js:95:5)
listening on 3000
Error: just a message",
    );
    assert_eq!(out.len(), 2, "{out:#?}");
    let e = &out[0];
    assert_eq!(e.code.as_deref(), Some("TypeError"));
    assert_eq!(
        (e.file.as_deref(), e.line),
        (Some("/app/src/server.js"), Some(12))
    );
    assert_eq!(e.stack[0].function.as_deref(), Some("handler"));
    assert_eq!(e.stack[0].column, Some(20));
    assert!(e.stack[1].vendor);
    assert_eq!(
        out[1].kind,
        EventKind::Log,
        "no stack: an error line, not an exception"
    );
}

#[test]
fn python_traceback() {
    let out = feed(
        Some("python"),
        "Traceback (most recent call last):
  File \"/app/.venv/lib/python3.12/site-packages/flask/app.py\", line 880, in full_dispatch_request
    rv = self.dispatch_request()
  File \"/app/app.py\", line 10, in index
    return 1 / 0
ZeroDivisionError: division by zero",
    );
    assert_eq!(out.len(), 1);
    let e = &out[0];
    assert_eq!(e.code.as_deref(), Some("ZeroDivisionError"));
    assert_eq!(e.message, "division by zero");
    assert_eq!((e.file.as_deref(), e.line), (Some("/app/app.py"), Some(10)));
    assert_eq!(
        e.stack[0].function.as_deref(),
        Some("index"),
        "innermost first"
    );
    assert!(e.stack[1].vendor);
}

#[test]
fn rust_panics_and_tracing() {
    let out = feed(
        Some("rust"),
        "thread 'main' panicked at src/main.rs:10:5:
called `Option::unwrap()` on a `None` value
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
2026-11-05T10:00:00.123456Z ERROR api::db: connection refused
2026-11-05T10:00:01.000000Z  INFO api: listening",
    );
    assert_eq!(out.len(), 2, "{out:#?}");
    assert_eq!(
        out[0].message,
        "called `Option::unwrap()` on a `None` value"
    );
    assert_eq!(
        (out[0].file.as_deref(), out[0].line),
        (Some("src/main.rs"), Some(10))
    );
    assert_eq!(out[1].severity, EventSeverity::Error);
    assert_eq!(out[1].message, "connection refused");
    assert_eq!(
        out[1].context.get("target").map(String::as_str),
        Some("api::db")
    );
    let old = feed(
        Some("rust"),
        "thread 'main' panicked at 'boom', src/lib.rs:3:9",
    );
    assert_eq!((old[0].message.as_str(), old[0].line), ("boom", Some(3)));
}

#[test]
fn build_errors() {
    let out = feed(
        Some("node"),
        "\u{1b}[31m[vite] Internal server error: Failed to resolve import \"./missing\" from \"src/main.ts\". Does the file exist?\u{1b}[39m
  Plugin: vite:import-analysis
  File: /app/src/main.ts:3:7
ready
src/a.ts(3,7): error TS2322: Type 'string' is not assignable to type 'number'.",
    );
    assert_eq!(out.len(), 2, "{out:#?}");
    assert_eq!(out[0].kind, EventKind::Build);
    assert_eq!(
        (out[0].file.as_deref(), out[0].line),
        (Some("/app/src/main.ts"), Some(3))
    );
    assert_eq!(out[1].code.as_deref(), Some("TS2322"));
    assert_eq!(out[1].line, Some(3));
}

#[test]
fn plain_lines_need_a_level() {
    let mut a = Assembler::new(None, None, EventSeverity::Warn);
    assert!(
        a.push(Line {
            text: "GET / 200",
            at: 1,
            level: Some(EventSeverity::Info)
        })
        .is_empty()
    );
    let out = a.push(Line {
        text: "[WARN] cache miss",
        at: 1,
        level: Some(EventSeverity::Warn),
    });
    assert_eq!(out[0].severity, EventSeverity::Warn);
    assert!(feed(Some("python"), "ERROR:app.views:payment failed")[0].message == "payment failed");
}

#[test]
fn php_built_in_server_errors() {
    let out = feed(
        Some("php"),
        "[Thu Nov  5 01:17:21 2026] 127.0.0.1:53150 [200]: GET /?x=boom - Uncaught RuntimeException: Order 42 failed in /site/index.php:4
Stack trace:
#0 {main}
  thrown in /site/index.php on line 4
[Thu Nov  5 01:17:21 2026] 127.0.0.1:53150 Closing",
    );
    assert_eq!(out.len(), 1, "{out:#?}");
    let e = &out[0];
    assert_eq!(e.kind, EventKind::Exception);
    assert_eq!(e.code.as_deref(), Some("RuntimeException"));
    assert_eq!(e.message, "Order 42 failed");
    assert_eq!(
        (e.file.as_deref(), e.line),
        (Some("/site/index.php"), Some(4))
    );
    assert_eq!(e.request.as_ref().unwrap().url.as_deref(), Some("/?x=boom"));
}
