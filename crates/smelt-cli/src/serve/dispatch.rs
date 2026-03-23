//! Dispatch engine: `run_job_task` and `dispatch_loop`.
//!
//! `run_job_task` bridges `run_with_cancellation()`'s generic cancel Future to
//! `CancellationToken::cancelled()` via the one-liner adapter:
//!   `async { token.cancelled().await; Ok(()) }`
//!
//! `dispatch_loop` polls `ServerState::try_dispatch()` on a 2-second interval and
//! spawns a `run_job_task` tokio task for each dispatchable job. The loop breaks
//! cleanly when `cancel_token.cancelled()` fires, and every spawned child task
//! receives a child token so the broadcast propagates to all running jobs.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::commands::run::RunArgs;
use crate::serve::queue::ServerState;
use crate::serve::types::JobId;

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
) {
    // --- transition to Running ---
    let attempt = {
        let mut s = state.lock().unwrap();
        if let Some(job) = s.jobs.iter_mut().find(|j| j.id == job_id) {
            job.status = crate::serve::types::JobStatus::Running;
            job.started_at = Some(Instant::now());
            job.attempt
        } else {
            warn!(job_id = %job_id, "run_job_task: job not found in state");
            return;
        }
    };

    info!(job_id = %job_id, manifest = %manifest_path.display(), attempt, "job started");

    let args = RunArgs {
        manifest: manifest_path.clone(),
        dry_run: false,
        no_pr: false,
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

/// Poll `ServerState::try_dispatch()` every 2 seconds and spawn `run_job_task`
/// for each dispatchable job. Stops cleanly when `cancel_token.cancelled()` fires.
///
/// Every spawned job task receives a *child* token so `cancel_token.cancel()`
/// broadcasts to all in-flight jobs.
pub(crate) async fn dispatch_loop(
    state: Arc<Mutex<ServerState>>,
    cancel_token: CancellationToken,
    max_attempts: u32,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    // `MissedTickBehavior::Skip` keeps the loop from catching up after a long
    // tick (e.g. slow lock acquisition).
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!("dispatch_loop started");

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
                            let state2 = Arc::clone(&state);
                            let child_token = cancel_token.child_token();
                            let id = job.id.clone();
                            info!(job_id = %id, "dispatching job");
                            tokio::spawn(run_job_task(
                                job.manifest_path,
                                job.id,
                                state2,
                                child_token,
                                max_attempts,
                            ));
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
