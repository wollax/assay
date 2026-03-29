#![cfg(feature = "smelt")]

//! Contract tests for `SmeltBackend`.
//!
//! These tests exercise [`StateBackend`] methods on `SmeltBackend` using
//! `mockito` as the mock HTTP server. No real Smelt server is contacted.
//!
//! The tests verify:
//! - Capability flags (`supports_signals: true`, `supports_annotations: true`)
//! - POST shape and URL (`/api/v1/events?job=<job_id>`)
//! - Bearer auth header presence/absence
//! - Graceful degradation on 500 and connection refused
//! - `annotate_run` tagged body
//! - Factory dispatch via `backend_from_config`

use assay_core::{CapabilitySet, StateBackend};
use assay_types::{FailurePolicy, OrchestratorPhase, OrchestratorStatus, StateBackendConfig};
use mockito::{Matcher, Server};

use assay_backends::factory::backend_from_config;
use assay_backends::smelt::SmeltBackend;

/// Helper: build a minimal `OrchestratorStatus` for testing.
fn sample_status() -> OrchestratorStatus {
    OrchestratorStatus {
        run_id: "test-run-001".to_string(),
        phase: OrchestratorPhase::Running,
        failure_policy: FailurePolicy::SkipDependents,
        sessions: vec![],
        started_at: chrono::Utc::now(),
        completed_at: None,
        mesh_status: None,
        gossip_status: None,
    }
}

/// Helper: construct a `SmeltBackend` pointed at a mockito server with auth.
fn make_backend(server: &Server) -> SmeltBackend {
    SmeltBackend::new(
        server.url(),
        "test-job".to_string(),
        Some("test-token".to_string()),
    )
}

// ── Capability flags ──────────────────────────────────────────────────

#[test]
fn test_capabilities_returns_correct_flags() {
    let server = Server::new();
    let backend = make_backend(&server);
    let caps = backend.capabilities();
    assert_eq!(
        caps,
        CapabilitySet {
            supports_signals: true,
            supports_messaging: false,
            supports_gossip_manifest: false,
            supports_annotations: true,
            supports_checkpoints: false,
            supports_peer_registry: true,
        }
    );
}

// ── push_session_event ────────────────────────────────────────────────

#[test]
fn test_push_session_event_posts_to_correct_url() {
    let mut server = Server::new();
    let run_dir = tempfile::tempdir().unwrap();

    let mock = server
        .mock("POST", "/api/v1/events")
        .match_query(Matcher::UrlEncoded("job".into(), "test-job".into()))
        .match_body(Matcher::Regex("run_id".to_string()))
        .with_status(200)
        .create();

    let backend = make_backend(&server);
    let status = sample_status();

    backend
        .push_session_event(run_dir.path(), &status)
        .expect("push_session_event should succeed");

    mock.assert();
}

#[test]
fn test_push_session_event_sends_auth_header() {
    let mut server = Server::new();
    let run_dir = tempfile::tempdir().unwrap();

    let mock = server
        .mock("POST", "/api/v1/events")
        .match_query(Matcher::UrlEncoded("job".into(), "test-job".into()))
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .create();

    let backend = make_backend(&server);
    let status = sample_status();

    backend
        .push_session_event(run_dir.path(), &status)
        .expect("push_session_event should succeed");

    mock.assert();
}

#[test]
fn test_push_session_event_no_auth_when_no_token() {
    let mut server = Server::new();
    let run_dir = tempfile::tempdir().unwrap();

    let mock = server
        .mock("POST", "/api/v1/events")
        .match_query(Matcher::UrlEncoded("job".into(), "test-job".into()))
        .match_header("authorization", Matcher::Missing)
        .with_status(200)
        .create();

    let backend = SmeltBackend::new(server.url(), "test-job".to_string(), None);
    let status = sample_status();

    backend
        .push_session_event(run_dir.path(), &status)
        .expect("push_session_event should succeed without auth");

    mock.assert();
}

#[test]
fn test_push_session_event_graceful_on_500() {
    let mut server = Server::new();
    let run_dir = tempfile::tempdir().unwrap();

    let mock = server
        .mock("POST", "/api/v1/events")
        .match_query(Matcher::UrlEncoded("job".into(), "test-job".into()))
        .with_status(500)
        .with_body("Internal Server Error")
        .create();

    let backend = make_backend(&server);
    let status = sample_status();

    let result = backend.push_session_event(run_dir.path(), &status);
    assert!(
        result.is_ok(),
        "push_session_event should return Ok(()) on 500 (D190 graceful degradation)"
    );

    mock.assert();
}

#[test]
fn test_push_session_event_graceful_on_connection_refused() {
    let run_dir = tempfile::tempdir().unwrap();

    // Port 1 should be unreachable
    let backend = SmeltBackend::new(
        "http://127.0.0.1:1".to_string(),
        "test-job".to_string(),
        None,
    );
    let status = sample_status();

    let result = backend.push_session_event(run_dir.path(), &status);
    assert!(
        result.is_ok(),
        "push_session_event should return Ok(()) on connection refused (D190 graceful degradation)"
    );
}

// ── annotate_run ──────────────────────────────────────────────────────

#[test]
fn test_annotate_run_posts_tagged_body() {
    let mut server = Server::new();
    let run_dir = tempfile::tempdir().unwrap();

    let mock = server
        .mock("POST", "/api/v1/events")
        .match_query(Matcher::UrlEncoded("job".into(), "test-job".into()))
        .match_body(Matcher::Regex(r"\[assay:manifest\]".to_string()))
        .with_status(200)
        .create();

    let backend = make_backend(&server);

    backend
        .annotate_run(run_dir.path(), "spec.toml")
        .expect("annotate_run should succeed");

    mock.assert();
}

// ── Factory dispatch ──────────────────────────────────────────────────

#[test]
fn test_factory_dispatch_creates_smelt_backend() {
    let dir = tempfile::tempdir().unwrap();
    let config = StateBackendConfig::Smelt {
        url: "http://localhost:9000".to_string(),
        job_id: "job-1".to_string(),
        token: None,
    };
    let backend = backend_from_config(&config, dir.path().to_path_buf());
    let caps = backend.capabilities();
    assert!(
        caps.supports_signals,
        "factory-created Smelt backend should have supports_signals: true"
    );
    assert!(
        caps.supports_annotations,
        "factory-created Smelt backend should have supports_annotations: true"
    );
    assert!(!caps.supports_messaging);
    assert!(!caps.supports_gossip_manifest);
    assert!(!caps.supports_checkpoints);
}

// ── Peer registry tests ─────────────────────────────────────────────

fn sample_peer_info() -> assay_types::PeerInfo {
    assay_types::PeerInfo {
        peer_id: "test-peer-1".to_string(),
        signal_url: "http://localhost:7432".to_string(),
        registered_at: chrono::Utc::now(),
    }
}

#[test]
fn test_register_peer_posts_to_peers_endpoint() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/api/v1/peers")
        .match_body(Matcher::PartialJsonString(
            r#"{"peer_id":"test-peer-1"}"#.to_string(),
        ))
        .with_status(201)
        .create();

    let backend = SmeltBackend::new(
        server.url(),
        "test-job".to_string(),
        Some("test-token".to_string()),
    );

    let result = backend.register_peer(&sample_peer_info());
    assert!(result.is_ok());
    mock.assert();
}

#[test]
fn test_register_peer_graceful_on_failure() {
    let mut server = Server::new();
    let _mock = server
        .mock("POST", "/api/v1/peers")
        .with_status(500)
        .with_body("internal server error")
        .create();

    let backend = SmeltBackend::new(
        server.url(),
        "test-job".to_string(),
        Some("test-token".to_string()),
    );

    let result = backend.register_peer(&sample_peer_info());
    assert!(result.is_ok(), "should degrade gracefully on 500");
}
