use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use tracing::{info, warn};

use crate::serve::types::{now_epoch, JobId, JobSource, JobStatus, QueuedJob};

#[derive(serde::Serialize, serde::Deserialize)]
struct QueueState {
    jobs: Vec<QueuedJob>,
}

/// Persist the current queue to `queue_dir/.smelt-queue-state.toml` atomically.
///
/// Writes to a `.tmp` file first then renames — guarantees readers never see a
/// partially-written file.  All failures are logged at `WARN` and the function
/// returns without propagating the error so the daemon is never interrupted.
pub fn write_queue_state(queue_dir: &Path, jobs: &VecDeque<QueuedJob>) {
    let state = QueueState {
        jobs: jobs.iter().cloned().collect(),
    };
    let toml_str = match toml::to_string_pretty(&state) {
        Ok(s) => s,
        Err(e) => {
            warn!("write_queue_state: failed to serialize queue state: {e}");
            return;
        }
    };
    if let Err(e) = std::fs::create_dir_all(queue_dir) {
        warn!(
            "write_queue_state: failed to create queue dir {}: {e}",
            queue_dir.display()
        );
        return;
    }
    let tmp_path = queue_dir.join(".smelt-queue-state.toml.tmp");
    if let Err(e) = std::fs::write(&tmp_path, &toml_str) {
        warn!(
            "write_queue_state: failed to write tmp file {}: {e}",
            tmp_path.display()
        );
        return;
    }
    let final_path = queue_dir.join(".smelt-queue-state.toml");
    if let Err(e) = std::fs::rename(&tmp_path, &final_path) {
        warn!(
            "write_queue_state: failed to rename {} -> {}: {e}",
            tmp_path.display(),
            final_path.display()
        );
    }
}

/// Read the persisted queue from `queue_dir/.smelt-queue-state.toml`.
///
/// Returns an empty `Vec` when the file does not exist, cannot be read, or
/// cannot be parsed — all errors are logged at `WARN`.
pub fn read_queue_state(queue_dir: &Path) -> Vec<QueuedJob> {
    let path = queue_dir.join(".smelt-queue-state.toml");
    if !path.exists() {
        return vec![];
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("read_queue_state: failed to read {}: {e}", path.display());
            return vec![];
        }
    };
    match toml::from_str::<QueueState>(&content) {
        Ok(state) => state.jobs,
        Err(e) => {
            warn!("read_queue_state: failed to parse {}: {e}", path.display());
            vec![]
        }
    }
}

fn new_job_id() -> JobId {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    JobId::new(format!("job-{n}"))
}

/// In-memory queue and concurrency controller for smelt-serve jobs.
pub struct ServerState {
    pub jobs: VecDeque<QueuedJob>,
    pub running_count: usize,
    pub max_concurrent: usize,
    pub queue_dir: Option<PathBuf>,
}

impl ServerState {
    pub fn new(max_concurrent: usize) -> Self {
        ServerState {
            jobs: VecDeque::new(),
            running_count: 0,
            max_concurrent,
            queue_dir: None,
        }
    }

    /// Load persisted queue state from `queue_dir` and return a `ServerState`
    /// ready for use.  Any job whose status is `Dispatching` or `Running` is
    /// remapped to `Queued` so interrupted work is re-queued on the next
    /// dispatch cycle.  The `attempt` count is preserved unchanged.
    ///
    /// When no state file exists (first run or empty dir) the function returns
    /// an empty queue — equivalent to calling `new_with_persistence`.
    pub fn load_or_new(queue_dir: PathBuf, max_concurrent: usize) -> Self {
        let mut jobs: Vec<QueuedJob> = read_queue_state(&queue_dir);
        let n = jobs.len();
        let mut remapped = 0usize;
        for job in jobs.iter_mut() {
            if matches!(job.status, JobStatus::Dispatching | JobStatus::Running) {
                job.status = JobStatus::Queued;
                remapped += 1;
            }
        }
        info!(
            "load_or_new: loaded {n} jobs from {}, {remapped} remapped to Queued",
            queue_dir.display()
        );
        let mut state = Self::new_with_persistence(max_concurrent, queue_dir);
        state.jobs = VecDeque::from(jobs);
        state
    }

    /// Create a `ServerState` that persists queue state to `queue_dir` after
    /// every durable mutation (`enqueue`, `complete`, `cancel`).
    pub fn new_with_persistence(max_concurrent: usize, queue_dir: PathBuf) -> Self {
        ServerState {
            jobs: VecDeque::new(),
            running_count: 0,
            max_concurrent,
            queue_dir: Some(queue_dir),
        }
    }

    /// Append a new `Queued` job and return its `JobId`.
    pub fn enqueue(&mut self, manifest_path: PathBuf, source: JobSource) -> JobId {
        let id = new_job_id();
        self.jobs.push_back(QueuedJob {
            id: id.clone(),
            manifest_path,
            source,
            attempt: 0,
            status: JobStatus::Queued,
            queued_at: now_epoch(),
            started_at: None,
        });
        if let Some(ref dir) = self.queue_dir {
            write_queue_state(dir, &self.jobs);
        }
        id
    }

    /// If a slot is available and a dispatchable job exists (Queued or Retrying),
    /// promote it to `Dispatching`, increment the running count, and return it.
    pub fn try_dispatch(&mut self) -> Option<QueuedJob> {
        if self.running_count >= self.max_concurrent {
            return None;
        }
        let job = self
            .jobs
            .iter_mut()
            .find(|j| j.status == JobStatus::Queued || j.status == JobStatus::Retrying)?;
        job.status = JobStatus::Dispatching;
        job.started_at = Some(now_epoch());
        self.running_count += 1;
        Some(job.clone())
    }

    /// Record job completion (success or failure).
    ///
    /// * On failure with remaining attempts: set `Retrying` in-place (job stays
    ///   in the queue and will be re-dispatched on the next `try_dispatch` call).
    /// * On final failure or success: set `Complete`/`Failed`, release the slot.
    pub fn complete(&mut self, id: &JobId, success: bool, attempt: u32, max_attempts: u32) {
        if let Some(job) = self.jobs.iter_mut().find(|j| &j.id == id) {
            if !success && attempt < max_attempts {
                job.status = JobStatus::Retrying;
                job.attempt = attempt + 1;
                self.running_count = self.running_count.saturating_sub(1);
            } else {
                job.status = if success {
                    JobStatus::Complete
                } else {
                    JobStatus::Failed
                };
                self.running_count = self.running_count.saturating_sub(1);
            }
        }
        if let Some(ref dir) = self.queue_dir {
            write_queue_state(dir, &self.jobs);
        }
    }

    /// Cancel a `Queued` job. Returns `true` if it was found and removed,
    /// `false` if the job is missing, already running/dispatching, or in any
    /// non-Queued terminal state.
    pub fn cancel(&mut self, id: &JobId) -> bool {
        let cancelled = if let Some(pos) = self.jobs.iter().position(|j| &j.id == id) {
            if self.jobs[pos].status == JobStatus::Queued {
                self.jobs.remove(pos);
                true
            } else {
                false
            }
        } else {
            false
        };
        if cancelled {
            if let Some(ref dir) = self.queue_dir {
                write_queue_state(dir, &self.jobs);
            }
        }
        cancelled
    }

    /// Return `true` if the job exists, is in `Retrying` state, and has not
    /// exhausted `max_attempts`.
    pub fn retry_eligible(&self, id: &JobId, max_attempts: u32) -> bool {
        self.jobs.iter().any(|j| {
            &j.id == id && j.status == JobStatus::Retrying && j.attempt < max_attempts
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::types::{JobId, JobSource, JobStatus};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_job(id: &str, status: JobStatus, started_at: Option<u64>) -> QueuedJob {
        QueuedJob {
            id: JobId::new(id),
            manifest_path: PathBuf::from(format!("/tmp/{id}.smelt.toml")),
            source: JobSource::HttpApi,
            attempt: 0,
            status,
            queued_at: 1_000_000,
            started_at,
        }
    }

    #[test]
    fn test_queue_state_round_trip() {
        let dir = TempDir::new().unwrap();
        let job0 = make_job("job-1", JobStatus::Queued, None);
        let job1 = make_job("job-2", JobStatus::Complete, Some(1_000_042));

        let mut jobs: VecDeque<QueuedJob> = VecDeque::new();
        jobs.push_back(job0.clone());
        jobs.push_back(job1.clone());

        write_queue_state(dir.path(), &jobs);

        let read_back = read_queue_state(dir.path());
        assert_eq!(read_back.len(), 2);

        // Job 0 — all 7 fields
        assert_eq!(read_back[0].id, job0.id);
        assert_eq!(read_back[0].manifest_path, job0.manifest_path);
        assert_eq!(format!("{:?}", read_back[0].source), format!("{:?}", job0.source));
        assert_eq!(read_back[0].attempt, job0.attempt);
        assert_eq!(read_back[0].status, job0.status);
        assert_eq!(read_back[0].queued_at, job0.queued_at);
        assert_eq!(read_back[0].started_at, job0.started_at);

        // Job 1 — all 7 fields
        assert_eq!(read_back[1].id, job1.id);
        assert_eq!(read_back[1].manifest_path, job1.manifest_path);
        assert_eq!(format!("{:?}", read_back[1].source), format!("{:?}", job1.source));
        assert_eq!(read_back[1].attempt, job1.attempt);
        assert_eq!(read_back[1].status, job1.status);
        assert_eq!(read_back[1].queued_at, job1.queued_at);
        assert_eq!(read_back[1].started_at, job1.started_at);
    }

    #[test]
    fn test_read_queue_state_missing_file() {
        let dir = TempDir::new().unwrap();
        let result = read_queue_state(dir.path());
        assert!(result.is_empty());
    }

    #[test]
    fn test_read_queue_state_corrupt_file() {
        let dir = TempDir::new().unwrap();
        let state_path = dir.path().join(".smelt-queue-state.toml");
        std::fs::write(&state_path, b"not toml at all!!!").unwrap();
        let result = read_queue_state(dir.path());
        assert!(result.is_empty());
    }

    #[test]
    fn test_server_state_writes_on_enqueue() {
        let tmp = TempDir::new().unwrap();
        let mut state = ServerState::new_with_persistence(2, tmp.path().to_path_buf());
        state.enqueue(PathBuf::from("/tmp/test.toml"), JobSource::HttpApi);

        // State file must exist after enqueue
        let state_file = tmp.path().join(".smelt-queue-state.toml");
        assert!(state_file.exists(), "state file should exist after enqueue");

        // read_queue_state should return 1 job with status Queued
        let jobs = read_queue_state(tmp.path());
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, JobStatus::Queued);
    }

    #[test]
    fn test_load_or_new_restart_recovery() {
        let dir = TempDir::new().unwrap();

        // Build 3 jobs: Queued/attempt=0, Running/attempt=2, Queued/attempt=1
        let mut job_a = make_job("job-a", JobStatus::Queued, None);
        job_a.attempt = 0;
        let mut job_b = make_job("job-b", JobStatus::Running, Some(1_000_000));
        job_b.attempt = 2;
        let mut job_c = make_job("job-c", JobStatus::Queued, None);
        job_c.attempt = 1;

        let mut jobs: VecDeque<QueuedJob> = VecDeque::new();
        jobs.push_back(job_a);
        jobs.push_back(job_b);
        jobs.push_back(job_c);
        write_queue_state(dir.path(), &jobs);

        let state = ServerState::load_or_new(dir.path().to_path_buf(), 2);

        assert_eq!(state.jobs.len(), 3);
        // All jobs must be Queued after recovery
        assert_eq!(state.jobs[0].status, JobStatus::Queued);
        assert_eq!(state.jobs[1].status, JobStatus::Queued);
        assert_eq!(state.jobs[2].status, JobStatus::Queued);
        // Attempt counts preserved
        assert_eq!(state.jobs[0].attempt, 0);
        assert_eq!(state.jobs[1].attempt, 2);
        assert_eq!(state.jobs[2].attempt, 1);
        // queue_dir is set so future mutations persist
        assert_eq!(state.queue_dir, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn test_load_or_new_missing_file() {
        let dir = TempDir::new().unwrap(); // no writes
        let state = ServerState::load_or_new(dir.path().to_path_buf(), 4);

        assert!(state.jobs.is_empty());
        assert!(state.queue_dir.is_some());
        assert_eq!(state.max_concurrent, 4);
    }
}
