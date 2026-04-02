use super::VALID_MANIFEST_TOML;

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

    let state = Arc::new(Mutex::new(ServerState::new_without_events(2)));
    {
        let mut s = state.lock().unwrap();
        s.enqueue(m1, JobSource::DirectoryWatch);
        s.enqueue(m2, JobSource::DirectoryWatch);
    }

    let cancel = CancellationToken::new();
    let cancel2 = cancel.clone();
    let state2 = Arc::clone(&state);
    let handle = tokio::spawn(async move {
        dispatch_loop(
            state2,
            cancel2,
            1,
            vec![],
            crate::serve::SubprocessSshClient,
            3,
            None,
        )
        .await;
    });

    // Wait up to 60 s for both jobs to reach a terminal state.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let done = {
            let s = state.lock().unwrap();
            s.jobs
                .iter()
                .all(|j| matches!(j.status, JobStatus::Complete | JobStatus::Failed))
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
    std::fs::write(&manifest_path, VALID_MANIFEST_TOML).unwrap();

    let state = Arc::new(Mutex::new(ServerState::new_without_events(2)));
    let watcher = DirectoryWatcher::new(queue_dir.clone(), Arc::clone(&state));

    let handle = tokio::spawn(async move {
        watcher.watch().await;
    });

    // Wait long enough for at least one poll cycle (2s interval).
    // Use 6s to allow for CI runner scheduling jitter.
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;

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
    std::fs::write(&manifest_path, VALID_MANIFEST_TOML).unwrap();

    let state = Arc::new(Mutex::new(ServerState::new_without_events(2)));
    let watcher = DirectoryWatcher::new(queue_dir.clone(), Arc::clone(&state));

    let handle = tokio::spawn(async move {
        watcher.watch().await;
    });

    // Wait for watcher to pick up the file.
    // Use 6s to allow for CI runner scheduling jitter.
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;

    // Original file should be gone from queue_dir root.
    assert!(
        !manifest_path.exists(),
        "original TOML should no longer exist in queue_dir root"
    );

    // dispatched/ should contain exactly 1 file matching *-move-test.toml.
    let dispatched_dir = queue_dir.join("dispatched");
    assert!(
        dispatched_dir.exists(),
        "dispatched/ directory should exist"
    );

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
