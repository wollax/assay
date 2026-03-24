use std::path::PathBuf;

use crate::serve::config::ServerConfig;
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

// ──────────────────────────────────────────────
// M008/S01/T01 WorkerConfig unit tests
// ──────────────────────────────────────────────

#[test]
fn test_worker_config_roundtrip() {
    use crate::serve::config::ServerConfig;

    let toml = r#"
queue_dir = "/tmp/smelt-queue"
max_concurrent = 2

[[workers]]
host = "worker1.example.com"
user = "smelt"
key_env = "WORKER_SSH_KEY"
port = 2222
"#;
    let config: ServerConfig = toml::from_str(toml).expect("valid TOML with workers should parse");
    assert_eq!(config.workers.len(), 1);
    let w = &config.workers[0];
    assert_eq!(w.host, "worker1.example.com");
    assert_eq!(w.user, "smelt");
    assert_eq!(w.key_env, "WORKER_SSH_KEY");
    assert_eq!(w.port, 2222);
}

#[test]
fn test_worker_config_defaults() {
    use crate::serve::config::ServerConfig;

    let toml = r#"
queue_dir = "/tmp/smelt-queue"
max_concurrent = 2

[[workers]]
host = "worker1.example.com"
user = "smelt"
key_env = "WORKER_SSH_KEY"
"#;
    let config: ServerConfig = toml::from_str(toml).expect("worker without port should parse");
    assert_eq!(config.workers.len(), 1);
    assert_eq!(config.workers[0].port, 22, "default port should be 22");
}

#[test]
fn test_server_config_no_workers_parses() {
    use crate::serve::config::ServerConfig;

    let toml = r#"
queue_dir = "/tmp/smelt-queue"
max_concurrent = 2
"#;
    let config: ServerConfig = toml::from_str(toml).expect("config without workers should parse");
    assert!(config.workers.is_empty(), "workers should default to empty vec");
    assert_eq!(config.ssh_timeout_secs, 3, "ssh_timeout_secs should default to 3");
}

#[test]
fn test_worker_config_deny_unknown_fields() {
    use crate::serve::config::ServerConfig;

    let toml = r#"
queue_dir = "/tmp/smelt-queue"
max_concurrent = 2

[[workers]]
host = "worker1.example.com"
user = "smelt"
key_env = "WORKER_SSH_KEY"
unknown_field = "should fail"
"#;
    let result: Result<ServerConfig, _> = toml::from_str(toml);
    assert!(result.is_err(), "unknown field in [[workers]] should fail to parse");
}

#[test]
fn test_worker_config_empty_host_fails_validation() {
    use std::io::Write;
    use tempfile::NamedTempFile;
    use crate::serve::config::ServerConfig;

    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, r#"queue_dir = "/tmp/smelt-queue""#).unwrap();
    writeln!(f, "max_concurrent = 2").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "[[workers]]").unwrap();
    writeln!(f, r#"host = """#).unwrap();
    writeln!(f, r#"user = "smelt""#).unwrap();
    writeln!(f, r#"key_env = "WORKER_SSH_KEY""#).unwrap();
    f.flush().unwrap();

    let result = ServerConfig::load(f.path());
    assert!(result.is_err(), "empty host should fail validation");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("host"),
        "error should mention 'host', got: {err_msg}"
    );
}

#[test]
fn test_worker_config_empty_user_fails_validation() {
    use std::io::Write;
    use tempfile::NamedTempFile;
    use crate::serve::config::ServerConfig;

    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, r#"queue_dir = "/tmp/smelt-queue""#).unwrap();
    writeln!(f, "max_concurrent = 2").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "[[workers]]").unwrap();
    writeln!(f, r#"host = "worker1.example.com""#).unwrap();
    writeln!(f, r#"user = """#).unwrap();
    writeln!(f, r#"key_env = "WORKER_SSH_KEY""#).unwrap();
    f.flush().unwrap();

    let result = ServerConfig::load(f.path());
    assert!(result.is_err(), "empty user should fail validation");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("user"),
        "error should mention 'user', got: {err_msg}"
    );
}

// ──────────────────────────────────────────────
// S03/T01 ServerConfig unit tests
// ──────────────────────────────────────────────

#[test]
fn test_server_config_roundtrip() {
    let toml = r#"
queue_dir = "/tmp/smelt-queue"
max_concurrent = 4
retry_attempts = 5
retry_backoff_secs = 10

[server]
host = "0.0.0.0"
port = 9000
"#;
    let config: ServerConfig = toml::from_str(toml).expect("valid TOML should parse");
    assert_eq!(config.queue_dir, std::path::PathBuf::from("/tmp/smelt-queue"));
    assert_eq!(config.max_concurrent, 4);
    assert_eq!(config.retry_attempts, 5);
    assert_eq!(config.retry_backoff_secs, 10);
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 9000);
}

#[test]
fn test_server_config_missing_queue_dir() {
    let toml = r#"
max_concurrent = 2
"#;
    let result: Result<ServerConfig, _> = toml::from_str(toml);
    assert!(result.is_err(), "missing required field queue_dir should fail");
}

#[test]
fn test_server_config_invalid_max_concurrent() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, r#"queue_dir = "/tmp/smelt-queue""#).unwrap();
    writeln!(f, "max_concurrent = 0").unwrap();
    f.flush().unwrap();

    let result = ServerConfig::load(f.path());
    assert!(result.is_err(), "max_concurrent=0 should fail validation");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("max_concurrent"),
        "error message should mention 'max_concurrent', got: {err_msg}"
    );
}

// ──────────────────────────────────────────────
// S03/T02 serve integration test
// ──────────────────────────────────────────────

/// Verify that `smelt serve --no-tui --config <tmpfile>` starts, the HTTP API
/// responds with `[]` on GET /api/v1/jobs, and the task handle can be aborted
/// cleanly (no panic, no zombie).
///
/// Uses port 18765 to avoid needing port-0 extraction across tokio::spawn.
#[tokio::test]
async fn test_serve_http_responds_while_running() {
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    use crate::commands::serve::{ServeArgs, execute};

    let queue_dir = TempDir::new().unwrap();
    let queue_dir_path = queue_dir.path().to_path_buf();

    // Write a minimal server.toml pointing to the temp queue dir.
    let mut cfg_file = NamedTempFile::new().unwrap();
    writeln!(
        cfg_file,
        r#"queue_dir = "{}"
max_concurrent = 2

[server]
host = "127.0.0.1"
port = 18765
"#,
        queue_dir_path.display()
    )
    .unwrap();
    cfg_file.flush().unwrap();

    let cfg_path = cfg_file.path().to_path_buf();

    let handle = tokio::spawn(async move {
        let args = ServeArgs {
            config: cfg_path,
            no_tui: true,
        };
        execute(&args).await.expect("serve execute failed");
    });

    // Give the server time to bind and start accepting connections.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get("http://127.0.0.1:18765/api/v1/jobs")
        .send()
        .await
        .expect("GET /api/v1/jobs should succeed");

    assert_eq!(resp.status(), 200, "GET /api/v1/jobs should return 200");
    let body: Vec<serde_json::Value> = resp.json().await.expect("response should be JSON array");
    assert!(body.is_empty(), "initial job list should be empty");

    // Abort the server task — simulates clean teardown in tests.
    handle.abort();
    // A brief wait ensures the OS releases the port before any subsequent test.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}


// ──────────────────────────────────────────────
// S02 gated integration test — manifest delivery + remote exec
// ──────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn test_manifest_delivery_and_remote_exec() {
    if std::env::var("SMELT_SSH_TEST").is_err() {
        return;
    }

    use tempfile::NamedTempFile;
    use std::io::Write;

    use crate::serve::config::WorkerConfig;
    use crate::serve::ssh::{SubprocessSshClient, SshClient, deliver_manifest};
    use crate::serve::types::JobId;

    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "root".to_string());

    let worker = WorkerConfig {
        host: "127.0.0.1".to_string(),
        user,
        key_env: "SMELT_SSH_KEY".to_string(),
        port: 22,
    };

    // Write a valid manifest to a temp file.
    let mut tmp = NamedTempFile::new().expect("create temp file");
    write!(tmp, "{}", VALID_MANIFEST_TOML).expect("write manifest");
    tmp.flush().expect("flush");

    let client = SubprocessSshClient;
    let job_id = JobId::new("integration-test-1");

    // Deliver the manifest.
    let remote_path = deliver_manifest(&client, &worker, 5, &job_id, tmp.path())
        .await
        .expect("deliver_manifest should succeed");
    assert_eq!(remote_path, "/tmp/smelt-integration-test-1.toml");

    // Verify the file exists on remote.
    let check = client
        .exec(&worker, 5, &format!("test -f {remote_path}"))
        .await
        .expect("exec test -f");
    assert_eq!(check.exit_code, 0, "remote file should exist");

    // Run with --dry-run to avoid needing Docker on the test host.
    let dry_run_cmd = format!("smelt run --dry-run {remote_path}");
    let run_output = client
        .exec(&worker, 10, &dry_run_cmd)
        .await
        .expect("exec smelt run --dry-run");
    assert_eq!(
        run_output.exit_code, 0,
        "smelt run --dry-run should exit 0, stderr: {}",
        run_output.stderr.trim()
    );
    assert!(
        !run_output.stderr.contains("not found"),
        "stderr should not contain 'not found': {}",
        run_output.stderr.trim()
    );

    // Clean up — best-effort.
    let _ = client
        .exec(&worker, 5, &format!("rm -f {remote_path}"))
        .await;
}

#[test]
fn test_tui_render_no_panic() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use crate::serve::tui::render;
    use crate::serve::queue::ServerState;
    use std::sync::{Arc, Mutex};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = Arc::new(Mutex::new(ServerState::new(2)));

    // Render with empty state — must not panic
    terminal.draw(|frame| render(frame, &state)).unwrap();

    // Add a mock job entry to state and render again
    // (directly mutate queue for test — no manifest file needed)
    {
        use std::path::PathBuf;
        use crate::serve::types::{JobSource, JobStatus, QueuedJob, JobId, now_epoch};
        let mut s = state.lock().unwrap();
        s.jobs.push_back(QueuedJob {
            id: JobId::new("job-1"),
            manifest_path: PathBuf::from("test-manifest.toml"),
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
        });
    }
    terminal.draw(|frame| render(frame, &state)).unwrap();
}
