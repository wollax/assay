use crate::serve::signals::{
    GateSummary, PeerUpdate, SignalRequest, deliver_peer_update, validate_run_id,
};
use std::path::{Path, PathBuf};

/// Helper: build a sample PeerUpdate for tests.
fn sample_peer_update() -> PeerUpdate {
    PeerUpdate {
        source_job: "job-alpha".to_string(),
        source_session: "session-1".to_string(),
        changed_files: vec!["src/main.rs".to_string(), "README.md".to_string()],
        gate_summary: GateSummary {
            passed: 3,
            failed: 0,
            skipped: 1,
        },
        branch: "results/job-alpha".to_string(),
    }
}

#[test]
fn test_deliver_peer_update_writes_correct_json() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_path = tmp.path();
    let run_id = "01TESTRUN123";
    let session_name = "agent-1";
    let peer_update = sample_peer_update();

    let written_path = deliver_peer_update(repo_path, run_id, session_name, &peer_update).unwrap();

    // File should exist at the returned path.
    assert!(written_path.exists(), "written file should exist");

    // File should be inside the expected inbox directory.
    let expected_inbox = repo_path
        .join(".assay/orchestrator")
        .join(run_id)
        .join("mesh")
        .join(session_name)
        .join("inbox");
    assert!(
        written_path.starts_with(&expected_inbox),
        "written file should be under inbox dir: {}",
        written_path.display()
    );

    // File should have a .json extension.
    assert_eq!(
        written_path.extension().and_then(|e| e.to_str()),
        Some("json")
    );

    // Read back and deserialize — all fields must match.
    let content = std::fs::read_to_string(&written_path).unwrap();
    let deserialized: PeerUpdate = serde_json::from_str(&content).unwrap();
    assert_eq!(deserialized, peer_update);
}

#[test]
fn test_deliver_peer_update_creates_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    // Use a nested path that doesn't exist yet.
    let repo_path = tmp.path().join("deep/nested/repo");
    let run_id = "01RUNID456";
    let session_name = "worker-2";
    let peer_update = sample_peer_update();

    let written_path = deliver_peer_update(&repo_path, run_id, session_name, &peer_update).unwrap();

    assert!(
        written_path.exists(),
        "file should exist even with deep dirs"
    );

    // The full inbox directory tree should have been created.
    let inbox_dir = repo_path
        .join(".assay/orchestrator")
        .join(run_id)
        .join("mesh")
        .join(session_name)
        .join("inbox");
    assert!(inbox_dir.is_dir(), "inbox directory should exist");
}

#[test]
fn test_deliver_peer_update_path_traversal_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let peer_update = sample_peer_update();

    let bad_names = vec![
        ("../evil", "path traversal with .."),
        ("/slash", "absolute path component"),
        ("\\back", "backslash"),
        (".", "dot"),
        ("..", "double dot"),
        ("", "empty string"),
    ];

    for (bad_name, desc) in bad_names {
        let result = deliver_peer_update(tmp.path(), "run-1", bad_name, &peer_update);
        assert!(
            result.is_err(),
            "session name {desc} ({bad_name:?}) should be rejected"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::InvalidInput,
            "error kind should be InvalidInput for {desc}"
        );
    }
}

#[test]
fn test_deliver_peer_update_run_id_traversal_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let peer_update = sample_peer_update();

    let bad_run_ids = vec![
        ("../evil", "path traversal with .."),
        ("/absolute", "absolute path"),
        ("a/b", "contains slash"),
        ("\\back", "backslash"),
        (".", "dot"),
        ("..", "double dot"),
        ("", "empty string"),
    ];

    for (bad_id, desc) in bad_run_ids {
        // validate_run_id should reject it.
        assert!(
            validate_run_id(bad_id).is_err(),
            "run_id {desc} ({bad_id:?}) should fail validate_run_id"
        );
        // deliver_peer_update should also reject it.
        let result = deliver_peer_update(tmp.path(), bad_id, "agent-1", &peer_update);
        assert!(
            result.is_err(),
            "run_id {desc} ({bad_id:?}) should be rejected by deliver_peer_update"
        );
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
    }
}

// ─── Integration test helpers ──────────────────────────────────────────

/// Create a manifest TOML string that points `job.repo` to the given path.
fn manifest_toml_with_repo(repo_path: &Path) -> String {
    format!(
        r#"[job]
name = "signal-test-job"
repo = "{}"
base_ref = "main"

[environment]
runtime = "docker"
image = "alpine:3.18"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "main"
spec = "Run test"
harness = "echo hello"
timeout = 300

[merge]
strategy = "sequential"
target = "main"
"#,
        repo_path.display()
    )
}

/// Set up a test server with a job whose manifest points `repo` to a tempdir.
/// Returns (base_url, job_id, repo_tempdir).
/// The manifest is written to a file in a separate tempdir so it persists.
async fn setup_signal_test_server() -> (
    String,
    String,
    tempfile::TempDir,
    tempfile::TempDir,
    std::sync::Arc<std::sync::Mutex<crate::serve::queue::ServerState>>,
) {
    use crate::serve::types::{JobId, JobSource, JobStatus, QueuedJob, now_epoch};

    let repo_dir = tempfile::tempdir().unwrap();
    let manifest_dir = tempfile::tempdir().unwrap();

    let manifest_content = manifest_toml_with_repo(repo_dir.path());
    let manifest_path = manifest_dir.path().join("signal-test.smelt.toml");
    std::fs::write(&manifest_path, &manifest_content).unwrap();

    let state = std::sync::Arc::new(std::sync::Mutex::new(
        crate::serve::queue::ServerState::new_without_events(4),
    ));

    let job_id = {
        let mut s = state.lock().unwrap();
        s.jobs.push_back(QueuedJob {
            id: JobId::new("signal-job-1"),
            manifest_path,
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
            worker_host: None,
        });
        "signal-job-1".to_string()
    };

    let base = super::start_test_server(state.clone()).await;
    (base, job_id, repo_dir, manifest_dir, state)
}

// ─── HTTP integration tests ────────────────────────────────────────────

#[tokio::test]
async fn test_signal_http_round_trip() {
    let (base, job_id, repo_dir, _manifest_dir, _state) = setup_signal_test_server().await;
    let client = reqwest::Client::new();

    // First, POST an event with run_id so the signal handler can resolve the inbox path.
    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": job_id,
            "run_id": "01TESTRUN999",
            "phase": "running",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "event POST should succeed");

    // Now POST a signal (SignalRequest format).
    let signal_body = serde_json::json!({
        "target_session": "agent-1",
        "update": {
            "source_job": "job-alpha",
            "source_session": "session-1",
            "changed_files": ["src/main.rs", "README.md"],
            "gate_summary": { "passed": 3, "failed": 0, "skipped": 1 },
            "branch": "results/job-alpha"
        }
    });

    let resp = client
        .post(format!("{base}/api/v1/jobs/{job_id}/signals"))
        .json(&signal_body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "signal POST should succeed");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    let written_path = body["path"].as_str().expect("should have path in response");

    // Verify the file exists at the returned path.
    assert!(
        std::path::Path::new(written_path).exists(),
        "signal file should exist at {written_path}"
    );

    // Verify the file is in the correct inbox directory.
    // Use canonicalize on repo_dir because resolve_repo_path canonicalizes
    // (e.g. /tmp → /private/tmp on macOS).
    let canonical_repo = std::fs::canonicalize(repo_dir.path()).unwrap();
    let expected_inbox = canonical_repo.join(".assay/orchestrator/01TESTRUN999/mesh/agent-1/inbox");
    assert!(
        written_path.starts_with(expected_inbox.to_string_lossy().as_ref()),
        "file should be under inbox dir, got: {written_path}, expected prefix: {}",
        expected_inbox.display()
    );

    // Verify file content matches the PeerUpdate.
    let content = std::fs::read_to_string(written_path).unwrap();
    let deserialized: PeerUpdate = serde_json::from_str(&content).unwrap();
    assert_eq!(deserialized.source_job, "job-alpha");
    assert_eq!(deserialized.source_session, "session-1");
    assert_eq!(deserialized.changed_files, vec!["src/main.rs", "README.md"]);
    assert_eq!(
        deserialized.gate_summary,
        GateSummary {
            passed: 3,
            failed: 0,
            skipped: 1
        }
    );
    assert_eq!(deserialized.branch, "results/job-alpha");
}

#[tokio::test]
async fn test_signal_unknown_job_404() {
    let (base, _job_id, _repo_dir, _manifest_dir, _state) = setup_signal_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/jobs/nonexistent-job/signals"))
        .json(&serde_json::json!({
            "target_session": "agent-1",
            "update": {
                "source_job": "x",
                "source_session": "x",
                "changed_files": [],
                "gate_summary": { "passed": 0, "failed": 0, "skipped": 0 },
                "branch": "x"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404, "unknown job should return 404");
}

#[tokio::test]
async fn test_signal_no_run_id_409() {
    let (base, job_id, _repo_dir, _manifest_dir, _state) = setup_signal_test_server().await;
    let client = reqwest::Client::new();

    // Don't POST any event — run_id is unknown.
    let resp = client
        .post(format!("{base}/api/v1/jobs/{job_id}/signals"))
        .json(&serde_json::json!({
            "target_session": "agent-1",
            "update": {
                "source_job": "x",
                "source_session": "x",
                "changed_files": [],
                "gate_summary": { "passed": 0, "failed": 0, "skipped": 0 },
                "branch": "x"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        409,
        "signal before first event should return 409"
    );

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["error"].as_str().unwrap().contains("no run_id known"),
        "409 error should mention run_id"
    );
}

#[tokio::test]
async fn test_signal_auth() {
    use crate::serve::http_api::ResolvedAuth;
    use crate::serve::types::{JobId, JobSource, JobStatus, QueuedJob, now_epoch};

    let state = std::sync::Arc::new(std::sync::Mutex::new(
        crate::serve::queue::ServerState::new_without_events(4),
    ));
    {
        let mut s = state.lock().unwrap();
        s.jobs.push_back(QueuedJob {
            id: JobId::new("auth-job"),
            manifest_path: PathBuf::from("test.toml"),
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
            worker_host: None,
        });
    }

    let auth = ResolvedAuth {
        write_token: "write-secret".to_string(),
        read_token: Some("read-secret".to_string()),
    };

    let base = super::start_test_server_with_auth(state, Some(auth)).await;
    let client = reqwest::Client::new();

    // POST without any auth header → 401.
    let resp = client
        .post(format!("{base}/api/v1/jobs/auth-job/signals"))
        .json(&serde_json::json!({
            "target_session": "a",
            "update": {
                "source_job": "x",
                "source_session": "x",
                "changed_files": [],
                "gate_summary": { "passed": 0, "failed": 0, "skipped": 0 },
                "branch": "x"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401, "signal without auth should return 401");

    // POST with read token → 403 (write operation).
    let resp = client
        .post(format!("{base}/api/v1/jobs/auth-job/signals"))
        .header("Authorization", "Bearer read-secret")
        .json(&serde_json::json!({
            "target_session": "a",
            "update": {
                "source_job": "x",
                "source_session": "x",
                "changed_files": [],
                "gate_summary": { "passed": 0, "failed": 0, "skipped": 0 },
                "branch": "x"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        403,
        "signal with read token should return 403"
    );

    // POST with write token → passes auth (will return 409 since no run_id cached,
    // but confirms auth is not blocking the request).
    let resp = client
        .post(format!("{base}/api/v1/jobs/auth-job/signals"))
        .header("Authorization", "Bearer write-secret")
        .json(&serde_json::json!({
            "target_session": "a",
            "update": {
                "source_job": "x",
                "source_session": "x",
                "changed_files": [],
                "gate_summary": { "passed": 0, "failed": 0, "skipped": 0 },
                "branch": "x"
            }
        }))
        .send()
        .await
        .unwrap();

    // 409 = auth passed, request reached validation (no run_id cached yet).
    assert_ne!(resp.status(), 401, "write token should not return 401");
    assert_ne!(resp.status(), 403, "write token should not return 403");
}

#[tokio::test]
async fn test_signal_invalid_session_name_returns_400() {
    let (base, job_id, _repo_dir, _manifest_dir, state) = setup_signal_test_server().await;
    let client = reqwest::Client::new();

    // Seed a run_id so we reach the session_name validation branch.
    client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": job_id,
            "run_id": "01VALID",
            "phase": "running",
        }))
        .send()
        .await
        .unwrap();

    let bad_session_names = ["../evil", "/absolute", "a/b", "\\back", ".", "..", ""];

    for bad_name in &bad_session_names {
        let resp = client
            .post(format!("{base}/api/v1/jobs/{job_id}/signals"))
            .json(&serde_json::json!({
                "target_session": bad_name,
                "update": {
                    "source_job": "x",
                    "source_session": "x",
                    "changed_files": [],
                    "gate_summary": { "passed": 0, "failed": 0, "skipped": 0 },
                    "branch": "x"
                }
            }))
            .send()
            .await
            .unwrap();

        let expected = 400u16;
        assert_eq!(
            resp.status(),
            expected,
            "session_name {bad_name:?} should return 400"
        );
    }
    // Suppress unused warning for state
    let _ = state;
}

// ─── Pipeline tests (T03) ──────────────────────────────────────────────

#[tokio::test]
async fn test_signal_full_pipeline() {
    let (base, job_id, repo_dir, _manifest_dir, _state) = setup_signal_test_server().await;
    let client = reqwest::Client::new();

    let run_id = "01ABC123";

    // Step 1: POST event with run_id — this caches the run_id in ServerState.
    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": job_id,
            "run_id": run_id,
            "phase": "running",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "event POST should succeed");

    // Step 2: POST signal — uses the cached run_id to resolve inbox path.
    let resp = client
        .post(format!("{base}/api/v1/jobs/{job_id}/signals"))
        .json(&serde_json::json!({
            "target_session": "agent-1",
            "update": {
                "source_job": "job-a",
                "source_session": "session-1",
                "changed_files": ["src/main.rs"],
                "gate_summary": { "passed": 1, "failed": 0, "skipped": 0 },
                "branch": "results/job-a"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "signal POST should succeed");

    // Step 3: Verify the file on disk.
    // Canonicalize because resolve_repo_path canonicalizes (/tmp → /private/tmp on macOS).
    let canonical_repo = std::fs::canonicalize(repo_dir.path()).unwrap();
    let inbox_dir = canonical_repo
        .join(".assay/orchestrator")
        .join(run_id)
        .join("mesh/agent-1/inbox");

    assert!(inbox_dir.is_dir(), "inbox directory should exist");

    let entries: Vec<_> = std::fs::read_dir(&inbox_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1, "exactly one signal file should exist");

    let file_content = std::fs::read_to_string(entries[0].path()).unwrap();
    let peer_update: PeerUpdate = serde_json::from_str(&file_content).unwrap();
    assert_eq!(peer_update.source_job, "job-a");
    assert_eq!(peer_update.source_session, "session-1");
    assert_eq!(peer_update.changed_files, vec!["src/main.rs"]);
    assert_eq!(
        peer_update.gate_summary,
        GateSummary {
            passed: 1,
            failed: 0,
            skipped: 0
        }
    );
    assert_eq!(peer_update.branch, "results/job-a");
}

#[tokio::test]
async fn test_signal_run_id_updates_on_new_event() {
    let (base, job_id, repo_dir, _manifest_dir, _state) = setup_signal_test_server().await;
    let client = reqwest::Client::new();

    // POST first event with run_id "AAA".
    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": job_id,
            "run_id": "AAA",
            "phase": "running",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // POST second event with run_id "BBB" — should overwrite "AAA".
    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": job_id,
            "run_id": "BBB",
            "phase": "completed",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // POST signal — should use "BBB", not "AAA".
    let resp = client
        .post(format!("{base}/api/v1/jobs/{job_id}/signals"))
        .json(&serde_json::json!({
            "target_session": "worker-1",
            "update": {
                "source_job": "x",
                "source_session": "x",
                "changed_files": [],
                "gate_summary": { "passed": 0, "failed": 0, "skipped": 0 },
                "branch": "x"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "signal should succeed with updated run_id"
    );

    let body: serde_json::Value = resp.json().await.unwrap();
    let written_path = body["path"].as_str().unwrap();

    // The path should contain "BBB" (the second run_id), not "AAA".
    assert!(
        written_path.contains("/BBB/"),
        "signal path should use updated run_id 'BBB', got: {written_path}"
    );

    // Also verify "AAA" directory was NOT created.
    let canonical_repo = std::fs::canonicalize(repo_dir.path()).unwrap();
    let aaa_dir = canonical_repo.join(".assay/orchestrator/AAA");
    assert!(
        !aaa_dir.exists(),
        "AAA directory should not exist — run_id was overwritten to BBB"
    );
}

// ─── Wire format tests (D012 — canonical types from assay-types) ─────────────
//
// These tests verify two things:
//   1. The canonical types round-trip through JSON correctly (serde sanity).
//   2. The wire-format JSON contains the exact field names Smelt emits and Assay
//      expects. Asserting on literal JSON keys provides the same protection the
//      old mirror-struct tests did: if assay-types renames a field, these checks
//      fail immediately, catching drift before it silently breaks serialization.

#[test]
fn test_gate_summary_schema_round_trip() {
    let original = GateSummary {
        passed: 5,
        failed: 2,
        skipped: 1,
    };
    let json = serde_json::to_string(&original).unwrap();

    // Wire-format assertions: exact field names must match the Smelt/Assay protocol.
    assert!(
        json.contains("\"passed\""),
        "GateSummary must serialize 'passed' field"
    );
    assert!(
        json.contains("\"failed\""),
        "GateSummary must serialize 'failed' field"
    );
    assert!(
        json.contains("\"skipped\""),
        "GateSummary must serialize 'skipped' field"
    );

    let deserialized: GateSummary =
        serde_json::from_str(&json).expect("GateSummary round-trip failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_peer_update_schema_round_trip() {
    let original = PeerUpdate {
        source_job: "job-1".into(),
        source_session: "session-1".into(),
        changed_files: vec!["a.rs".into(), "b.rs".into()],
        gate_summary: GateSummary {
            passed: 3,
            failed: 1,
            skipped: 0,
        },
        branch: "results/job-1".into(),
    };
    let json = serde_json::to_string(&original).unwrap();

    // Wire-format assertions: exact field names must match the Smelt/Assay protocol.
    assert!(
        json.contains("\"source_job\""),
        "PeerUpdate must serialize 'source_job'"
    );
    assert!(
        json.contains("\"source_session\""),
        "PeerUpdate must serialize 'source_session'"
    );
    assert!(
        json.contains("\"changed_files\""),
        "PeerUpdate must serialize 'changed_files'"
    );
    assert!(
        json.contains("\"gate_summary\""),
        "PeerUpdate must serialize 'gate_summary'"
    );
    assert!(
        json.contains("\"branch\""),
        "PeerUpdate must serialize 'branch'"
    );

    let deserialized: PeerUpdate =
        serde_json::from_str(&json).expect("PeerUpdate round-trip failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_signal_request_schema_round_trip() {
    let original = SignalRequest {
        target_session: "agent-1".into(),
        update: PeerUpdate {
            source_job: "job-1".into(),
            source_session: "session-1".into(),
            changed_files: vec!["src/main.rs".into()],
            gate_summary: GateSummary {
                passed: 1,
                failed: 0,
                skipped: 0,
            },
            branch: "main".into(),
        },
    };
    let json = serde_json::to_string(&original).unwrap();

    // Wire-format assertions: exact field names must match the Smelt/Assay protocol.
    assert!(
        json.contains("\"target_session\""),
        "SignalRequest must serialize 'target_session'"
    );
    assert!(
        json.contains("\"update\""),
        "SignalRequest must serialize 'update'"
    );

    let deserialized: SignalRequest =
        serde_json::from_str(&json).expect("SignalRequest round-trip failed");
    assert_eq!(deserialized, original);
}

// ─── HTTP signal delivery tests ────────────────────────────────────────

use crate::serve::signals::{deliver_signal_http, make_signal_client};
use axum::Json as AxumJson;

fn sample_signal_request() -> SignalRequest {
    SignalRequest {
        target_session: "agent-1".into(),
        update: sample_peer_update(),
    }
}

/// Start a minimal mock signal server that captures the received body.
/// Returns (url, captured_body_handle).
async fn start_mock_signal_server() -> (
    String,
    std::sync::Arc<tokio::sync::Mutex<Option<SignalRequest>>>,
) {
    use axum::{Router, http::StatusCode, routing::post};

    let captured: std::sync::Arc<tokio::sync::Mutex<Option<SignalRequest>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let captured_clone = captured.clone();

    let app = Router::new().route(
        "/api/v1/signal",
        post(move |AxumJson(body): AxumJson<SignalRequest>| {
            let cap = captured_clone.clone();
            async move {
                *cap.lock().await = Some(body);
                StatusCode::ACCEPTED
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://127.0.0.1:{}", addr.port()), captured)
}

#[tokio::test]
async fn test_deliver_signal_http_success() {
    let (url, captured) = start_mock_signal_server().await;
    let client = make_signal_client();
    let signal = sample_signal_request();

    let status = deliver_signal_http(&client, &format!("{url}/api/v1/signal"), &signal, None)
        .await
        .expect("HTTP delivery should succeed");

    assert_eq!(status, reqwest::StatusCode::ACCEPTED);

    // Verify the mock received the correct body.
    let received = captured.lock().await;
    let received = received.as_ref().expect("mock should have captured a body");
    assert_eq!(received.target_session, "agent-1");
    assert_eq!(received.update.source_job, "job-alpha");
    assert_eq!(received.update.branch, "results/job-alpha");
    assert_eq!(received.update.gate_summary.passed, 3);
}

#[tokio::test]
async fn test_deliver_signal_http_with_auth() {
    use axum::{Router, http::StatusCode, routing::post};

    let app = Router::new().route(
        "/api/v1/signal",
        post(
            |headers: axum::http::HeaderMap,
             AxumJson(_body): AxumJson<serde_json::Value>| async move {
                match headers.get("authorization") {
                    Some(val) if val == "Bearer test-token" => StatusCode::ACCEPTED,
                    Some(_) => StatusCode::FORBIDDEN,
                    None => StatusCode::UNAUTHORIZED,
                }
            },
        ),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let url = format!("http://127.0.0.1:{}/api/v1/signal", addr.port());
    let client = make_signal_client();
    let signal = sample_signal_request();

    // With correct token → 202.
    let status = deliver_signal_http(&client, &url, &signal, Some("test-token"))
        .await
        .unwrap();
    assert_eq!(status, reqwest::StatusCode::ACCEPTED);

    // Without token → 401.
    let status = deliver_signal_http(&client, &url, &signal, None)
        .await
        .unwrap();
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_deliver_signal_http_timeout() {
    use axum::{Router, http::StatusCode, routing::post};

    let app = Router::new().route(
        "/api/v1/signal",
        post(|AxumJson(_body): AxumJson<serde_json::Value>| async {
            // Sleep longer than the client's 5-second timeout.
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            StatusCode::ACCEPTED
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let url = format!("http://127.0.0.1:{}/api/v1/signal", addr.port());
    let client = make_signal_client();
    let signal = sample_signal_request();

    // Wrap in a generous outer timeout to prevent test hangs.
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60), // outer: 60s > 5s client timeout, guards against CI jitter
        deliver_signal_http(&client, &url, &signal, None),
    )
    .await
    .expect("outer timeout should not fire — client timeout should fire first");

    assert!(result.is_err(), "should get a timeout error from reqwest");
    let err = result.unwrap_err();
    assert!(err.is_timeout(), "error should be a timeout: {err}");
}

#[tokio::test]
async fn test_deliver_signal_http_connection_refused() {
    let client = make_signal_client();
    let signal = sample_signal_request();

    // Port 1 is almost certainly not listening.
    let result =
        deliver_signal_http(&client, "http://127.0.0.1:1/api/v1/signal", &signal, None).await;

    assert!(result.is_err(), "connection to a closed port should fail");
    let err = result.unwrap_err();
    assert!(
        err.is_connect(),
        "error should be a connection error: {err}"
    );
}

// ─── HTTP-first delivery integration tests (T03) ──────────────────────

#[tokio::test]
async fn test_post_signal_http_first_delivery() {
    let (base, job_id, repo_dir, _manifest_dir, state) = setup_signal_test_server().await;
    let client = reqwest::Client::new();

    // Start a mock signal endpoint.
    let (mock_url, captured) = start_mock_signal_server().await;

    // Cache the mock URL as the job's signal URL.
    {
        let mut s = state.lock().unwrap();
        s.signal_urls
            .insert(job_id.clone(), format!("{mock_url}/api/v1/signal"));
    }

    // POST an event with run_id (needed for fallback path, but HTTP should bypass it).
    client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": job_id,
            "run_id": "01HTTP_TEST",
            "phase": "running",
        }))
        .send()
        .await
        .unwrap();

    // POST a SignalRequest → should use HTTP delivery, not filesystem.
    let resp = client
        .post(format!("{base}/api/v1/jobs/{job_id}/signals"))
        .json(&serde_json::json!({
            "target_session": "agent-1",
            "update": {
                "source_job": "job-alpha",
                "source_session": "session-1",
                "changed_files": ["src/main.rs"],
                "gate_summary": { "passed": 2, "failed": 0, "skipped": 0 },
                "branch": "results/job-alpha"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "signal POST should succeed");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(
        body["delivery"], "http",
        "should use HTTP delivery when signal URL is cached"
    );
    assert_eq!(body["signal_status"], 202);

    // Verify the mock received the correct SignalRequest.
    let received = captured.lock().await;
    let received = received
        .as_ref()
        .expect("mock should have captured a SignalRequest");
    assert_eq!(received.target_session, "agent-1");
    assert_eq!(received.update.source_job, "job-alpha");
    assert_eq!(received.update.branch, "results/job-alpha");
    assert_eq!(received.update.gate_summary.passed, 2);

    // Verify filesystem fallback did NOT run (HTTP succeeded — no double delivery).
    let canonical_repo = std::fs::canonicalize(repo_dir.path()).unwrap();
    let inbox_dir = canonical_repo.join(".assay/orchestrator/01HTTP_TEST/mesh/agent-1/inbox");
    assert!(
        !inbox_dir.exists(),
        "inbox dir should NOT exist when HTTP delivery succeeds — no double delivery"
    );
}

#[tokio::test]
async fn test_post_signal_fallback_to_filesystem() {
    let (base, job_id, repo_dir, _manifest_dir, state) = setup_signal_test_server().await;
    let client = reqwest::Client::new();

    // Cache a signal URL pointing to a non-existent server (will fail HTTP delivery).
    {
        let mut s = state.lock().unwrap();
        s.signal_urls.insert(
            job_id.clone(),
            "http://127.0.0.1:1/api/v1/signal".to_string(),
        );
    }

    // POST an event with run_id (needed for filesystem fallback).
    client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": job_id,
            "run_id": "01FALLBACK_TEST",
            "phase": "running",
        }))
        .send()
        .await
        .unwrap();

    // POST a SignalRequest → HTTP should fail, fallback to filesystem.
    let resp = client
        .post(format!("{base}/api/v1/jobs/{job_id}/signals"))
        .json(&serde_json::json!({
            "target_session": "worker-1",
            "update": {
                "source_job": "job-beta",
                "source_session": "session-2",
                "changed_files": [],
                "gate_summary": { "passed": 0, "failed": 1, "skipped": 0 },
                "branch": "main"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "signal should succeed via fallback");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(
        body["delivery"], "filesystem",
        "should fall back to filesystem when HTTP fails"
    );
    assert!(
        body["path"].as_str().is_some(),
        "filesystem delivery should include path"
    );

    // Verify the file actually exists on disk.
    let canonical_repo = std::fs::canonicalize(repo_dir.path()).unwrap();
    let inbox_dir = canonical_repo.join(".assay/orchestrator/01FALLBACK_TEST/mesh/worker-1/inbox");
    assert!(inbox_dir.is_dir(), "inbox directory should exist");

    let entries: Vec<_> = std::fs::read_dir(&inbox_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1, "exactly one signal file should exist");

    let file_content = std::fs::read_to_string(entries[0].path()).unwrap();
    let peer_update: crate::serve::signals::PeerUpdate =
        serde_json::from_str(&file_content).unwrap();
    assert_eq!(peer_update.source_job, "job-beta");
    assert_eq!(
        peer_update.gate_summary,
        GateSummary {
            passed: 0,
            failed: 1,
            skipped: 0
        }
    );
}

// ─── End-to-end tests (T05) ───────────────────────────────────────────

#[tokio::test]
async fn test_signal_end_to_end_http_delivery_with_fallback() {
    // This E2E test proves the full pipeline: HTTP delivery succeeds → mock receives
    // correct SignalRequest → then mock shuts down → next delivery falls back to filesystem.

    let (base, job_id, repo_dir, _manifest_dir, state) = setup_signal_test_server().await;
    let client = reqwest::Client::new();

    // POST an event with run_id (needed for filesystem fallback path).
    client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": job_id,
            "run_id": "01E2E_RUN",
            "phase": "running",
        }))
        .send()
        .await
        .unwrap();

    // Start a mock signal endpoint and cache its URL.
    let (mock_url, captured) = start_mock_signal_server().await;
    {
        let mut s = state.lock().unwrap();
        s.signal_urls
            .insert(job_id.clone(), format!("{mock_url}/api/v1/signal"));
    }

    // --- Phase 1: HTTP delivery succeeds ---
    let resp = client
        .post(format!("{base}/api/v1/jobs/{job_id}/signals"))
        .json(&serde_json::json!({
            "target_session": "agent-1",
            "update": {
                "source_job": "e2e-source",
                "source_session": "session-1",
                "changed_files": ["src/lib.rs"],
                "gate_summary": { "passed": 5, "failed": 0, "skipped": 2 },
                "branch": "results/e2e"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["delivery"], "http", "first delivery should use HTTP");
    assert_eq!(body["signal_status"], 202);

    // Verify mock received correct SignalRequest with all schema-aligned fields.
    let received = captured.lock().await;
    let received = received.as_ref().expect("mock should capture the request");
    assert_eq!(received.target_session, "agent-1");
    assert_eq!(received.update.source_job, "e2e-source");
    assert_eq!(received.update.branch, "results/e2e");
    assert_eq!(received.update.gate_summary.passed, 5);
    assert_eq!(received.update.gate_summary.failed, 0);
    assert_eq!(received.update.gate_summary.skipped, 2);
    assert_eq!(received.update.changed_files, vec!["src/lib.rs"]);

    // --- Phase 2: Simulate dead signal endpoint by pointing to a closed port ---
    {
        let mut s = state.lock().unwrap();
        s.signal_urls.insert(
            job_id.clone(),
            "http://127.0.0.1:1/api/v1/signal".to_string(),
        );
    }

    let resp = client
        .post(format!("{base}/api/v1/jobs/{job_id}/signals"))
        .json(&serde_json::json!({
            "target_session": "agent-2",
            "update": {
                "source_job": "e2e-source",
                "source_session": "session-2",
                "changed_files": [],
                "gate_summary": { "passed": 1, "failed": 1, "skipped": 0 },
                "branch": "main"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body["delivery"], "filesystem",
        "second delivery should fall back to filesystem"
    );
    assert!(body["path"].as_str().is_some(), "should include file path");

    // Verify the file exists on disk.
    let canonical_repo = std::fs::canonicalize(repo_dir.path()).unwrap();
    let inbox_dir = canonical_repo.join(".assay/orchestrator/01E2E_RUN/mesh/agent-2/inbox");
    assert!(inbox_dir.is_dir(), "inbox directory should exist");
    let entries: Vec<_> = std::fs::read_dir(&inbox_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1, "one fallback file should exist");

    let content = std::fs::read_to_string(entries[0].path()).unwrap();
    let peer_update: PeerUpdate = serde_json::from_str(&content).unwrap();
    assert_eq!(peer_update.source_job, "e2e-source");
    assert_eq!(
        peer_update.gate_summary,
        GateSummary {
            passed: 1,
            failed: 1,
            skipped: 0
        }
    );
    assert_eq!(peer_update.branch, "main");
}
