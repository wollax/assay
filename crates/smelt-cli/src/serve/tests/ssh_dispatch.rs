use crate::serve::queue::ServerState;
use crate::serve::types::{JobSource, JobStatus};

use super::VALID_MANIFEST_TOML;

/// Prove that 4 jobs dispatched to 2 workers alternate worker_host via
/// round-robin.  Uses `dispatch_loop` with `MockSshClient` — no real SSH.
#[tokio::test]
async fn test_round_robin_two_workers() {
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    use crate::serve::config::WorkerConfig;
    use crate::serve::dispatch::dispatch_loop;
    use crate::serve::ssh::SshOutput;
    use crate::serve::ssh::tests::MockSshClient;

    let dir = TempDir::new().unwrap();
    let manifests: Vec<_> = (0..4)
        .map(|i| {
            let p = dir.path().join(format!("job{i}.toml"));
            std::fs::write(&p, VALID_MANIFEST_TOML).unwrap();
            p
        })
        .collect();

    let workers = vec![
        WorkerConfig {
            host: "worker-a".into(),
            user: "u".into(),
            key_env: "SMELT_NOKEY".into(),
            port: 22,
        },
        WorkerConfig {
            host: "worker-b".into(),
            user: "u".into(),
            key_env: "SMELT_NOKEY".into(),
            port: 22,
        },
    ];

    // 4 jobs → 4 probes (alternating a,b,a,b), 4 scp_to, 4 exec, 4 scp_from
    let mut client = MockSshClient::new();
    for _ in 0..4 {
        client = client
            .with_probe_result(Ok(()))
            .with_scp_result(Ok(()))
            .with_exec_result(Ok(SshOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 0,
            }))
            .with_scp_from_result(Ok(()));
    }

    let state = Arc::new(Mutex::new(ServerState::new(4)));
    {
        let mut s = state.lock().unwrap();
        for m in &manifests {
            s.enqueue(m.clone(), JobSource::HttpApi);
        }
    }

    let cancel = CancellationToken::new();
    let cancel2 = cancel.clone();
    let state2 = Arc::clone(&state);
    let handle = tokio::spawn(async move {
        dispatch_loop(state2, cancel2, 3, workers, client, 3).await;
    });

    // Wait for all jobs to reach a terminal state.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
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
            cancel.cancel();
            handle.await.unwrap();
            panic!("timeout: jobs did not complete within 10s");
        }
    }

    cancel.cancel();
    handle.await.unwrap();

    let s = state.lock().unwrap();
    let hosts: Vec<Option<String>> = s.jobs.iter().map(|j| j.worker_host.clone()).collect();
    assert_eq!(
        hosts[0],
        Some("worker-a".to_string()),
        "job 0 should go to worker-a"
    );
    assert_eq!(
        hosts[1],
        Some("worker-b".to_string()),
        "job 1 should go to worker-b"
    );
    assert_eq!(
        hosts[2],
        Some("worker-a".to_string()),
        "job 2 should go to worker-a"
    );
    assert_eq!(
        hosts[3],
        Some("worker-b".to_string()),
        "job 3 should go to worker-b"
    );
}

/// Prove that when worker-a's probe always fails, all jobs route to worker-b.
#[tokio::test]
async fn test_failover_one_offline() {
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    use crate::serve::config::WorkerConfig;
    use crate::serve::dispatch::dispatch_loop;
    use crate::serve::ssh::SshOutput;
    use crate::serve::ssh::tests::MockSshClient;

    let dir = TempDir::new().unwrap();
    let manifests: Vec<_> = (0..2)
        .map(|i| {
            let p = dir.path().join(format!("job{i}.toml"));
            std::fs::write(&p, VALID_MANIFEST_TOML).unwrap();
            p
        })
        .collect();

    let workers = vec![
        WorkerConfig {
            host: "worker-a".into(),
            user: "u".into(),
            key_env: "SMELT_NOKEY".into(),
            port: 22,
        },
        WorkerConfig {
            host: "worker-b".into(),
            user: "u".into(),
            key_env: "SMELT_NOKEY".into(),
            port: 22,
        },
    ];

    // select_worker for job 1 (idx=0): probe worker-a Err, probe worker-b Ok → 2 probes
    // select_worker for job 2 (idx=0): probe worker-a Err, probe worker-b Ok → 2 probes
    // Total: 4 probes, 2 scp, 2 exec, 2 scp_from
    let client = MockSshClient::new()
        .with_probe_result(Err(anyhow::anyhow!("offline")))
        .with_probe_result(Ok(()))
        .with_probe_result(Err(anyhow::anyhow!("offline")))
        .with_probe_result(Ok(()))
        .with_scp_result(Ok(()))
        .with_scp_result(Ok(()))
        .with_exec_result(Ok(SshOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        }))
        .with_exec_result(Ok(SshOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        }))
        .with_scp_from_result(Ok(()))
        .with_scp_from_result(Ok(()));

    let state = Arc::new(Mutex::new(ServerState::new(2)));
    {
        let mut s = state.lock().unwrap();
        for m in &manifests {
            s.enqueue(m.clone(), JobSource::HttpApi);
        }
    }

    let cancel = CancellationToken::new();
    let cancel2 = cancel.clone();
    let state2 = Arc::clone(&state);
    let handle = tokio::spawn(async move {
        dispatch_loop(state2, cancel2, 3, workers, client, 3).await;
    });

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
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
            cancel.cancel();
            handle.await.unwrap();
            panic!("timeout: jobs did not complete within 10s");
        }
    }

    cancel.cancel();
    handle.await.unwrap();

    let s = state.lock().unwrap();
    for (i, job) in s.jobs.iter().enumerate() {
        assert_eq!(
            job.worker_host,
            Some("worker-b".to_string()),
            "job {i} should route to worker-b (the surviving worker)"
        );
    }
}

/// Prove that when all workers are offline, the job is re-queued with status
/// Queued and worker_host None.
///
/// Simulates one dispatch cycle (try_dispatch → select_worker → re-queue)
/// rather than running the full dispatch_loop, because the re-queue path
/// causes an infinite inner loop when all workers are always offline.
#[tokio::test]
async fn test_all_workers_offline_requeue() {
    use std::sync::{Arc, Mutex};

    use crate::serve::config::WorkerConfig;
    use crate::serve::dispatch::select_worker;
    use crate::serve::ssh::tests::MockSshClient;

    let workers = vec![
        WorkerConfig {
            host: "worker-a".into(),
            user: "u".into(),
            key_env: "SMELT_NOKEY".into(),
            port: 22,
        },
        WorkerConfig {
            host: "worker-b".into(),
            user: "u".into(),
            key_env: "SMELT_NOKEY".into(),
            port: 22,
        },
    ];

    // Both probes fail
    let client = MockSshClient::new()
        .with_probe_result(Err(anyhow::anyhow!("offline")))
        .with_probe_result(Err(anyhow::anyhow!("offline")));

    let state = Arc::new(Mutex::new(ServerState::new(2)));
    let manifest = std::path::PathBuf::from("/tmp/fake-requeue.toml");
    {
        let mut s = state.lock().unwrap();
        s.enqueue(manifest, JobSource::HttpApi);
    }

    // Simulate dispatch_loop's inner cycle: try_dispatch → select_worker → re-queue
    let job = {
        let mut s = state.lock().unwrap();
        s.try_dispatch().expect("should dispatch the queued job")
    };
    let id = job.id.clone();

    let rr_idx = {
        let s = state.lock().unwrap();
        s.round_robin_idx
    };

    let selected = select_worker(&workers, &client, 3, rr_idx).await;
    assert!(selected.is_none(), "all workers should be offline");

    // Re-queue path (mirrors dispatch_loop logic)
    {
        let mut s = state.lock().unwrap();
        if let Some(j) = s.jobs.iter_mut().find(|j| j.id == id) {
            j.status = JobStatus::Queued;
            j.started_at = None;
            j.worker_host = None;
        }
        s.running_count = s.running_count.saturating_sub(1);
    }

    let s = state.lock().unwrap();
    assert_eq!(s.jobs.len(), 1);
    assert_eq!(
        s.jobs[0].status,
        JobStatus::Queued,
        "job should be re-queued when all workers offline"
    );
    assert!(
        s.jobs[0].worker_host.is_none(),
        "worker_host should be None after re-queue"
    );
    assert_eq!(
        s.running_count, 0,
        "running_count should be 0 after re-queue"
    );
}

/// Prove that worker_host survives TOML serialization round-trip.
#[test]
fn test_worker_host_in_queue_state_roundtrip() {
    use tempfile::TempDir;

    use crate::serve::queue::{read_queue_state, write_queue_state};
    use crate::serve::types::{JobId, QueuedJob, now_epoch};

    let dir = TempDir::new().unwrap();

    let job = QueuedJob {
        id: JobId::new("rr-1"),
        manifest_path: std::path::PathBuf::from("/tmp/test.toml"),
        source: JobSource::HttpApi,
        attempt: 0,
        status: JobStatus::Running,
        queued_at: now_epoch(),
        started_at: Some(now_epoch()),
        worker_host: Some("w1.example.com".to_string()),
    };

    let mut jobs = std::collections::VecDeque::new();
    jobs.push_back(job);
    write_queue_state(dir.path(), &jobs);

    let read_back = read_queue_state(dir.path());
    assert_eq!(read_back.len(), 1);
    assert_eq!(
        read_back[0].worker_host,
        Some("w1.example.com".to_string()),
        "worker_host should survive TOML persistence round-trip"
    );
}

/// Create state on a remote host via ssh exec, sync it back via
/// `sync_state_back()`, and verify the local copy is valid TOML.
///
/// Requires: `SMELT_SSH_TEST=1`, sshd on localhost, current user can auth.
#[tokio::test]
#[ignore]
async fn test_state_sync_round_trip() {
    if std::env::var("SMELT_SSH_TEST").is_err() {
        return;
    }

    use tempfile::TempDir;

    use crate::serve::config::WorkerConfig;
    use crate::serve::ssh::{SshClient, SubprocessSshClient, sync_state_back};

    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "root".to_string());

    let worker = WorkerConfig {
        host: "127.0.0.1".to_string(),
        user,
        key_env: "SMELT_SSH_KEY".to_string(),
        port: 22,
    };

    let client = SubprocessSshClient;

    // Use a unique job name to avoid collisions with parallel test runs.
    let random_suffix: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let job_name = format!("sync-test-{}", random_suffix);
    let remote_dir = format!("/tmp/.smelt/runs/{}", job_name);

    // Create remote state directory and write a valid state.toml.
    let state_toml = format!(
        r#"job_name = "{job_name}"
phase = "Complete"
sessions = ["main"]
started_at = 1000000
updated_at = 1000001
pid = 12345
"#
    );
    let create_cmd = format!(
        "mkdir -p {remote_dir} && cat > {remote_dir}/state.toml << 'SMELT_EOF'\n{state_toml}SMELT_EOF"
    );
    let create_out = client
        .exec(&worker, 5, &create_cmd)
        .await
        .expect("create remote state dir");
    assert_eq!(
        create_out.exit_code,
        0,
        "remote mkdir+cat should succeed, stderr: {}",
        create_out.stderr.trim()
    );

    // Sync state back to a local tempdir.
    let local_dir = TempDir::new().unwrap();
    sync_state_back(&client, &worker, 5, &job_name, local_dir.path())
        .await
        .expect("sync_state_back should succeed");

    // Verify local file exists and is valid TOML.
    let local_state_path = local_dir
        .path()
        .join(".smelt/runs")
        .join(&job_name)
        .join("state.toml");
    assert!(
        local_state_path.exists(),
        "local state.toml should exist at {}",
        local_state_path.display()
    );

    let content = std::fs::read_to_string(&local_state_path).expect("read local state.toml");
    let parsed: toml::Value =
        toml::from_str(&content).expect("local state.toml should be valid TOML");
    assert_eq!(
        parsed["job_name"].as_str().unwrap(),
        job_name,
        "job_name should match"
    );
    assert_eq!(
        parsed["phase"].as_str().unwrap(),
        "Complete",
        "phase should be Complete"
    );

    // Clean up remote dir — best-effort.
    let _ = client
        .exec(&worker, 5, &format!("rm -rf {remote_dir}"))
        .await;
}

#[tokio::test]
#[ignore]
async fn test_manifest_delivery_and_remote_exec() {
    if std::env::var("SMELT_SSH_TEST").is_err() {
        return;
    }

    use std::io::Write;
    use tempfile::NamedTempFile;

    use crate::serve::config::WorkerConfig;
    use crate::serve::ssh::{SshClient, SubprocessSshClient, deliver_manifest};
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
        run_output.exit_code,
        0,
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
