use std::convert::Infallible;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::extract::DefaultBodyLimit;
use axum::extract::{self, Query, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use smelt_core::manifest::JobManifest;
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::sync::CancellationToken;

use crate::serve::config::AuthConfig;
use crate::serve::queue::ServerState;
use crate::serve::signals::{PeerUpdate, deliver_peer_update, validate_session_name};
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
pub(crate) fn build_router(
    state: SharedState,
    auth: Option<ResolvedAuth>,
    cancel_token: CancellationToken,
) -> Router {
    // API routes — auth middleware enforced when auth is Some, passthrough when None.
    // 64 KB body limit on the event endpoint — prevents memory exhaustion
    // from arbitrarily large payloads (256 events × N jobs in memory).
    let api_routes = Router::new()
        .route(
            "/api/v1/events",
            post(post_event)
                .get(get_events)
                .layer(DefaultBodyLimit::max(64 * 1024)),
        )
        .route("/api/v1/jobs", post(post_job))
        .route("/api/v1/jobs", get(list_jobs))
        .route("/api/v1/jobs/{id}", get(get_job))
        .route("/api/v1/jobs/{id}", delete(delete_job))
        .route(
            "/api/v1/jobs/{id}/signals",
            post(post_signal).layer(DefaultBodyLimit::max(64 * 1024)),
        )
        .layer(axum::middleware::from_fn_with_state(auth, auth_middleware))
        .layer(Extension(cancel_token))
        .with_state(state);

    // Health routes — no auth, no shared state needed.
    let health_routes = Router::new().route("/health", get(health_check));

    api_routes.merge(health_routes)
}

/// Query parameters for the SSE event stream endpoint.
#[derive(Deserialize)]
struct EventStreamParams {
    /// Optional job_id filter. When present, only events for this job are streamed.
    job: Option<String>,
}

/// GET /api/v1/events — SSE stream of Assay events.
///
/// Subscribes to the `EventBus` broadcast channel and streams events as SSE.
/// Supports an optional `?job=<id>` query parameter to filter by job_id.
///
/// Lagged subscribers receive a synthetic `event: lagged` with the drop count
/// and the stream continues (does NOT close on lag).
///
/// The stream terminates when the `CancellationToken` is cancelled (server
/// shutdown) or when the broadcast sender is dropped.
async fn get_events(
    State(state): State<SharedState>,
    Extension(cancel_token): Extension<CancellationToken>,
    Query(params): Query<EventStreamParams>,
) -> Sse<impl futures_util::Stream<Item = Result<SseEvent, Infallible>>> {
    // Clone the event_bus sender (briefly lock state, then release).
    let event_bus = {
        let s = state.lock().map_err(|_| {
            tracing::error!("ServerState mutex poisoned in get_events");
        });
        match s {
            Ok(guard) => guard.event_bus.clone(),
            Err(()) => {
                // Return an empty stream on mutex poisoning (extremely rare).
                let (tx, _rx) =
                    tokio::sync::broadcast::channel::<crate::serve::events::AssayEvent>(1);
                tx
            }
        }
    };

    let job_filter = params.job;
    tracing::info!(
        job_filter = ?job_filter,
        "SSE subscriber connected"
    );

    let rx = event_bus.subscribe();
    let raw_stream = BroadcastStream::new(rx);

    // Map broadcast items to SSE events, handling Lagged errors gracefully.
    // Uses futures_util::StreamExt::filter_map (async closure) so that
    // take_until (also from futures_util) works on the resulting stream.
    let mapped = raw_stream.filter_map(move |result| {
        let job_filter = job_filter.clone();
        async move {
            match result {
                Ok(event) => {
                    // Apply job filter if specified.
                    if let Some(ref filter_job) = job_filter
                        && event.job_id != *filter_job
                    {
                        return None;
                    }
                    match SseEvent::default().json_data(&event) {
                        Ok(sse_event) => Some(Ok(sse_event)),
                        Err(e) => {
                            tracing::error!(error = %e, "SSE: failed to serialize event");
                            None
                        }
                    }
                }
                Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                    tracing::warn!(dropped = n, "SSE: subscriber lagged");
                    let data = format!(r#"{{"dropped":{n}}}"#);
                    Some(Ok(SseEvent::default().event("lagged").data(data)))
                }
            }
        }
    });

    // Terminate the stream on CancellationToken (server shutdown).
    // CancellationToken::cancelled() borrows &self; we need a 'static future.
    // Clone the token and move it into an async block to produce an owned future.
    let cancel_fut = {
        let token = cancel_token.clone();
        async move { token.cancelled().await }
    };
    let stream = mapped.take_until(cancel_fut);

    Sse::new(stream).keep_alive(KeepAlive::default())
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

/// POST /api/v1/jobs/:id/signals — deliver a PeerUpdate signal to a running session.
///
/// Validates:
/// - Job exists (404 if not)
/// - `run_id` is known from prior event ingestion (409 if not)
/// - `session_name` passes path traversal validation (400 if invalid)
/// - Required `PeerUpdate` fields are present (400 if missing)
///
/// On success, writes the PeerUpdate as an atomic JSON file to the session's
/// inbox path on the host filesystem and returns 200 with the file path.
async fn post_signal(
    State(state): State<SharedState>,
    extract::Path(job_id): extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Extract session_name from body (required).
    let session_name = body
        .get("session_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "missing or non-string session_name field"})),
            )
        })?
        .to_string();

    // Validate session_name for path traversal.
    validate_session_name(&session_name).map_err(|msg| {
        tracing::warn!(session_name = %session_name, "signal: invalid session_name");
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("invalid session_name: {msg}")})),
        )
    })?;

    // Parse PeerUpdate fields from body (consume body — no further use).
    let peer_update: PeerUpdate = serde_json::from_value(body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("invalid PeerUpdate payload: {e}")})),
        )
    })?;

    // Lock state to validate job and extract needed data.
    let (run_id, manifest_path) = {
        let s = state.lock().map_err(|_| {
            tracing::error!("ServerState mutex poisoned in post_signal");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal server error"})),
            )
        })?;

        // Validate job exists.
        let job = s.jobs.iter().find(|j| j.id.0 == job_id).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "unknown job_id"})),
            )
        })?;

        // Look up cached run_id.
        let run_id = s.run_ids.get(&job_id).cloned().ok_or_else(|| {
            tracing::warn!(job_id = %job_id, "signal: no run_id known for job — waiting for first event");
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "no run_id known for job — waiting for first event"})),
            )
        })?;

        let manifest_path = job.manifest_path.clone();
        (run_id, manifest_path)
    };
    // Mutex is released here.

    // Resolve repo path from the manifest file.
    let manifest_content = std::fs::read_to_string(&manifest_path).map_err(|e| {
        tracing::error!(path = %manifest_path.display(), error = %e, "signal: cannot read manifest");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("cannot read manifest: {e}")})),
        )
    })?;

    let manifest_dir = manifest_path.parent().ok_or_else(|| {
        tracing::error!(
            path = %manifest_path.display(),
            "signal: manifest path has no parent directory"
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "manifest path has no parent directory"})),
        )
    })?;
    let manifest = JobManifest::from_str(&manifest_content, manifest_dir).map_err(|e| {
        tracing::error!(error = %e, "signal: cannot parse manifest");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("cannot parse manifest: {e}")})),
        )
    })?;

    let repo_path = smelt_core::manifest::resolve_repo_path(&manifest.job.repo).map_err(|e| {
        tracing::error!(repo = %manifest.job.repo, error = %e, "signal: cannot resolve repo path");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("cannot resolve repo path: {e}")})),
        )
    })?;

    // Deliver the signal (filesystem I/O outside the Mutex lock).
    let written_path = deliver_peer_update(&repo_path, &run_id, &session_name, &peer_update)
        .map_err(|e| {
            tracing::error!(
                job_id = %job_id,
                session_name = %session_name,
                error = %e,
                "signal: delivery failed"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("signal delivery failed: {e}")})),
            )
        })?;

    tracing::info!(
        job_id = %job_id,
        session_name = %session_name,
        path = %written_path.display(),
        "signal delivered"
    );

    Ok(Json(serde_json::json!({
        "status": "ok",
        "path": written_path.display().to_string()
    })))
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
