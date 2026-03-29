//! Tests for the signal_server HTTP endpoint.
//!
//! Uses `tower::ServiceExt::oneshot` to exercise axum handlers without
//! a real TCP listener.

use std::sync::Arc;
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
        backend,
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
