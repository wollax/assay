use std::path::PathBuf;

use crate::serve::queue::ServerState;
use crate::serve::types::{JobSource, JobStatus};

/// Minimal valid manifest TOML for watcher and HTTP tests.
const VALID_MANIFEST_TOML: &str = r#"[job]
name = "test-job"
repo = "."
base_ref = "main"

[environment]
runtime = "docker"
image = "alpine:3.18"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "main"
spec = "Run the test"
harness = "echo hello"
timeout = 300

[merge]
strategy = "sequential"
target = "main"
"#;

fn manifest() -> PathBuf {
    PathBuf::from("/tmp/test.toml")
}

// ──────────────────────────────────────────────
// T01 queue unit tests (no Docker, no async I/O)
// ──────────────────────────────────────────────

#[test]
fn test_queue_fifo_order() {
    let mut state = ServerState::new(10);
    let id1 = state.enqueue(manifest(), JobSource::HttpApi);
    let id2 = state.enqueue(manifest(), JobSource::HttpApi);
    let id3 = state.enqueue(manifest(), JobSource::HttpApi);

    let j1 = state.try_dispatch().expect("first job");
    let j2 = state.try_dispatch().expect("second job");
    let j3 = state.try_dispatch().expect("third job");

    assert_eq!(j1.id, id1, "first dispatched should be first enqueued");
    assert_eq!(j2.id, id2);
    assert_eq!(j3.id, id3);
}

#[test]
fn test_queue_max_concurrent() {
    let mut state = ServerState::new(1);
    state.enqueue(manifest(), JobSource::DirectoryWatch);
    state.enqueue(manifest(), JobSource::DirectoryWatch);

    // First dispatch should succeed.
    let first = state.try_dispatch();
    assert!(first.is_some(), "first dispatch should succeed");
    assert_eq!(state.running_count, 1);

    // Second dispatch should be blocked by the cap.
    let second = state.try_dispatch();
    assert!(second.is_none(), "second dispatch should be blocked (max_concurrent=1)");

    // Complete the first job, then the second should dispatch.
    state.complete(&first.unwrap().id, true, 0, 3);
    assert_eq!(state.running_count, 0);

    let second = state.try_dispatch();
    assert!(second.is_some(), "second job should dispatch after first completes");
}

#[test]
fn test_queue_cancel_queued() {
    let mut state = ServerState::new(2);
    let id_queued = state.enqueue(manifest(), JobSource::HttpApi);
    let id_dispatching = state.enqueue(manifest(), JobSource::HttpApi);

    // Dispatch the second job so it becomes Dispatching.
    state.try_dispatch(); // dispatches id_queued (FIFO)
    state.try_dispatch(); // dispatches id_dispatching

    // Now enqueue a third one that stays Queued.
    let id_waiting = state.enqueue(manifest(), JobSource::HttpApi);

    // Cancelling a Queued job should succeed.
    assert!(state.cancel(&id_waiting), "cancel of Queued job should return true");

    // Cancelling a Dispatching job should fail.
    assert!(!state.cancel(&id_queued), "cancel of Dispatching job should return false");
    assert!(!state.cancel(&id_dispatching), "cancel of Dispatching job should return false");
}

#[test]
fn test_queue_retry_eligible() {
    let mut state = ServerState::new(3);
    let id = state.enqueue(manifest(), JobSource::HttpApi);
    state.try_dispatch();

    // Simulate a failure with attempt=0, max_attempts=3 → should retry.
    state.complete(&id, false, 0, 3);

    // After complete with retry, a new Queued entry exists; find the Retrying marker.
    // Our implementation sets Retrying on the old entry then re-enqueues a fresh Queued one.
    // retry_eligible inspects the Retrying entry.
    // Let's find the re-enqueued entry (status=Queued, attempt=1) and the original (Retrying).
    let retrying_id = state
        .jobs
        .iter()
        .find(|j| j.status == JobStatus::Retrying)
        .map(|j| j.id.clone());

    // retry_eligible should return true for that entry.
    if let Some(rid) = retrying_id {
        assert!(state.retry_eligible(&rid, 3), "should be retry eligible (attempt < max)");
        // Simulate reaching max_attempts.
        assert!(!state.retry_eligible(&rid, 1), "should NOT be eligible if attempt >= max_attempts");
    } else {
        panic!("expected a Retrying job after failure with remaining attempts");
    }
}

// ──────────────────────────────────────────────
// T02–T04 integration test stubs (ignored)
// ──────────────────────────────────────────────

#[tokio::test]
async fn test_dispatch_loop_two_jobs_concurrent() {
    // Skip if Docker is unavailable.
    if bollard::Docker::connect_with_local_defaults().is_err() {
        println!("SKIP: Docker unavailable");
        return;
    }
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    use crate::serve::dispatch::dispatch_loop;
    use crate::serve::queue::ServerState;
    use crate::serve::types::{JobSource, JobStatus};

    let dir = TempDir::new().unwrap();
    // Write two valid manifest TOMLs using the canonical VALID_MANIFEST_TOML constant
    // so that manifest parsing and validation succeed and real dispatch is exercised.
    let m1 = dir.path().join("job1.toml");
    let m2 = dir.path().join("job2.toml");
    for p in [&m1, &m2] {
        std::fs::write(p, VALID_MANIFEST_TOML).unwrap();
    }

    let state = Arc::new(Mutex::new(ServerState::new(2)));
    {
        let mut s = state.lock().unwrap();
        s.enqueue(m1, JobSource::DirectoryWatch);
        s.enqueue(m2, JobSource::DirectoryWatch);
    }

    let cancel = CancellationToken::new();
    let cancel2 = cancel.clone();
    let state2 = Arc::clone(&state);
    let handle = tokio::spawn(async move {
        dispatch_loop(state2, cancel2, 1).await;
    });

    // Wait up to 60 s for both jobs to reach a terminal state.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let done = {
            let s = state.lock().unwrap();
            s.jobs.iter().all(|j| {
                matches!(j.status, JobStatus::Complete | JobStatus::Failed)
            })
        };
        if done {
            break;
        }
        if tokio::time::Instant::now() > deadline {
            panic!("timeout: jobs did not complete within 60s");
        }
    }

    cancel.cancel();
    handle.await.unwrap();

    let s = state.lock().unwrap();
    for job in &s.jobs {
        assert!(
            matches!(job.status, JobStatus::Complete | JobStatus::Failed),
            "job {} ended in unexpected state {:?}",
            job.id,
            job.status
        );
    }
}

/// Prove that `CancellationToken::cancel()` broadcasts to N concurrent tasks.
///
/// Uses tokio oneshot channels as mock cancel futures — no Docker required.
#[tokio::test]
async fn test_cancellation_broadcast() {
    use tokio::sync::oneshot;
    use tokio_util::sync::CancellationToken;

    let token = CancellationToken::new();

    // Spawn two tasks that each wait on a child token and signal completion via
    // a oneshot channel.
    let (tx1, rx1) = oneshot::channel::<()>();
    let (tx2, rx2) = oneshot::channel::<()>();

    let child1 = token.child_token();
    let child2 = token.child_token();

    tokio::spawn(async move {
        child1.cancelled().await;
        let _ = tx1.send(());
    });

    tokio::spawn(async move {
        child2.cancelled().await;
        let _ = tx2.send(());
    });

    // Give tasks a moment to start.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Broadcast cancellation.
    token.cancel();

    // Both receivers must fire within 500 ms.
    let timeout = std::time::Duration::from_millis(500);
    tokio::time::timeout(timeout, rx1)
        .await
        .expect("rx1 did not fire within 500ms")
        .expect("tx1 dropped without sending");
    tokio::time::timeout(timeout, rx2)
        .await
        .expect("rx2 did not fire within 500ms")
        .expect("tx2 dropped without sending");
}

#[tokio::test]
async fn test_watcher_picks_up_manifest() {
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    use crate::serve::queue::ServerState;
    use crate::serve::queue_watcher::DirectoryWatcher;
    use crate::serve::types::JobStatus;

    let dir = TempDir::new().unwrap();
    let queue_dir = dir.path().to_path_buf();

    // Write a valid manifest TOML.
    let manifest_path = queue_dir.join("my-job.toml");
    std::fs::write(
        &manifest_path,
        VALID_MANIFEST_TOML,
    )
    .unwrap();

    let state = Arc::new(Mutex::new(ServerState::new(2)));
    let watcher = DirectoryWatcher::new(queue_dir.clone(), Arc::clone(&state));

    let handle = tokio::spawn(async move {
        watcher.watch().await;
    });

    // Wait long enough for at least one poll cycle (2s interval).
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let s = state.lock().unwrap();
    assert_eq!(s.jobs.len(), 1, "expected 1 job enqueued by watcher");
    assert_eq!(s.jobs[0].status, JobStatus::Queued, "job should be Queued");

    handle.abort();
}

#[tokio::test]
async fn test_watcher_moves_to_dispatched() {
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    use crate::serve::queue::ServerState;
    use crate::serve::queue_watcher::DirectoryWatcher;

    let dir = TempDir::new().unwrap();
    let queue_dir = dir.path().to_path_buf();

    // Write a valid manifest TOML.
    let manifest_path = queue_dir.join("move-test.toml");
    std::fs::write(
        &manifest_path,
        VALID_MANIFEST_TOML,
    )
    .unwrap();

    let state = Arc::new(Mutex::new(ServerState::new(2)));
    let watcher = DirectoryWatcher::new(queue_dir.clone(), Arc::clone(&state));

    let handle = tokio::spawn(async move {
        watcher.watch().await;
    });

    // Wait for watcher to pick up the file.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Original file should be gone from queue_dir root.
    assert!(
        !manifest_path.exists(),
        "original TOML should no longer exist in queue_dir root"
    );

    // dispatched/ should contain exactly 1 file matching *-move-test.toml.
    let dispatched_dir = queue_dir.join("dispatched");
    assert!(dispatched_dir.exists(), "dispatched/ directory should exist");

    let dispatched_files: Vec<_> = std::fs::read_dir(&dispatched_dir)
        .unwrap()
        .flatten()
        .filter(|e| {
            e.path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .ends_with("-move-test.toml")
        })
        .collect();

    assert_eq!(
        dispatched_files.len(),
        1,
        "expected exactly 1 dispatched file matching *-move-test.toml, found {}",
        dispatched_files.len()
    );

    handle.abort();
}

// ──────────────────────────────────────────────
// T04 HTTP API integration tests
// ──────────────────────────────────────────────

/// Helper: spin up an axum server on an OS-assigned port, return the base URL.
async fn start_test_server(state: std::sync::Arc<std::sync::Mutex<ServerState>>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = crate::serve::http_api::build_router(state);
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn test_http_post_enqueues_job() {
    let state = std::sync::Arc::new(std::sync::Mutex::new(ServerState::new(4)));
    let base = start_test_server(state.clone()).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .body(VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "POST valid TOML should return 200");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["job_id"].is_string(), "response should contain job_id");

    let s = state.lock().unwrap();
    assert_eq!(s.jobs.len(), 1, "one job should be enqueued");
}

#[tokio::test]
async fn test_http_post_invalid_toml() {
    let state = std::sync::Arc::new(std::sync::Mutex::new(ServerState::new(4)));
    let base = start_test_server(state.clone()).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .body("this is not valid toml {{{{")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 422, "POST invalid TOML should return 422");

    let s = state.lock().unwrap();
    assert_eq!(s.jobs.len(), 0, "no job should be enqueued on parse failure");
}

#[tokio::test]
async fn test_http_get_jobs() {
    let state = std::sync::Arc::new(std::sync::Mutex::new(ServerState::new(4)));
    let base = start_test_server(state.clone()).await;

    let client = reqwest::Client::new();

    // Enqueue one job via POST.
    client
        .post(format!("{base}/api/v1/jobs"))
        .body(VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();

    let resp = client
        .get(format!("{base}/api/v1/jobs"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let jobs: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(jobs.len(), 1, "GET /api/v1/jobs should return 1 job");
    assert_eq!(jobs[0]["status"], "queued");
}

#[tokio::test]
async fn test_http_get_job_by_id() {
    let state = std::sync::Arc::new(std::sync::Mutex::new(ServerState::new(4)));
    let base = start_test_server(state.clone()).await;

    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .body(VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let job_id = body["job_id"].as_str().unwrap().to_string();

    let resp = client
        .get(format!("{base}/api/v1/jobs/{job_id}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let job: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(job["id"], job_id);
    assert_eq!(job["status"], "queued");

    // Non-existent ID → 404.
    let resp = client
        .get(format!("{base}/api/v1/jobs/no-such-job"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_http_delete_queued_job() {
    let state = std::sync::Arc::new(std::sync::Mutex::new(ServerState::new(4)));
    let base = start_test_server(state.clone()).await;

    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .body(VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let job_id = body["job_id"].as_str().unwrap().to_string();

    let resp = client
        .delete(format!("{base}/api/v1/jobs/{job_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "DELETE queued job should return 200");

    let s = state.lock().unwrap();
    assert_eq!(s.jobs.len(), 0, "job should be removed from queue");
}

#[tokio::test]
async fn test_http_delete_running_job() {
    let state = std::sync::Arc::new(std::sync::Mutex::new(ServerState::new(4)));
    let base = start_test_server(state.clone()).await;

    // Enqueue and then dispatch the job so it becomes Dispatching.
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .body(VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let job_id = body["job_id"].as_str().unwrap().to_string();

    // Manually dispatch the job to move it to Dispatching state.
    {
        let mut s = state.lock().unwrap();
        let dispatched = s.try_dispatch();
        assert!(dispatched.is_some(), "should dispatch the enqueued job");
    }

    let resp = client
        .delete(format!("{base}/api/v1/jobs/{job_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        409,
        "DELETE dispatching/running job should return 409 Conflict"
    );
}
