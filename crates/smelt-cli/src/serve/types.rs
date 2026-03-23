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
    pub fn new(id: impl Into<String>) -> Self {
        JobId(id.into())
    }
}

/// How a job entered the queue.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum JobSource {
    DirectoryWatch,
    HttpApi,
}

/// Lifecycle state of a job.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Dispatching,
    Running,
    Retrying,
    Complete,
    Failed,
}

/// A single job entry in the queue.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueuedJob {
    pub id: JobId,
    pub manifest_path: PathBuf,
    pub source: JobSource,
    pub attempt: u32,
    pub status: JobStatus,
    pub queued_at: u64,
    pub started_at: Option<u64>,
}
