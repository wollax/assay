use std::path::PathBuf;

use crate::serve::events::AssayEvent;
use crate::serve::notify::{
    QueuedJobSnapshot, evaluate_notify_rules, extract_gate_summary, extract_session_name,
    is_session_complete,
};
use crate::serve::queue::ServerState;
use crate::serve::signals::GateSummary;
use crate::serve::types::{JobId, JobSource, JobStatus, QueuedJob, now_epoch};

/// Test helper: call evaluate_notify_rules from a ServerState + event,
/// building QueuedJobSnapshot from the state's jobs automatically.
fn eval_rules(event: &AssayEvent, state: &ServerState) -> Vec<crate::serve::notify::NotifyTarget> {
    // Find source job's manifest path.
    let src_manifest = match state.jobs.iter().find(|j| j.id.0 == event.job_id) {
        Some(j) => j.manifest_path.clone(),
        None => return Vec::new(),
    };
    // Build snapshots: job_name = job_id (tests set up jobs by name).
    let snapshots: Vec<QueuedJobSnapshot> = state
        .jobs
        .iter()
        .map(|j| QueuedJobSnapshot {
            job_id: j.id.clone(),
            job_name: j.id.0.clone(),
            manifest_path: j.manifest_path.clone(),
            is_terminal: matches!(j.status, JobStatus::Complete | JobStatus::Failed),
        })
        .collect();
    evaluate_notify_rules(event, &src_manifest, &snapshots)
}

fn make_event(job_id: &str, phase: &str) -> AssayEvent {
    AssayEvent {
        job_id: job_id.to_string(),
        event_id: None,
        received_at: now_epoch(),
        payload: serde_json::json!({
            "run_id": "01TESTRUN",
            "phase": phase,
            "sessions": [{"name": "main-session", "passed": true}],
        }),
    }
}

fn make_event_with_sessions(job_id: &str, sessions: serde_json::Value) -> AssayEvent {
    AssayEvent {
        job_id: job_id.to_string(),
        event_id: None,
        received_at: now_epoch(),
        payload: serde_json::json!({
            "run_id": "01TESTRUN",
            "phase": "complete",
            "sessions": sessions,
        }),
    }
}

fn state_with_jobs(jobs: Vec<(&str, JobStatus, PathBuf)>) -> ServerState {
    let mut s = ServerState::new_without_events(4);
    for (id, status, manifest_path) in jobs {
        s.jobs.push_back(QueuedJob {
            id: JobId::new(id),
            manifest_path,
            source: JobSource::HttpApi,
            attempt: 0,
            status,
            queued_at: now_epoch(),
            started_at: None,
            worker_host: None,
        });
    }
    s
}

/// Write a manifest file with [[notify]] rules to a tempdir, return the path.
fn write_notify_manifest(dir: &std::path::Path, targets: &[&str]) -> PathBuf {
    let mut notify_sections = String::new();
    for target in targets {
        notify_sections.push_str(&format!(
            "\n[[notify]]\ntarget_job = \"{target}\"\non_session_complete = true\n"
        ));
    }
    let toml = format!(
        r#"[job]
name = "source-job"
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
spec = "test"
harness = "echo hello"
timeout = 300

[merge]
strategy = "sequential"
target = "main"
{notify_sections}
"#,
        dir.display()
    );
    let manifest_path = dir.join("test.smelt.toml");
    std::fs::write(&manifest_path, toml).unwrap();
    manifest_path
}

// ─── is_session_complete tests ────────────────────────────────

#[test]
fn test_is_session_complete_true() {
    let event = make_event("job-1", "complete");
    assert!(is_session_complete(&event));
}

#[test]
fn test_is_session_complete_false_for_running() {
    let event = make_event("job-1", "running");
    assert!(!is_session_complete(&event));
}

#[test]
fn test_is_session_complete_false_when_no_phase() {
    let event = AssayEvent {
        job_id: "job-1".to_string(),
        event_id: None,
        received_at: now_epoch(),
        payload: serde_json::json!({"run_id": "01TESTRUN"}),
    };
    assert!(!is_session_complete(&event));
}

// ─── extract_session_name tests ───────────────────────────────

#[test]
fn test_extract_session_name_from_payload() {
    let event = make_event("job-1", "complete");
    assert_eq!(extract_session_name(&event), "main-session");
}

#[test]
fn test_extract_session_name_fallback_to_job_id() {
    let event = AssayEvent {
        job_id: "job-1".to_string(),
        event_id: None,
        received_at: now_epoch(),
        payload: serde_json::json!({"phase": "complete"}),
    };
    assert_eq!(extract_session_name(&event), "job-1");
}

// ─── extract_gate_summary tests ───────────────────────────────

#[test]
fn test_extract_gate_summary_all_passed() {
    let event = make_event_with_sessions(
        "job-1",
        serde_json::json!([
            {"name": "s1", "passed": true},
            {"name": "s2", "passed": true},
        ]),
    );
    assert_eq!(
        extract_gate_summary(&event),
        GateSummary {
            passed: 2,
            failed: 0,
            skipped: 0
        }
    );
}

#[test]
fn test_extract_gate_summary_some_failed() {
    let event = make_event_with_sessions(
        "job-1",
        serde_json::json!([
            {"name": "s1", "passed": true},
            {"name": "s2", "passed": false},
        ]),
    );
    assert_eq!(
        extract_gate_summary(&event),
        GateSummary {
            passed: 1,
            failed: 1,
            skipped: 0
        }
    );
}

#[test]
fn test_extract_gate_summary_with_state_field() {
    let event = make_event_with_sessions(
        "job-1",
        serde_json::json!([
            {"name": "s1", "state": "completed"},
            {"name": "s2", "state": "failed"},
            {"name": "s3", "state": "skipped"},
        ]),
    );
    assert_eq!(
        extract_gate_summary(&event),
        GateSummary {
            passed: 1,
            failed: 1,
            skipped: 1
        }
    );
}

#[test]
fn test_extract_gate_summary_fallback() {
    let event = AssayEvent {
        job_id: "job-1".to_string(),
        event_id: None,
        received_at: now_epoch(),
        payload: serde_json::json!({"phase": "complete"}),
    };
    assert_eq!(
        extract_gate_summary(&event),
        GateSummary {
            passed: 0,
            failed: 0,
            skipped: 0
        }
    );
}

// ─── evaluate_notify_rules tests ──────────────────────────────

#[test]
fn test_evaluate_notify_no_rules_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    // Write a manifest without [[notify]] rules.
    let toml = format!(
        r#"[job]
name = "source"
repo = "{}"
base_ref = "main"
[environment]
runtime = "docker"
image = "alpine:3.18"
[credentials]
provider = "anthropic"
model = "m"
[[session]]
name = "s"
spec = "x"
harness = "echo"
timeout = 60
[merge]
strategy = "sequential"
target = "main"
"#,
        tmp.path().display()
    );
    let manifest_path = tmp.path().join("test.smelt.toml");
    std::fs::write(&manifest_path, &toml).unwrap();

    let state = state_with_jobs(vec![("source-job", JobStatus::Running, manifest_path)]);
    let event = make_event("source-job", "complete");
    let targets = eval_rules(&event, &state);
    assert!(targets.is_empty());
}

#[test]
fn test_evaluate_notify_non_complete_event_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest_path = write_notify_manifest(tmp.path(), &["target-job"]);
    let state = state_with_jobs(vec![
        ("source-job", JobStatus::Running, manifest_path),
        (
            "target-job",
            JobStatus::Running,
            tmp.path().join("target.smelt.toml"),
        ),
    ]);
    let event = make_event("source-job", "running"); // not complete
    let targets = eval_rules(&event, &state);
    assert!(targets.is_empty());
}

#[test]
fn test_evaluate_notify_absent_target_silently_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest_path = write_notify_manifest(tmp.path(), &["nonexistent-job"]);
    let state = state_with_jobs(vec![("source-job", JobStatus::Running, manifest_path)]);
    let event = make_event("source-job", "complete");
    let targets = eval_rules(&event, &state);
    // nonexistent-job is not in the queue — should be silently skipped
    assert!(targets.is_empty());
}

#[test]
fn test_evaluate_notify_completed_target_silently_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest_path = write_notify_manifest(tmp.path(), &["target-job"]);
    let state = state_with_jobs(vec![
        ("source-job", JobStatus::Running, manifest_path),
        (
            "target-job",
            JobStatus::Complete, // already done
            tmp.path().join("target.smelt.toml"),
        ),
    ]);
    let event = make_event("source-job", "complete");
    let targets = eval_rules(&event, &state);
    assert!(targets.is_empty());
}

#[test]
fn test_evaluate_notify_matching_running_target() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest_path = write_notify_manifest(tmp.path(), &["target-job"]);
    let target_manifest = tmp.path().join("target.smelt.toml");
    std::fs::write(&target_manifest, "# placeholder").unwrap();

    let state = state_with_jobs(vec![
        ("source-job", JobStatus::Running, manifest_path),
        ("target-job", JobStatus::Running, target_manifest),
    ]);
    let event = make_event("source-job", "complete");
    let targets = eval_rules(&event, &state);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].job_id.0, "target-job");
    assert_eq!(targets[0].peer_update.source_job, "source-job");
    assert_eq!(targets[0].peer_update.source_session, "main-session");
}

#[test]
fn test_evaluate_notify_on_session_complete_false_not_triggered() {
    let tmp = tempfile::tempdir().unwrap();
    // Write a manifest with on_session_complete = false.
    let toml = format!(
        r#"[job]
name = "source"
repo = "{}"
base_ref = "main"
[environment]
runtime = "docker"
image = "alpine:3.18"
[credentials]
provider = "anthropic"
model = "m"
[[session]]
name = "s"
spec = "x"
harness = "echo"
timeout = 60
[merge]
strategy = "sequential"
target = "main"
[[notify]]
target_job = "target-job"
on_session_complete = false
"#,
        tmp.path().display()
    );
    let manifest_path = tmp.path().join("test.smelt.toml");
    std::fs::write(&manifest_path, &toml).unwrap();
    let target_manifest = tmp.path().join("target.smelt.toml");
    std::fs::write(&target_manifest, "# placeholder").unwrap();

    let state = state_with_jobs(vec![
        ("source-job", JobStatus::Running, manifest_path),
        ("target-job", JobStatus::Running, target_manifest),
    ]);
    let event = make_event("source-job", "complete");
    let targets = eval_rules(&event, &state);
    assert!(targets.is_empty());
}

// ─── HTTP integration test ────────────────────────────────────

#[tokio::test]
async fn test_notify_integration_post_event_triggers_peer_update() {
    let tmp = tempfile::tempdir().unwrap();
    let source_repo = tmp.path().join("source-repo");
    let target_repo = tmp.path().join("target-repo");
    std::fs::create_dir_all(&source_repo).unwrap();
    std::fs::create_dir_all(&target_repo).unwrap();

    // Write source manifest with [[notify]] rule pointing to target job.
    let source_manifest_path = tmp.path().join("source.smelt.toml");
    let source_toml = format!(
        r#"[job]
name = "backend"
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
spec = "test"
harness = "echo hello"
timeout = 300
[merge]
strategy = "sequential"
target = "main"
[[notify]]
target_job = "frontend"
on_session_complete = true
"#,
        source_repo.display()
    );
    std::fs::write(&source_manifest_path, &source_toml).unwrap();

    // Write target manifest.
    let target_manifest_path = tmp.path().join("target.smelt.toml");
    let target_toml = format!(
        r#"[job]
name = "frontend"
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
spec = "test"
harness = "echo hello"
timeout = 300
[merge]
strategy = "sequential"
target = "main"
"#,
        target_repo.display()
    );
    std::fs::write(&target_manifest_path, &target_toml).unwrap();

    // Set up server state with both jobs.
    let state = std::sync::Arc::new(std::sync::Mutex::new(ServerState::new_without_events(4)));
    {
        let mut s = state.lock().unwrap();
        s.jobs.push_back(QueuedJob {
            id: JobId::new("backend"),
            manifest_path: source_manifest_path,
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
            worker_host: None,
        });
        s.jobs.push_back(QueuedJob {
            id: JobId::new("frontend"),
            manifest_path: target_manifest_path,
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
            worker_host: None,
        });
        // Seed run_id for the target job (so signal delivery can resolve inbox path).
        s.run_ids
            .insert("frontend".to_string(), "01TARGET_RUN".to_string());
    }

    let base = super::start_test_server(state).await;
    let client = reqwest::Client::new();

    // POST a session-completion event for backend.
    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": "backend",
            "run_id": "01SOURCE_RUN",
            "phase": "complete",
            "sessions": [{"name": "main-session", "passed": true}],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "event POST should succeed");

    // Verify a PeerUpdate file appeared in frontend's inbox.
    // The target manifest declares [[session]] name = "main", so the PeerUpdate
    // is delivered to mesh/main/inbox (the target's session, not the source's).
    let canonical_target = std::fs::canonicalize(&target_repo).unwrap();
    let inbox_dir = canonical_target.join(".assay/orchestrator/01TARGET_RUN/mesh/main/inbox");

    // Give any async filesystem ops a moment (though this is synchronous in practice).
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    assert!(inbox_dir.is_dir(), "inbox directory should be created");
    let entries: Vec<_> = std::fs::read_dir(&inbox_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "exactly one PeerUpdate file should be in the inbox"
    );

    let content = std::fs::read_to_string(entries[0].path()).unwrap();
    let peer_update: crate::serve::signals::PeerUpdate = serde_json::from_str(&content).unwrap();
    assert_eq!(peer_update.source_job, "backend");
    assert_eq!(peer_update.source_session, "main-session");
    assert_eq!(
        peer_update.gate_summary,
        GateSummary {
            passed: 1,
            failed: 0,
            skipped: 0
        }
    );
}

/// E2E test: [[notify]] routing delivers PeerUpdate via HTTP when signal URL is cached.
///
/// Two jobs (backend → frontend) with [[notify]] rule. A signal URL is cached for
/// the target job pointing to a mock endpoint. When backend's session completes,
/// the PeerUpdate is delivered via HTTP to the mock instead of filesystem.
#[tokio::test]
async fn test_notify_http_delivery_to_mock_signal_endpoint() {
    use crate::serve::signals::SignalRequest;

    let tmp = tempfile::tempdir().unwrap();
    let source_repo = tmp.path().join("source-repo");
    let target_repo = tmp.path().join("target-repo");
    std::fs::create_dir_all(&source_repo).unwrap();
    std::fs::create_dir_all(&target_repo).unwrap();

    // Write source manifest with [[notify]] rule pointing to target job.
    let source_manifest_path = tmp.path().join("source.smelt.toml");
    let source_toml = format!(
        r#"[job]
name = "backend"
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
spec = "test"
harness = "echo hello"
timeout = 300
[merge]
strategy = "sequential"
target = "main"
[[notify]]
target_job = "frontend"
on_session_complete = true
"#,
        source_repo.display()
    );
    std::fs::write(&source_manifest_path, &source_toml).unwrap();

    // Write target manifest.
    let target_manifest_path = tmp.path().join("target.smelt.toml");
    let target_toml = format!(
        r#"[job]
name = "frontend"
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
spec = "test"
harness = "echo hello"
timeout = 300
[merge]
strategy = "sequential"
target = "main"
"#,
        target_repo.display()
    );
    std::fs::write(&target_manifest_path, &target_toml).unwrap();

    // Start a mock signal endpoint to capture the forwarded PeerUpdate.
    let captured: std::sync::Arc<tokio::sync::Mutex<Option<SignalRequest>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let captured_clone = captured.clone();
    let mock_app = axum::Router::new().route(
        "/api/v1/signal",
        axum::routing::post(move |axum::Json(body): axum::Json<SignalRequest>| {
            let cap = captured_clone.clone();
            async move {
                *cap.lock().await = Some(body);
                axum::http::StatusCode::ACCEPTED
            }
        }),
    );
    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(mock_listener, mock_app).await.unwrap();
    });
    let mock_signal_url = format!("http://127.0.0.1:{}/api/v1/signal", mock_addr.port());

    // Set up server state with both jobs + signal URL for target.
    let state = std::sync::Arc::new(std::sync::Mutex::new(ServerState::new_without_events(4)));
    {
        let mut s = state.lock().unwrap();
        s.jobs.push_back(QueuedJob {
            id: JobId::new("backend"),
            manifest_path: source_manifest_path,
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
            worker_host: None,
        });
        s.jobs.push_back(QueuedJob {
            id: JobId::new("frontend"),
            manifest_path: target_manifest_path,
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
            worker_host: None,
        });
        // Seed run_id for the target job (filesystem fallback needs it).
        s.run_ids
            .insert("frontend".to_string(), "01TARGET_RUN".to_string());
        // Cache signal URL for the target job → mock endpoint.
        s.signal_urls
            .insert("frontend".to_string(), mock_signal_url);
    }

    let base = super::start_test_server(state).await;
    let client = reqwest::Client::new();

    // POST a session-completion event for backend.
    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": "backend",
            "run_id": "01SOURCE_RUN",
            "phase": "complete",
            "sessions": [{"name": "main-session", "state": "completed"}],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "event POST should succeed");

    // Give the async notify delivery a moment to complete.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Verify the mock signal endpoint received the PeerUpdate via HTTP.
    let received = captured.lock().await;
    let received = received
        .as_ref()
        .expect("mock signal endpoint should have received a SignalRequest");

    // The target session name comes from the target manifest's [[session]] name = "main".
    assert_eq!(received.target_session, "main");
    assert_eq!(received.update.source_job, "backend");
    assert_eq!(received.update.source_session, "main-session");
    assert_eq!(
        received.update.gate_summary,
        GateSummary {
            passed: 1,
            failed: 0,
            skipped: 0
        }
    );
    assert_eq!(received.update.branch, "main");

    // Verify NO file was written to filesystem (HTTP delivery succeeded → no fallback).
    let canonical_target = std::fs::canonicalize(&target_repo).unwrap();
    let inbox_dir = canonical_target.join(".assay/orchestrator/01TARGET_RUN/mesh/main/inbox");
    assert!(
        !inbox_dir.exists(),
        "inbox directory should NOT exist — HTTP delivery should have bypassed filesystem"
    );
}
