use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::extract::{self, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Serialize;
use smelt_core::manifest::JobManifest;

use crate::serve::queue::ServerState;
use crate::serve::types::{JobSource, JobStatus, QueuedJob, elapsed_secs_since};

/// JSON-serialisable snapshot of a single job's state.
///
/// Time fields are *ages* (seconds elapsed since the event), not Unix timestamps.
/// `queued_age_secs` — seconds since the job was enqueued.
/// `elapsed_secs`    — seconds since the job last started executing (None if not yet started).
#[derive(Serialize)]
pub(crate) struct JobStateResponse {
    id: String,
    manifest_name: String,
    status: String,
    attempt: u32,
    /// Seconds elapsed since this job was enqueued (age, not a Unix timestamp).
    queued_age_secs: u64,
    /// Seconds elapsed since this job last started executing (age, not a Unix timestamp).
    /// `None` if the job has not yet been dispatched.
    elapsed_secs: Option<f64>,
    /// Which worker host this job was dispatched to.
    /// `None` means locally dispatched.
    worker_host: Option<String>,
}

impl From<&QueuedJob> for JobStateResponse {
    fn from(job: &QueuedJob) -> Self {
        let status_str = match &job.status {
            JobStatus::Queued => "queued",
            JobStatus::Dispatching => "dispatching",
            JobStatus::Running => "running",
            JobStatus::Retrying => "retrying",
            JobStatus::Complete => "complete",
            JobStatus::Failed => "failed",
        };
        JobStateResponse {
            id: job.id.to_string(),
            manifest_name: job
                .manifest_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            status: status_str.to_string(),
            attempt: job.attempt,
            queued_age_secs: elapsed_secs_since(job.queued_at) as u64,
            elapsed_secs: job.started_at.map(elapsed_secs_since),
            worker_host: job.worker_host.clone(),
        }
    }
}

type SharedState = Arc<Mutex<ServerState>>;

/// Build the axum router for the smelt-serve HTTP API.
pub(crate) fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/api/v1/jobs", post(post_job))
        .route("/api/v1/jobs", get(list_jobs))
        .route("/api/v1/jobs/{id}", get(get_job))
        .route("/api/v1/jobs/{id}", delete(delete_job))
        .with_state(state)
}

/// POST /api/v1/jobs — accept raw TOML body, parse, validate, enqueue.
async fn post_job(
    State(state): State<SharedState>,
    body: String,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Parse the manifest from the TOML body.
    let manifest = JobManifest::from_str(&body, Path::new("http-post"))
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, format!("{e}")))?;

    // Validate semantic constraints.
    manifest
        .validate()
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, format!("{e}")))?;

    // Write body to a named temp file so dispatch can read the path later.
    let mut tmp = tempfile::NamedTempFile::new()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("tempfile: {e}")))?;
    tmp.write_all(body.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("write: {e}")))?;

    // Keep the temp file alive by persisting it (leak the path).
    let tmp_path = tmp.into_temp_path();
    let path = tmp_path.to_path_buf();
    // Intentionally keep tmp_path alive so the file isn't deleted.
    std::mem::forget(tmp_path);

    let job_id = {
        let mut s = state.lock().unwrap();
        s.enqueue(path, JobSource::HttpApi)
    };

    Ok(Json(serde_json::json!({ "job_id": job_id.to_string() })))
}

/// GET /api/v1/jobs — return all jobs as JSON array.
async fn list_jobs(State(state): State<SharedState>) -> Json<Vec<JobStateResponse>> {
    let s = state.lock().unwrap();
    let jobs: Vec<JobStateResponse> = s.jobs.iter().map(JobStateResponse::from).collect();
    Json(jobs)
}

/// GET /api/v1/jobs/:id — return a single job or 404.
async fn get_job(
    State(state): State<SharedState>,
    extract::Path(id): extract::Path<String>,
) -> Result<Json<JobStateResponse>, StatusCode> {
    let s = state.lock().unwrap();
    s.jobs
        .iter()
        .find(|j| j.id.to_string() == id)
        .map(|j| Json(JobStateResponse::from(j)))
        .ok_or(StatusCode::NOT_FOUND)
}

/// DELETE /api/v1/jobs/:id — cancel a queued job, or 409 if running/dispatching.
async fn delete_job(
    State(state): State<SharedState>,
    extract::Path(id): extract::Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut s = state.lock().unwrap();
    // Find the job first to check its status.
    let job = s
        .jobs
        .iter()
        .find(|j| j.id.to_string() == id)
        .ok_or(StatusCode::NOT_FOUND)?;

    match job.status {
        JobStatus::Queued => {
            let jid = job.id.clone();
            s.cancel(&jid);
            Ok(StatusCode::OK)
        }
        JobStatus::Running | JobStatus::Dispatching => Err(StatusCode::CONFLICT),
        // Terminal states — nothing to cancel.
        _ => Err(StatusCode::NOT_FOUND),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::types::{JobId, JobSource, JobStatus, QueuedJob, now_epoch};
    use std::path::PathBuf;

    #[test]
    fn test_worker_host_in_api_response() {
        let job = QueuedJob {
            id: JobId::new("job-api-1"),
            manifest_path: PathBuf::from("test.smelt.toml"),
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
            worker_host: Some("remote-host-1".into()),
        };
        let resp = JobStateResponse::from(&job);
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["worker_host"], "remote-host-1");
    }

    #[test]
    fn test_worker_host_none_in_api_response() {
        let job = QueuedJob {
            id: JobId::new("job-api-2"),
            manifest_path: PathBuf::from("local.smelt.toml"),
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Queued,
            queued_at: now_epoch(),
            started_at: None,
            worker_host: None,
        };
        let resp = JobStateResponse::from(&job);
        let json = serde_json::to_value(&resp).unwrap();
        assert!(
            json["worker_host"].is_null(),
            "expected null worker_host for local job"
        );
    }
}
