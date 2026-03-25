use crate::serve::queue::ServerState;

use super::{VALID_MANIFEST_TOML, start_test_server};

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
    assert_eq!(
        s.jobs.len(),
        0,
        "no job should be enqueued on parse failure"
    );
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
