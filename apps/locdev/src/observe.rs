//! `locdev metrics` (what the proxy saw for a site) and `locdev tinker`
//! (Laravel Tinker with the site's PHP and environment).

use std::io::{BufRead, IsTerminal, Read, Write};
use std::process::ExitCode;

use anyhow::bail;
use locdev_core::metrics::RequestMetrics;
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::daemon_call;
use crate::output::Output;
use crate::site;

pub async fn metrics(
    paths: &Paths,
    out: &Output,
    site: Option<String>,
    reset: bool,
) -> anyhow::Result<ExitCode> {
    let name = site::resolve(paths, site).await?.project.name;
    if reset {
        daemon_call(paths, Request::ResetRequestMetrics { name: name.clone() }).await?;
        out.message(&format!("request metrics of {name} reset"));
        return Ok(ExitCode::SUCCESS);
    }
    let Response::RequestMetrics(m) = daemon_call(paths, Request::RequestMetrics { name }).await?
    else {
        bail!("unexpected response from daemon");
    };
    if out.is_json() {
        out.print_json(&m);
    } else {
        print_metrics(&m);
    }
    Ok(ExitCode::SUCCESS)
}

fn print_metrics(m: &RequestMetrics) {
    if m.total == 0 {
        println!("{}: no requests through LocDev's proxy yet", m.site);
        return;
    }
    let s = m.status;
    println!("{}  {} requests", m.site, m.total);
    println!(
        "  status   2xx {}  3xx {}  4xx {}  5xx {}  (server down: {})",
        s.success, s.redirect, s.client_error, s.server_error, m.upstream_down
    );
    let l = m.latency;
    println!(
        "  latency  avg {} ms  p50 {} ms  p95 {} ms  p99 {} ms  max {} ms  (last {})",
        l.avg_ms, l.p50_ms, l.p95_ms, l.p99_ms, l.max_ms, l.samples
    );
    if let Some(last) = m.per_minute.last() {
        println!(
            "  minute   {} requests, {} errors in the last minute with traffic",
            last.requests, last.errors
        );
    }
    if !m.top_paths.is_empty() {
        println!("  busiest paths");
        for p in m.top_paths.iter().take(10) {
            println!(
                "    {:>6}  {:<6} {}  avg {} ms{}",
                p.requests,
                p.method,
                p.path,
                p.avg_ms,
                if p.errors > 0 {
                    format!("  {} errors", p.errors)
                } else {
                    String::new()
                }
            );
        }
    }
}

/// Runs `code` (or each line typed, or stdin) in the site's Laravel app.
pub async fn tinker(
    paths: &Paths,
    out: &Output,
    site: Option<String>,
    code: Option<String>,
) -> anyhow::Result<ExitCode> {
    let name = site::resolve(paths, site).await?.project.name;
    let run = |code: String| {
        let name = name.clone();
        async move {
            match daemon_call(paths, Request::TinkerRun { name, code }).await? {
                Response::Tinker(result) => Ok(result),
                other => bail!("unexpected response from daemon: {other:?}"),
            }
        }
    };
    let print = |r: &locdev_core::model::TinkerOutput| {
        if out.is_json() {
            out.print_json(r);
            return;
        }
        print!("{}", r.output);
        if !r.output.ends_with('\n') && !r.output.is_empty() {
            println!();
        }
        if r.timed_out {
            out.note("stopped: took longer than 30 s");
        }
        if r.truncated {
            out.note("output cut at 256 KB");
        }
    };
    if let Some(code) = code {
        let result = run(code).await?;
        print(&result);
        return Ok(if result.code == Some(0) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }
    if !std::io::stdin().is_terminal() {
        let mut code = String::new();
        std::io::stdin().read_to_string(&mut code)?;
        let result = run(code).await?;
        print(&result);
        return Ok(ExitCode::SUCCESS);
    }
    // Each line runs on its own (a fresh app per line): no shared variables.
    out.note(&format!(
        "Tinker for {name}: each line runs on its own; Ctrl+D quits."
    ));
    let stdin = std::io::stdin();
    loop {
        print!(">>> ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            println!();
            return Ok(ExitCode::SUCCESS);
        }
        if line.trim().is_empty() {
            continue;
        }
        match run(line).await {
            Ok(result) => print(&result),
            Err(error) => out.note(&error.to_string()),
        }
    }
}
