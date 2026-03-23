use std::fmt;
use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;

/// Unique identifier for a queued job.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum JobSource {
    DirectoryWatch,
    HttpApi,
}

/// Lifecycle state of a job.
#[derive(Serialize, Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug)]
pub struct QueuedJob {
    pub id: JobId,
    pub manifest_path: PathBuf,
    pub source: JobSource,
    pub attempt: u32,
    pub status: JobStatus,
    pub queued_at: Instant,
    pub started_at: Option<Instant>,
}
