use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use crate::serve::types::{JobId, JobSource, JobStatus, QueuedJob};

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
}

impl ServerState {
    pub fn new(max_concurrent: usize) -> Self {
        ServerState {
            jobs: VecDeque::new(),
            running_count: 0,
            max_concurrent,
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
            queued_at: Instant::now(),
            started_at: None,
        });
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
        job.started_at = Some(Instant::now());
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
    }

    /// Cancel a `Queued` job. Returns `true` if it was found and removed,
    /// `false` if the job is missing, already running/dispatching, or in any
    /// non-Queued terminal state.
    pub fn cancel(&mut self, id: &JobId) -> bool {
        if let Some(pos) = self.jobs.iter().position(|j| &j.id == id) {
            if self.jobs[pos].status == JobStatus::Queued {
                self.jobs.remove(pos);
                return true;
            }
        }
        false
    }

    /// Return `true` if the job exists, is in `Retrying` state, and has not
    /// exhausted `max_attempts`.
    pub fn retry_eligible(&self, id: &JobId, max_attempts: u32) -> bool {
        self.jobs.iter().any(|j| {
            &j.id == id && j.status == JobStatus::Retrying && j.attempt < max_attempts
        })
    }
}
