//! Tests for the signal_server HTTP endpoint.
//!
//! Uses `tower::ServiceExt::oneshot` to exercise axum handlers without
//! a real TCP listener.

use std::sync::{Arc, RwLock};
use std::time::Instant;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use assay_core::{LocalFsBackend, StateBackend};
use assay_mcp::signal_server::{RunEntry, RunRegistry, SignalServerState, build_router};
use assay_types::signal::{AssayServerState, GateSummary, PeerUpdate, SignalRequest};

/// Build a `SignalServerState` backed by `LocalFsBackend` with the given
/// token and a pre-registered session at `run_dir`.
fn make_state(backend: Arc<dyn StateBackend>, token: Option<String>) -> Arc<SignalServerState> {
    Arc::new(SignalServerState {
        backend: Arc::new(RwLock::new(backend)),
        registry: Arc::new(RunRegistry::new()),
        token,
        started_at: Instant::now(),
    })
}

/// Build a sample `SignalRequest` JSON body.
fn sample_signal_json(target_session: &str) -> String {
    let req = SignalRequest {
        target_session: target_session.to_string(),
        update: PeerUpdate {
            source_job: "job-abc".to_string(),
            source_session: "worker-1".to_string(),
            changed_files: vec!["src/main.rs".to_string()],
            gate_summary: GateSummary {
                passed: 5,
                failed: 0,
                skipped: 1,
            },
            branch: "feature/test".to_string(),
        },
    };
    serde_json::to_string(&req).unwrap()
}

/// Register a session in the state's registry.
fn register_session(state: &SignalServerState, session_name: &str, run_dir: std::path::PathBuf) {
    state.registry.register_session(
        session_name.to_string(),
        RunEntry {
            run_id: "run-001".to_string(),
            run_dir,
            spec_name: "test-spec".to_string(),
            started_at: Instant::now(),
            session_count: 2,
        },
    );
}

// ── POST /api/v1/signal tests ───────────────────────────────────────

#[tokio::test]
async fn test_signal_valid_request_returns_202() {
    let tmp = tempfile::tempdir().unwrap();
    let backend = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let state = make_state(backend, None);

    // Create the inbox directory structure.
    let run_dir = tmp.path().join("orchestrator/run-001");
    let inbox_dir = run_dir.join("mesh/worker-1/inbox");
    std::fs::create_dir_all(&inbox_dir).unwrap();

    register_session(&state, "worker-1", run_dir);

    let router = build_router(state);
    let body = sample_signal_json("worker-1");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/signal")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_signal_unknown_session_returns_404() {
    let tmp = tempfile::tempdir().unwrap();
    let backend = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let state = make_state(backend, None);

    let router = build_router(state);
    let body = sample_signal_json("nonexistent-session");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/signal")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let error: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(
        error["error"]
            .as_str()
            .unwrap()
            .contains("nonexistent-session"),
        "error body should mention the unknown session name"
    );
}

#[tokio::test]
async fn test_signal_malformed_json_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    let backend = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let state = make_state(backend, None);

    let router = build_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/signal")
                .header("content-type", "application/json")
                .body(Body::from("not valid json {{{"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_signal_auth_required_returns_401() {
    let tmp = tempfile::tempdir().unwrap();
    let backend = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let state = make_state(backend, Some("secret-token".to_string()));

    let run_dir = tmp.path().join("orchestrator/run-001");
    register_session(&state, "worker-1", run_dir);

    let router = build_router(state);
    let body = sample_signal_json("worker-1");

    // No auth header.
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/signal")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_signal_auth_not_required_when_no_token() {
    let tmp = tempfile::tempdir().unwrap();
    let backend = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let state = make_state(backend, None); // No token configured.

    let run_dir = tmp.path().join("orchestrator/run-001");
    std::fs::create_dir_all(run_dir.join("mesh/worker-1/inbox")).unwrap();
    register_session(&state, "worker-1", run_dir);

    let router = build_router(state);
    let body = sample_signal_json("worker-1");

    // No auth header — should succeed.
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/signal")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_signal_routes_to_inbox() {
    let tmp = tempfile::tempdir().unwrap();
    let backend = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let state = make_state(backend, None);

    let run_dir = tmp.path().join("orchestrator/run-001");
    let inbox_dir = run_dir.join("mesh/worker-1/inbox");
    std::fs::create_dir_all(&inbox_dir).unwrap();

    register_session(&state, "worker-1", run_dir);

    let router = build_router(state);
    let body = sample_signal_json("worker-1");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/signal")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);

    // Verify a file was written to the inbox directory.
    let files: Vec<_> = std::fs::read_dir(&inbox_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        files.len(),
        1,
        "exactly one message file should appear in inbox"
    );

    // Verify the file content is valid PeerUpdate JSON.
    let content = std::fs::read(files[0].path()).unwrap();
    let update: PeerUpdate = serde_json::from_slice(&content).unwrap();
    assert_eq!(update.source_job, "job-abc");
    assert_eq!(update.branch, "feature/test");
}

// ── GET /api/v1/state tests ─────────────────────────────────────────

#[tokio::test]
async fn test_state_returns_correct_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let backend = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let state = make_state(backend, None);

    // Register some sessions.
    register_session(&state, "worker-1", tmp.path().join("orchestrator/run-001"));
    register_session(&state, "worker-2", tmp.path().join("orchestrator/run-001"));

    let router = build_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/state")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let server_state: AssayServerState = serde_json::from_slice(&body_bytes).unwrap();

    // Both sessions share run-001, so we should see 1 run.
    assert_eq!(server_state.active_runs.len(), 1);
    assert_eq!(server_state.active_runs[0].run_id, "run-001");
    assert_eq!(server_state.active_runs[0].spec_name, "test-spec");
    assert!(
        server_state.uptime_secs < 5,
        "uptime should be small in tests"
    );
}

// ── RwLock backend swap tests ───────────────────────────────────────

#[tokio::test]
async fn test_backend_swap_routes_through_new_backend() {
    // Start with NoopBackend (signals silently fail).
    let registry = Arc::new(RunRegistry::new());
    let backend_lock: Arc<RwLock<Arc<dyn StateBackend>>> =
        Arc::new(RwLock::new(Arc::new(assay_core::NoopBackend) as _));

    let state = Arc::new(SignalServerState {
        backend: backend_lock.clone(),
        registry: registry.clone(),
        token: None,
        started_at: Instant::now(),
    });

    let tmp = tempfile::tempdir().unwrap();
    let run_dir = tmp.path().join("orchestrator/run-swap");
    let inbox_dir = run_dir.join("mesh/worker-1/inbox");
    std::fs::create_dir_all(&inbox_dir).unwrap();

    register_session(&state, "worker-1", run_dir);

    // Signal with NoopBackend — returns 503 SERVICE_UNAVAILABLE with a clear message.
    let router = build_router(state.clone());
    let body = sample_signal_json("worker-1");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/signal")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // NoopBackend is not ready — returns 503 so the caller knows to retry.
    assert_eq!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "NoopBackend should return 503 instead of silently dropping the signal"
    );
    let files_before: Vec<_> = std::fs::read_dir(&inbox_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        files_before.len(),
        0,
        "NoopBackend should not write any files"
    );

    // Swap to LocalFsBackend.
    let real_backend = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    *backend_lock.write().unwrap() = real_backend as Arc<dyn StateBackend>;

    // Now signal should succeed.
    let router = build_router(state);
    let body = sample_signal_json("worker-1");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/signal")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "LocalFsBackend should deliver signals after swap"
    );

    // Verify file appeared in inbox.
    let files: Vec<_> = std::fs::read_dir(&inbox_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        files.len(),
        1,
        "exactly one message file should appear in inbox after backend swap"
    );
}

#[tokio::test]
async fn test_shared_registry_between_state_instances() {
    // Prove that two structs sharing the same Arc<RunRegistry> see each other's registrations.
    let tmp = tempfile::tempdir().unwrap();
    let backend: Arc<dyn StateBackend> = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let registry = Arc::new(RunRegistry::new());
    let backend_lock: Arc<RwLock<Arc<dyn StateBackend>>> = Arc::new(RwLock::new(backend));

    let state = Arc::new(SignalServerState {
        backend: backend_lock.clone(),
        registry: registry.clone(),
        token: None,
        started_at: Instant::now(),
    });

    // Register through the shared registry (simulating what AssayServer handler does).
    let run_dir = tmp.path().join("orchestrator/run-shared");
    let inbox_dir = run_dir.join("mesh/agent-a/inbox");
    std::fs::create_dir_all(&inbox_dir).unwrap();

    registry.register_session(
        "agent-a".to_string(),
        RunEntry {
            run_id: "run-shared".to_string(),
            run_dir,
            spec_name: "test-spec".to_string(),
            started_at: Instant::now(),
            session_count: 1,
        },
    );

    // Signal through the state (simulating what signal_server does).
    let router = build_router(state);
    let body = sample_signal_json("agent-a");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/signal")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "signal should route through shared registry"
    );

    // Verify delivery.
    let files: Vec<_> = std::fs::read_dir(&inbox_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 1);
}

// ── ASSAY_SIGNAL_BIND tests ─────────────────────────────────────────

#[tokio::test]
async fn test_start_signal_server_bind_addr_localhost() {
    use assay_mcp::signal_server::start_signal_server;

    let tmp = tempfile::tempdir().unwrap();
    let backend: Arc<dyn StateBackend> = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let state = Arc::new(SignalServerState {
        backend: Arc::new(RwLock::new(backend)),
        registry: Arc::new(RunRegistry::new()),
        token: None,
        started_at: Instant::now(),
    });

    // Bind to 127.0.0.1 on a random high port.
    let result = start_signal_server(state, "127.0.0.1", 0).await;
    assert!(
        result.is_ok(),
        "start_signal_server should succeed on 127.0.0.1:0"
    );
}

#[tokio::test]
async fn test_start_signal_server_bind_addr_all_interfaces() {
    use assay_mcp::signal_server::start_signal_server;

    let tmp = tempfile::tempdir().unwrap();
    let backend: Arc<dyn StateBackend> = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let state = Arc::new(SignalServerState {
        backend: Arc::new(RwLock::new(backend)),
        registry: Arc::new(RunRegistry::new()),
        token: None,
        started_at: Instant::now(),
    });

    // Bind to 0.0.0.0 (all interfaces) on a random high port.
    let result = start_signal_server(state, "0.0.0.0", 0).await;
    assert!(
        result.is_ok(),
        "start_signal_server should succeed on 0.0.0.0:0"
    );
}

// ── poll_signals MCP tool tests ─────────────────────────────────────

use assay_mcp::{AssayServer, Parameters, PollSignalsParams, SendSignalParams};
use assay_types::PollSignalsResult;
use rmcp::model::RawContent;

/// Extract text content from a CallToolResult.
fn extract_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match &c.raw {
            RawContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

#[tokio::test]
async fn test_poll_signals_unknown_session() {
    // AssayServer::new() creates an empty registry — any session is unknown.
    let server = AssayServer::new();
    let params = Parameters(PollSignalsParams {
        session_name: "nonexistent".to_string(),
    });
    let result = server.poll_signals(params).await.unwrap();
    assert!(
        result.is_error.unwrap_or(false),
        "should be an error result"
    );
    let text = extract_text(&result);
    assert!(
        text.contains("session not found"),
        "error should mention session not found, got: {text}"
    );
}

#[tokio::test]
async fn test_poll_signals_empty_inbox() {
    let tmp = tempfile::tempdir().unwrap();
    let backend: Arc<dyn StateBackend> = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let registry = Arc::new(RunRegistry::new());
    let backend_lock = Arc::new(std::sync::RwLock::new(backend));

    // Register a session.
    let run_dir = tmp.path().join("orchestrator/run-001");
    let inbox_dir = run_dir.join("mesh/worker-1/inbox");
    std::fs::create_dir_all(&inbox_dir).unwrap();
    registry.register_session(
        "worker-1".to_string(),
        RunEntry {
            run_id: "run-001".to_string(),
            run_dir,
            spec_name: "test".to_string(),
            started_at: Instant::now(),
            session_count: 1,
        },
    );

    let server = AssayServer::new().with_signal_state(registry, backend_lock);
    let params = Parameters(PollSignalsParams {
        session_name: "worker-1".to_string(),
    });
    let result = server.poll_signals(params).await.unwrap();
    assert!(
        !result.is_error.unwrap_or(false),
        "empty inbox should not be an error"
    );
    let text = extract_text(&result);
    let poll_result: PollSignalsResult = serde_json::from_str(&text).unwrap();
    assert!(
        poll_result.signals.is_empty(),
        "empty inbox should return no signals"
    );
}

#[tokio::test]
async fn test_poll_signals_returns_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let backend: Arc<dyn StateBackend> = Arc::new(LocalFsBackend::new(tmp.path().to_path_buf()));
    let registry = Arc::new(RunRegistry::new());
    let backend_lock = Arc::new(std::sync::RwLock::new(backend.clone()));

    // Register a session.
    let run_dir = tmp.path().join("orchestrator/run-001");
    let inbox_dir = run_dir.join("mesh/worker-1/inbox");
    std::fs::create_dir_all(&inbox_dir).unwrap();
    registry.register_session(
        "worker-1".to_string(),
        RunEntry {
            run_id: "run-001".to_string(),
            run_dir,
            spec_name: "test".to_string(),
            started_at: Instant::now(),
            session_count: 1,
        },
    );

    // Write a PeerUpdate JSON file to the inbox (same as what handle_signal writes).
    let update = PeerUpdate {
        source_job: "job-abc".to_string(),
        source_session: "orchestrator".to_string(),
        changed_files: vec!["src/main.rs".to_string()],
        gate_summary: GateSummary {
            passed: 5,
            failed: 0,
            skipped: 1,
        },
        branch: "feature/test".to_string(),
    };
    let json_bytes = serde_json::to_vec(&update).unwrap();
    std::fs::write(inbox_dir.join("signal-12345"), &json_bytes).unwrap();

    let server = AssayServer::new().with_signal_state(registry, backend_lock);
    let params = Parameters(PollSignalsParams {
        session_name: "worker-1".to_string(),
    });
    let result = server.poll_signals(params).await.unwrap();
    assert!(
        !result.is_error.unwrap_or(false),
        "should succeed with messages"
    );
    let text = extract_text(&result);
    let poll_result: PollSignalsResult = serde_json::from_str(&text).unwrap();
    assert_eq!(poll_result.signals.len(), 1, "should have one signal");
    assert_eq!(poll_result.signals[0].source_job, "job-abc");
    assert_eq!(poll_result.signals[0].branch, "feature/test");
}

// ── send_signal MCP tool tests ──────────────────────────────────────

fn sample_peer_update() -> PeerUpdate {
    PeerUpdate {
        source_job: "job-abc".to_string(),
        source_session: "worker-1".to_string(),
        changed_files: vec!["src/main.rs".to_string()],
        gate_summary: GateSummary {
            passed: 5,
            failed: 0,
            skipped: 1,
        },
        branch: "feature/test".to_string(),
    }
}

#[tokio::test]
async fn test_send_signal_posts_correct_json() {
    let mut mock_server = mockito::Server::new_async().await;
    let mock = mock_server
        .mock("POST", "/api/v1/signal")
        .match_header("content-type", "application/json")
        .match_body(mockito::Matcher::PartialJsonString(
            r#"{"target_session":"worker-1"}"#.to_string(),
        ))
        .with_status(202)
        .with_body("accepted")
        .create_async()
        .await;

    let server = AssayServer::new();
    let params = Parameters(SendSignalParams {
        url: format!("{}/api/v1/signal", mock_server.url()),
        target_session: "worker-1".to_string(),
        update: sample_peer_update(),
    });
    let result = server.send_signal(params).await.unwrap();
    assert!(
        !result.is_error.unwrap_or(false),
        "should succeed: {}",
        extract_text(&result)
    );
    let text = extract_text(&result);
    assert!(
        text.contains("202"),
        "should contain status 202, got: {text}"
    );

    mock.assert_async().await;
}

#[tokio::test]
async fn test_send_signal_returns_non_2xx_as_result() {
    let mut mock_server = mockito::Server::new_async().await;
    let _mock = mock_server
        .mock("POST", "/api/v1/signal")
        .with_status(404)
        .with_body("not found")
        .create_async()
        .await;

    let server = AssayServer::new();
    let params = Parameters(SendSignalParams {
        url: format!("{}/api/v1/signal", mock_server.url()),
        target_session: "worker-1".to_string(),
        update: sample_peer_update(),
    });
    let result = server.send_signal(params).await.unwrap();
    // Non-2xx is NOT a tool error — it's returned as a success result with the status.
    assert!(
        !result.is_error.unwrap_or(false),
        "non-2xx should not be a tool error"
    );
    let text = extract_text(&result);
    assert!(
        text.contains("404"),
        "should contain status 404, got: {text}"
    );
}

#[tokio::test]
async fn test_send_signal_unreachable_url() {
    let server = AssayServer::new();
    let params = Parameters(SendSignalParams {
        url: "http://127.0.0.1:1/api/v1/signal".to_string(),
        target_session: "worker-1".to_string(),
        update: sample_peer_update(),
    });
    let result = server.send_signal(params).await.unwrap();
    assert!(
        result.is_error.unwrap_or(false),
        "unreachable URL should be a domain error"
    );
    let text = extract_text(&result);
    assert!(
        text.contains("send_signal failed"),
        "error should mention send_signal, got: {text}"
    );
}
