use smelt_core::assay::compute_smelt_event_env;

use crate::serve::events::{AssayEvent, DEFAULT_EVENT_STORE_CAPACITY, EventStore};

fn make_event(job_id: &str, seq: u64) -> AssayEvent {
    AssayEvent {
        job_id: job_id.to_string(),
        event_id: Some(format!("evt-{seq}")),
        received_at: 1_000_000 + seq,
        payload: serde_json::json!({ "seq": seq }),
    }
}

// ─── EventStore unit tests ─────────────────────────────────────────────

#[test]
fn test_event_store_push_within_capacity() {
    let mut store = EventStore::new(10);
    for i in 0..5 {
        store.push(make_event("job-1", i));
    }
    assert_eq!(store.len(), 5);
    assert_eq!(store.dropped(), 0);
}

#[test]
fn test_event_store_overflow_drops_oldest() {
    let mut store = EventStore::new(DEFAULT_EVENT_STORE_CAPACITY);
    // Push one more than capacity
    for i in 0..257 {
        store.push(make_event("job-1", i));
    }
    assert_eq!(store.len(), 256);
    assert_eq!(store.dropped(), 1);
    // Oldest remaining should be seq=1 (seq=0 was dropped)
    let first = store.iter().next().unwrap();
    assert_eq!(first.received_at, 1_000_001);
}

#[test]
fn test_event_store_drop_counter() {
    let mut store = EventStore::new(100);
    for i in 0..300 {
        store.push(make_event("job-1", i));
    }
    assert_eq!(store.len(), 100);
    assert_eq!(store.dropped(), 200);
}

#[test]
fn test_event_store_default_capacity() {
    assert_eq!(DEFAULT_EVENT_STORE_CAPACITY, 256);
}

#[test]
fn test_assay_event_clone() {
    let event = make_event("job-1", 42);
    let cloned = event.clone();
    assert_eq!(cloned.job_id, "job-1");
    assert_eq!(cloned.received_at, event.received_at);
}

#[test]
fn test_broadcast_send_no_receivers() {
    // Sending on broadcast with no active receivers must not panic
    let (tx, _rx) = tokio::sync::broadcast::channel::<AssayEvent>(16);
    drop(_rx); // ensure no receivers
    let event = make_event("job-1", 1);
    // SendError is expected but must not panic
    let _ = tx.send(event);
}

// ─── Event env computation tests ───────────────────────────────────────

#[test]
fn test_compute_smelt_event_env_url_format() {
    let env = compute_smelt_event_env("host.docker.internal", 8765, "job-42", None);
    assert_eq!(
        env.get("SMELT_EVENT_URL").unwrap(),
        "http://host.docker.internal:8765/api/v1/events",
        "SMELT_EVENT_URL must use correct format"
    );
    assert_eq!(
        env.get("SMELT_JOB_ID").unwrap(),
        "job-42",
        "SMELT_JOB_ID must match job_id"
    );
    assert!(
        !env.contains_key("SMELT_WRITE_TOKEN"),
        "no token when auth_token is None"
    );
}

#[test]
fn test_compute_smelt_event_env_with_auth_token() {
    let env = compute_smelt_event_env("172.17.0.1", 9090, "build-job", Some("tok-123"));
    assert_eq!(
        env.get("SMELT_EVENT_URL").unwrap(),
        "http://172.17.0.1:9090/api/v1/events"
    );
    assert_eq!(env.get("SMELT_JOB_ID").unwrap(), "build-job");
    assert_eq!(env.get("SMELT_WRITE_TOKEN").unwrap(), "tok-123");
}

// ─── HTTP event POST integration tests ─────────────────────────────────

#[tokio::test]
async fn test_post_event_valid() {
    let (state, job_id) = super::state_with_job();

    // Subscribe to broadcast BEFORE posting, so we can verify delivery.
    let mut rx = {
        let s = state.lock().unwrap();
        s.event_bus.subscribe()
    };

    let base = super::start_test_server(state.clone()).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": job_id,
            "event_id": "evt-1",
            "phase": "running",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "POST valid event should return 200");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // Verify event stored in ServerState.
    {
        let s = state.lock().unwrap();
        let store = s
            .events
            .get(&job_id)
            .expect("EventStore should exist for job");
        assert_eq!(store.len(), 1, "one event should be stored");
        let stored = store.iter().next().unwrap();
        assert_eq!(stored.job_id, job_id);
        assert_eq!(stored.event_id.as_deref(), Some("evt-1"));
        // Control fields (job_id, event_id) should be stripped from payload.
        assert!(
            stored.payload.get("job_id").is_none(),
            "job_id should be stripped from payload"
        );
        assert!(
            stored.payload.get("event_id").is_none(),
            "event_id should be stripped from payload"
        );
        // But other fields remain.
        assert_eq!(stored.payload["phase"], "running");
    }

    // Verify broadcast delivery.
    let received = rx.try_recv().expect("should receive broadcast event");
    assert_eq!(received.job_id, job_id);
    assert_eq!(received.event_id.as_deref(), Some("evt-1"));
}

#[tokio::test]
async fn test_post_event_unknown_job() {
    let (state, _job_id) = super::state_with_job();
    let base = super::start_test_server(state).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": "nonexistent-job",
            "phase": "running",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404, "unknown job_id should return 404");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["error"].as_str().unwrap().contains("unknown"),
        "error should mention unknown job_id"
    );
}

#[tokio::test]
async fn test_post_event_missing_job_id() {
    let (state, _job_id) = super::state_with_job();
    let base = super::start_test_server(state).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "phase": "running",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400, "missing job_id should return 400");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["error"].as_str().unwrap().contains("job_id"),
        "error should mention job_id"
    );
}

#[tokio::test]
async fn test_post_event_auth_required() {
    let (state, job_id) = super::state_with_job();
    let auth = crate::serve::http_api::ResolvedAuth {
        write_token: "write-secret".to_string(),
        read_token: Some("read-secret".to_string()),
    };
    let base = super::start_test_server_with_auth(state.clone(), Some(auth)).await;
    let client = reqwest::Client::new();

    // POST without token → 401
    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({ "job_id": job_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        401,
        "POST event without auth should return 401"
    );

    // POST with read token → 403 (write required)
    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({ "job_id": job_id }))
        .header("Authorization", "Bearer read-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        403,
        "POST event with read token should return 403"
    );

    // POST with write token → 200
    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({ "job_id": job_id }))
        .header("Authorization", "Bearer write-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "POST event with write token should return 200"
    );
}

#[tokio::test]
async fn test_broadcast_no_receivers_returns_200() {
    let (state, job_id) = super::state_with_job();

    // Ensure no broadcast subscribers exist (the default receiver from
    // new_without_events is dropped).
    let base = super::start_test_server(state.clone()).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/events"))
        .json(&serde_json::json!({
            "job_id": job_id,
            "event_id": "evt-no-recv",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        200,
        "POST event with no broadcast receivers should still return 200"
    );

    // Event should still be stored even though broadcast had no receivers.
    let s = state.lock().unwrap();
    let store = s.events.get(&job_id).expect("EventStore should exist");
    assert_eq!(store.len(), 1);
}

// ─── SSE endpoint tests ────────────────────────────────────────────────

/// Helper: read SSE chunks from a response until we find one containing `needle`, or timeout.
async fn read_sse_until(
    mut response: reqwest::Response,
    needle: &str,
    timeout_secs: u64,
) -> String {
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        let chunk = tokio::time::timeout_at(deadline, response.chunk()).await;
        match chunk {
            Ok(Ok(Some(bytes))) => {
                let text = String::from_utf8_lossy(&bytes);
                collected.push_str(&text);
                if collected.contains(needle) {
                    return collected;
                }
            }
            Ok(Ok(None)) => panic!("SSE stream ended before finding '{needle}'. Got: {collected}"),
            Ok(Err(e)) => panic!("SSE stream error: {e}"),
            Err(_) => panic!("Timeout waiting for '{needle}' in SSE stream. Got: {collected}"),
        }
    }
}

#[tokio::test]
async fn test_sse_global_receives_event() {
    let (state, job_id) = super::state_with_job();
    let base = super::start_test_server(state.clone()).await;
    let client = reqwest::Client::new();

    // Connect to SSE endpoint.
    let sse_resp = client
        .get(format!("{base}/api/v1/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(sse_resp.status(), 200, "SSE endpoint should return 200");
    assert!(
        sse_resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/event-stream"),
        "SSE should return text/event-stream content type"
    );

    // POST an event to the ingestion endpoint (in background — SSE read blocks).
    let base2 = base.clone();
    let job_id2 = job_id.clone();
    tokio::spawn(async move {
        // Small delay to ensure SSE subscriber is connected before posting.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let c = reqwest::Client::new();
        let post_resp = c
            .post(format!("{base2}/api/v1/events"))
            .json(&serde_json::json!({
                "job_id": job_id2,
                "phase": "running",
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(post_resp.status(), 200);
    });

    // Read SSE chunks until we see the job_id.
    let body: String = read_sse_until(sse_resp, &job_id, 3).await;
    assert!(
        body.contains("running"),
        "SSE stream should contain the phase"
    );
}

#[tokio::test]
async fn test_sse_job_filtered() {
    use crate::serve::types::{JobId, JobSource, JobStatus, QueuedJob, now_epoch};

    // Create state with two jobs.
    let state = std::sync::Arc::new(std::sync::Mutex::new(
        crate::serve::queue::ServerState::new_without_events(4),
    ));
    {
        let mut s = state.lock().unwrap();
        for name in ["job-a", "job-b"] {
            s.jobs.push_back(QueuedJob {
                id: JobId::new(name),
                manifest_path: std::path::PathBuf::from("test.toml"),
                source: JobSource::HttpApi,
                attempt: 0,
                status: JobStatus::Running,
                queued_at: now_epoch(),
                started_at: Some(now_epoch()),
                worker_host: None,
            });
        }
    }

    let base = super::start_test_server(state.clone()).await;
    let client = reqwest::Client::new();

    // Connect to SSE filtered for job-a.
    let sse_resp = client
        .get(format!("{base}/api/v1/events?job=job-a"))
        .send()
        .await
        .unwrap();
    assert_eq!(sse_resp.status(), 200);

    // POST events in background after a small delay for SSE to connect.
    let base2 = base.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let c = reqwest::Client::new();
        // POST event for job-b (should NOT appear on filtered stream).
        c.post(format!("{base2}/api/v1/events"))
            .json(&serde_json::json!({ "job_id": "job-b", "phase": "running" }))
            .send()
            .await
            .unwrap();
        // POST event for job-a (SHOULD appear on filtered stream).
        c.post(format!("{base2}/api/v1/events"))
            .json(&serde_json::json!({ "job_id": "job-a", "phase": "complete" }))
            .send()
            .await
            .unwrap();
    });

    // Read SSE until we see job-a data.
    let body: String = read_sse_until(sse_resp, "job-a", 3).await;
    assert!(
        body.contains("complete"),
        "filtered SSE should contain job-a's phase"
    );
    // job-b events should not appear (they're filtered out).
    assert!(
        !body.contains("job-b"),
        "filtered SSE should NOT contain job-b events: got {body}"
    );
}

#[tokio::test]
async fn test_sse_lagged_subscriber() {
    let (state, _job_id) = super::state_with_job();

    // Get a reference to the broadcast sender before starting the server.
    let event_bus = {
        let s = state.lock().unwrap();
        s.event_bus.clone()
    };

    let base = super::start_test_server(state.clone()).await;
    let client = reqwest::Client::new();

    // Connect SSE subscriber.
    let sse_resp = client
        .get(format!("{base}/api/v1/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(sse_resp.status(), 200);

    // new_without_events uses broadcast capacity 16.
    // Send more than capacity in background to trigger Lagged error.
    let event_bus2 = event_bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        for i in 0..20 {
            let _ = event_bus2.send(make_event("test-job-1", i));
        }
    });

    // Read SSE until we see the synthetic "lagged" event.
    let body: String = read_sse_until(sse_resp, "lagged", 3).await;
    assert!(
        body.contains("dropped"),
        "lagged event should contain dropped count: got {body}"
    );
}

#[tokio::test]
async fn test_sse_auth() {
    let (state, _job_id) = super::state_with_job();
    let auth = crate::serve::http_api::ResolvedAuth {
        write_token: "write-secret".to_string(),
        read_token: Some("read-secret".to_string()),
    };
    let base = super::start_test_server_with_auth(state.clone(), Some(auth)).await;
    let client = reqwest::Client::new();

    // GET SSE without token → 401
    let resp = client
        .get(format!("{base}/api/v1/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "SSE without auth should return 401");

    // GET SSE with read token → 200 (read operations allow read token)
    let resp = client
        .get(format!("{base}/api/v1/events"))
        .header("Authorization", "Bearer read-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "SSE with read token should return 200");

    // GET SSE with write token → 200
    let resp = client
        .get(format!("{base}/api/v1/events"))
        .header("Authorization", "Bearer write-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "SSE with write token should return 200");
}

#[tokio::test]
async fn test_sse_end_to_end_post_to_stream() {
    let (state, job_id) = super::state_with_job();
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let base =
        super::start_test_server_with_cancel(state.clone(), None, cancel_token.clone()).await;
    let client = reqwest::Client::new();

    // Connect to SSE endpoint.
    let sse_resp = client
        .get(format!("{base}/api/v1/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(sse_resp.status(), 200);

    // POST an event via the ingestion endpoint (in background).
    let base2 = base.clone();
    let job_id2 = job_id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let c = reqwest::Client::new();
        let resp = c
            .post(format!("{base2}/api/v1/events"))
            .json(&serde_json::json!({
                "job_id": job_id2,
                "phase": "complete",
                "sessions": [{"name": "frontend", "passed": true}],
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    });

    // Read SSE stream until we see the event data.
    let body: String = read_sse_until(sse_resp, "complete", 3).await;

    // Verify the SSE event contains the correct job_id and payload.
    assert!(
        body.contains(&job_id),
        "SSE stream should contain job_id '{job_id}': got {body}"
    );
    assert!(
        body.contains("frontend"),
        "SSE stream should contain session data from payload"
    );

    // Also verify the event is stored in ServerState (TUI-readable path).
    {
        let s = state.lock().unwrap();
        let store = s.events.get(&job_id).expect("EventStore should exist");
        assert!(!store.is_empty(), "at least 1 event should be stored");
    }
}

#[tokio::test]
async fn test_sse_shutdown_closes_streams() {
    let (state, _job_id) = super::state_with_job();
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let base =
        super::start_test_server_with_cancel(state.clone(), None, cancel_token.clone()).await;
    let client = reqwest::Client::new();

    // Connect to SSE endpoint.
    let sse_resp = client
        .get(format!("{base}/api/v1/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(sse_resp.status(), 200);

    // Cancel the token after a short delay — SSE stream should close.
    let token = cancel_token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        token.cancel();
    });

    // Reading the full body should complete (stream terminates) within the timeout.
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), sse_resp.text()).await;

    match result {
        Ok(Ok(_body)) => {
            // Stream closed cleanly — this is the expected path.
        }
        Ok(Err(e)) => {
            // Connection error is also acceptable if the server shut down the stream.
            // reqwest may report a connection reset.
            let msg = e.to_string();
            assert!(
                msg.contains("reset") || msg.contains("closed") || msg.contains("eof"),
                "unexpected error on shutdown: {msg}"
            );
        }
        Err(_) => {
            panic!("SSE stream did not close within 3s after CancellationToken was cancelled");
        }
    }
}
