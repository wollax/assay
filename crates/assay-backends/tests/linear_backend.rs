#![cfg(feature = "linear")]

//! Contract tests for `LinearBackend`.
//!
//! These tests exercise every [`StateBackend`] method on `LinearBackend` using
//! `mockito` as the mock HTTP server. No real Linear API calls are made.
//!
//! The tests verify:
//! - GraphQL request shapes (mutations and queries)
//! - `.linear-issue-id` file lifecycle (create on first push, read on subsequent)
//! - Capability flags (D164)
//! - Error handling (missing API key, GraphQL error responses)

use std::fs;
use std::path::Path;

use assay_core::{CapabilitySet, StateBackend};
use assay_types::{FailurePolicy, OrchestratorPhase, OrchestratorStatus};
use mockito::{Matcher, Server};

// The module under test — will not compile until T02 implements it.
use assay_backends::linear::LinearBackend;

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

/// Helper: construct a `LinearBackend` pointed at a mockito server.
fn make_backend(server: &Server) -> LinearBackend {
    LinearBackend::new(
        "fake-api-key-for-testing".to_string(),
        server.url(),
        "TEAM_ID".to_string(),
        Some("PROJECT_ID".to_string()),
    )
}

// ── Capability flags ──────────────────────────────────────────────────

#[test]
fn test_capabilities_returns_d164_flags() {
    let server = Server::new();
    let backend = make_backend(&server);
    let caps = backend.capabilities();
    assert_eq!(
        caps,
        CapabilitySet {
            supports_messaging: false,
            supports_gossip_manifest: false,
            supports_annotations: true,
            supports_checkpoints: false,
        }
    );
}

// ── push_session_event ────────────────────────────────────────────────

#[test]
fn test_push_first_event_creates_issue() {
    let mut server = Server::new();
    let run_dir = tempfile::tempdir().unwrap();

    // Mock the issueCreate mutation
    let mock = server
        .mock("POST", "/graphql")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex("issueCreate".to_string()),
            Matcher::Regex("IssueCreateInput".to_string()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "issueCreate": {
                        "success": true,
                        "issue": { "id": "issue-uuid-123" }
                    }
                }
            }"#,
        )
        .create();

    let backend = make_backend(&server);
    let status = sample_status();

    backend
        .push_session_event(run_dir.path(), &status)
        .expect("push_session_event should succeed");

    // Verify .linear-issue-id was written
    let issue_id_path = run_dir.path().join(".linear-issue-id");
    assert!(issue_id_path.exists(), ".linear-issue-id file should exist");
    let stored_id = fs::read_to_string(&issue_id_path).unwrap();
    assert_eq!(stored_id.trim(), "issue-uuid-123");

    mock.assert();
}

#[test]
fn test_push_subsequent_event_creates_comment() {
    let mut server = Server::new();
    let run_dir = tempfile::tempdir().unwrap();

    // Pre-write .linear-issue-id (simulates a prior push)
    fs::write(
        run_dir.path().join(".linear-issue-id"),
        "existing-issue-456",
    )
    .unwrap();

    // Mock the commentCreate mutation
    let mock = server
        .mock("POST", "/graphql")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex("commentCreate".to_string()),
            Matcher::Regex("CommentCreateInput".to_string()),
            Matcher::Regex("existing-issue-456".to_string()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "commentCreate": {
                        "success": true,
                        "comment": { "id": "comment-uuid-789", "body": "{}" }
                    }
                }
            }"#,
        )
        .create();

    let backend = make_backend(&server);
    let status = sample_status();

    backend
        .push_session_event(run_dir.path(), &status)
        .expect("push_session_event should succeed for subsequent event");

    mock.assert();
}

// ── read_run_state ────────────────────────────────────────────────────

#[test]
fn test_read_run_state_deserializes_latest_comment() {
    let mut server = Server::new();
    let run_dir = tempfile::tempdir().unwrap();

    // Pre-write .linear-issue-id
    fs::write(
        run_dir.path().join(".linear-issue-id"),
        "issue-for-read-state",
    )
    .unwrap();

    let status = sample_status();
    let status_json = serde_json::to_string(&status).unwrap();

    // Mock the issue comments query
    let mock = server
        .mock("POST", "/graphql")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex("issue".to_string()),
            Matcher::Regex("comments".to_string()),
            Matcher::Regex("issue-for-read-state".to_string()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{
                "data": {{
                    "issue": {{
                        "comments": {{
                            "nodes": [
                                {{ "body": {} }}
                            ]
                        }}
                    }}
                }}
            }}"#,
            serde_json::to_string(&status_json).unwrap()
        ))
        .create();

    let backend = make_backend(&server);

    let result = backend
        .read_run_state(run_dir.path())
        .expect("read_run_state should succeed");

    assert!(result.is_some(), "should return Some status");
    let returned = result.unwrap();
    assert_eq!(returned.run_id, "test-run-001");
    assert_eq!(returned.phase, OrchestratorPhase::Running);

    mock.assert();
}

#[test]
fn test_read_run_state_returns_none_when_no_issue() {
    let run_dir = tempfile::tempdir().unwrap();
    // No .linear-issue-id file — fresh run_dir

    let server = Server::new();
    let backend = make_backend(&server);

    let result = backend
        .read_run_state(run_dir.path())
        .expect("read_run_state should return Ok");

    assert!(result.is_none(), "should return None when no issue exists");
}

// ── annotate_run ──────────────────────────────────────────────────────

#[test]
fn test_annotate_run_posts_tagged_comment() {
    let mut server = Server::new();
    let run_dir = tempfile::tempdir().unwrap();

    // Pre-write .linear-issue-id
    fs::write(
        run_dir.path().join(".linear-issue-id"),
        "issue-for-annotation",
    )
    .unwrap();

    // Mock expects a commentCreate with body starting with [assay:manifest]
    let mock = server
        .mock("POST", "/graphql")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex("commentCreate".to_string()),
            Matcher::Regex(r"\[assay:manifest\]".to_string()),
            Matcher::Regex("issue-for-annotation".to_string()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "commentCreate": {
                        "success": true,
                        "comment": { "id": "annotation-comment-id", "body": "[assay:manifest] /path/to/manifest" }
                    }
                }
            }"#,
        )
        .create();

    let backend = make_backend(&server);

    backend
        .annotate_run(run_dir.path(), "/path/to/manifest")
        .expect("annotate_run should succeed");

    mock.assert();
}

// ── Construction errors ───────────────────────────────────────────────

#[test]
fn test_construction_fails_without_api_key() {
    // LinearBackend::from_env should fail when LINEAR_API_KEY is not set.
    // We test the from_env constructor which reads from the environment.
    // Temporarily ensure the key is absent by using a constructor that
    // validates the key.
    let result = LinearBackend::from_env(
        "https://api.linear.app".to_string(),
        "TEAM".to_string(),
        None,
    );
    // This may or may not fail depending on whether LINEAR_API_KEY is set
    // in the test environment. The contract is that if it's absent, we get
    // an error mentioning LINEAR_API_KEY.
    // In CI / clean environments, it should be absent.
    if std::env::var("LINEAR_API_KEY").is_err() {
        assert!(result.is_err(), "should fail without LINEAR_API_KEY");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("LINEAR_API_KEY"),
            "error should mention LINEAR_API_KEY, got: {err}"
        );
    }
}

// ── GraphQL error handling ────────────────────────────────────────────

#[test]
fn test_push_handles_graphql_error_response() {
    let mut server = Server::new();
    let run_dir = tempfile::tempdir().unwrap();

    // Mock returns 200 but with GraphQL errors
    let mock = server
        .mock("POST", "/graphql")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": null,
                "errors": [
                    {
                        "message": "Team not found",
                        "extensions": { "code": "NOT_FOUND" }
                    }
                ]
            }"#,
        )
        .create();

    let backend = make_backend(&server);
    let status = sample_status();

    let result = backend.push_session_event(run_dir.path(), &status);

    assert!(result.is_err(), "should surface GraphQL errors");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Team not found") || err.contains("GraphQL"),
        "error should contain the GraphQL error message, got: {err}"
    );

    mock.assert();
}
