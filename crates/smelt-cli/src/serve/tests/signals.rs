use crate::serve::signals::{PeerUpdate, deliver_peer_update, validate_run_id};
use std::path::{Path, PathBuf};

/// Helper: build a sample PeerUpdate for tests.
fn sample_peer_update() -> PeerUpdate {
    PeerUpdate {
        source_job: "job-alpha".to_string(),
        source_session: "session-1".to_string(),
        changed_files: vec!["src/main.rs".to_string(), "README.md".to_string()],
        gate_summary: "all gates passed".to_string(),
        branch_name: "results/job-alpha".to_string(),
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

    // Now POST a signal.
    let signal_body = serde_json::json!({
        "session_name": "agent-1",
        "source_job": "job-alpha",
        "source_session": "session-1",
        "changed_files": ["src/main.rs", "README.md"],
        "gate_summary": "all gates passed",
        "branch_name": "results/job-alpha",
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
    assert_eq!(deserialized.gate_summary, "all gates passed");
    assert_eq!(deserialized.branch_name, "results/job-alpha");
}

#[tokio::test]
async fn test_signal_unknown_job_404() {
    let (base, _job_id, _repo_dir, _manifest_dir, _state) = setup_signal_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/jobs/nonexistent-job/signals"))
        .json(&serde_json::json!({
            "session_name": "agent-1",
            "source_job": "x",
            "source_session": "x",
            "changed_files": [],
            "gate_summary": "x",
            "branch_name": "x",
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
            "session_name": "agent-1",
            "source_job": "x",
            "source_session": "x",
            "changed_files": [],
            "gate_summary": "x",
            "branch_name": "x",
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
            "session_name": "a",
            "source_job": "x",
            "source_session": "x",
            "changed_files": [],
            "gate_summary": "x",
            "branch_name": "x",
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
            "session_name": "a",
            "source_job": "x",
            "source_session": "x",
            "changed_files": [],
            "gate_summary": "x",
            "branch_name": "x",
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
            "session_name": "a",
            "source_job": "x",
            "source_session": "x",
            "changed_files": [],
            "gate_summary": "x",
            "branch_name": "x",
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
                "session_name": bad_name,
                "source_job": "x",
                "source_session": "x",
                "changed_files": [],
                "gate_summary": "x",
                "branch_name": "x",
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
            "session_name": "agent-1",
            "source_job": "job-a",
            "source_session": "session-1",
            "changed_files": ["src/main.rs"],
            "gate_summary": "all passed",
            "branch_name": "results/job-a",
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
    assert_eq!(peer_update.gate_summary, "all passed");
    assert_eq!(peer_update.branch_name, "results/job-a");
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
            "session_name": "worker-1",
            "source_job": "x",
            "source_session": "x",
            "changed_files": [],
            "gate_summary": "x",
            "branch_name": "x",
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
