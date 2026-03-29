//! HTTP signal endpoint for cross-job signaling.
//!
//! Embeds an axum HTTP listener alongside the stdio MCP server.
//! - `POST /api/v1/signal` — receives [`SignalRequest`] and routes the
//!   [`PeerUpdate`] into the named session's inbox.
//! - `GET /api/v1/state` — returns [`AssayServerState`].

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};

use assay_core::StateBackend;
use assay_types::signal::{AssayServerState, RunSummary, SignalRequest};

// ── RunRegistry ─────────────────────────────────────────────────────

/// Metadata for a registered session, used to resolve inbox paths.
#[derive(Debug, Clone)]
pub struct RunEntry {
    /// Unique run identifier.
    pub run_id: String,
    /// Base path for the run directory (e.g. `.assay/orchestrator/<run_id>`).
    pub run_dir: PathBuf,
    /// Spec name being executed.
    pub spec_name: String,
    /// When the run was registered.
    pub started_at: Instant,
    /// Number of sessions in this run.
    pub session_count: u32,
}

/// In-process registry of active orchestrator sessions.
///
/// Session names map to their run entry. The signal handler uses this to
/// resolve the inbox path for a target session:
/// `<run_dir>/mesh/<session_name>/inbox/`
#[derive(Debug, Default)]
pub struct RunRegistry {
    entries: Mutex<HashMap<String, RunEntry>>,
}

impl RunRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a session name with its run entry.
    pub fn register_session(&self, session_name: String, entry: RunEntry) {
        self.entries.lock().unwrap().insert(session_name, entry);
    }

    /// Remove a session from the registry.
    pub fn unregister_session(&self, session_name: &str) {
        self.entries.lock().unwrap().remove(session_name);
    }

    /// Look up a session's run entry by name.
    pub fn lookup_session(&self, session_name: &str) -> Option<RunEntry> {
        self.entries.lock().unwrap().get(session_name).cloned()
    }

    /// List all registered sessions as `RunSummary` entries.
    pub fn list_runs(&self) -> Vec<RunSummary> {
        let entries = self.entries.lock().unwrap();
        // Deduplicate by run_id — multiple sessions may share a run.
        let mut seen = HashMap::new();
        for entry in entries.values() {
            seen.entry(entry.run_id.clone())
                .or_insert_with(|| RunSummary {
                    run_id: entry.run_id.clone(),
                    spec_name: entry.spec_name.clone(),
                    session_count: entry.session_count,
                    // We store Instant, but RunSummary needs DateTime<Utc>.
                    // Use current time minus elapsed as an approximation.
                    started_at: chrono::Utc::now()
                        - chrono::Duration::from_std(entry.started_at.elapsed())
                            .unwrap_or_default(),
                });
        }
        seen.into_values().collect()
    }
}

// ── SignalServerState ───────────────────────────────────────────────

/// Shared state for the signal server axum handlers.
pub struct SignalServerState {
    /// State backend for routing messages into session inboxes.
    pub backend: Arc<dyn StateBackend>,
    /// Registry of active sessions.
    pub registry: Arc<RunRegistry>,
    /// Optional bearer token for authentication.
    pub token: Option<String>,
    /// When the server started (for uptime calculation).
    pub started_at: Instant,
}

// ── Router ──────────────────────────────────────────────────────────

/// Build the axum router without binding to a port.
///
/// Exposed for testing — callers can use `tower::ServiceExt::oneshot`
/// to exercise handlers without a TCP listener.
pub fn build_router(state: Arc<SignalServerState>) -> Router {
    Router::new()
        .route("/api/v1/signal", post(handle_signal))
        .route("/api/v1/state", get(handle_state))
        .with_state(state)
}

/// Start the signal server on the given port.
///
/// Returns a `JoinHandle` — the caller should `tokio::spawn` this or
/// store the handle. The server runs until dropped.
pub async fn start_signal_server(
    state: Arc<SignalServerState>,
    port: u16,
) -> tokio::task::JoinHandle<()> {
    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap_or_else(|e| panic!("failed to bind signal server on port {port}: {e}"));

    tracing::info!(port, "signal server listening");

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!(error = %e, "signal server error");
        }
    })
}

// ── Handlers ────────────────────────────────────────────────────────

/// JSON error body for HTTP error responses.
#[derive(serde::Serialize)]
struct ErrorBody {
    error: String,
}

/// `POST /api/v1/signal` — route a `PeerUpdate` into a session's inbox.
async fn handle_signal(
    State(state): State<Arc<SignalServerState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // Auth check.
    if let Some(expected) = &state.token {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));

        match provided {
            Some(token) if token == expected => {} // OK
            _ => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorBody {
                        error: "invalid or missing bearer token".to_string(),
                    }),
                )
                    .into_response();
            }
        }
    }

    // Deserialize.
    let request: SignalRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: format!("invalid JSON: {e}"),
                }),
            )
                .into_response();
        }
    };

    // Look up session.
    let entry = match state.registry.lookup_session(&request.target_session) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorBody {
                    error: format!("unknown session: {}", request.target_session),
                }),
            )
                .into_response();
        }
    };

    // Resolve inbox path: <run_dir>/mesh/<session_name>/inbox/
    let inbox_path = entry
        .run_dir
        .join("mesh")
        .join(&request.target_session)
        .join("inbox");

    // Serialize PeerUpdate as the message payload.
    let payload = match serde_json::to_vec(&request.update) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to serialize PeerUpdate");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "internal serialization error".to_string(),
                }),
            )
                .into_response();
        }
    };

    // Generate a unique message name.
    let name = format!(
        "signal-{}-{}",
        request.update.source_job,
        chrono::Utc::now().timestamp_millis()
    );

    // Route into inbox.
    match state.backend.send_message(&inbox_path, &name, &payload) {
        Ok(()) => {
            tracing::debug!(
                target_session = %request.target_session,
                source_job = %request.update.source_job,
                "signal routed to inbox"
            );
            StatusCode::ACCEPTED.into_response()
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                target_session = %request.target_session,
                "failed to route signal to inbox"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: format!("failed to deliver signal: {e}"),
                }),
            )
                .into_response()
        }
    }
}

/// `GET /api/v1/state` — return global server state.
async fn handle_state(State(state): State<Arc<SignalServerState>>) -> impl IntoResponse {
    let active_runs = state.registry.list_runs();
    let uptime_secs = state.started_at.elapsed().as_secs();

    Json(AssayServerState {
        active_runs,
        uptime_secs,
    })
}
