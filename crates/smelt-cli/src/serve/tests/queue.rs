use crate::serve::queue::ServerState;
use crate::serve::types::{JobSource, JobStatus};

use super::manifest;

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
    assert!(
        second.is_none(),
        "second dispatch should be blocked (max_concurrent=1)"
    );

    // Complete the first job, then the second should dispatch.
    state.complete(&first.unwrap().id, true, 0, 3);
    assert_eq!(state.running_count, 0);

    let second = state.try_dispatch();
    assert!(
        second.is_some(),
        "second job should dispatch after first completes"
    );
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
    assert!(
        state.cancel(&id_waiting),
        "cancel of Queued job should return true"
    );

    // Cancelling a Dispatching job should fail.
    assert!(
        !state.cancel(&id_queued),
        "cancel of Dispatching job should return false"
    );
    assert!(
        !state.cancel(&id_dispatching),
        "cancel of Dispatching job should return false"
    );
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
        assert!(
            state.retry_eligible(&rid, 3),
            "should be retry eligible (attempt < max)"
        );
        // Simulate reaching max_attempts.
        assert!(
            !state.retry_eligible(&rid, 1),
            "should NOT be eligible if attempt >= max_attempts"
        );
    } else {
        panic!("expected a Retrying job after failure with remaining attempts");
    }
}
