//! Checks on workflow YAML: structure, then security (script injection,
//! `pull_request_target`, token permissions, unpinned actions) and outdated
//! actions. actionlint, when installed, adds its own findings.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use locdev_core::actions::{Finding, FindingLevel, Validation};
use serde_norway::Value;

use crate::parse::{self, key_str};
use crate::versions::{Versions, major_number};

/// Expressions an outsider controls (a PR title, a branch name, a comment…).
/// Interpolated into `run:`, they become shell code.
const UNTRUSTED: &[&str] = &[
    "github.event.issue.title",
    "github.event.issue.body",
    "github.event.pull_request.title",
    "github.event.pull_request.body",
    "github.event.pull_request.head.ref",
    "github.event.pull_request.head.label",
    "github.event.pull_request.head.repo.default_branch",
    "github.event.comment.body",
    "github.event.review.body",
    "github.event.review_comment.body",
    "github.event.discussion.title",
    "github.event.discussion.body",
    "github.event.head_commit.message",
    "github.event.head_commit.author.email",
    "github.event.head_commit.author.name",
    "github.event.commits",
    "github.event.pages",
    "github.event.workflow_run.head_branch",
    "github.event.workflow_run.head_commit.message",
    "github.event.workflow_run.head_commit.author",
    "github.head_ref",
];

struct Checker<'a> {
    text: &'a str,
    findings: Vec<Finding>,
}

impl Checker<'_> {
    fn add(&mut self, level: FindingLevel, rule: &str, message: String, near: Option<&str>) {
        let line = near.and_then(|needle| line_of(self.text, needle));
        self.findings.push(Finding {
            level,
            rule: rule.into(),
            message,
            line,
            source: "locdev".into(),
            fix: None,
        });
    }

    /// A community practice that isn't followed, with the YAML that fixes it.
    fn suggest(
        &mut self,
        level: FindingLevel,
        rule: &str,
        message: String,
        near: Option<&str>,
        fix: &str,
    ) {
        self.add(level, rule, message, near);
        if let Some(last) = self.findings.last_mut() {
            last.fix = Some(fix.to_string());
        }
    }
}

/// The first line containing `needle` (findings point at the YAML; the
/// parser doesn't keep positions).
fn line_of(text: &str, needle: &str) -> Option<u32> {
    let first = needle.lines().next().unwrap_or(needle).trim();
    if first.is_empty() {
        return None;
    }
    text.lines()
        .position(|l| l.contains(first))
        .map(|i| i as u32 + 1)
}

/// Validates YAML text. `actionlint` is used when found on `path_var`.
pub fn validate(text: &str, versions: &Versions, path_var: Option<&str>) -> Validation {
    let mut checker = Checker {
        text,
        findings: Vec::new(),
    };
    let value: Value = match serde_norway::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            checker.findings.push(Finding {
                level: FindingLevel::Error,
                rule: "syntax".into(),
                message: e.to_string(),
                line: e.location().map(|l| l.line() as u32),
                source: "locdev".into(),
                fix: None,
            });
            return Validation {
                findings: checker.findings,
                unsupported: Some("the YAML doesn't parse".into()),
                ..Validation::default()
            };
        }
    };
    structure(&mut checker, &value);
    security(&mut checker, &value);
    actions(&mut checker, &value, versions);
    practices(&mut checker, &value);

    let mut validation = Validation::default();
    match parse::parse(&value) {
        Ok(workflow) => validation.workflow = Some(workflow),
        Err(reason) => validation.unsupported = Some(reason),
    }
    if let Some(path_var) = path_var
        && let Some(program) = find(path_var, "actionlint")
    {
        match actionlint(&program, text) {
            Ok((version, findings)) => {
                validation.actionlint = Some(version);
                checker.findings.extend(findings);
            }
            Err(e) => checker.add(
                FindingLevel::Info,
                "actionlint",
                format!("actionlint didn't run: {e}"),
                None,
            ),
        }
    }
    checker
        .findings
        .sort_by(|a, b| b.level.cmp(&a.level).then(a.line.cmp(&b.line)));
    validation.findings = checker.findings;
    validation
}

fn structure(c: &mut Checker, value: &Value) {
    let Some(root) = value.as_mapping() else {
        c.add(
            FindingLevel::Error,
            "structure",
            "A workflow is a YAML mapping (name:, on:, jobs:).".into(),
            None,
        );
        return;
    };
    let get = |key: &str| root.iter().find(|(k, _)| key_str(k) == key).map(|(_, v)| v);
    if get("on").is_none() {
        c.add(
            FindingLevel::Error,
            "structure",
            "No `on:`: the workflow never runs.".into(),
            None,
        );
    }
    let Some(jobs) = get("jobs")
        .and_then(Value::as_mapping)
        .filter(|j| !j.is_empty())
    else {
        c.add(
            FindingLevel::Error,
            "structure",
            "No jobs.".into(),
            Some("jobs:"),
        );
        return;
    };
    if let Some(on) = get("on").and_then(Value::as_mapping) {
        for (event, body) in on {
            if key_str(event) != "schedule" {
                continue;
            }
            for cron in body
                .as_sequence()
                .into_iter()
                .flatten()
                .filter_map(|s| s.get("cron")?.as_str())
            {
                if cron.split_whitespace().count() != 5 {
                    c.add(FindingLevel::Error, "schedule", format!("`{cron}` isn't a cron expression (5 fields: minute hour day month weekday)."), Some(cron));
                }
            }
        }
    }

    let ids: BTreeSet<String> = jobs.keys().map(key_str).collect();
    let mut needs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, job) in jobs {
        let id = key_str(id);
        let valid = id
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !valid {
            c.add(
                FindingLevel::Error,
                "job-id",
                format!(
                    "Job id `{id}`: use letters, digits, - and _, starting with a letter or _."
                ),
                Some(&format!("{id}:")),
            );
        }
        let Some(job) = job.as_mapping() else {
            c.add(
                FindingLevel::Error,
                "structure",
                format!("Job `{id}` must be a mapping."),
                Some(&format!("{id}:")),
            );
            continue;
        };
        let field = |key: &str| job.iter().find(|(k, _)| key_str(k) == key).map(|(_, v)| v);
        let reusable = field("uses").is_some();
        if !reusable && field("runs-on").is_none() {
            c.add(
                FindingLevel::Error,
                "structure",
                format!("Job `{id}` has no `runs-on`."),
                Some(&format!("{id}:")),
            );
        }
        let steps = field("steps").and_then(Value::as_sequence);
        if !reusable && steps.is_none_or(|s| s.is_empty()) {
            c.add(
                FindingLevel::Error,
                "structure",
                format!("Job `{id}` has no steps."),
                Some(&format!("{id}:")),
            );
        }
        for (i, step) in steps.into_iter().flatten().enumerate() {
            let has = |key: &str| step.get(key).is_some();
            if has("uses") == has("run") {
                c.add(
                    FindingLevel::Error,
                    "step",
                    format!(
                        "Step {} of `{id}` needs exactly one of `uses` or `run`.",
                        i + 1
                    ),
                    step.get("name").and_then(Value::as_str),
                );
            }
        }
        let list: Vec<String> = match field("needs") {
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Sequence(s)) => s
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => vec![],
        };
        for need in &list {
            if !ids.contains(need) {
                c.add(
                    FindingLevel::Error,
                    "needs",
                    format!("`{id}` needs `{need}`, which isn't a job here."),
                    Some(need),
                );
            }
        }
        needs.insert(id, list);
    }
    if let Some(cycle) = find_cycle(&needs) {
        c.add(
            FindingLevel::Error,
            "needs",
            format!("Jobs wait on each other in a loop: {}.", cycle.join(" → ")),
            None,
        );
    }
}

fn find_cycle(needs: &BTreeMap<String, Vec<String>>) -> Option<Vec<String>> {
    fn visit(
        id: &str,
        needs: &BTreeMap<String, Vec<String>>,
        path: &mut Vec<String>,
        done: &mut BTreeSet<String>,
    ) -> Option<Vec<String>> {
        if let Some(at) = path.iter().position(|p| p == id) {
            let mut cycle = path[at..].to_vec();
            cycle.push(id.to_string());
            return Some(cycle);
        }
        if done.contains(id) {
            return None;
        }
        path.push(id.to_string());
        for next in needs.get(id).into_iter().flatten() {
            if let Some(cycle) = visit(next, needs, path, done) {
                return Some(cycle);
            }
        }
        path.pop();
        done.insert(id.to_string());
        None
    }
    let mut done = BTreeSet::new();
    needs
        .keys()
        .find_map(|id| visit(id, needs, &mut Vec::new(), &mut done))
}

/// Every step of every job, with its job id.
fn steps(value: &Value) -> Vec<(String, &Value)> {
    value
        .get("jobs")
        .and_then(Value::as_mapping)
        .into_iter()
        .flatten()
        .flat_map(|(id, job)| {
            let id = key_str(id);
            job.get("steps")
                .and_then(Value::as_sequence)
                .into_iter()
                .flatten()
                .map(move |s| (id.clone(), s))
        })
        .collect()
}

fn events(value: &Value) -> BTreeSet<String> {
    let on = value
        .as_mapping()
        .and_then(|m| m.iter().find(|(k, _)| key_str(k) == "on").map(|(_, v)| v));
    match on {
        Some(Value::String(s)) => [s.clone()].into(),
        Some(Value::Sequence(s)) => s
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(Value::Mapping(m)) => m.keys().map(key_str).collect(),
        _ => BTreeSet::new(),
    }
}

fn security(c: &mut Checker, value: &Value) {
    // Script injection: untrusted text pasted into a shell script.
    for (job, step) in steps(value) {
        let Some(run) = step.get("run").and_then(Value::as_str) else {
            continue;
        };
        for expression in expressions(run) {
            if let Some(source) = UNTRUSTED.iter().find(|u| expression.contains(*u)) {
                c.add(
                    FindingLevel::Error,
                    "injection",
                    format!(
                        "`{source}` is pasted into a shell script in `{job}`: anyone who controls it can run commands. Pass it through `env:` and use \"$VAR\" in the script."
                    ),
                    Some(&expression),
                );
            }
            if expression.contains("secrets.") {
                c.add(
                    FindingLevel::Info,
                    "secret-in-run",
                    format!("A secret is interpolated into a script in `{job}`; pass it through `env:` so it isn't part of the command line."),
                    Some(&expression),
                );
            }
        }
    }

    let events = events(value);
    if events.contains("pull_request_target") || events.contains("workflow_run") {
        let trigger = if events.contains("pull_request_target") {
            "pull_request_target"
        } else {
            "workflow_run"
        };
        let checks_out_untrusted = steps(value).iter().any(|(_, s)| {
            s.get("uses")
                .and_then(Value::as_str)
                .is_some_and(|u| u.starts_with("actions/checkout"))
                && s.get("with")
                    .and_then(|w| w.get("ref"))
                    .and_then(Value::as_str)
                    .is_some_and(|r| r.contains("head") || r.contains("merge"))
        });
        if checks_out_untrusted {
            c.add(
                FindingLevel::Error,
                "untrusted-checkout",
                format!("`{trigger}` runs with secrets and a write token, and this workflow checks out the pull request's code: a fork can run anything with them. Use `pull_request` instead."),
                Some(trigger),
            );
        } else {
            c.add(
                FindingLevel::Warning,
                "privileged-trigger",
                format!("`{trigger}` runs with secrets even for forks. Make sure no step runs code from the pull request."),
                Some(trigger),
            );
        }
    }

    // GITHUB_TOKEN permissions.
    let top = value.get("permissions");
    let jobs = value.get("jobs").and_then(Value::as_mapping);
    let every_job_scoped =
        jobs.is_some_and(|j| j.values().all(|job| job.get("permissions").is_some()));
    if top.is_none() && !every_job_scoped {
        c.add(
            FindingLevel::Warning,
            "permissions",
            "No `permissions:`: the token gets the repository's default, which can be write access to everything. Start from `contents: read` and add what jobs need.".into(),
            None,
        );
    }
    let write_all = |v: Option<&Value>| v.and_then(Value::as_str) == Some("write-all");
    if write_all(top)
        || jobs
            .into_iter()
            .flatten()
            .any(|(_, j)| write_all(j.get("permissions")))
    {
        c.add(
            FindingLevel::Warning,
            "permissions",
            "`write-all` gives every job write access to everything; list only the scopes needed."
                .into(),
            Some("write-all"),
        );
    }
}

/// `${{ … }}` expressions in `text`.
fn expressions(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("${{") {
        let Some(end) = rest[start..].find("}}") else {
            break;
        };
        out.push(rest[start..start + end + 2].to_string());
        rest = &rest[start + end + 2..];
    }
    out
}

fn actions(c: &mut Checker, value: &Value, versions: &Versions) {
    let mut seen = BTreeSet::new();
    for (job, step) in steps(value) {
        let Some(uses) = step.get("uses").and_then(Value::as_str) else {
            continue;
        };
        if uses.starts_with("./") || uses.starts_with("docker://") || !seen.insert(uses.to_string())
        {
            continue;
        }
        let Some((action, reference)) = uses.split_once('@') else {
            c.add(
                FindingLevel::Error,
                "unpinned-action",
                format!("`{uses}` in `{job}` has no version: add @v… (or a commit SHA)."),
                Some(uses),
            );
            continue;
        };
        if ["main", "master", "HEAD", "latest"].contains(&reference) {
            c.add(FindingLevel::Warning, "unpinned-action", format!("`{uses}` follows a branch: any push to it changes what runs here. Use a release tag or a commit SHA."), Some(uses));
            continue;
        }
        // `github/codeql-action/init` → `github/codeql-action`.
        let repo: String = action.split('/').take(2).collect::<Vec<_>>().join("/");
        let sha = reference.len() == 40 && reference.chars().all(|c| c.is_ascii_hexdigit());
        if let (Some(current), Some(used)) = (versions.get(&repo), major_number(reference))
            && let Some(latest) = major_number(current)
            && used < latest
        {
            c.add(
                FindingLevel::Warning,
                "outdated-action",
                format!("`{uses}`: {repo} is at {current}."),
                Some(uses),
            );
        }
        let owner = repo.split('/').next().unwrap_or_default();
        if !sha && !["actions", "github"].contains(&owner) {
            c.add(
                FindingLevel::Info,
                "third-party-action",
                format!("`{repo}` is a third-party action pinned to a tag, which its owner can move. Pinning the commit SHA is safer."),
                Some(uses),
            );
        }
    }
}

// ── Community practices ─────────────────────────────────────────────────────
// What GitHub's docs and the usual CI hardening guides recommend, to avoid
// stuck runs, wasted minutes and "command not found" on the runner. They
// are suggestions (info/warning), never errors.

/// Tools a `run:` may call that GitHub's hosted runners don't have, and the
/// setup action that installs them.
const SETUP: &[(&str, &str, &str)] = &[
    ("pnpm", "pnpm/action-setup", "- uses: pnpm/action-setup@v4"),
    ("bun", "oven-sh/setup-bun", "- uses: oven-sh/setup-bun@v2"),
    (
        "deno",
        "denoland/setup-deno",
        "- uses: denoland/setup-deno@v2",
    ),
];

fn uses_of(value: &Value) -> Vec<String> {
    steps(value)
        .into_iter()
        .filter_map(|(_, s)| s.get("uses")?.as_str().map(str::to_string))
        .collect()
}

/// Whether a `run:` script calls `tool` as a command.
fn calls(script: &str, tool: &str) -> bool {
    script.lines().any(|line| {
        line.split(|c: char| c.is_whitespace() || c == ';' || c == '&' || c == '|' || c == '(')
            .any(|word| word == tool)
    })
}

fn practices(c: &mut Checker, value: &Value) {
    let Some(jobs) = value.get("jobs").and_then(Value::as_mapping) else {
        return;
    };
    let events = events(value);
    let has_concurrency = value.get("concurrency").is_some();
    if !has_concurrency && events.contains("pull_request") {
        c.suggest(
            FindingLevel::Info,
            "practice-concurrency",
            "No `concurrency:`: every push to a pull request starts another run while the previous ones keep going. Cancelling superseded runs saves minutes and avoids racing deploys.".into(),
            Some("jobs:"),
            "concurrency:\n  group: ${{ github.workflow }}-${{ github.ref }}\n  cancel-in-progress: true",
        );
    }
    for (id, job) in jobs {
        let id = key_str(id);
        // Reusable workflow calls have no runner of their own.
        if job.get("uses").is_some() {
            continue;
        }
        if job.get("timeout-minutes").is_none() {
            c.suggest(
                FindingLevel::Info,
                "practice-timeout",
                format!("Job `{id}` has no `timeout-minutes`: a hung step runs for GitHub's default of 6 hours."),
                Some(&format!("{id}:")),
                "timeout-minutes: 15",
            );
        }
    }
    let uses = uses_of(value);
    let used = |action: &str| uses.iter().any(|u| u.split('@').next() == Some(action));
    for (job, step) in steps(value) {
        if let Some(script) = step.get("run").and_then(Value::as_str) {
            for (tool, action, fix) in SETUP {
                if calls(script, tool) && !used(action) {
                    c.suggest(
                        FindingLevel::Warning,
                        "missing-setup",
                        format!("`{tool}` runs in `{job}` but isn't on GitHub's runners: add {action} before it."),
                        Some(script),
                        fix,
                    );
                }
            }
            if script
                .lines()
                .any(|l| l.trim() == "npm install" || l.trim() == "npm i")
            {
                c.suggest(
                    FindingLevel::Info,
                    "practice-npm-ci",
                    "`npm install` can change the lockfile on the runner; `npm ci` installs exactly what package-lock.json says (and fails when it's out of date).".into(),
                    Some(script),
                    "- run: npm ci",
                );
            }
            if script.contains("composer install") && !script.contains("--no-interaction") {
                c.suggest(
                    FindingLevel::Info,
                    "practice-composer",
                    "`composer install` on CI should be non-interactive and quiet about progress."
                        .into(),
                    Some(script),
                    "- run: composer install --no-interaction --no-progress --prefer-dist",
                );
            }
        }
        let Some(uses) = step.get("uses").and_then(Value::as_str) else {
            continue;
        };
        let action = uses.split('@').next().unwrap_or(uses);
        let with = step.get("with");
        let has = |key: &str| with.and_then(|w| w.get(key)).is_some();
        match action {
            "actions/setup-node" if !has("cache") => c.suggest(
                FindingLevel::Info,
                "practice-cache",
                format!("`{uses}` in `{job}` without `cache:` downloads every dependency on each run."),
                Some(uses),
                "with:\n  node-version: 22\n  cache: npm # or pnpm, yarn",
            ),
            "actions/setup-python" if !has("cache") => c.suggest(
                FindingLevel::Info,
                "practice-cache",
                format!("`{uses}` in `{job}` without `cache:` reinstalls every package on each run."),
                Some(uses),
                "with:\n  cache: pip",
            ),
            "actions/checkout" if !has("persist-credentials") && !events.contains("release") => c.suggest(
                FindingLevel::Info,
                "practice-credentials",
                format!("`{uses}` in `{job}` leaves the job's token in .git/config for every later step; turn it off unless a step pushes."),
                Some(uses),
                "with:\n  persist-credentials: false",
            ),
            _ => {}
        }
    }
}

// ── actionlint ──────────────────────────────────────────────────────────────

fn find(path_var: &str, program: &str) -> Option<std::path::PathBuf> {
    let name = if cfg!(windows) {
        format!("{program}.exe")
    } else {
        program.to_string()
    };
    std::env::split_paths(path_var)
        .map(|dir| dir.join(&name))
        .find(|candidate| candidate.is_file())
}

/// Runs actionlint on `text` (from stdin, so nothing is written).
fn actionlint(program: &Path, text: &str) -> Result<(String, Vec<Finding>), String> {
    let version = locdev_core::process::command(program)
        .arg("-version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(str::to_string))
        .unwrap_or_else(|| "actionlint".into());
    let mut child = locdev_core::process::command(program)
        .args(["-format", "{{json .}}", "-no-color", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .ok_or("no stdin")?
        .write_all(text.as_bytes())
        .map_err(|e| e.to_string())?;
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("timed out".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    Ok((
        version,
        parse_actionlint(&String::from_utf8_lossy(&output.stdout))?,
    ))
}

/// actionlint's `{{json .}}` output: a list of errors.
pub fn parse_actionlint(json: &str) -> Result<Vec<Finding>, String> {
    if json.trim().is_empty() || json.trim() == "null" {
        return Ok(vec![]);
    }
    let errors: Vec<serde_json::Value> =
        serde_json::from_str(json.trim()).map_err(|e| format!("unexpected output: {e}"))?;
    Ok(errors
        .iter()
        .map(|e| Finding {
            fix: None,
            level: FindingLevel::Error,
            rule: e["kind"].as_str().unwrap_or("actionlint").to_string(),
            message: e["message"].as_str().unwrap_or_default().to_string(),
            line: e["line"].as_u64().map(|l| l as u32),
            source: "actionlint".into(),
        })
        .collect())
}
