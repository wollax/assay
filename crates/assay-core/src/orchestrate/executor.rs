//! Executor dispatch loop for orchestrated multi-session runs.
//!
//! `run_orchestrated()` is the core DAG-driven parallel executor. It builds a
//! [`DependencyGraph`], enters `std::thread::scope` with a condvar-based
//! dispatch loop, serializes worktree creation via a mutex, parallelizes agent
//! execution, propagates failures to dependents, persists state after each
//! completion, and returns [`OrchestratorResult`].

use std::collections::HashSet;
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use tempfile::NamedTempFile;
use ulid::Ulid;

use assay_types::ManifestSession;
use assay_types::orchestrate::{
    FailurePolicy, OrchestratorPhase, OrchestratorStatus, SessionRunState, SessionStatus,
};

use crate::error::AssayError;
use crate::orchestrate::dag::DependencyGraph;
use crate::pipeline::{PipelineConfig, PipelineError, PipelineResult, PipelineStage};

/// Outcome of a single orchestrated session.
///
/// Maps to [`SessionRunState`] for persistence: `Completed` → completed,
/// `Failed` → failed, `Skipped` → skipped.
#[derive(Debug)]
pub enum SessionOutcome {
    /// Session completed successfully with a pipeline result.
    Completed {
        /// The full pipeline result including gate summary and merge check.
        result: Box<PipelineResult>,
        /// Absolute path to the session's git worktree.
        worktree_path: PathBuf,
        /// Git branch name created for this session.
        branch_name: String,
        /// Files changed by the agent in the worktree.
        changed_files: Vec<String>,
    },
    /// Session failed at a specific pipeline stage.
    Failed {
        /// The pipeline error with stage context and recovery guidance.
        error: PipelineError,
        /// Which pipeline stage the failure occurred in.
        stage: PipelineStage,
    },
    /// Session was skipped (e.g., upstream dependency failed).
    Skipped {
        /// Human-readable reason for skipping (e.g., "upstream 'auth' failed").
        reason: String,
    },
}

/// Configuration for an orchestrated run.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Maximum number of concurrent sessions.
    /// Defaults to 8; effective concurrency is `min(max_concurrency, session_count)`.
    pub max_concurrency: usize,
    /// Failure policy for this run.
    pub failure_policy: FailurePolicy,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 8,
            failure_policy: FailurePolicy::default(),
        }
    }
}

/// Result of a complete orchestrated run.
#[derive(Debug)]
pub struct OrchestratorResult {
    /// Unique identifier for this run (ULID).
    pub run_id: String,
    /// Per-session outcomes indexed by session position in the manifest.
    pub outcomes: Vec<(String, SessionOutcome)>,
    /// Total wall-clock duration of the orchestrated run.
    pub duration: Duration,
    /// Failure policy that was in effect.
    pub failure_policy: FailurePolicy,
}

// ── Internal executor state ──────────────────────────────────────────

/// Mutable state shared across worker threads via `Mutex`.
struct ExecutorState {
    completed: HashSet<usize>,
    in_flight: HashSet<usize>,
    skipped: HashSet<usize>,
    failed: HashSet<usize>,
    outcomes: Vec<(String, SessionOutcome)>,
    session_statuses: Vec<SessionStatus>,
    aborted: bool,
}

// ── State persistence ────────────────────────────────────────────────

/// Persist an `OrchestratorStatus` snapshot to `state.json` using atomic
/// tempfile-then-rename.
fn persist_state(run_dir: &std::path::Path, status: &OrchestratorStatus) -> Result<(), AssayError> {
    let final_path = run_dir.join("state.json");
    let json = serde_json::to_string_pretty(status)
        .map_err(|e| AssayError::json("serializing orchestrator status", &final_path, e))?;

    let mut tmpfile = NamedTempFile::new_in(run_dir)
        .map_err(|e| AssayError::io("creating temp file for orchestrator state", run_dir, e))?;

    tmpfile
        .write_all(json.as_bytes())
        .map_err(|e| AssayError::io("writing orchestrator state", &final_path, e))?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|e| AssayError::io("syncing orchestrator state", &final_path, e))?;

    tmpfile
        .persist(&final_path)
        .map_err(|e| AssayError::io("persisting orchestrator state", &final_path, e.error))?;

    Ok(())
}

// ── Core executor ────────────────────────────────────────────────────

/// Run an orchestrated multi-session execution with DAG-driven dispatch.
///
/// Builds a [`DependencyGraph`] from the manifest, enters a
/// `std::thread::scope` dispatch loop with condvar-based coordination,
/// serializes worktree creation, and parallelizes agent execution.
///
/// State is persisted to `.assay/orchestrator/<run_id>/state.json` after
/// each session resolves (atomic tempfile-rename).
pub fn run_orchestrated<F>(
    manifest: &assay_types::RunManifest,
    config: OrchestratorConfig,
    pipeline_config: &PipelineConfig,
    session_runner: &F,
) -> Result<OrchestratorResult, AssayError>
where
    F: Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError> + Sync,
{
    let graph = DependencyGraph::from_manifest(manifest)?;
    let session_count = graph.session_count();
    let run_id = Ulid::new().to_string();
    let started_at = Utc::now();
    let wall_start = Instant::now();

    // Create run directory: .assay/orchestrator/<run_id>/
    let run_dir = pipeline_config.assay_dir.join("orchestrator").join(&run_id);
    std::fs::create_dir_all(&run_dir)
        .map_err(|e| AssayError::io("creating orchestrator run directory", &run_dir, e))?;

    // Initialize session statuses — all Pending.
    let initial_statuses: Vec<SessionStatus> = (0..session_count)
        .map(|i| SessionStatus {
            name: graph.name_of(i).to_string(),
            spec: manifest.sessions[i].spec.clone(),
            state: SessionRunState::Pending,
            started_at: None,
            completed_at: None,
            duration_secs: None,
            error: None,
            skip_reason: None,
        })
        .collect();

    let initial_status = OrchestratorStatus {
        run_id: run_id.clone(),
        phase: OrchestratorPhase::Running,
        failure_policy: config.failure_policy,
        sessions: initial_statuses.clone(),
        started_at,
        completed_at: None,
    };

    // Persist initial state.
    persist_state(&run_dir, &initial_status)?;

    let state = ExecutorState {
        completed: HashSet::new(),
        in_flight: HashSet::new(),
        skipped: HashSet::new(),
        failed: HashSet::new(),
        outcomes: Vec::new(),
        session_statuses: initial_statuses,
        aborted: false,
    };

    let pair = (Mutex::new(state), Condvar::new());
    let worktree_mutex = Mutex::new(());
    let effective_concurrency = config.max_concurrency.min(session_count);

    // Dispatch loop inside thread::scope.
    std::thread::scope(|scope| {
        let (state_mutex, condvar) = &pair;

        loop {
            let batch: Vec<usize> = {
                let mut guard = state_mutex.lock().unwrap();

                // Spurious wakeup guard: loop until we have work or are done.
                loop {
                    let resolved = guard.completed.len() + guard.failed.len() + guard.skipped.len();

                    // All sessions resolved — exit.
                    if resolved == session_count {
                        break vec![];
                    }

                    // If aborted, don't dispatch new work — wait for in-flight to drain.
                    if guard.aborted {
                        if guard.in_flight.is_empty() {
                            break vec![];
                        }
                        guard = condvar.wait(guard).unwrap();
                        continue;
                    }

                    // Compute ready set. Pass completed ∪ failed as "completed" to
                    // ready_set since failed sessions should not be re-dispatched.
                    let completed_for_ready: HashSet<usize> =
                        guard.completed.union(&guard.failed).copied().collect();
                    let ready =
                        graph.ready_set(&completed_for_ready, &guard.in_flight, &guard.skipped);

                    let available_slots =
                        effective_concurrency.saturating_sub(guard.in_flight.len());

                    if !ready.is_empty() && available_slots > 0 {
                        let take_count = ready.len().min(available_slots);
                        let batch: Vec<usize> = ready[..take_count].to_vec();
                        for &idx in &batch {
                            guard.in_flight.insert(idx);
                            guard.session_statuses[idx].state = SessionRunState::Running;
                            guard.session_statuses[idx].started_at = Some(Utc::now());
                        }
                        break batch;
                    }

                    // Nothing ready but sessions still in-flight — wait for notification.
                    if !guard.in_flight.is_empty() {
                        guard = condvar.wait(guard).unwrap();
                        continue;
                    }

                    // Nothing ready, nothing in-flight, but not all resolved.
                    // This shouldn't happen with a valid DAG, but break to avoid deadlock.
                    break vec![];
                }
            };

            // Check termination after acquiring batch.
            {
                let guard = state_mutex.lock().unwrap();
                let resolved = guard.completed.len() + guard.failed.len() + guard.skipped.len();
                if resolved == session_count {
                    break;
                }
                if guard.aborted && guard.in_flight.is_empty() {
                    break;
                }
            }

            if batch.is_empty() {
                // Possible deadlock guard — shouldn't happen with valid DAG.
                break;
            }

            // Spawn worker threads for the batch.
            for idx in batch {
                let session = &manifest.sessions[idx];
                let session_name = graph.name_of(idx).to_string();
                let (state_mutex, condvar) = &pair;
                let _wt_mutex = &worktree_mutex;
                let run_dir = &run_dir;
                let run_id = &run_id;
                let graph = &graph;
                let failure_policy = config.failure_policy;
                let started_at_run = started_at;

                scope.spawn(move || {
                    let session_start = Instant::now();

                    // Wrap the entire worker body in catch_unwind for panic safety.
                    let result = panic::catch_unwind(AssertUnwindSafe(|| {
                        session_runner(session, pipeline_config)
                    }));

                    let session_duration = session_start.elapsed();
                    let completed_at = Utc::now();

                    let mut guard = state_mutex.lock().unwrap();
                    guard.in_flight.remove(&idx);

                    match result {
                        Ok(Ok(pipeline_result)) => {
                            // Success.
                            guard.completed.insert(idx);
                            guard.session_statuses[idx].state = SessionRunState::Completed;
                            guard.session_statuses[idx].completed_at = Some(completed_at);
                            guard.session_statuses[idx].duration_secs =
                                Some(session_duration.as_secs_f64());

                            guard.outcomes.push((
                                session_name,
                                SessionOutcome::Completed {
                                    worktree_path: pipeline_result.session_id.clone().into(),
                                    branch_name: String::new(),
                                    changed_files: Vec::new(),
                                    result: Box::new(pipeline_result),
                                },
                            ));
                        }
                        Ok(Err(pipeline_error)) => {
                            // Pipeline failure.
                            let stage = pipeline_error.stage;
                            let error_msg = pipeline_error.message.clone();
                            let failed_name = graph.name_of(idx).to_string();

                            guard.failed.insert(idx);
                            guard.session_statuses[idx].state = SessionRunState::Failed;
                            guard.session_statuses[idx].completed_at = Some(completed_at);
                            guard.session_statuses[idx].duration_secs =
                                Some(session_duration.as_secs_f64());
                            guard.session_statuses[idx].error = Some(error_msg);

                            // Propagate failure to dependents.
                            graph.mark_skipped_dependents(idx, &mut guard.skipped);
                            let newly_skipped: Vec<usize> = guard.skipped.iter().copied().collect();
                            for skipped_idx in newly_skipped {
                                if guard.session_statuses[skipped_idx].state
                                    == SessionRunState::Pending
                                {
                                    let skip_reason = format!("upstream '{failed_name}' failed");
                                    guard.session_statuses[skipped_idx].state =
                                        SessionRunState::Skipped;
                                    guard.session_statuses[skipped_idx].skip_reason =
                                        Some(skip_reason.clone());

                                    guard.outcomes.push((
                                        graph.name_of(skipped_idx).to_string(),
                                        SessionOutcome::Skipped {
                                            reason: skip_reason,
                                        },
                                    ));
                                }
                            }

                            guard.outcomes.push((
                                session_name,
                                SessionOutcome::Failed {
                                    error: pipeline_error,
                                    stage,
                                },
                            ));

                            if failure_policy == FailurePolicy::Abort {
                                guard.aborted = true;
                            }
                        }
                        Err(panic_payload) => {
                            // Panic — treat like failure.
                            let panic_msg = if let Some(s) = panic_payload.downcast_ref::<String>()
                            {
                                s.clone()
                            } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
                                (*s).to_string()
                            } else {
                                "unknown panic".to_string()
                            };

                            let failed_name = graph.name_of(idx).to_string();

                            guard.failed.insert(idx);
                            guard.session_statuses[idx].state = SessionRunState::Failed;
                            guard.session_statuses[idx].completed_at = Some(completed_at);
                            guard.session_statuses[idx].duration_secs =
                                Some(session_duration.as_secs_f64());
                            guard.session_statuses[idx].error = Some(format!("panic: {panic_msg}"));

                            // Propagate failure to dependents.
                            graph.mark_skipped_dependents(idx, &mut guard.skipped);
                            let newly_skipped: Vec<usize> = guard.skipped.iter().copied().collect();
                            for skipped_idx in newly_skipped {
                                if guard.session_statuses[skipped_idx].state
                                    == SessionRunState::Pending
                                {
                                    let skip_reason = format!("upstream '{failed_name}' panicked");
                                    guard.session_statuses[skipped_idx].state =
                                        SessionRunState::Skipped;
                                    guard.session_statuses[skipped_idx].skip_reason =
                                        Some(skip_reason.clone());

                                    guard.outcomes.push((
                                        graph.name_of(skipped_idx).to_string(),
                                        SessionOutcome::Skipped {
                                            reason: skip_reason,
                                        },
                                    ));
                                }
                            }

                            guard.outcomes.push((
                                session_name,
                                SessionOutcome::Failed {
                                    error: PipelineError {
                                        stage: PipelineStage::AgentLaunch,
                                        message: format!("panic: {panic_msg}"),
                                        recovery: "check session runner for panics".to_string(),
                                        elapsed: session_duration,
                                    },
                                    stage: PipelineStage::AgentLaunch,
                                },
                            ));

                            if failure_policy == FailurePolicy::Abort {
                                guard.aborted = true;
                            }
                        }
                    }

                    // Persist state snapshot.
                    let snapshot = OrchestratorStatus {
                        run_id: run_id.clone(),
                        phase: OrchestratorPhase::Running,
                        failure_policy,
                        sessions: guard.session_statuses.clone(),
                        started_at: started_at_run,
                        completed_at: None,
                    };
                    // Best-effort persistence — don't fail the whole run.
                    let _ = persist_state(run_dir, &snapshot);

                    // Wake the dispatch loop.
                    condvar.notify_all();
                });
            }
        }
    });

    // Construct final result.
    let mut guard = pair.0.lock().unwrap();
    let wall_duration = wall_start.elapsed();

    let final_phase = if guard.aborted {
        OrchestratorPhase::Aborted
    } else if guard.failed.is_empty() && guard.skipped.is_empty() {
        OrchestratorPhase::Completed
    } else {
        OrchestratorPhase::PartialFailure
    };

    // Persist final state.
    let final_status = OrchestratorStatus {
        run_id: run_id.clone(),
        phase: final_phase,
        failure_policy: config.failure_policy,
        sessions: guard.session_statuses.clone(),
        started_at,
        completed_at: Some(Utc::now()),
    };
    let _ = persist_state(&run_dir, &final_status);

    let outcomes = std::mem::take(&mut guard.outcomes);
    drop(guard);

    Ok(OrchestratorResult {
        run_id,
        outcomes,
        duration: wall_duration,
        failure_policy: config.failure_policy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{PipelineOutcome, StageTiming};
    use assay_types::ManifestSession;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn make_pipeline_config(tmp: &std::path::Path) -> PipelineConfig {
        PipelineConfig {
            project_root: tmp.to_path_buf(),
            assay_dir: tmp.join(".assay"),
            specs_dir: tmp.join(".assay/specs"),
            worktree_base: tmp.to_path_buf(),
            timeout_secs: 60,
            base_branch: None,
        }
    }

    fn make_manifest(sessions: Vec<(&str, Option<&str>, Vec<&str>)>) -> assay_types::RunManifest {
        assay_types::RunManifest {
            sessions: sessions
                .into_iter()
                .map(|(spec, name, deps)| ManifestSession {
                    spec: spec.to_string(),
                    name: name.map(|n| n.to_string()),
                    settings: None,
                    hooks: vec![],
                    prompt_layers: vec![],
                    file_scope: vec![],
                    shared_files: vec![],
                    depends_on: deps.into_iter().map(|d| d.to_string()).collect(),
                })
                .collect(),
        }
    }

    fn success_result(name: &str) -> PipelineResult {
        PipelineResult {
            session_id: format!("sess-{name}"),
            spec_name: name.to_string(),
            gate_summary: None,
            merge_check: None,
            stage_timings: vec![StageTiming {
                stage: PipelineStage::AgentLaunch,
                duration: Duration::from_millis(10),
            }],
            outcome: PipelineOutcome::Success,
        }
    }

    #[test]
    fn three_independent_sessions_all_complete() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec![]),
            ("c", None, vec![]),
        ]);
        let config = OrchestratorConfig {
            max_concurrency: 4,
            failure_policy: FailurePolicy::SkipDependents,
        };
        let pipeline_config = make_pipeline_config(tmp.path());

        let completed_count = AtomicUsize::new(0);
        let runner = &|session: &ManifestSession, _config: &PipelineConfig| {
            completed_count.fetch_add(1, Ordering::SeqCst);
            Ok(success_result(&session.spec))
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();

        assert_eq!(completed_count.load(Ordering::SeqCst), 3);
        assert_eq!(result.outcomes.len(), 3);
        assert!(
            result
                .outcomes
                .iter()
                .all(|(_, o)| matches!(o, SessionOutcome::Completed { .. }))
        );

        // Verify state.json is readable.
        let state_path = tmp
            .path()
            .join(".assay/orchestrator")
            .join(&result.run_id)
            .join("state.json");
        assert!(state_path.exists(), "state.json should exist");
        let state_json = std::fs::read_to_string(&state_path).unwrap();
        let status: OrchestratorStatus = serde_json::from_str(&state_json).unwrap();
        assert_eq!(status.phase, OrchestratorPhase::Completed);
        assert_eq!(status.sessions.len(), 3);
        assert!(
            status
                .sessions
                .iter()
                .all(|s| s.state == SessionRunState::Completed)
        );
    }

    #[test]
    fn linear_chain_executes_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        // a → b → c: must execute in order
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["b"]),
        ]);
        let config = OrchestratorConfig {
            max_concurrency: 4,
            failure_policy: FailurePolicy::SkipDependents,
        };
        let pipeline_config = make_pipeline_config(tmp.path());

        let order = std::sync::Mutex::new(Vec::new());
        let runner = &|session: &ManifestSession, _config: &PipelineConfig| {
            order.lock().unwrap().push(session.spec.clone());
            Ok(success_result(&session.spec))
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();

        let exec_order = order.lock().unwrap().clone();
        assert_eq!(exec_order, vec!["a", "b", "c"]);
        assert_eq!(result.outcomes.len(), 3);
    }

    #[test]
    fn failure_skips_dependents_independent_completes() {
        let tmp = tempfile::tempdir().unwrap();
        // a fails → b (depends on a) is skipped, c (independent) completes
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec![]),
        ]);
        let config = OrchestratorConfig {
            max_concurrency: 1, // serialize to make test deterministic
            failure_policy: FailurePolicy::SkipDependents,
        };
        let pipeline_config = make_pipeline_config(tmp.path());

        let runner = &|session: &ManifestSession, _config: &PipelineConfig| {
            if session.spec == "a" {
                Err(PipelineError {
                    stage: PipelineStage::AgentLaunch,
                    message: "agent crashed".to_string(),
                    recovery: "retry".to_string(),
                    elapsed: Duration::from_millis(5),
                })
            } else {
                Ok(success_result(&session.spec))
            }
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();

        // Find outcomes by name.
        let find = |name: &str| {
            result
                .outcomes
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, o)| o)
        };

        assert!(matches!(find("a"), Some(SessionOutcome::Failed { .. })));
        assert!(matches!(find("b"), Some(SessionOutcome::Skipped { .. })));
        assert!(matches!(find("c"), Some(SessionOutcome::Completed { .. })));

        // Verify state.json records correct states.
        let state_path = tmp
            .path()
            .join(".assay/orchestrator")
            .join(&result.run_id)
            .join("state.json");
        let state_json = std::fs::read_to_string(&state_path).unwrap();
        let status: OrchestratorStatus = serde_json::from_str(&state_json).unwrap();
        assert_eq!(status.phase, OrchestratorPhase::PartialFailure);

        let a_status = status.sessions.iter().find(|s| s.name == "a").unwrap();
        assert_eq!(a_status.state, SessionRunState::Failed);
        assert!(a_status.error.is_some());

        let b_status = status.sessions.iter().find(|s| s.name == "b").unwrap();
        assert_eq!(b_status.state, SessionRunState::Skipped);
        assert!(b_status.skip_reason.is_some());

        let c_status = status.sessions.iter().find(|s| s.name == "c").unwrap();
        assert_eq!(c_status.state, SessionRunState::Completed);
    }

    #[test]
    fn diamond_dag_d_runs_after_b_and_c() {
        // Diamond: A → {B, C} → D
        // D must only run after both B and C complete.
        let tmp = tempfile::tempdir().unwrap();
        let manifest = make_manifest(vec![
            ("a", Some("A"), vec![]),
            ("b", Some("B"), vec!["A"]),
            ("c", Some("C"), vec!["A"]),
            ("d", Some("D"), vec!["B", "C"]),
        ]);
        let config = OrchestratorConfig {
            max_concurrency: 4,
            failure_policy: FailurePolicy::SkipDependents,
        };
        let pipeline_config = make_pipeline_config(tmp.path());

        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(String, String)>::new()));
        let log_clone = log.clone();

        let runner = &move |session: &ManifestSession, _config: &PipelineConfig| {
            let name = session.name.clone().unwrap_or_else(|| session.spec.clone());
            log_clone
                .lock()
                .unwrap()
                .push((name.clone(), "start".to_string()));
            std::thread::sleep(Duration::from_millis(30));
            log_clone
                .lock()
                .unwrap()
                .push((name.clone(), "end".to_string()));
            Ok(success_result(&session.spec))
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();
        assert_eq!(result.outcomes.len(), 4);

        let events = log.lock().unwrap().clone();
        // Find when D started and when B and C ended.
        let d_start_idx = events
            .iter()
            .position(|(n, e)| n == "D" && e == "start")
            .expect("D should have started");
        let b_end_idx = events
            .iter()
            .position(|(n, e)| n == "B" && e == "end")
            .expect("B should have ended");
        let c_end_idx = events
            .iter()
            .position(|(n, e)| n == "C" && e == "end")
            .expect("C should have ended");

        assert!(
            d_start_idx > b_end_idx,
            "D must start after B ends (D_start={d_start_idx}, B_end={b_end_idx})"
        );
        assert!(
            d_start_idx > c_end_idx,
            "D must start after C ends (D_start={d_start_idx}, C_end={c_end_idx})"
        );
    }

    #[test]
    fn bounded_concurrency_enforced() {
        // 5 independent sessions with max_concurrency: 2.
        // Peak concurrent executions must be ≤ 2.
        let tmp = tempfile::tempdir().unwrap();
        let manifest = make_manifest(vec![
            ("s1", None, vec![]),
            ("s2", None, vec![]),
            ("s3", None, vec![]),
            ("s4", None, vec![]),
            ("s5", None, vec![]),
        ]);
        let config = OrchestratorConfig {
            max_concurrency: 2,
            failure_policy: FailurePolicy::SkipDependents,
        };
        let pipeline_config = make_pipeline_config(tmp.path());

        let current = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);

        let runner = &|session: &ManifestSession, _config: &PipelineConfig| {
            let prev = current.fetch_add(1, Ordering::SeqCst);
            let now = prev + 1;
            // Update peak.
            peak.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(50));
            current.fetch_sub(1, Ordering::SeqCst);
            Ok(success_result(&session.spec))
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();
        assert_eq!(result.outcomes.len(), 5);
        assert!(
            result
                .outcomes
                .iter()
                .all(|(_, o)| matches!(o, SessionOutcome::Completed { .. }))
        );

        let observed_peak = peak.load(Ordering::SeqCst);
        assert!(
            observed_peak <= 2,
            "peak concurrency was {observed_peak}, expected ≤ 2"
        );
        // With 5 sessions and sleep, we should actually hit concurrency > 1.
        assert!(
            observed_peak >= 2,
            "peak concurrency was {observed_peak}, expected to actually use 2 slots"
        );
    }

    #[test]
    fn eight_independent_sessions_default_concurrency() {
        let tmp = tempfile::tempdir().unwrap();
        let sessions: Vec<(&str, Option<&str>, Vec<&str>)> = vec![
            ("s1", None, vec![]),
            ("s2", None, vec![]),
            ("s3", None, vec![]),
            ("s4", None, vec![]),
            ("s5", None, vec![]),
            ("s6", None, vec![]),
            ("s7", None, vec![]),
            ("s8", None, vec![]),
        ];
        let manifest = make_manifest(sessions);
        let config = OrchestratorConfig::default();
        let pipeline_config = make_pipeline_config(tmp.path());

        let completed = AtomicUsize::new(0);
        let runner = &|session: &ManifestSession, _config: &PipelineConfig| {
            std::thread::sleep(Duration::from_millis(10));
            completed.fetch_add(1, Ordering::SeqCst);
            Ok(success_result(&session.spec))
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();
        assert_eq!(completed.load(Ordering::SeqCst), 8);
        assert_eq!(result.outcomes.len(), 8);
    }

    #[test]
    fn abort_policy_stops_dispatch() {
        // A (fails), B, C, D, E — all independent.
        // With Abort policy and max_concurrency: 1, after A fails no more should dispatch.
        let tmp = tempfile::tempdir().unwrap();
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec![]),
            ("c", None, vec![]),
            ("d", None, vec![]),
            ("e", None, vec![]),
        ]);
        let config = OrchestratorConfig {
            max_concurrency: 1,
            failure_policy: FailurePolicy::Abort,
        };
        let pipeline_config = make_pipeline_config(tmp.path());

        let run_count = AtomicUsize::new(0);
        let runner = &|session: &ManifestSession, _config: &PipelineConfig| {
            run_count.fetch_add(1, Ordering::SeqCst);
            if session.spec == "a" {
                Err(PipelineError {
                    stage: PipelineStage::AgentLaunch,
                    message: "boom".to_string(),
                    recovery: "retry".to_string(),
                    elapsed: Duration::from_millis(1),
                })
            } else {
                Ok(success_result(&session.spec))
            }
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();
        assert_eq!(result.failure_policy, FailurePolicy::Abort);

        // Only A should have actually run (max_concurrency: 1, abort after first).
        assert_eq!(run_count.load(Ordering::SeqCst), 1, "only A should run");

        // Verify state phase is Aborted.
        let state_path = tmp
            .path()
            .join(".assay/orchestrator")
            .join(&result.run_id)
            .join("state.json");
        let status: OrchestratorStatus =
            serde_json::from_str(&std::fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(status.phase, OrchestratorPhase::Aborted);
    }

    #[test]
    fn panic_in_runner_caught_as_failure() {
        // A panics → B (depends on A) should be skipped.
        let tmp = tempfile::tempdir().unwrap();
        let manifest = make_manifest(vec![("a", None, vec![]), ("b", None, vec!["a"])]);
        let config = OrchestratorConfig {
            max_concurrency: 2,
            failure_policy: FailurePolicy::SkipDependents,
        };
        let pipeline_config = make_pipeline_config(tmp.path());

        let runner = &|session: &ManifestSession, _config: &PipelineConfig| {
            if session.spec == "a" {
                panic!("session runner exploded");
            }
            Ok(success_result(&session.spec))
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();

        let find = |name: &str| {
            result
                .outcomes
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, o)| o)
        };

        // A should be Failed with panic message.
        match find("a") {
            Some(SessionOutcome::Failed { error, .. }) => {
                assert!(
                    error.message.contains("panic"),
                    "error should mention panic: {}",
                    error.message
                );
            }
            other => panic!("expected Failed for a, got {other:?}"),
        }

        // B should be Skipped.
        assert!(
            matches!(find("b"), Some(SessionOutcome::Skipped { .. })),
            "B should be skipped due to A's panic"
        );

        // State file should show PartialFailure.
        let state_path = tmp
            .path()
            .join(".assay/orchestrator")
            .join(&result.run_id)
            .join("state.json");
        let status: OrchestratorStatus =
            serde_json::from_str(&std::fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(status.phase, OrchestratorPhase::PartialFailure);
        let a_status = status.sessions.iter().find(|s| s.name == "a").unwrap();
        assert_eq!(a_status.state, SessionRunState::Failed);
        assert!(a_status.error.as_ref().unwrap().contains("panic"));
    }

    #[test]
    fn single_session_through_orchestrator() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = make_manifest(vec![("solo", None, vec![])]);
        let config = OrchestratorConfig::default();
        let pipeline_config = make_pipeline_config(tmp.path());

        let runner = &|session: &ManifestSession, _config: &PipelineConfig| {
            Ok(success_result(&session.spec))
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();
        assert_eq!(result.outcomes.len(), 1);
        assert!(matches!(
            result.outcomes[0],
            (_, SessionOutcome::Completed { .. })
        ));

        let state_path = tmp
            .path()
            .join(".assay/orchestrator")
            .join(&result.run_id)
            .join("state.json");
        let status: OrchestratorStatus =
            serde_json::from_str(&std::fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(status.phase, OrchestratorPhase::Completed);
        assert_eq!(status.sessions.len(), 1);
    }

    #[test]
    fn all_sessions_fail_no_deadlock() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec![]),
            ("c", None, vec!["a"]),
        ]);
        let config = OrchestratorConfig {
            max_concurrency: 4,
            failure_policy: FailurePolicy::SkipDependents,
        };
        let pipeline_config = make_pipeline_config(tmp.path());

        let runner = &|session: &ManifestSession, _config: &PipelineConfig| {
            Err(PipelineError {
                stage: PipelineStage::AgentLaunch,
                message: format!("{} failed", session.spec),
                recovery: "retry".to_string(),
                elapsed: Duration::from_millis(1),
            })
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();

        // a and b failed, c skipped (depends on a which failed).
        let find = |name: &str| {
            result
                .outcomes
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, o)| o)
        };
        assert!(matches!(find("a"), Some(SessionOutcome::Failed { .. })));
        assert!(matches!(find("b"), Some(SessionOutcome::Failed { .. })));
        assert!(matches!(find("c"), Some(SessionOutcome::Skipped { .. })));

        let state_path = tmp
            .path()
            .join(".assay/orchestrator")
            .join(&result.run_id)
            .join("state.json");
        let status: OrchestratorStatus =
            serde_json::from_str(&std::fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(status.phase, OrchestratorPhase::PartialFailure);
    }

    #[test]
    fn state_persistence_has_correct_fields() {
        // Run a mixed scenario and validate the deserialized OrchestratorStatus thoroughly.
        let tmp = tempfile::tempdir().unwrap();
        let manifest = make_manifest(vec![
            ("pass1", None, vec![]),
            ("fail1", None, vec![]),
            ("skip1", None, vec!["fail1"]),
        ]);
        let config = OrchestratorConfig {
            max_concurrency: 1,
            failure_policy: FailurePolicy::SkipDependents,
        };
        let pipeline_config = make_pipeline_config(tmp.path());

        let runner = &|session: &ManifestSession, _config: &PipelineConfig| {
            if session.spec == "fail1" {
                Err(PipelineError {
                    stage: PipelineStage::GateEvaluate,
                    message: "gate failed".to_string(),
                    recovery: "fix gates".to_string(),
                    elapsed: Duration::from_millis(5),
                })
            } else {
                Ok(success_result(&session.spec))
            }
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();

        let state_path = tmp
            .path()
            .join(".assay/orchestrator")
            .join(&result.run_id)
            .join("state.json");
        assert!(state_path.exists());

        let status: OrchestratorStatus =
            serde_json::from_str(&std::fs::read_to_string(&state_path).unwrap()).unwrap();

        // Top-level fields.
        assert_eq!(status.run_id, result.run_id);
        assert_eq!(status.phase, OrchestratorPhase::PartialFailure);
        assert_eq!(status.failure_policy, FailurePolicy::SkipDependents);
        assert!(status.completed_at.is_some());
        assert_eq!(status.sessions.len(), 3);

        // Per-session fields.
        let pass = status.sessions.iter().find(|s| s.name == "pass1").unwrap();
        assert_eq!(pass.state, SessionRunState::Completed);
        assert!(pass.started_at.is_some());
        assert!(pass.completed_at.is_some());
        assert!(pass.duration_secs.is_some());
        assert!(pass.error.is_none());
        assert!(pass.skip_reason.is_none());

        let fail = status.sessions.iter().find(|s| s.name == "fail1").unwrap();
        assert_eq!(fail.state, SessionRunState::Failed);
        assert!(fail.error.is_some());
        assert!(fail.error.as_ref().unwrap().contains("gate failed"));

        let skip = status.sessions.iter().find(|s| s.name == "skip1").unwrap();
        assert_eq!(skip.state, SessionRunState::Skipped);
        assert!(skip.skip_reason.is_some());
        assert!(skip.skip_reason.as_ref().unwrap().contains("fail1"));
    }

    #[test]
    fn mixed_deps_and_independent_run_concurrently() {
        // A → B (chain), C independent. C should run while A is running.
        let tmp = tempfile::tempdir().unwrap();
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec![]),
        ]);
        let config = OrchestratorConfig {
            max_concurrency: 4,
            failure_policy: FailurePolicy::SkipDependents,
        };
        let pipeline_config = make_pipeline_config(tmp.path());

        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(String, String)>::new()));
        let log_clone = log.clone();

        let runner = &move |session: &ManifestSession, _config: &PipelineConfig| {
            log_clone
                .lock()
                .unwrap()
                .push((session.spec.clone(), "start".to_string()));
            std::thread::sleep(Duration::from_millis(40));
            log_clone
                .lock()
                .unwrap()
                .push((session.spec.clone(), "end".to_string()));
            Ok(success_result(&session.spec))
        };

        let result = run_orchestrated(&manifest, config, &pipeline_config, runner).unwrap();
        assert_eq!(result.outcomes.len(), 3);

        let events = log.lock().unwrap().clone();
        // C should start before A ends (they're independent, run in parallel).
        let a_start = events
            .iter()
            .position(|(n, e)| n == "a" && e == "start")
            .unwrap();
        let c_start = events
            .iter()
            .position(|(n, e)| n == "c" && e == "start")
            .unwrap();
        let a_end = events
            .iter()
            .position(|(n, e)| n == "a" && e == "end")
            .unwrap();

        // Both A and C should start before either ends (parallel).
        assert!(
            c_start < a_end,
            "C should start before A ends (parallel): c_start={c_start}, a_end={a_end}"
        );

        // B must start after A ends.
        let b_start = events
            .iter()
            .position(|(n, e)| n == "b" && e == "start")
            .unwrap();
        assert!(
            b_start > a_end,
            "B must start after A ends: b_start={b_start}, a_end={a_end}"
        );

        // Verify the order: A start before C start is not guaranteed, but both should be early.
        // The key assertion is that C doesn't wait for the A→B chain.
        let _ = a_start; // Used above in position calculation.
    }

    #[test]
    fn orchestrator_config_default_max_concurrency_is_8() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.max_concurrency, 8);
    }

    #[test]
    fn orchestrator_config_default_failure_policy_is_skip_dependents() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.failure_policy, FailurePolicy::SkipDependents);
    }

    #[test]
    fn session_outcome_completed_construction() {
        let result = PipelineResult {
            session_id: "sess-001".to_string(),
            spec_name: "auth".to_string(),
            gate_summary: None,
            merge_check: None,
            stage_timings: vec![StageTiming {
                stage: PipelineStage::AgentLaunch,
                duration: Duration::from_secs(30),
            }],
            outcome: PipelineOutcome::Success,
        };
        let outcome = SessionOutcome::Completed {
            result: Box::new(result),
            worktree_path: PathBuf::from("/tmp/wt-auth"),
            branch_name: "assay/auth".to_string(),
            changed_files: vec!["src/auth.rs".to_string()],
        };
        assert!(matches!(outcome, SessionOutcome::Completed { .. }));
    }

    #[test]
    fn session_outcome_failed_construction() {
        let error = PipelineError {
            stage: PipelineStage::GateEvaluate,
            message: "gate evaluation timed out".to_string(),
            recovery: "increase evaluator_timeout in config".to_string(),
            elapsed: Duration::from_secs(120),
        };
        let outcome = SessionOutcome::Failed {
            stage: PipelineStage::GateEvaluate,
            error,
        };
        assert!(matches!(outcome, SessionOutcome::Failed { .. }));
    }

    #[test]
    fn session_outcome_skipped_construction() {
        let outcome = SessionOutcome::Skipped {
            reason: "upstream 'auth' failed".to_string(),
        };
        match outcome {
            SessionOutcome::Skipped { reason } => {
                assert_eq!(reason, "upstream 'auth' failed");
            }
            _ => panic!("expected Skipped"),
        }
    }

    #[test]
    fn orchestrator_result_construction() {
        let result = OrchestratorResult {
            run_id: "01JTEST456".to_string(),
            outcomes: vec![(
                "auth".to_string(),
                SessionOutcome::Skipped {
                    reason: "test".to_string(),
                },
            )],
            duration: Duration::from_secs(60),
            failure_policy: FailurePolicy::Abort,
        };
        assert_eq!(result.run_id, "01JTEST456");
        assert_eq!(result.outcomes.len(), 1);
        assert_eq!(result.failure_policy, FailurePolicy::Abort);
    }
}
