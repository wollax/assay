use std::fmt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Returns the current time as seconds since the Unix epoch.
/// `unwrap_or_default()` handles the impossible pre-1970 case.
pub fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Returns elapsed seconds since a stored Unix epoch value as `f64`.
/// Uses `as_secs_f64()` to preserve sub-second precision from `SystemTime`.
/// Returns 0.0 if the stored epoch is in the future (clock skew guard).
pub fn elapsed_secs_since(epoch: u64) -> f64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    (now - epoch as f64).max(0.0)
}

/// Unique identifier for a queued job.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(pub String);

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl JobId {
    /// Wrap an arbitrary string as a `JobId`.
    pub fn new(id: impl Into<String>) -> Self {
        JobId(id.into())
    }
}

/// How a job entered the queue.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum JobSource {
    /// Job was triggered by a filesystem watcher.
    DirectoryWatch,
    /// Job was submitted via the HTTP API.
    HttpApi,
}

/// Lifecycle state of a job.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Job is waiting in the queue.
    Queued,
    /// Job is being dispatched to an SSH host.
    Dispatching,
    /// Job is executing on the remote host.
    Running,
    /// Job failed and is scheduled for retry.
    Retrying,
    /// Job finished successfully.
    Complete,
    /// Job finished with an error.
    Failed,
}

/// A single job entry in the queue.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueuedJob {
    /// Unique identifier assigned at enqueue time.
    pub id: JobId,
    /// Path to the `.smelt.toml` manifest that defines this job.
    pub manifest_path: PathBuf,
    /// How this job entered the queue (HTTP API vs directory watch).
    pub source: JobSource,
    /// Zero-based retry counter; incremented on each re-queue after failure.
    pub attempt: u32,
    /// Current lifecycle state of the job.
    pub status: JobStatus,
    /// Unix epoch seconds when the job was first enqueued.
    pub queued_at: u64,
    /// Unix epoch seconds when the job most recently started executing.
    pub started_at: Option<u64>,
    /// Which worker host this job was dispatched to, if any.
    /// `None` means locally dispatched. `#[serde(default)]` ensures backward
    /// compatibility with state files written before this field existed.
    #[serde(default)]
    pub worker_host: Option<String>,
}
