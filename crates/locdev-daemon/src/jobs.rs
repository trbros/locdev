//! The persistent job queue every background task goes through: dependency
//! checks, updates and runtime installs. Kept in `state/jobs.json` and
//! saved on every change (written atomically), so a restart, a crash or a
//! power cut loses no queued work: jobs that were running are queued again.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use locdev_core::deps::{Job, JobKind, JobProgress, JobStatus};
use locdev_core::model::now_ms;
use locdev_core::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::fsutil::write_atomic;

const FORMAT: u32 = 1;
/// Finished jobs kept for the history.
const HISTORY: usize = 200;
/// Lines of log kept per job.
const LOG_LINES: usize = 400;
const BACKOFF_BASE_MS: u64 = 30_000;
const BACKOFF_MAX_MS: u64 = 3_600_000;

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoreFile {
    format: u32,
    next_id: u64,
    jobs: Vec<Job>,
}

pub struct Jobs {
    path: PathBuf,
    file: Mutex<StoreFile>,
}

/// A job to queue.
pub struct Spec {
    pub kind: JobKind,
    pub project: Option<String>,
    pub input: serde_json::Value,
    /// Another active job with this key makes this a no-op.
    pub dedup: String,
    pub automatic: bool,
    pub max_attempts: u32,
    /// Not before this (ms); `None` runs as soon as possible.
    pub not_before: Option<u64>,
}

/// How a run ended.
pub enum Outcome {
    Completed(String),
    /// `retry` when trying again later could help (network, rate limit).
    Failed {
        error: String,
        retry: bool,
        after_ms: Option<u64>,
    },
    Cancelled,
}

impl Jobs {
    /// Loads the queue. Jobs a previous daemon was running are queued again
    /// (counted as an attempt), or failed once out of attempts.
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let mut file = match std::fs::read(path) {
            Ok(raw) => match serde_json::from_slice::<StoreFile>(&raw) {
                Ok(file) if file.format <= FORMAT => file,
                Ok(file) => {
                    return Err(AppError::new(
                        ErrorCode::Config,
                        format!(
                            "{}: written by a newer LocDev (format {})",
                            path.display(),
                            file.format
                        ),
                    ));
                }
                Err(error) => {
                    warn!(%error, "job queue file is damaged; keeping a copy and starting empty");
                    let _ = std::fs::rename(path, path.with_extension("json.broken"));
                    StoreFile::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => StoreFile::default(),
            Err(e) => return Err(e.into()),
        };
        let now = now_ms();
        for job in file
            .jobs
            .iter_mut()
            .filter(|j| j.status == JobStatus::Running)
        {
            if job.attempts >= job.max_attempts {
                job.status = JobStatus::Failed;
                job.finished_at = Some(now);
                job.error = Some("interrupted by a daemon restart, out of attempts".into());
            } else {
                job.status = JobStatus::Pending;
                job.scheduled_at = now;
                job.progress = None;
                push_log(job, "interrupted by a daemon restart; queued again".into());
            }
        }
        let jobs = Self {
            path: path.to_path_buf(),
            file: Mutex::new(file),
        };
        jobs.save(&jobs.file.lock().unwrap());
        Ok(jobs)
    }

    fn save(&self, file: &StoreFile) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = StoreFile {
            format: FORMAT,
            next_id: file.next_id,
            jobs: file.jobs.clone(),
        };
        match serde_json::to_vec_pretty(&file) {
            Ok(raw) => {
                if let Err(error) = write_atomic(&self.path, &raw) {
                    warn!(%error, "could not save the job queue");
                }
            }
            Err(error) => warn!(%error, "could not serialize the job queue"),
        }
    }

    /// Queues a job, or returns the active one with the same key.
    pub fn enqueue(&self, spec: Spec) -> (Job, bool) {
        let mut file = self.file.lock().unwrap();
        if let Some(active) = file
            .jobs
            .iter()
            .find(|j| j.dedup == spec.dedup && !j.status.finished())
        {
            return (active.clone(), false);
        }
        file.next_id += 1;
        let now = now_ms();
        let job = Job {
            id: file.next_id,
            kind: spec.kind,
            project: spec.project,
            status: JobStatus::Pending,
            automatic: spec.automatic,
            created_at: now,
            scheduled_at: spec.not_before.unwrap_or(now),
            started_at: None,
            finished_at: None,
            attempts: 0,
            max_attempts: spec.max_attempts.max(1),
            progress: None,
            result: None,
            error: None,
            log: Vec::new(),
            input: spec.input,
            dedup: spec.dedup,
        };
        file.jobs.push(job.clone());
        self.save(&file);
        (job, true)
    }

    /// Takes the next due job, if fewer than `limit` are running.
    pub fn take_due(&self, now: u64, limit: usize) -> Option<Job> {
        let mut file = self.file.lock().unwrap();
        let running = file
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Running)
            .count();
        if running >= limit {
            return None;
        }
        let job = file
            .jobs
            .iter_mut()
            .filter(|j| j.status == JobStatus::Pending && j.scheduled_at <= now)
            .min_by_key(|j| (j.scheduled_at, j.id))?;
        job.status = JobStatus::Running;
        job.started_at = Some(now);
        job.attempts += 1;
        job.error = None;
        let job = job.clone();
        self.save(&file);
        Some(job)
    }

    /// When the next pending job is due, for the executor to sleep until.
    pub fn next_due(&self) -> Option<u64> {
        self.file
            .lock()
            .unwrap()
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Pending)
            .map(|j| j.scheduled_at)
            .min()
    }

    pub fn finish(&self, id: u64, outcome: Outcome) -> Option<Job> {
        let mut file = self.file.lock().unwrap();
        let now = now_ms();
        let job = file.jobs.iter_mut().find(|j| j.id == id)?;
        match outcome {
            Outcome::Completed(result) => {
                job.status = JobStatus::Completed;
                job.result = Some(result);
                job.finished_at = Some(now);
            }
            Outcome::Cancelled => {
                job.status = JobStatus::Cancelled;
                job.finished_at = Some(now);
                push_log(job, "cancelled".into());
            }
            Outcome::Failed {
                error,
                retry,
                after_ms,
            } => {
                if retry && job.attempts < job.max_attempts {
                    let backoff = after_ms.unwrap_or_else(|| {
                        (BACKOFF_BASE_MS << job.attempts.saturating_sub(1).min(10))
                            .min(BACKOFF_MAX_MS)
                    });
                    job.status = JobStatus::Pending;
                    job.scheduled_at = now + backoff;
                    push_log(
                        job,
                        format!(
                            "attempt {} failed: {error}; retrying in {}s",
                            job.attempts,
                            backoff / 1000
                        ),
                    );
                    job.error = Some(error);
                } else {
                    job.status = JobStatus::Failed;
                    job.finished_at = Some(now);
                    push_log(job, format!("failed: {error}"));
                    job.error = Some(error);
                }
            }
        }
        job.progress = None;
        let job = job.clone();
        trim_history(&mut file.jobs);
        self.save(&file);
        Some(job)
    }

    pub fn progress(&self, id: u64, progress: JobProgress) {
        let mut file = self.file.lock().unwrap();
        if let Some(job) = file.jobs.iter_mut().find(|j| j.id == id) {
            job.progress = Some(progress);
        }
        // Progress is transient: not worth a disk write each time.
    }

    pub fn log(&self, id: u64, line: String) {
        let mut file = self.file.lock().unwrap();
        if let Some(job) = file.jobs.iter_mut().find(|j| j.id == id) {
            push_log(job, line);
        }
        self.save(&file);
    }

    pub fn get(&self, id: u64) -> Option<Job> {
        self.file
            .lock()
            .unwrap()
            .jobs
            .iter()
            .find(|j| j.id == id)
            .cloned()
    }

    /// Newest first, optionally only one project's.
    pub fn list(&self, project: Option<&str>) -> Vec<Job> {
        let file = self.file.lock().unwrap();
        let mut out: Vec<Job> = file
            .jobs
            .iter()
            .filter(|j| project.is_none_or(|p| j.project.as_deref() == Some(p)))
            .cloned()
            .collect();
        out.sort_by_key(|j| std::cmp::Reverse(j.id));
        out
    }

    /// Cancels pending/paused jobs matching `pred` outright and returns the
    /// ids of running ones, which the executor stops cooperatively.
    pub fn cancel_where(&self, pred: impl Fn(&Job) -> bool) -> Vec<u64> {
        let mut file = self.file.lock().unwrap();
        let now = now_ms();
        let mut running = Vec::new();
        for job in file.jobs.iter_mut().filter(|j| pred(j)) {
            match job.status {
                JobStatus::Pending | JobStatus::Paused => {
                    job.status = JobStatus::Cancelled;
                    job.finished_at = Some(now);
                    push_log(job, "cancelled".into());
                }
                JobStatus::Running => running.push(job.id),
                _ => {}
            }
        }
        self.save(&file);
        running
    }

    /// Pauses a pending job, resumes a paused one or re-queues a finished
    /// failed/cancelled one.
    pub fn set_state(&self, id: u64, action: JobControl) -> Result<Job, AppError> {
        let mut file = self.file.lock().unwrap();
        let job = file
            .jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| AppError::not_found(format!("no job {id}")))?;
        match (action, job.status) {
            (JobControl::Pause, JobStatus::Pending) => job.status = JobStatus::Paused,
            (JobControl::Resume, JobStatus::Paused) => {
                job.status = JobStatus::Pending;
                job.scheduled_at = now_ms();
            }
            (JobControl::Retry, JobStatus::Failed | JobStatus::Cancelled) => {
                job.status = JobStatus::Pending;
                job.scheduled_at = now_ms();
                job.attempts = 0;
                job.finished_at = None;
                job.error = None;
                push_log(job, "queued again by hand".into());
            }
            (action, status) => {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    format!("can't {action:?} a job that is {status:?}").to_lowercase(),
                ));
            }
        }
        let job = job.clone();
        self.save(&file);
        Ok(job)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobControl {
    Cancel,
    Pause,
    Resume,
    Retry,
}

fn push_log(job: &mut Job, line: String) {
    if job.log.len() >= LOG_LINES {
        job.log.remove(0);
    }
    job.log.push(line);
}

fn trim_history(jobs: &mut Vec<Job>) {
    let finished = jobs.iter().filter(|j| j.status.finished()).count();
    if finished <= HISTORY {
        return;
    }
    let mut excess = finished - HISTORY;
    jobs.retain(|j| {
        if excess > 0 && j.status.finished() {
            excess -= 1;
            false
        } else {
            true
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(dedup: &str) -> Spec {
        Spec {
            kind: JobKind::DepsAudit,
            project: Some("blog".into()),
            input: serde_json::Value::Null,
            dedup: dedup.into(),
            automatic: true,
            max_attempts: 3,
            not_before: None,
        }
    }

    fn store() -> (tempfile::TempDir, Jobs) {
        let dir = tempfile::tempdir().unwrap();
        let jobs = Jobs::load(&dir.path().join("state/jobs.json")).unwrap();
        (dir, jobs)
    }

    #[test]
    fn deduplicates_active_jobs() {
        let (_dir, jobs) = store();
        let (first, created) = jobs.enqueue(spec("audit:blog"));
        assert!(created);
        let (again, created) = jobs.enqueue(spec("audit:blog"));
        assert!(!created);
        assert_eq!(again.id, first.id);
        // Once finished, the same key can run again.
        let taken = jobs.take_due(now_ms(), 2).unwrap();
        jobs.finish(taken.id, Outcome::Completed("ok".into()));
        assert!(jobs.enqueue(spec("audit:blog")).1);
    }

    #[test]
    fn respects_concurrency_and_schedule() {
        let (_dir, jobs) = store();
        jobs.enqueue(spec("a"));
        jobs.enqueue(spec("b"));
        let later = Spec {
            not_before: Some(now_ms() + 60_000),
            ..spec("c")
        };
        jobs.enqueue(later);
        let now = now_ms();
        assert!(jobs.take_due(now, 1).is_some());
        assert!(jobs.take_due(now, 1).is_none(), "one at a time");
        assert!(jobs.take_due(now, 2).is_some());
        assert!(jobs.take_due(now, 5).is_none(), "c is not due yet");
        assert!(jobs.next_due().unwrap() > now);
    }

    #[test]
    fn retries_with_backoff_then_fails() {
        let (_dir, jobs) = store();
        let (job, _) = jobs.enqueue(spec("a"));
        for attempt in 1..=3 {
            let taken = jobs.take_due(u64::MAX, 1).unwrap();
            assert_eq!(taken.attempts, attempt);
            let after = jobs
                .finish(
                    job.id,
                    Outcome::Failed {
                        error: "offline".into(),
                        retry: true,
                        after_ms: None,
                    },
                )
                .unwrap();
            if attempt < 3 {
                assert_eq!(after.status, JobStatus::Pending);
                assert!(after.scheduled_at > now_ms(), "backed off");
            } else {
                assert_eq!(after.status, JobStatus::Failed);
            }
        }
    }

    #[test]
    fn restart_requeues_running_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jobs.json");
        let jobs = Jobs::load(&path).unwrap();
        let (job, _) = jobs.enqueue(spec("a"));
        jobs.take_due(now_ms(), 1).unwrap();
        drop(jobs); // the daemon dies mid-run

        let reloaded = Jobs::load(&path).unwrap();
        let job = reloaded.get(job.id).unwrap();
        assert_eq!(job.status, JobStatus::Pending);
        assert!(job.log.iter().any(|l| l.contains("interrupted")));
        assert!(reloaded.take_due(now_ms(), 1).is_some(), "it runs again");
    }

    #[test]
    fn cancelling_automatic_work() {
        let (_dir, jobs) = store();
        let (pending, _) = jobs.enqueue(spec("a"));
        let (running, _) = jobs.enqueue(spec("b"));
        let manual = Spec {
            automatic: false,
            ..spec("c")
        };
        let (manual, _) = jobs.enqueue(manual);
        // `a` is due first, so take it, then make `b` the running one.
        let first = jobs.take_due(now_ms(), 3).unwrap();
        assert_eq!(first.id, pending.id);
        jobs.finish(first.id, Outcome::Completed(String::new()));
        let (pending, _) = jobs.enqueue(spec("a"));
        jobs.take_due(now_ms(), 3).unwrap(); // b

        let running_ids = jobs.cancel_where(|j| j.automatic);
        assert_eq!(running_ids, vec![running.id]);
        assert_eq!(jobs.get(pending.id).unwrap().status, JobStatus::Cancelled);
        assert_ne!(jobs.get(manual.id).unwrap().status, JobStatus::Cancelled);
    }

    #[test]
    fn pause_resume_retry() {
        let (_dir, jobs) = store();
        let (job, _) = jobs.enqueue(spec("a"));
        assert_eq!(
            jobs.set_state(job.id, JobControl::Pause).unwrap().status,
            JobStatus::Paused
        );
        assert!(
            jobs.take_due(u64::MAX, 1).is_none(),
            "paused jobs don't run"
        );
        assert_eq!(
            jobs.set_state(job.id, JobControl::Resume).unwrap().status,
            JobStatus::Pending
        );
        let taken = jobs.take_due(u64::MAX, 1).unwrap();
        jobs.finish(
            taken.id,
            Outcome::Failed {
                error: "x".into(),
                retry: false,
                after_ms: None,
            },
        );
        let retried = jobs.set_state(job.id, JobControl::Retry).unwrap();
        assert_eq!((retried.status, retried.attempts), (JobStatus::Pending, 0));
        assert!(jobs.set_state(job.id, JobControl::Resume).is_err());
    }

    #[test]
    fn keeps_a_bounded_history() {
        let (_dir, jobs) = store();
        for i in 0..(HISTORY + 20) {
            let (job, _) = jobs.enqueue(spec(&format!("j{i}")));
            jobs.take_due(u64::MAX, 1).unwrap();
            jobs.finish(job.id, Outcome::Completed(String::new()));
        }
        assert_eq!(jobs.list(None).len(), HISTORY);
    }
}
