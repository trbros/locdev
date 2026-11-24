//! YAML → `Workflow`, for opening a file in the visual editor. Anything the
//! model can't hold is reported instead of being dropped, so saving from the
//! visual editor never loses part of a file.

use std::collections::BTreeMap;

use locdev_core::actions::{Concurrency, Filter, Job, Permissions, Step, Triggers, Workflow};
use serde_norway::{Mapping, Value};

type Result<T> = std::result::Result<T, String>;

pub fn parse(value: &Value) -> Result<Workflow> {
    let root = value.as_mapping().ok_or("the file isn't a YAML mapping")?;
    let mut workflow = Workflow::default();
    for (k, v) in root {
        match key_str(k).as_str() {
            "name" => workflow.name = string(v, "name")?,
            "on" => workflow.on = triggers(v)?,
            "permissions" => workflow.permissions = Some(permissions(v)?),
            "concurrency" => workflow.concurrency = Some(concurrency(v)?),
            "env" => workflow.env = string_map(v, "env")?,
            "jobs" => {
                let jobs = v.as_mapping().ok_or("jobs must be a mapping")?;
                for (id, job_value) in jobs {
                    workflow.jobs.push(job(&key_str(id), job_value)?);
                }
            }
            other => return Err(unsupported(other)),
        }
    }
    Ok(workflow)
}

/// `on` is read as `true` by YAML 1.1 parsers; accept both spellings.
pub fn key_str(key: &Value) -> String {
    match key {
        Value::Bool(true) => "on".into(),
        Value::String(s) => s.clone(),
        other => serde_norway::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn unsupported(what: &str) -> String {
    format!("uses `{what}`, which the visual editor doesn't edit")
}

/// Strings, numbers and bools all become the text YAML would show.
fn string(v: &Value, what: &str) -> Result<String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(format!("{what} must be a single value")),
    }
}

fn strings(v: &Value, what: &str) -> Result<Vec<String>> {
    match v {
        Value::Sequence(items) => items.iter().map(|i| string(i, what)).collect(),
        Value::Null => Ok(vec![]),
        other => Ok(vec![string(other, what)?]),
    }
}

fn string_map(v: &Value, what: &str) -> Result<BTreeMap<String, String>> {
    match v {
        Value::Null => Ok(BTreeMap::new()),
        Value::Mapping(m) => m
            .iter()
            .map(|(k, v)| Ok((key_str(k), string(v, what)?)))
            .collect(),
        _ => Err(format!("{what} must be a mapping")),
    }
}

fn mapping<'a>(v: &'a Value, what: &str) -> Result<Option<&'a Mapping>> {
    match v {
        Value::Null => Ok(None),
        Value::Mapping(m) => Ok(Some(m)),
        _ => Err(format!("{what} must be a mapping")),
    }
}

fn triggers(v: &Value) -> Result<Triggers> {
    let mut t = Triggers::default();
    let mut event = |name: &str, body: &Value| -> Result<()> {
        match name {
            "push" => t.push = Some(filter(body, "push")?),
            "pull_request" => t.pull_request = Some(filter(body, "pull_request")?),
            "workflow_dispatch" => {
                if mapping(body, name)?.is_some_and(|m| !m.is_empty()) {
                    return Err(unsupported("workflow_dispatch inputs"));
                }
                t.workflow_dispatch = true;
            }
            "schedule" => {
                for item in body.as_sequence().ok_or("schedule must be a list")? {
                    let cron = item.get("cron").ok_or("each schedule entry needs a cron")?;
                    t.schedule.push(string(cron, "cron")?);
                }
            }
            "release" => {
                let types = body
                    .get("types")
                    .map(|t| strings(t, "release types"))
                    .transpose()?;
                if types.is_some_and(|t| t != ["published"]) {
                    return Err(unsupported("release types other than published"));
                }
                t.release = true;
            }
            other => return Err(unsupported(&format!("on: {other}"))),
        }
        Ok(())
    };
    match v {
        Value::String(name) => event(name, &Value::Null)?,
        Value::Sequence(names) => {
            for name in names {
                event(&string(name, "on")?, &Value::Null)?;
            }
        }
        Value::Mapping(m) => {
            for (k, body) in m {
                event(&key_str(k), body)?;
            }
        }
        _ => return Err("on must name events".into()),
    }
    Ok(t)
}

fn filter(v: &Value, what: &str) -> Result<Filter> {
    let mut f = Filter::default();
    if let Some(m) = mapping(v, what)? {
        for (k, v) in m {
            match key_str(k).as_str() {
                "branches" => f.branches = strings(v, "branches")?,
                "tags" => f.tags = strings(v, "tags")?,
                "paths" => f.paths = strings(v, "paths")?,
                other => return Err(unsupported(&format!("{what}: {other}"))),
            }
        }
    }
    Ok(f)
}

fn permissions(v: &Value) -> Result<Permissions> {
    match v {
        Value::String(s) if s == "read-all" => Ok(Permissions::ReadAll),
        Value::String(s) if s == "write-all" => Ok(Permissions::WriteAll),
        Value::Mapping(_) | Value::Null => Ok(Permissions::Scopes(string_map(v, "permissions")?)),
        _ => Err("permissions must be read-all, write-all or a mapping".into()),
    }
}

fn concurrency(v: &Value) -> Result<Concurrency> {
    match v {
        Value::String(group) => Ok(Concurrency {
            group: group.clone(),
            cancel_in_progress: false,
        }),
        Value::Mapping(m) => {
            let mut c = Concurrency::default();
            for (k, v) in m {
                match key_str(k).as_str() {
                    "group" => c.group = string(v, "group")?,
                    "cancel-in-progress" => {
                        c.cancel_in_progress = v
                            .as_bool()
                            .ok_or_else(|| unsupported("cancel-in-progress as an expression"))?
                    }
                    other => return Err(unsupported(&format!("concurrency: {other}"))),
                }
            }
            Ok(c)
        }
        _ => Err("concurrency must be a string or a mapping".into()),
    }
}

fn job(id: &str, v: &Value) -> Result<Job> {
    let m = v
        .as_mapping()
        .ok_or_else(|| format!("job {id} must be a mapping"))?;
    let mut job = Job {
        id: id.to_string(),
        ..Job::default()
    };
    for (k, v) in m {
        match key_str(k).as_str() {
            "name" => job.name = Some(string(v, "name")?),
            "runs-on" => {
                job.runs_on = string(v, "runs-on")
                    .map_err(|_| unsupported("runs-on with labels or groups"))?
            }
            "needs" => job.needs = strings(v, "needs")?,
            "if" => job.condition = Some(string(v, "if")?),
            "permissions" => job.permissions = Some(permissions(v)?),
            "environment" => {
                job.environment = Some(
                    string(v, "environment").map_err(|_| unsupported("environment with a url"))?,
                )
            }
            "timeout-minutes" => {
                job.timeout_minutes = Some(
                    v.as_u64()
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or_else(|| unsupported("timeout-minutes as an expression"))?,
                )
            }
            "env" => job.env = string_map(v, "env")?,
            "strategy" => {
                let strategy = v.as_mapping().ok_or("strategy must be a mapping")?;
                for (k, v) in strategy {
                    if key_str(k) != "matrix" {
                        return Err(unsupported(&format!("strategy: {}", key_str(k))));
                    }
                    let matrix = v
                        .as_mapping()
                        .ok_or_else(|| unsupported("a matrix from an expression"))?;
                    for (name, values) in matrix {
                        let name = key_str(name);
                        if name == "include" || name == "exclude" || !values.is_sequence() {
                            return Err(unsupported(&format!("matrix: {name}")));
                        }
                        job.matrix.insert(name, strings(values, "matrix")?);
                    }
                }
            }
            "steps" => {
                for s in v.as_sequence().ok_or("steps must be a list")? {
                    job.steps.push(step(s)?);
                }
            }
            "uses" => return Err(unsupported("reusable workflows (jobs.<id>.uses)")),
            other => return Err(unsupported(&format!("jobs.{id}.{other}"))),
        }
    }
    Ok(job)
}

fn step(v: &Value) -> Result<Step> {
    let m = v.as_mapping().ok_or("each step must be a mapping")?;
    let mut step = Step::default();
    for (k, v) in m {
        match key_str(k).as_str() {
            "id" => step.id = Some(string(v, "id")?),
            "name" => step.name = Some(string(v, "name")?),
            "uses" => step.uses = Some(string(v, "uses")?),
            "run" => step.run = Some(string(v, "run")?),
            "if" => step.condition = Some(string(v, "if")?),
            "working-directory" => step.working_directory = Some(string(v, "working-directory")?),
            "with" => step.with = string_map(v, "with")?,
            "env" => step.env = string_map(v, "env")?,
            other => return Err(unsupported(&format!("step {other}"))),
        }
    }
    Ok(step)
}
