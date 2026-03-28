use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::extract::DefaultBodyLimit;
use axum::extract::{self, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Serialize;
use smelt_core::manifest::JobManifest;

use crate::serve::config::AuthConfig;
use crate::serve::queue::ServerState;
use crate::serve::types::{JobSource, JobStatus, QueuedJob, elapsed_secs_since, now_epoch};

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

/// Resolved bearer-token credentials ready for runtime use.
///
/// Created by [`resolve_auth`] from an [`AuthConfig`].  Token values come
/// from environment variables — never from the config file directly.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedAuth {
    /// The read-write (full-access) token.
    pub(crate) write_token: String,
    /// An optional read-only token.  When `None`, only `write_token` grants
    /// access.
    pub(crate) read_token: Option<String>,
}

/// Resolve an [`AuthConfig`] into [`ResolvedAuth`] by reading environment
/// variables.
///
/// Returns an error if any referenced env var is unset or empty, naming the
/// offending variable in the message so operators can fix it quickly.
pub(crate) fn resolve_auth(config: &AuthConfig) -> anyhow::Result<ResolvedAuth> {
    let write_token = read_env_var(&config.write_token_env)?;
    let read_token = config
        .read_token_env
        .as_deref()
        .map(read_env_var)
        .transpose()?;
    Ok(ResolvedAuth {
        write_token,
        read_token,
    })
}

/// Read a non-empty value from the environment variable named `var_name`.
fn read_env_var(var_name: &str) -> anyhow::Result<String> {
    match std::env::var(var_name) {
        Ok(val) if val.is_empty() => {
            anyhow::bail!("environment variable {var_name} is set but empty")
        }
        Ok(val) => Ok(val),
        Err(_) => anyhow::bail!("environment variable {var_name} is not set"),
    }
}

/// Axum middleware that enforces bearer-token authentication.
///
/// When `auth` is `None` (no `[auth]` config section), all requests pass
/// through.  Otherwise the `Authorization: Bearer <token>` header is
/// required:
///
/// * **Read operations** (`GET`, `HEAD`) accept the read token *or* the
///   write token.
/// * **Write operations** (all other methods) accept *only* the write token.
///
/// Returns `401 Unauthorized` for missing/malformed headers and
/// `403 Forbidden` for valid tokens that lack the required permission.
async fn auth_middleware(
    State(auth): State<Option<ResolvedAuth>>,
    request: axum::extract::Request,
    next: Next,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    let auth = match auth {
        Some(a) => a,
        None => return Ok(next.run(request).await),
    };

    // Extract and validate the Authorization header.
    let header_value = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let token = match header_value {
        Some(v) if v.starts_with("Bearer ") => &v[7..],
        _ => {
            tracing::warn!(
                method = %request.method(),
                path = %request.uri().path(),
                "auth: missing or malformed Authorization header"
            );
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(
                    serde_json::json!({"error": "missing or malformed Authorization: Bearer <token> header"}),
                ),
            ));
        }
    };

    let is_read = matches!(*request.method(), Method::GET | Method::HEAD);

    let authorized = if is_read {
        // Read ops: accept read_token OR write_token.
        token == auth.write_token || auth.read_token.as_deref().is_some_and(|rt| token == rt)
    } else {
        // Write ops: accept only write_token.
        token == auth.write_token
    };

    if !authorized {
        let kind = if is_read { "read" } else { "write" };
        tracing::warn!(
            method = %request.method(),
            path = %request.uri().path(),
            "auth: token lacks {kind} permission"
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": format!("token does not have {kind} permission")})),
        ));
    }

    Ok(next.run(request).await)
}

/// Health check endpoint that returns `{"status": "ok"}` with a 200 status code.
///
/// This endpoint is served on a separate, stateless router merged into the
/// main router via `Router::merge()`, placing it structurally outside the
/// auth middleware layer.  See [`build_router`].  Do not move this route
/// into the API router chain — doing so would subject it to auth enforcement.
///
/// Load balancers, orchestrators, and monitoring probes can reach this
/// endpoint without credentials regardless of whether `[auth]` is configured.
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

/// Build the axum router for the smelt-serve HTTP API.
///
/// The `/health` endpoint is always unauthenticated regardless of the `auth`
/// value — load balancers and probes can reach it without credentials.
///
/// When `auth` is `Some`, bearer-token authentication is enforced on all
/// `/api/v1/*` routes.  When `auth` is `None`, those routes are also
/// unauthenticated (preserving the pre-auth behaviour).
pub(crate) fn build_router(state: SharedState, auth: Option<ResolvedAuth>) -> Router {
    // API routes — auth middleware enforced when auth is Some, passthrough when None.
    // 64 KB body limit on the event endpoint — prevents memory exhaustion
    // from arbitrarily large payloads (256 events × N jobs in memory).
    let api_routes = Router::new()
        .route(
            "/api/v1/events",
            post(post_event).layer(DefaultBodyLimit::max(64 * 1024)),
        )
        .route("/api/v1/jobs", post(post_job))
        .route("/api/v1/jobs", get(list_jobs))
        .route("/api/v1/jobs/{id}", get(get_job))
        .route("/api/v1/jobs/{id}", delete(delete_job))
        .layer(axum::middleware::from_fn_with_state(auth, auth_middleware))
        .with_state(state);

    // Health routes — no auth, no shared state needed.
    let health_routes = Router::new().route("/health", get(health_check));

    api_routes.merge(health_routes)
}

/// POST /api/v1/events — ingest an Assay event for a known job.
///
/// Expects a JSON body with at least a `job_id` string field. Stores the event
/// in the per-job `EventStore` and broadcasts it on the `EventBus`. Returns:
/// - 200 on success
/// - 400 if `job_id` is missing or not a string
/// - 404 if `job_id` does not match any known job
/// - 500 on internal error (e.g. poisoned mutex)
///
/// Body size is capped at 64 KB by the [`EVENT_BODY_LIMIT`] layer.
async fn post_event(
    State(state): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let job_id = body
        .get("job_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "missing or non-string job_id field"})),
            )
        })?
        .to_string();

    let event_id = body
        .get("event_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut s = state.lock().map_err(|_| {
        tracing::error!("ServerState mutex poisoned in post_event");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "internal server error"})),
        )
    })?;

    // Validate that the job_id exists in the server's job queue (any status).
    let job_exists = s.jobs.iter().any(|j| j.id.0 == job_id);
    if !job_exists {
        tracing::warn!(job_id = %job_id, "event POST: unknown job_id");
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "unknown job_id"})),
        ));
    }

    // Strip control fields from payload to avoid duplication with struct fields.
    let mut payload = body;
    if let Some(obj) = payload.as_object_mut() {
        obj.remove("job_id");
        obj.remove("event_id");
    }

    let event = crate::serve::events::AssayEvent {
        job_id: job_id.clone(),
        event_id,
        received_at: now_epoch(),
        payload,
    };

    // Store in per-job EventStore and broadcast via the encapsulated method.
    s.ingest_event(event);

    tracing::info!(job_id = %job_id, "event ingested");
    Ok(Json(serde_json::json!({"status": "ok"})))
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
