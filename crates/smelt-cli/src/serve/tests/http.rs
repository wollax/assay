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

// ─── Auth integration tests ────────────────────────────────────────────

/// Helper: create a `ResolvedAuth` with both read and write tokens.
fn auth_both_tokens() -> crate::serve::http_api::ResolvedAuth {
    crate::serve::http_api::ResolvedAuth {
        write_token: "write-secret".to_string(),
        read_token: Some("read-secret".to_string()),
    }
}

/// Helper: create a `ResolvedAuth` with only a write token (write-only mode).
fn auth_write_only() -> crate::serve::http_api::ResolvedAuth {
    crate::serve::http_api::ResolvedAuth {
        write_token: "write-secret".to_string(),
        read_token: None,
    }
}

/// Helper: start an auth-enabled test server.
async fn start_auth_server(
    auth: crate::serve::http_api::ResolvedAuth,
) -> (
    String,
    std::sync::Arc<std::sync::Mutex<super::super::queue::ServerState>>,
) {
    let state = std::sync::Arc::new(std::sync::Mutex::new(
        super::super::queue::ServerState::new(4),
    ));
    let base = super::start_test_server_with_auth(state.clone(), Some(auth)).await;
    (base, state)
}

/// Helper: seed a job so DELETE endpoints have something to target.
async fn seed_job(client: &reqwest::Client, base: &str) -> String {
    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Bearer write-secret")
        .body(super::VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    body["job_id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_auth_missing_header_returns_401() {
    let (base, _state) = start_auth_server(auth_both_tokens()).await;
    let client = reqwest::Client::new();

    // GET without Authorization → 401
    let resp = client
        .get(format!("{base}/api/v1/jobs"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["error"].as_str().unwrap().contains("Authorization"),
        "401 body should mention Authorization header"
    );

    // POST without Authorization → 401
    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .body(VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"].is_string());

    // DELETE without Authorization → 401
    let resp = client
        .delete(format!("{base}/api/v1/jobs/fake-id"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn test_auth_invalid_token_returns_403() {
    let (base, _state) = start_auth_server(auth_both_tokens()).await;
    let client = reqwest::Client::new();

    // GET with wrong token → 403 (token extracted but unrecognized)
    let resp = client
        .get(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Bearer wrong-token")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        403,
        "unrecognized token on GET should be 403"
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["error"].as_str().unwrap().contains("read"),
        "should mention read permission"
    );

    // POST with wrong token → 403
    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Bearer wrong-token")
        .body(VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        403,
        "unrecognized token on POST should be 403"
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["error"].as_str().unwrap().contains("write"),
        "should mention write permission"
    );

    // Also test truly malformed header (no "Bearer " prefix) → 401
    let resp = client
        .get(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Token wrong-token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "malformed auth header should be 401");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("Authorization"));
}

#[tokio::test]
async fn test_auth_read_token_permission_split() {
    let (base, _state) = start_auth_server(auth_both_tokens()).await;
    let client = reqwest::Client::new();

    // Read token on GET → 200 (read permission)
    let resp = client
        .get(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Bearer read-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "read token should allow GET");

    // Read token on POST → 403 (needs write permission)
    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Bearer read-secret")
        .body(VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "read token should be denied on POST");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["error"].as_str().unwrap().contains("write"),
        "403 body should mention write permission"
    );

    // Read token on DELETE → 403
    let resp = client
        .delete(format!("{base}/api/v1/jobs/fake-id"))
        .header("Authorization", "Bearer read-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "read token should be denied on DELETE");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("write"));

    // Write token on GET → 200
    let resp = client
        .get(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Bearer write-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "write token should allow GET");

    // Write token on POST → 200
    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Bearer write-secret")
        .body(VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "write token should allow POST");

    // Write token on DELETE → 200 (job exists from the POST above)
    let job_id = {
        let resp = client
            .get(format!("{base}/api/v1/jobs"))
            .header("Authorization", "Bearer write-secret")
            .send()
            .await
            .unwrap();
        let jobs: Vec<serde_json::Value> = resp.json().await.unwrap();
        jobs[0]["id"].as_str().unwrap().to_string()
    };
    let resp = client
        .delete(format!("{base}/api/v1/jobs/{job_id}"))
        .header("Authorization", "Bearer write-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "write token should allow DELETE");
}

#[tokio::test]
async fn test_auth_write_only_mode() {
    let (base, _state) = start_auth_server(auth_write_only()).await;
    let client = reqwest::Client::new();

    // Write token on GET → 200 (write token grants read access too)
    let resp = client
        .get(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Bearer write-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "write token should allow GET in write-only mode"
    );

    // Write token on POST → 200
    let resp = client
        .post(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Bearer write-secret")
        .body(VALID_MANIFEST_TOML)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "write token should allow POST");

    // Write token on DELETE → 200
    let job_id = seed_job(&client, &base).await;
    let resp = client
        .delete(format!("{base}/api/v1/jobs/{job_id}"))
        .header("Authorization", "Bearer write-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "write token should allow DELETE");

    // Some other token on GET → 403 (no read token configured, not the write token)
    let resp = client
        .get(format!("{base}/api/v1/jobs"))
        .header("Authorization", "Bearer other-token")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        403,
        "unknown token should be rejected in write-only mode"
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["error"].as_str().unwrap().contains("read"),
        "should mention read permission"
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
