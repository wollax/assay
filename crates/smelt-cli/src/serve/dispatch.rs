//! Dispatch engine: `run_job_task`, `run_ssh_job_task`, `select_worker`, and `dispatch_loop`.
//!
//! `run_job_task` bridges `run_with_cancellation()`'s generic cancel Future to
//! `CancellationToken::cancelled()` via the one-liner adapter:
//!   `async { token.cancelled().await; Ok(()) }`
//!
//! `dispatch_loop` polls `ServerState::try_dispatch()` on a 2-second interval and
//! spawns a `run_job_task` (local) or `run_ssh_job_task` (remote) tokio task for
//! each dispatchable job. The loop breaks cleanly when `cancel_token.cancelled()`
//! fires, and every spawned child task receives a child token so the broadcast
//! propagates to all running jobs.
//!
//! When `workers` is non-empty, `select_worker` performs round-robin selection
//! with probe-based offline skip. If all workers are offline, the job is
//! re-queued (status reverted to `Queued`, running_count decremented).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::commands::run::RunArgs;
use crate::serve::config::WorkerConfig;
use crate::serve::queue::ServerState;
use crate::serve::ssh::{SshClient, deliver_manifest, run_remote_job, sync_state_back};
use crate::serve::types::{JobId, JobStatus};

/// Configuration for Smelt event environment injection into dispatched jobs.
///
/// When present, `run_job_task` injects `SMELT_EVENT_URL`, `SMELT_JOB_ID`,
/// and optionally `SMELT_WRITE_TOKEN` into the container environment.
#[derive(Clone, Debug)]
pub(crate) struct EventEnvConfig {
    /// Host address reachable from inside containers (e.g. `host.docker.internal`).
    pub host: String,
    /// HTTP port of the Smelt server.
    pub port: u16,
    /// Optional write auth token value for SmeltBackend to authenticate.
    pub write_token: Option<String>,
}

/// Execute a single job under the given `CancellationToken`.
///
/// Lifecycle:
/// 1. Transition the job to `Running` in `ServerState`.
/// 2. Build `RunArgs` and the cancel future adapter.
/// 3. Call `run_with_cancellation()`.
/// 4. Call `state.complete()` with the result.
pub(crate) async fn run_job_task(
    manifest_path: PathBuf,
    job_id: JobId,
    state: Arc<Mutex<ServerState>>,
    cancel_token: CancellationToken,
    max_attempts: u32,
    event_env: Option<EventEnvConfig>,
) {
    // --- transition to Running ---
    let attempt = {
        let mut s = state.lock().unwrap();
        if let Some(job) = s.jobs.iter_mut().find(|j| j.id == job_id) {
            job.status = crate::serve::types::JobStatus::Running;
            job.started_at = Some(crate::serve::types::now_epoch());
            job.attempt
        } else {
            warn!(job_id = %job_id, "run_job_task: job not found in state");
            return;
        }
    };

    info!(job_id = %job_id, manifest = %manifest_path.display(), attempt, "job started");

    // Compute event env vars for container injection.
    let runtime_env = match &event_env {
        Some(cfg) => {
            let env = smelt_core::assay::compute_smelt_event_env(
                &cfg.host,
                cfg.port,
                &job_id.0,
                cfg.write_token.as_deref(),
            );
            let event_url = env
                .get("SMELT_EVENT_URL")
                .expect("compute_smelt_event_env must always set SMELT_EVENT_URL");
            info!(
                job_id = %job_id,
                smelt_event_url = %event_url,
                "injecting SMELT_EVENT_URL and SMELT_JOB_ID into container env"
            );
            env
        }
        None => HashMap::new(),
    };

    let args = RunArgs {
        manifest: manifest_path.clone(),
        dry_run: false,
        no_pr: false,
        runtime_env,
    };

    // CancellationToken adapter: convert `cancelled()` (returns ()) to the
    // `Future<Output = std::io::Result<()>>` that `run_with_cancellation` expects.
    let cancel_fut = {
        let t = cancel_token.clone();
        async move {
            t.cancelled().await;
            Ok::<(), std::io::Error>(())
        }
    };

    let result = crate::commands::run::run_with_cancellation(&args, cancel_fut).await;

    let success = match &result {
        Ok(exit_code) => {
            if *exit_code == 0 {
                true
            } else {
                warn!(job_id = %job_id, exit_code, "job exited with non-zero code");
                false
            }
        }
        Err(e) => {
            warn!(job_id = %job_id, error = %e, "job task returned error");
            false
        }
    };

    {
        let mut s = state.lock().unwrap();
        s.complete(&job_id, success, attempt, max_attempts);
    }

    if success {
        info!(job_id = %job_id, "job complete");
    } else {
        // Determine whether we retried or failed permanently.
        let retrying = {
            let s = state.lock().unwrap();
            s.jobs
                .iter()
                .any(|j| j.id == job_id && j.status == crate::serve::types::JobStatus::Retrying)
        };
        if retrying {
            info!(job_id = %job_id, attempt, "job queued for retry");
        } else {
            info!(job_id = %job_id, attempt, "job failed permanently");
        }
    }
}

/// Execute a single job on a remote SSH worker.
///
/// Lifecycle:
/// 1. Transition the job to `Running` in `ServerState`.
/// 2. Call `deliver_manifest` to SCP the manifest to the worker.
/// 3. Call `run_remote_job` to execute `smelt run` on the worker.
/// 4. Call `sync_state_back` to retrieve run state from the worker.
/// 5. Call `state.complete()` with the result.
pub(crate) async fn run_ssh_job_task<C: SshClient>(
    manifest_path: PathBuf,
    job_id: JobId,
    worker: WorkerConfig,
    state: Arc<Mutex<ServerState>>,
    ssh_client: C,
    timeout_secs: u64,
    max_attempts: u32,
) {
    // --- transition to Running ---
    let attempt = {
        let mut s = state.lock().unwrap();
        if let Some(job) = s.jobs.iter_mut().find(|j| j.id == job_id) {
            job.status = crate::serve::types::JobStatus::Running;
            job.started_at = Some(crate::serve::types::now_epoch());
            job.attempt
        } else {
            warn!(job_id = %job_id, "run_ssh_job_task: job not found in state");
            return;
        }
    };

    info!(
        job_id = %job_id,
        worker_host = %worker.host,
        manifest = %manifest_path.display(),
        attempt,
        "dispatching job to worker"
    );

    // Step 1: deliver manifest
    let remote_manifest = match deliver_manifest(
        &ssh_client,
        &worker,
        timeout_secs,
        &job_id,
        &manifest_path,
    )
    .await
    {
        Ok(path) => path,
        Err(e) => {
            warn!(job_id = %job_id, worker_host = %worker.host, error = %e, "deliver_manifest failed");
            let mut s = state.lock().unwrap();
            s.complete(&job_id, false, attempt, max_attempts);
            return;
        }
    };

    // Step 2: run remote job
    let exit_code = match run_remote_job(&ssh_client, &worker, timeout_secs, &remote_manifest).await
    {
        Ok(code) => code,
        Err(e) => {
            warn!(job_id = %job_id, worker_host = %worker.host, error = %e, "run_remote_job failed");
            let mut s = state.lock().unwrap();
            s.complete(&job_id, false, attempt, max_attempts);
            return;
        }
    };

    // Step 3: sync state back — parse job_name from manifest
    let job_name = match std::fs::read_to_string(&manifest_path) {
        Ok(content) => {
            match smelt_core::manifest::JobManifest::from_str(&content, &manifest_path) {
                Ok(m) => m.job.name,
                Err(e) => {
                    warn!(
                        job_id = %job_id,
                        error = %e,
                        "failed to parse manifest for job_name; skipping state sync"
                    );
                    String::new()
                }
            }
        }
        Err(e) => {
            warn!(
                job_id = %job_id,
                error = %e,
                "failed to read manifest for job_name; skipping state sync"
            );
            String::new()
        }
    };

    if !job_name.is_empty() {
        // Use the manifest's parent directory as the local dest for state sync
        let local_dest = manifest_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        if let Err(e) =
            sync_state_back(&ssh_client, &worker, timeout_secs, &job_name, local_dest).await
        {
            warn!(
                job_id = %job_id,
                worker_host = %worker.host,
                error = %e,
                "sync_state_back failed (non-fatal)"
            );
        }
    }

    let success = exit_code == 0;
    if !success {
        warn!(
            job_id = %job_id,
            worker_host = %worker.host,
            exit_code,
            "remote job exited with non-zero code"
        );
    }

    {
        let mut s = state.lock().unwrap();
        s.complete(&job_id, success, attempt, max_attempts);
    }

    if success {
        info!(job_id = %job_id, worker_host = %worker.host, "remote job complete");
    } else {
        let retrying = {
            let s = state.lock().unwrap();
            s.jobs
                .iter()
                .any(|j| j.id == job_id && j.status == JobStatus::Retrying)
        };
        if retrying {
            info!(job_id = %job_id, worker_host = %worker.host, attempt, "remote job queued for retry");
        } else {
            info!(job_id = %job_id, worker_host = %worker.host, attempt, "remote job failed permanently");
        }
    }
}

/// Select the next online worker using round-robin with probe-based skip.
///
/// Starting at `round_robin_idx % workers.len()`, probes up to `workers.len()`
/// workers in order. Returns the first that responds successfully along with
/// the updated index (next position after the selected worker).
///
/// Returns `None` if all workers are offline.
pub(crate) async fn select_worker<C: SshClient>(
    workers: &[WorkerConfig],
    ssh_client: &C,
    timeout_secs: u64,
    round_robin_idx: usize,
) -> Option<(WorkerConfig, usize)> {
    if workers.is_empty() {
        return None;
    }
    let len = workers.len();
    let start = round_robin_idx % len;
    for i in 0..len {
        let idx = (start + i) % len;
        let worker = &workers[idx];
        match ssh_client.probe(worker, timeout_secs).await {
            Ok(()) => {
                let new_idx = (idx + 1) % len;
                return Some((worker.clone(), new_idx));
            }
            Err(e) => {
                warn!(
                    host = %worker.host,
                    error = %e,
                    "probe failed for worker"
                );
            }
        }
    }
    None
}

/// Poll `ServerState::try_dispatch()` every 2 seconds and spawn `run_job_task`
/// (local) or `run_ssh_job_task` (remote) for each dispatchable job. Stops
/// cleanly when `cancel_token.cancelled()` fires.
///
/// When `workers` is non-empty, jobs are dispatched to SSH workers via
/// round-robin selection with probe-based offline skip. If all workers are
/// offline, the job is re-queued.
///
/// When `workers` is empty, the existing local `run_job_task` path is used.
///
/// Every spawned job task receives a *child* token so `cancel_token.cancel()`
/// broadcasts to all in-flight jobs.
pub(crate) async fn dispatch_loop<C: SshClient + Clone + Send + Sync + 'static>(
    state: Arc<Mutex<ServerState>>,
    cancel_token: CancellationToken,
    max_attempts: u32,
    workers: Vec<WorkerConfig>,
    ssh_client: C,
    ssh_timeout_secs: u64,
    event_env: Option<EventEnvConfig>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    // `MissedTickBehavior::Skip` keeps the loop from catching up after a long
    // tick (e.g. slow lock acquisition).
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!(workers = workers.len(), "dispatch_loop started");

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Drain all immediately dispatchable jobs each tick.
                loop {
                    let maybe_job = {
                        let mut s = state.lock().unwrap();
                        s.try_dispatch()
                    };
                    match maybe_job {
                        None => break,
                        Some(job) => {
                            let id = job.id.clone();

                            if workers.is_empty() {
                                // --- Local dispatch path ---
                                let state2 = Arc::clone(&state);
                                let child_token = cancel_token.child_token();
                                let env = event_env.clone();
                                info!(job_id = %id, "dispatching job locally");
                                tokio::spawn(run_job_task(
                                    job.manifest_path,
                                    job.id,
                                    state2,
                                    child_token,
                                    max_attempts,
                                    env,
                                ));
                            } else {
                                // --- SSH dispatch path ---
                                let rr_idx = {
                                    let s = state.lock().unwrap();
                                    s.round_robin_idx
                                };
                                let selected = select_worker(
                                    &workers,
                                    &ssh_client,
                                    ssh_timeout_secs,
                                    rr_idx,
                                )
                                .await;

                                match selected {
                                    Some((worker, new_idx)) => {
                                        // Update round_robin_idx and set worker_host
                                        {
                                            let mut s = state.lock().unwrap();
                                            s.round_robin_idx = new_idx;
                                            if let Some(j) = s.jobs.iter_mut().find(|j| j.id == id) {
                                                j.worker_host = Some(worker.host.clone());
                                            }
                                        }
                                        let state2 = Arc::clone(&state);
                                        let client_clone = ssh_client.clone();
                                        info!(
                                            job_id = %id,
                                            worker_host = %worker.host,
                                            "dispatching job to SSH worker"
                                        );
                                        tokio::spawn(run_ssh_job_task(
                                            job.manifest_path,
                                            job.id,
                                            worker,
                                            state2,
                                            client_clone,
                                            ssh_timeout_secs,
                                            max_attempts,
                                        ));
                                    }
                                    None => {
                                        // All workers offline — re-queue the job
                                        info!(
                                            job_id = %id,
                                            "all workers offline — re-queueing job"
                                        );
                                        let mut s = state.lock().unwrap();
                                        if let Some(j) = s.jobs.iter_mut().find(|j| j.id == id) {
                                            j.status = JobStatus::Queued;
                                            j.started_at = None;
                                            j.worker_host = None;
                                        }
                                        s.running_count = s.running_count.saturating_sub(1);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                info!("dispatch_loop received cancellation — stopping");
                break;
            }
        }
    }

    info!("dispatch_loop stopped");
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::serve::config::WorkerConfig;
    use crate::serve::ssh::tests::MockSshClient;
    use crate::serve::types::{JobSource, JobStatus};

    fn test_worker(host: &str) -> WorkerConfig {
        WorkerConfig {
            host: host.to_string(),
            user: "testuser".to_string(),
            key_env: "SMELT_SSH_KEY_NONEXISTENT_XYZ".to_string(),
            port: 22,
        }
    }

    // -----------------------------------------------------------------------
    // select_worker tests
    // -----------------------------------------------------------------------

    /// All workers online — round-robin cycles through them in order.
    #[tokio::test]
    async fn test_select_worker_all_online_round_robin() {
        let workers = vec![
            test_worker("worker-a"),
            test_worker("worker-b"),
            test_worker("worker-c"),
        ];

        // Mock: all probes succeed
        let client = MockSshClient::new()
            .with_probe_result(Ok(())) // worker-a
            .with_probe_result(Ok(())) // worker-b
            .with_probe_result(Ok(())); // worker-c

        // Start at idx 0 → selects worker-a, returns new idx 1
        let result = select_worker(&workers, &client, 3, 0).await;
        assert!(result.is_some());
        let (w, new_idx) = result.unwrap();
        assert_eq!(w.host, "worker-a");
        assert_eq!(new_idx, 1);

        // Start at idx 1 → selects worker-b, returns new idx 2
        let result = select_worker(&workers, &client, 3, 1).await;
        assert!(result.is_some());
        let (w, new_idx) = result.unwrap();
        assert_eq!(w.host, "worker-b");
        assert_eq!(new_idx, 2);

        // Start at idx 2 → selects worker-c, returns new idx 0 (wrap)
        let result = select_worker(&workers, &client, 3, 2).await;
        assert!(result.is_some());
        let (w, new_idx) = result.unwrap();
        assert_eq!(w.host, "worker-c");
        assert_eq!(new_idx, 0);
    }

    /// One worker offline — skip it, select the next online one.
    #[tokio::test]
    async fn test_select_worker_one_offline_skip() {
        let workers = vec![test_worker("worker-a"), test_worker("worker-b")];

        // worker-a probe fails, worker-b probe succeeds
        let client = MockSshClient::new()
            .with_probe_result(Err(anyhow::anyhow!("connection refused")))
            .with_probe_result(Ok(()));

        let result = select_worker(&workers, &client, 3, 0).await;
        assert!(result.is_some());
        let (w, new_idx) = result.unwrap();
        assert_eq!(w.host, "worker-b", "should skip offline worker-a");
        assert_eq!(new_idx, 0, "new idx should wrap to 0 after selecting idx 1");
    }

    /// All workers offline — returns None.
    #[tokio::test]
    async fn test_select_worker_all_offline() {
        let workers = vec![test_worker("worker-a"), test_worker("worker-b")];

        let client = MockSshClient::new()
            .with_probe_result(Err(anyhow::anyhow!("offline")))
            .with_probe_result(Err(anyhow::anyhow!("offline")));

        let result = select_worker(&workers, &client, 3, 0).await;
        assert!(
            result.is_none(),
            "should return None when all workers offline"
        );
    }

    // -----------------------------------------------------------------------
    // Re-queue path test
    // -----------------------------------------------------------------------

    /// When all workers are offline, the re-queue logic should revert the job
    /// to Queued status and decrement running_count.
    ///
    /// This test exercises the re-queue logic directly (without running the
    /// full dispatch_loop) to avoid needing an unbounded number of mock probe
    /// results for repeated tick cycles.
    #[tokio::test]
    async fn test_requeue_all_workers_offline() {
        let state = Arc::new(Mutex::new(ServerState::new_without_events(2)));
        let manifest = std::path::PathBuf::from("/tmp/fake.toml");
        {
            let mut s = state.lock().unwrap();
            s.enqueue(manifest, JobSource::HttpApi);
        }

        let workers = vec![test_worker("worker-a"), test_worker("worker-b")];

        // Both workers fail probe
        let client = MockSshClient::new()
            .with_probe_result(Err(anyhow::anyhow!("offline")))
            .with_probe_result(Err(anyhow::anyhow!("offline")));

        // Simulate what dispatch_loop does: try_dispatch → select_worker → re-queue
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

        // Re-queue path
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
        assert_eq!(
            s.running_count, 0,
            "running_count should be 0 after re-queue"
        );
        assert!(
            s.jobs[0].worker_host.is_none(),
            "worker_host should be None after re-queue"
        );
    }
}
