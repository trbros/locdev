//! Small self-contained HTML pages the proxy answers with when it cannot
//! forward a request. They share the desktop app's look.

use crate::{Origin, Route};

pub fn not_running(route: &Route) -> String {
    let name = escape(&route.name);
    let body = format!(
        r#"<p class="lede">The dev server for <code>{name}</code> is not running, so there is nothing to forward to.</p>
<p>Start it from the LocDev app, or run:</p>
<pre>$ locdev up {name}</pre>
<p class="hint">Then reload this page.</p>"#
    );
    layout(
        &format!("{name} is not running"),
        "502 · upstream down",
        &body,
    )
}

pub fn no_port(route: &Route) -> String {
    let name = escape(&route.name);
    let body = format!(
        r#"<p class="lede"><code>{name}</code> has no port configured, so LocDev does not know where to send requests.</p>
<pre>$ locdev edit {name} --port 8100 --command "…"</pre>"#
    );
    layout(&format!("{name} has no port"), "502 · not routable", &body)
}

pub fn subdomain_off(host: &str, route: &Route) -> String {
    let host = escape(host);
    let name = escape(&route.name);
    let body = format!(
        r#"<p class="lede"><code>{host}</code> isn't one of <code>{name}</code>'s subdomains: its wildcard is off and no rule matches.</p>
<pre>$ locdev subdomain add &lt;name&gt; {name}
$ locdev subdomain wildcard on {name}</pre>"#
    );
    layout(
        &format!("{host} isn't configured"),
        "404 · subdomain not configured",
        &body,
    )
}

pub fn sub_no_port(host: &str, label: &str) -> String {
    let host = escape(host);
    let label = escape(label);
    let body = format!(
        r#"<p class="lede"><code>{host}</code> goes to {label}, which has no port yet. Start it once from LocDev so it gets one.</p>"#
    );
    layout(&format!("{host} has no port"), "502 · not routable", &body)
}

pub fn sub_not_running(host: &str, label: &str) -> String {
    let host = escape(host);
    let label = escape(label);
    let body = format!(
        r#"<p class="lede"><code>{host}</code> goes to {label}, but nothing answers there.</p>
<p class="hint">Start it, then reload this page.</p>"#
    );
    layout(
        &format!("{host}: nothing answers"),
        "502 · upstream down",
        &body,
    )
}

pub fn unknown(host: &str, routes: &[Route], suffix: &str, origin: &Origin) -> String {
    let host = escape(host);
    let body = format!(
        r#"<p class="lede">No site is registered as <code>{host}</code>.</p>
<p>Register the project folder with:</p>
<pre>$ locdev add path/to/project</pre>
{}"#,
        site_list(routes, suffix, origin)
    );
    layout("Unknown site", "404 · no such site", &body)
}

pub fn index(routes: &[Route], origin: &Origin) -> String {
    let body = if routes.is_empty() {
        r#"<p class="lede">The LocDev proxy is up, but no sites are registered yet.</p>
<pre>$ locdev add path/to/project --start</pre>"#
            .to_string()
    } else {
        format!(
            r#"<p class="lede">The LocDev proxy is up. Registered sites:</p>{}"#,
            site_list(routes, "localhost", origin)
        )
    };
    layout("LocDev proxy", "200 · index", &body)
}

fn site_list(routes: &[Route], suffix: &str, origin: &Origin) -> String {
    if routes.is_empty() {
        return String::new();
    }
    let items: String = routes
        .iter()
        .map(|r| {
            let host = format!("{}.{suffix}{}", escape(&r.name), origin.port);
            format!(
                r#"<li><a href="{}://{host}/">{host}</a></li>"#,
                origin.scheme
            )
        })
        .collect();
    format!(r#"<ul class="sites">{items}</ul>"#)
}

fn layout(title: &str, tag: &str, body: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · LocDev</title>
<style>
  :root {{ color-scheme: dark light; --bg:#161412; --panel:#1d1a17; --ink:#efe9e1; --muted:#9a9187; --line:#ffffff14; --ember:#f08a4b; }}
  @media (prefers-color-scheme: light) {{ :root {{ --bg:#f7f3ec; --panel:#fffdf9; --ink:#2a241f; --muted:#776e65; --line:#2a241f1a; --ember:#c9581d; }} }}
  * {{ box-sizing: border-box; }}
  body {{ margin:0; min-height:100vh; display:grid; place-items:center; padding:24px; background:var(--bg); color:var(--ink);
         font:15px/1.6 ui-sans-serif, system-ui, sans-serif; }}
  main {{ width:min(560px,100%); border:1px solid var(--line); border-radius:14px; background:var(--panel); overflow:hidden; }}
  header {{ display:flex; align-items:center; gap:10px; padding:14px 20px; border-bottom:1px solid var(--line);
            font:12px ui-monospace, monospace; color:var(--muted); letter-spacing:.04em; }}
  header b {{ color:var(--ember); font-weight:600; }}
  .dot {{ width:8px; height:8px; border-radius:50%; background:var(--ember); box-shadow:0 0 10px var(--ember); }}
  section {{ padding:22px 20px 24px; }}
  h1 {{ margin:0 0 8px; font:600 22px/1.3 ui-monospace, monospace; }}
  .lede {{ color:var(--ink); }} p {{ color:var(--muted); margin:8px 0; }}
  code, pre {{ font-family: ui-monospace, monospace; }}
  code {{ color:var(--ink); }}
  pre {{ margin:12px 0; padding:12px 14px; border-radius:10px; background:var(--bg); border:1px solid var(--line); overflow:auto; }}
  .hint {{ font-size:13px; }}
  .sites {{ list-style:none; padding:0; margin:12px 0 0; display:grid; gap:6px; }}
  .sites a {{ display:block; padding:8px 12px; border:1px solid var(--line); border-radius:8px; color:var(--ink);
              text-decoration:none; font:14px ui-monospace, monospace; }}
  .sites a:hover {{ border-color:var(--ember); color:var(--ember); }}
</style>
</head>
<body>
<main>
  <header><span class="dot"></span><b>locdev</b> proxy · {tag}</header>
  <section><h1>{title}</h1>{body}</section>
</main>
</body>
</html>"#
    )
}

fn escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_hosts() {
        let origin = Origin {
            scheme: "http",
            port: String::new(),
        };
        let page = unknown("<script>x</script>.test", &[], "test", &origin);
        assert!(!page.contains("<script>x"));
        assert!(page.contains("&lt;script&gt;"));
    }
}
