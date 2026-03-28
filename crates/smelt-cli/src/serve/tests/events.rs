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
