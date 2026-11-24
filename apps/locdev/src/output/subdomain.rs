//! Rendering for `locdev subdomain …` and `locdev route`.

use locdev_core::model::ProjectView;
use locdev_core::subdomains::{HostExplain, SubdomainTarget};

use super::{Output, Style};

impl Output {
    pub fn subdomains(&self, view: &ProjectView) {
        let p = &view.project;
        if self.json {
            self.print_json(&p.subdomains);
            return;
        }
        let domain = &p.domain;
        println!(
            "{:<32} {}",
            format!("*.{domain}"),
            if p.subdomains.wildcard {
                self.paint(Style::Green, "the site (wildcard on)")
            } else {
                self.paint(Style::Dim, "not routed (wildcard off)")
            }
        );
        for rule in &p.subdomains.rules {
            let target = match &rule.target {
                SubdomainTarget::Site => "the site".to_string(),
                SubdomainTarget::Port { port } => format!("127.0.0.1:{port}"),
                SubdomainTarget::Process { id } => match p.processes.get(id).and_then(|x| x.port) {
                    Some(port) => format!("the {id} process (:{port})"),
                    None => format!("the {id} process (no port yet)"),
                },
            };
            println!("{:<32} {target}", format!("{}.{domain}", rule.pattern));
        }
        if p.subdomains.rules.is_empty() {
            println!("{}", self.paint(Style::Dim, "No rules. `locdev subdomain add api --port 9000` sends one subdomain elsewhere."));
        }
    }

    pub fn host_explain(&self, e: &HostExplain) {
        if self.json {
            self.print_json(e);
            return;
        }
        let ok = |good: bool| {
            if good {
                self.paint(Style::Green, "ok  ")
            } else {
                self.paint(Style::Red, "fail")
            }
        };
        println!("{}", self.paint(Style::Bold, &e.host));

        let local = e.dns.answers.iter().any(|a| a == "127.0.0.1" || a == "::1");
        let dns = match (&e.dns.error, e.dns.via_locdev) {
            (Some(error), _) => format!("doesn't resolve: {error}"),
            (None, Some(true)) => {
                format!("{} through LocDev's DNS server", e.dns.answers.join(", "))
            }
            (None, _) if local => format!(
                "{} (not through LocDev's DNS; another tool or the hosts file answers)",
                e.dns.answers.join(", ")
            ),
            (None, _) => format!("{}: not this machine", e.dns.answers.join(", ")),
        };
        println!("  {} dns    {dns}", ok(local));

        let r = &e.route;
        let route = match (&r.problem, &r.site) {
            (Some(problem), _) => problem.clone(),
            (None, Some(site)) => {
                let rule = r
                    .rule
                    .as_deref()
                    .map(|p| format!(" (rule {p})"))
                    .unwrap_or_default();
                // A port target already says its port.
                let port = r
                    .port
                    .filter(|_| !r.target.starts_with("port "))
                    .map(|p| format!(" on 127.0.0.1:{p}"))
                    .unwrap_or_default();
                format!("{site}: {}{port}{rule}", r.target)
            }
            (None, None) => r.target.clone(),
        };
        println!("  {} route  {route}", ok(r.problem.is_none()));
        if r.reachable == Some(false) {
            println!(
                "  {} server nothing answers on that port right now",
                self.paint(Style::Yellow, "warn")
            );
        }

        let https = match e.tls.https_port {
            Some(port) => format!(
                "served on :{port}, certificate for {}",
                e.tls.names.join(", ")
            ),
            None => "HTTPS is off".to_string(),
        };
        println!(
            "  {} https  {https}{}",
            ok(e.tls.https_port.is_some()),
            if e.tls.trusted {
                ""
            } else {
                " (CA not trusted: `locdev tls trust`)"
            }
        );
    }
}
