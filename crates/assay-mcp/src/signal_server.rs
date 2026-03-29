//! HTTP signal endpoint for cross-job signaling.
//!
//! Embeds an axum HTTP listener alongside the stdio MCP server.
//! - `POST /api/v1/signal` — receives [`SignalRequest`] and routes the
//!   [`PeerUpdate`] into the named session's inbox.
//! - `GET /api/v1/state` — returns [`AssayServerState`].

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use tower_http::limit::RequestBodyLimitLayer;

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
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(session_name, entry);
    }

    /// Remove a session from the registry.
    pub fn unregister_session(&self, session_name: &str) {
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(session_name);
    }

    /// Look up a session's run entry by name.
    pub fn lookup_session(&self, session_name: &str) -> Option<RunEntry> {
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(session_name)
            .cloned()
    }

    /// List all active runs as `RunSummary` entries, deduplicated by run ID.
    pub fn list_runs(&self) -> Vec<RunSummary> {
        let entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
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
///
/// The `backend` field is wrapped in `RwLock` so the MCP orchestrate
/// handlers can swap it from `NoopBackend` to the run's real backend
/// when a run starts, and restore it when the run completes.
pub struct SignalServerState {
    /// State backend for routing messages into session inboxes.
    pub backend: Arc<RwLock<Arc<dyn StateBackend>>>,
    /// Registry of active sessions.
    pub registry: Arc<RunRegistry>,
    /// Optional bearer token for authentication.
    pub token: Option<String>,
    /// When the server started (for uptime calculation).
    pub started_at: Instant,
    /// Shared HTTP client for forwarding signals to peer instances.
    /// `reqwest::Client` internally pools connections; one instance is shared
    /// across all forwarding requests.
    pub http_client: reqwest::Client,
}

// ── Router ──────────────────────────────────────────────────────────

/// Maximum accepted request body size for `/api/v1/signal`.
///
/// `SignalRequest` payloads are small JSON objects. 64 KiB is generous
/// and protects against accidental or malicious oversized bodies.
const MAX_SIGNAL_BODY_BYTES: usize = 64 * 1024; // 64 KiB

/// Build the axum router without binding to a port.
///
/// Exposed for testing — callers can use `tower::ServiceExt::oneshot`
/// to exercise handlers without a TCP listener.
pub fn build_router(state: Arc<SignalServerState>) -> Router {
    Router::new()
        .route("/api/v1/signal", post(handle_signal))
        .route("/api/v1/state", get(handle_state))
        .layer(RequestBodyLimitLayer::new(MAX_SIGNAL_BODY_BYTES))
        .with_state(state)
}

/// Start the signal server on the given address and port.
///
/// Returns a `JoinHandle` for the background task. The caller should
/// store the handle to track the server's lifetime, or `.await` it
/// to block until the server exits. Dropping the handle detaches the
/// task — it does not stop the server.
///
/// `bind_addr` controls the listen address (e.g. `"127.0.0.1"` for
/// local-only, `"0.0.0.0"` for all interfaces). Returns `Err` if the
/// address/port is already in use, allowing the caller to continue
/// without the signal server.
pub async fn start_signal_server(
    state: Arc<SignalServerState>,
    bind_addr: &str,
    port: u16,
) -> Result<tokio::task::JoinHandle<()>, std::io::Error> {
    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind(format!("{bind_addr}:{port}")).await?;

    tracing::info!(bind_addr, port, "signal server listening");

    Ok(tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!(error = %e, "signal server error");
        }
    }))
}

// ── Handlers ────────────────────────────────────────────────────────

// ── Path safety ─────────────────────────────────────────────────────

/// Validate that `s` is safe to use as a single filesystem path component.
///
/// Rejects:
/// - Empty strings
/// - `.` or `..`
/// - Strings containing `/`, `\`, or NUL
/// - Strings that are absolute paths (start with `/` or a Windows drive letter)
fn validate_path_component(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err("path component must not be empty".to_string());
    }
    if s == "." || s == ".." {
        return Err(format!("path component '{s}' is not allowed"));
    }
    if s.contains('/') || s.contains('\\') || s.contains('\0') {
        return Err(format!("path component '{s}' contains illegal character"));
    }
    // Reject absolute paths (e.g. '/etc/passwd' or 'C:\\Windows').
    if s.starts_with('/') {
        return Err(format!("path component '{s}' must not be absolute"));
    }
    Ok(())
}

// ── Handlers ────────────────────────────────────────────────────────

/// JSON error body for HTTP error responses.
#[derive(serde::Serialize)]
struct ErrorBody {
    error: String,
}

/// Attempt to forward a signal to peer instances.
///
/// Tries each peer sequentially; returns `true` on the first 202 response.
/// Returns `false` if all peers fail or the list is empty.
async fn forward_to_peers(
    peers: Vec<assay_types::PeerInfo>,
    request: &SignalRequest,
    token: Option<&str>,
    client: &reqwest::Client,
) -> bool {
    for peer in &peers {
        let url = format!("{}/api/v1/signal", peer.signal_url.trim_end_matches('/'));
        let mut req = client
            .post(&url)
            .header("X-Assay-Forwarded", "true")
            .json(request);
        if let Some(tok) = token {
            req = req.header("Authorization", format!("Bearer {tok}"));
        }
        match req.send().await {
            Ok(resp) if resp.status() == reqwest::StatusCode::ACCEPTED => {
                tracing::info!(
                    target_session = %request.target_session,
                    forwarded_to = %url,
                    "signal forwarded to peer"
                );
                return true;
            }
            Ok(resp) => {
                tracing::warn!(
                    target_session = %request.target_session,
                    url = %url,
                    status = %resp.status(),
                    "peer rejected forwarded signal"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target_session = %request.target_session,
                    url = %url,
                    error = %e,
                    "peer forward failed"
                );
            }
        }
    }
    false
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

    // Validate target_session: must not be empty, absolute, or contain path
    // components that could escape the run directory.
    if let Err(e) = validate_path_component(&request.target_session) {
        return (StatusCode::BAD_REQUEST, Json(ErrorBody { error: e })).into_response();
    }

    // Look up session.
    let entry = match state.registry.lookup_session(&request.target_session) {
        Some(e) => e,
        None => {
            // Loop prevention: if X-Assay-Forwarded is already set, this request
            // arrived from a peer — return 404 immediately to break forwarding loops.
            let is_forwarded = match headers.get("x-assay-forwarded") {
                None => false,
                Some(v) => match v.to_str() {
                    Ok("true") => true,
                    Ok(_) => false,
                    Err(_) => {
                        // Fail closed: treat invalid header as forwarded to prevent loops.
                        tracing::warn!(
                            target_session = %request.target_session,
                            "x-assay-forwarded header contained non-UTF-8 bytes; \
                             treating as forwarded to break potential loop"
                        );
                        true
                    }
                },
            };
            if is_forwarded {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorBody {
                        error: format!("unknown session: {}", request.target_session),
                    }),
                )
                    .into_response();
            }

            // Peer forwarding — extract data from the RwLock guard before the async
            // boundary (guard must not be held across .await).
            let (supports_registry, peers_result) = {
                let backend = state.backend.read().unwrap_or_else(|p| {
                    tracing::error!(
                        target_session = %request.target_session,
                        "signal_backend RwLock poisoned during peer lookup — recovering"
                    );
                    p.into_inner()
                });
                (
                    backend.capabilities().supports_peer_registry,
                    backend.list_peers(),
                )
            }; // RwLock guard dropped here — safe to .await below

            let peers = match peers_result {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        target_session = %request.target_session,
                        error = %e,
                        "failed to list peers for signal forwarding; treating as no peers"
                    );
                    vec![]
                }
            };

            if supports_registry && !peers.is_empty() {
                if forward_to_peers(peers, &request, state.token.as_deref(), &state.http_client)
                    .await
                {
                    return StatusCode::ACCEPTED.into_response();
                }
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorBody {
                        error: format!(
                            "no peer accepted signal for session: {}",
                            request.target_session
                        ),
                    }),
                )
                    .into_response();
            }

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

    // Generate a filename-safe message name. We use a timestamp-only name
    // rather than embedding `source_job` — source_job may contain path
    // separators or other characters that `send_message` rejects as
    // filename components. The `source_job` field is already present in the
    // serialized `PeerUpdate` payload, so no information is lost.
    let name = format!("signal-{}", chrono::Utc::now().timestamp_millis());

    // Route into inbox. Acquire read lock on the swappable backend.
    let backend = state
        .backend
        .read()
        .unwrap_or_else(|p| {
            tracing::error!("signal_backend RwLock poisoned — recovering");
            p.into_inner()
        })
        .clone();
    if !backend.capabilities().supports_messaging {
        tracing::warn!(
            target_session = %request.target_session,
            "signal backend not ready (NoopBackend active); signal will be dropped. \
             Signals are only routed while an orchestrated run is active."
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: "signal backend not yet initialized; retry when a run is active".to_string(),
            }),
        )
            .into_response();
    }
    match backend.send_message(&inbox_path, &name, &payload) {
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
