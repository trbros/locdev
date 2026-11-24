//! `Workflow` → YAML, written by hand for a stable, readable layout: the key
//! order people expect, flow lists for short ones, block literals for
//! scripts, and quotes only where YAML needs them.

use std::collections::BTreeMap;
use std::fmt::Write;

use locdev_core::actions::{Filter, Job, Permissions, Step, Workflow};

pub fn render(workflow: &Workflow) -> String {
    let mut out = String::new();
    if !workflow.name.is_empty() {
        line(&mut out, 0, "name", &scalar(&workflow.name));
        out.push('\n');
    }

    out.push_str("on:\n");
    let on = &workflow.on;
    if let Some(push) = &on.push {
        filter(&mut out, "push", push);
    }
    if let Some(pr) = &on.pull_request {
        filter(&mut out, "pull_request", pr);
    }
    if on.workflow_dispatch {
        out.push_str("  workflow_dispatch:\n");
    }
    if !on.schedule.is_empty() {
        out.push_str("  schedule:\n");
        for cron in &on.schedule {
            let _ = writeln!(out, "    - cron: {}", scalar(cron));
        }
    }
    if on.release {
        out.push_str("  release:\n    types: [published]\n");
    }

    if let Some(permissions) = &workflow.permissions {
        out.push('\n');
        permissions_block(&mut out, 0, permissions);
    }
    if let Some(c) = &workflow.concurrency {
        out.push_str("\nconcurrency:\n");
        line(&mut out, 1, "group", &scalar(&c.group));
        line(
            &mut out,
            1,
            "cancel-in-progress",
            if c.cancel_in_progress {
                "true"
            } else {
                "false"
            },
        );
    }
    if !workflow.env.is_empty() {
        out.push('\n');
        map(&mut out, 0, "env", &workflow.env);
    }

    out.push_str("\njobs:\n");
    for (i, job) in workflow.jobs.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        job_block(&mut out, job);
    }
    out
}

fn job_block(out: &mut String, job: &Job) {
    let _ = writeln!(out, "  {}:", key(&job.id));
    if let Some(name) = &job.name {
        line(out, 2, "name", &scalar(name));
    }
    line(out, 2, "runs-on", &scalar(&job.runs_on));
    if !job.needs.is_empty() {
        line(out, 2, "needs", &flow(&job.needs));
    }
    if let Some(condition) = &job.condition {
        line(out, 2, "if", &scalar(condition));
    }
    if let Some(permissions) = &job.permissions {
        permissions_block(out, 2, permissions);
    }
    if let Some(environment) = &job.environment {
        line(out, 2, "environment", &scalar(environment));
    }
    if let Some(minutes) = job.timeout_minutes {
        line(out, 2, "timeout-minutes", &minutes.to_string());
    }
    if !job.matrix.is_empty() {
        out.push_str("    strategy:\n      matrix:\n");
        for (name, values) in &job.matrix {
            line(out, 4, name, &flow(values));
        }
    }
    if !job.env.is_empty() {
        map(out, 2, "env", &job.env);
    }
    out.push_str("    steps:\n");
    for step in &job.steps {
        step_block(out, step);
    }
}

fn step_block(out: &mut String, step: &Step) {
    // The first key goes on the dash line; the rest line up under it.
    let mut first = true;
    let mut put = |out: &mut String, k: &str, v: &str| {
        let prefix = if first { "      - " } else { "        " };
        first = false;
        let _ = writeln!(out, "{prefix}{k}: {v}");
    };
    if let Some(name) = &step.name {
        put(out, "name", &scalar(name));
    }
    if let Some(id) = &step.id {
        put(out, "id", &scalar(id));
    }
    if let Some(condition) = &step.condition {
        put(out, "if", &scalar(condition));
    }
    if let Some(uses) = &step.uses {
        put(out, "uses", &scalar(uses));
    }
    if let Some(run) = &step.run {
        if run.contains('\n') {
            put(out, "run", if run.ends_with('\n') { "|" } else { "|-" });
            for l in run.trim_end_matches('\n').lines() {
                if l.is_empty() {
                    out.push('\n');
                } else {
                    let _ = writeln!(out, "          {l}");
                }
            }
        } else {
            put(out, "run", &scalar(run));
        }
    }
    if let Some(dir) = &step.working_directory {
        put(out, "working-directory", &scalar(dir));
    }
    if !step.with.is_empty() {
        put(out, "with", "");
        trim_trailing_space(out);
        for (k, v) in &step.with {
            line(out, 5, k, &scalar(v));
        }
    }
    if !step.env.is_empty() {
        put(out, "env", "");
        trim_trailing_space(out);
        for (k, v) in &step.env {
            line(out, 5, k, &scalar(v));
        }
    }
    if first {
        // A step with nothing in it still needs its dash.
        out.push_str("      - {}\n");
    }
}

fn trim_trailing_space(out: &mut String) {
    if out.ends_with(" \n") {
        out.truncate(out.len() - 2);
        out.push('\n');
    }
}

fn filter(out: &mut String, event: &str, f: &Filter) {
    if f.branches.is_empty() && f.tags.is_empty() && f.paths.is_empty() {
        let _ = writeln!(out, "  {event}:");
        return;
    }
    let _ = writeln!(out, "  {event}:");
    if !f.branches.is_empty() {
        line(out, 2, "branches", &flow(&f.branches));
    }
    if !f.tags.is_empty() {
        line(out, 2, "tags", &flow(&f.tags));
    }
    if !f.paths.is_empty() {
        line(out, 2, "paths", &flow(&f.paths));
    }
}

fn permissions_block(out: &mut String, indent: usize, permissions: &Permissions) {
    let pad = "  ".repeat(indent);
    match permissions {
        Permissions::ReadAll => {
            let _ = writeln!(out, "{pad}permissions: read-all");
        }
        Permissions::WriteAll => {
            let _ = writeln!(out, "{pad}permissions: write-all");
        }
        Permissions::Scopes(scopes) if scopes.is_empty() => {
            let _ = writeln!(out, "{pad}permissions: {{}}");
        }
        Permissions::Scopes(scopes) => map(out, indent, "permissions", scopes),
    }
}

fn map(out: &mut String, indent: usize, name: &str, values: &BTreeMap<String, String>) {
    let _ = writeln!(out, "{}{name}:", "  ".repeat(indent));
    for (k, v) in values {
        line(out, indent + 1, k, &scalar(v));
    }
}

fn line(out: &mut String, indent: usize, k: &str, v: &str) {
    let _ = writeln!(out, "{}{}: {v}", "  ".repeat(indent), key(k));
}

fn key(k: &str) -> String {
    if plain(k) { k.to_string() } else { quote(k) }
}

/// `[a, b]`, quoting items that need it.
fn flow(items: &[String]) -> String {
    let items: Vec<String> = items
        .iter()
        .map(|i| {
            if plain(i) && !i.contains([',', '[', ']', '{', '}']) {
                i.clone()
            } else {
                quote(i)
            }
        })
        .collect();
    format!("[{}]", items.join(", "))
}

/// A scalar as YAML: plain when that reads back as the same string.
pub fn scalar(value: &str) -> String {
    if plain(value) {
        value.to_string()
    } else {
        quote(value)
    }
}

fn quote(value: &str) -> String {
    if value.chars().any(|c| c.is_control()) {
        // Double quotes, the only style with escapes.
        serde_json::to_string(value).unwrap_or_default()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

/// Whether `value` can be written without quotes and still be read back as
/// this exact string (not a bool, number, null, alias, comment…).
fn plain(value: &str) -> bool {
    const RESERVED: &[&str] = &[
        "true", "false", "yes", "no", "on", "off", "y", "n", "null", "~", "",
    ];
    if RESERVED.contains(&value.to_ascii_lowercase().as_str()) {
        return false;
    }
    let first = value.chars().next().unwrap_or(' ');
    if "-?:,[]{}#&*!|>'\"%@`".contains(first) || first.is_whitespace() {
        // `${{` and `- ` are common; `-x` is fine as long as it isn't "- ".
        if !(first == '-' && value.len() > 1 && !value.starts_with("- ")) {
            return false;
        }
    }
    if value.ends_with(char::is_whitespace)
        || value.ends_with(':')
        || value.contains(": ")
        || value.contains(" #")
        || value.chars().any(|c| c.is_control())
    {
        return false;
    }
    // Anything that YAML would read as a number.
    let numeric = value.trim_start_matches(['+', '-']);
    !(numeric.parse::<f64>().is_ok()
        || numeric.starts_with("0x")
        || numeric.starts_with("0o")
        || [".inf", ".nan"].contains(&numeric.to_ascii_lowercase().as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_only_what_yaml_would_misread() {
        assert_eq!(scalar("ubuntu-latest"), "ubuntu-latest");
        assert_eq!(scalar("on"), "'on'");
        assert_eq!(scalar("22"), "'22'");
        assert_eq!(scalar("8.4"), "'8.4'");
        assert_eq!(scalar("**/*.md"), "'**/*.md'");
        assert_eq!(scalar("${{ github.ref }}"), "${{ github.ref }}");
        assert_eq!(scalar("a: b"), "'a: b'");
        assert_eq!(scalar("it's"), "it's");
        assert_eq!(scalar("'x'"), "'''x'''");
        assert_eq!(scalar("0 6 * * 1"), "0 6 * * 1");
        assert_eq!(scalar("--frozen-lockfile"), "--frozen-lockfile");
    }

    #[test]
    fn every_scalar_reads_back_as_itself() {
        for value in [
            "on",
            "22",
            "8.4",
            "**/*.md",
            "${{ x }}",
            "a: b",
            "it's",
            "'x'",
            "- a",
            "#c",
            "a #b",
            "null",
            "~",
            "0x1F",
            ".inf",
            "-",
            "tab\there",
            "x:",
            "@scope/pkg",
            "*",
            "v*",
        ] {
            let yaml = format!("k: {}\n", scalar(value));
            let parsed: BTreeMap<String, String> =
                serde_norway::from_str(&yaml).unwrap_or_else(|e| panic!("{yaml}: {e}"));
            assert_eq!(parsed["k"], value, "{yaml}");
        }
    }
}
