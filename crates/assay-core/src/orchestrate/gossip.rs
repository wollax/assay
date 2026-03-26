//! Gossip-mode executor for parallel multi-session runs with knowledge synthesis.
//!
//! `run_gossip()` launches all sessions in parallel (subject to
//! `max_concurrency`), injects a `PromptLayer` carrying the manifest path
//! into each session clone before launch, and runs an `mpsc`-based coordinator
//! thread that synthesizes session completions into an atomically-written
//! `knowledge.json`. `GossipStatus` is persisted to `state.json` after each
//! coordinator cycle.

use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tempfile::NamedTempFile;
use ulid::Ulid;

use assay_types::orchestrate::{
    GossipStatus, KnowledgeEntry, KnowledgeManifest, OrchestratorPhase, OrchestratorStatus,
    SessionRunState, SessionStatus,
};
use assay_types::{ManifestSession, PromptLayer, PromptLayerKind};

use tracing::{Span, info, info_span};

use crate::error::AssayError;
use crate::orchestrate::executor::{
    OrchestratorConfig, OrchestratorResult, SessionOutcome, persist_state,
};
use crate::pipeline::{PipelineConfig, PipelineError, PipelineResult, PipelineStage};

// ── Internal completion message ──────────────────────────────────────────────

/// Sent by a worker thread to the coordinator when a session completes.
struct GossipCompletion {
    session_name: String,
    spec: String,
    gate_pass_count: u32,
    gate_fail_count: u32,
    changed_files: Vec<String>,
    completed_at: DateTime<Utc>,
}

// ── Knowledge manifest persistence ──────────────────────────────────────────

/// Persist a `KnowledgeManifest` to `gossip_dir/knowledge.json` using atomic
/// tempfile-then-rename (same pattern as `persist_state()`).
fn persist_knowledge_manifest(
    gossip_dir: &Path,
    manifest: &KnowledgeManifest,
) -> Result<(), AssayError> {
    let final_path = gossip_dir.join("knowledge.json");
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| AssayError::json("serializing knowledge manifest", &final_path, e))?;

    let mut tmpfile = NamedTempFile::new_in(gossip_dir)
        .map_err(|e| AssayError::io("creating temp file for knowledge manifest", gossip_dir, e))?;

    tmpfile
        .write_all(json.as_bytes())
        .map_err(|e| AssayError::io("writing knowledge manifest", &final_path, e))?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|e| AssayError::io("syncing knowledge manifest", &final_path, e))?;

    tmpfile
        .persist(&final_path)
        .map_err(|e| AssayError::io("persisting knowledge manifest", &final_path, e.error))?;

    Ok(())
}

// ── Core executor ────────────────────────────────────────────────────────────

/// Run a gossip-mode multi-session execution.
///
/// All sessions are launched in parallel (subject to `max_concurrency`).
/// A `PromptLayer` named `"gossip-knowledge-manifest"` is injected into each
/// session clone so runners can discover the knowledge manifest path.
/// A coordinator thread receives `GossipCompletion` messages via `mpsc` and
/// synthesizes them into `gossip/knowledge.json`, updated atomically after
/// each session completes. `OrchestratorStatus` (including `gossip_status`)
/// is persisted to `state.json` after each coordinator cycle.
pub fn run_gossip<F>(
    manifest: &assay_types::RunManifest,
    config: &OrchestratorConfig,
    pipeline_config: &PipelineConfig,
    session_runner: &F,
) -> Result<OrchestratorResult, AssayError>
where
    F: Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError> + Sync,
{
    let session_count = manifest.sessions.len();

    let _root_span = info_span!(
        "orchestrate::gossip",
        session_count = session_count,
        mode = "gossip"
    )
    .entered();
    info!("starting Gossip orchestration");

    let run_id = Ulid::new().to_string();
    let started_at = Utc::now();
    let wall_start = Instant::now();

    // ── Directory setup ──────────────────────────────────────────────

    let run_dir = pipeline_config.assay_dir.join("orchestrator").join(&run_id);
    let gossip_dir = run_dir.join("gossip");
    std::fs::create_dir_all(&gossip_dir)
        .map_err(|e| AssayError::io("creating gossip run directory", &gossip_dir, e))?;

    let knowledge_manifest_path: PathBuf = run_dir.join("gossip").join("knowledge.json");

    // ── Warn for depends_on (ignored in gossip mode) ─────────────────

    for session in &manifest.sessions {
        let name = session.name.as_deref().unwrap_or(&session.spec);
        if !session.depends_on.is_empty() {
            tracing::warn!(
                session = %name,
                "depends_on is ignored in Gossip mode"
            );
        }
    }

    // ── Build cloned sessions with gossip PromptLayer ────────────────

    let mut cloned_sessions: Vec<(String, ManifestSession)> = Vec::with_capacity(session_count);

    for session in &manifest.sessions {
        let name = session.name.as_deref().unwrap_or(&session.spec).to_string();
        let mut session_clone = session.clone();

        let layer_content = format!(
            "# Gossip Mode — Knowledge Manifest\n\
             Knowledge manifest: {path}\n\
             Read this file at any point during your session to discover what other sessions have already completed.\n\
             The manifest is updated atomically as sessions finish.",
            path = knowledge_manifest_path.display()
        );

        session_clone.prompt_layers.push(PromptLayer {
            kind: PromptLayerKind::System,
            name: "gossip-knowledge-manifest".to_string(),
            priority: -5,
            content: layer_content,
        });

        cloned_sessions.push((name, session_clone));
    }

    // ── Write initial empty knowledge manifest ───────────────────────

    let initial_manifest = KnowledgeManifest {
        run_id: run_id.clone(),
        entries: vec![],
        last_updated_at: Utc::now(),
    };
    persist_knowledge_manifest(&gossip_dir, &initial_manifest)?;

    // ── Initialize status ────────────────────────────────────────────

    let gossip_status = GossipStatus {
        sessions_synthesized: 0,
        knowledge_manifest_path: knowledge_manifest_path.clone(),
        coordinator_rounds: 0,
    };

    let initial_session_statuses: Vec<SessionStatus> = cloned_sessions
        .iter()
        .zip(manifest.sessions.iter())
        .map(|((name, _), session)| SessionStatus {
            name: name.clone(),
            spec: session.spec.clone(),
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
        sessions: initial_session_statuses.clone(),
        started_at,
        completed_at: None,
        mesh_status: None,
        gossip_status: Some(gossip_status.clone()),
    };
    persist_state(&run_dir, &initial_status)?;

    // ── Shared state ─────────────────────────────────────────────────

    let active_count = Arc::new(AtomicUsize::new(session_count));
    let gossip_status_arc = Arc::new(Mutex::new(gossip_status));
    let session_statuses_arc = Arc::new(Mutex::new(initial_session_statuses));

    // Bounded concurrency semaphore: (Mutex<usize>, Condvar) — slot counter.
    let effective_concurrency = config.max_concurrency.min(session_count);
    let semaphore = Arc::new((Mutex::new(0usize), Condvar::new()));

    // Coordinator interval from gossip_config or default (5s).
    let coordinator_interval = Duration::from_secs(
        manifest
            .gossip_config
            .as_ref()
            .map(|gc| gc.coordinator_interval_secs)
            .unwrap_or(assay_types::GossipConfig::default().coordinator_interval_secs),
    );

    // mpsc channel: workers → coordinator.
    let (tx, rx) = mpsc::channel::<GossipCompletion>();

    // ── thread::scope ────────────────────────────────────────────────

    // Capture current span for cross-thread parenting.
    let parent_span = Span::current();

    std::thread::scope(|scope| {
        // ── Coordinator thread ────────────────────────────────────────
        let gossip_status_arc_coord = Arc::clone(&gossip_status_arc);
        let session_statuses_arc_coord = Arc::clone(&session_statuses_arc);
        let run_dir_coord = &run_dir;
        let run_id_coord = &run_id;
        let gossip_dir_coord = &gossip_dir;
        let failure_policy = config.failure_policy;
        let started_at_run = started_at;
        let coord_parent = parent_span.clone();

        scope.spawn(move || {
            let _parent_guard = coord_parent.enter();
            let _coord_span = info_span!("orchestrate::gossip::coordinator").entered();

            let mut knowledge_entries: Vec<KnowledgeEntry> = Vec::new();

            loop {
                match rx.recv_timeout(coordinator_interval) {
                    Ok(completion) => {
                        // A session completed — synthesize it into the manifest.
                        knowledge_entries.push(KnowledgeEntry {
                            session_name: completion.session_name,
                            spec: completion.spec,
                            gate_pass_count: completion.gate_pass_count,
                            gate_fail_count: completion.gate_fail_count,
                            changed_files: completion.changed_files,
                            completed_at: completion.completed_at,
                        });

                        let updated_manifest = KnowledgeManifest {
                            run_id: run_id_coord.clone(),
                            entries: knowledge_entries.clone(),
                            last_updated_at: Utc::now(),
                        };
                        let _ = persist_knowledge_manifest(gossip_dir_coord, &updated_manifest);

                        // Update gossip status.
                        let (sessions_synthesized, coordinator_rounds) = {
                            let mut gs = gossip_status_arc_coord.lock().unwrap();
                            gs.sessions_synthesized += 1;
                            gs.coordinator_rounds += 1;
                            (gs.sessions_synthesized, gs.coordinator_rounds)
                        };

                        tracing::debug!(
                            sessions_synthesized,
                            coordinator_rounds,
                            "gossip coordinator cycle"
                        );

                        // Best-effort state persist.
                        let statuses = session_statuses_arc_coord.lock().unwrap().clone();
                        let gs_snap = gossip_status_arc_coord.lock().unwrap().clone();
                        let snapshot = OrchestratorStatus {
                            run_id: run_id_coord.clone(),
                            phase: OrchestratorPhase::Running,
                            failure_policy,
                            sessions: statuses,
                            started_at: started_at_run,
                            completed_at: None,
                            mesh_status: None,
                            gossip_status: Some(gs_snap),
                        };
                        let _ = persist_state(run_dir_coord, &snapshot);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Timeout: coordinator round with no completion.
                        let coordinator_rounds = {
                            let mut gs = gossip_status_arc_coord.lock().unwrap();
                            gs.coordinator_rounds += 1;
                            gs.coordinator_rounds
                        };
                        let sessions_synthesized =
                            gossip_status_arc_coord.lock().unwrap().sessions_synthesized;

                        tracing::debug!(
                            sessions_synthesized,
                            coordinator_rounds,
                            "gossip coordinator cycle"
                        );

                        let statuses = session_statuses_arc_coord.lock().unwrap().clone();
                        let gs_snap = gossip_status_arc_coord.lock().unwrap().clone();
                        let snapshot = OrchestratorStatus {
                            run_id: run_id_coord.clone(),
                            phase: OrchestratorPhase::Running,
                            failure_policy,
                            sessions: statuses,
                            started_at: started_at_run,
                            completed_at: None,
                            mesh_status: None,
                            gossip_status: Some(gs_snap),
                        };
                        let _ = persist_state(run_dir_coord, &snapshot);
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // All senders dropped — drain any remaining messages, then exit.
                        while let Ok(completion) = rx.try_recv() {
                            knowledge_entries.push(KnowledgeEntry {
                                session_name: completion.session_name,
                                spec: completion.spec,
                                gate_pass_count: completion.gate_pass_count,
                                gate_fail_count: completion.gate_fail_count,
                                changed_files: completion.changed_files,
                                completed_at: completion.completed_at,
                            });
                            let mut gs = gossip_status_arc_coord.lock().unwrap();
                            gs.sessions_synthesized += 1;
                            gs.coordinator_rounds += 1;
                        }

                        // Final manifest write.
                        let final_manifest = KnowledgeManifest {
                            run_id: run_id_coord.clone(),
                            entries: knowledge_entries.clone(),
                            last_updated_at: Utc::now(),
                        };
                        let _ = persist_knowledge_manifest(gossip_dir_coord, &final_manifest);

                        break;
                    }
                }
            }
        });

        // ── Worker threads ────────────────────────────────────────────
        for (name, session_clone) in &cloned_sessions {
            let name = name.clone();
            let active_count = Arc::clone(&active_count);
            let gossip_status_arc = Arc::clone(&gossip_status_arc);
            let session_statuses_arc = Arc::clone(&session_statuses_arc);
            let semaphore = Arc::clone(&semaphore);
            let tx_worker = tx.clone();
            let run_dir = &run_dir;
            let run_id = &run_id;
            let started_at_run = started_at;
            let failure_policy = config.failure_policy;
            let worker_parent = parent_span.clone();

            scope.spawn(move || {
                // Re-enter the parent (orchestrate::gossip) span so session
                // spans are properly parented across the thread boundary.
                let _parent_guard = worker_parent.enter();
                let _session_span =
                    info_span!("orchestrate::gossip::session", session_name = %name).entered();
                // Acquire semaphore slot (bounded concurrency).
                {
                    let (lock, cvar) = &*semaphore;
                    let mut in_flight = lock.lock().unwrap();
                    while *in_flight >= effective_concurrency {
                        in_flight = cvar.wait(in_flight).unwrap();
                    }
                    *in_flight += 1;
                }

                let session_start = Instant::now();

                // Mark session Running.
                {
                    let mut statuses = session_statuses_arc.lock().unwrap();
                    if let Some(s) = statuses.iter_mut().find(|s| s.name == name) {
                        s.state = SessionRunState::Running;
                        s.started_at = Some(Utc::now());
                    }
                }

                tracing::info!(session = %name, "gossip session starting");

                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    session_runner(session_clone, pipeline_config)
                }));

                let session_duration = session_start.elapsed();
                let completed_at = Utc::now();

                // Extract outcome details.
                let (run_state, error_msg, pipeline_result_opt) = match &result {
                    Ok(Ok(pr)) => (SessionRunState::Completed, None, Some(pr)),
                    Ok(Err(e)) => (SessionRunState::Failed, Some(e.message.clone()), None),
                    Err(panic_payload) => {
                        let msg = if let Some(s) = panic_payload.downcast_ref::<String>() {
                            format!("panic: {s}")
                        } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
                            format!("panic: {s}")
                        } else {
                            "panic: unknown".to_string()
                        };
                        tracing::error!(
                            session = %name,
                            panic_message = %msg,
                            "gossip session worker panicked"
                        );
                        (SessionRunState::Failed, Some(msg), None)
                    }
                };

                // Extract gate counts and changed files from pipeline result.
                let gate_pass_count = pipeline_result_opt
                    .and_then(|pr| pr.gate_summary.as_ref())
                    .map(|gs| gs.passed as u32)
                    .unwrap_or(0);
                let gate_fail_count = pipeline_result_opt
                    .and_then(|pr| pr.gate_summary.as_ref())
                    .map(|gs| gs.failed as u32)
                    .unwrap_or(0);
                let changed_files: Vec<String> = pipeline_result_opt
                    .and_then(|pr| pr.merge_check.as_ref())
                    .map(|mc| mc.files.iter().map(|f| f.path.clone()).collect())
                    .unwrap_or_default();

                // Get spec from session statuses.
                let spec = {
                    let statuses = session_statuses_arc.lock().unwrap();
                    statuses
                        .iter()
                        .find(|s| s.name == name)
                        .map(|s| s.spec.clone())
                        .unwrap_or_default()
                };

                // Send completion to coordinator.
                let _ = tx_worker.send(GossipCompletion {
                    session_name: name.clone(),
                    spec,
                    gate_pass_count,
                    gate_fail_count,
                    changed_files,
                    completed_at,
                });
                drop(tx_worker);

                // Update session status.
                {
                    let mut statuses = session_statuses_arc.lock().unwrap();
                    if let Some(s) = statuses.iter_mut().find(|s| s.name == name) {
                        s.state = run_state;
                        s.completed_at = Some(completed_at);
                        s.duration_secs = Some(session_duration.as_secs_f64());
                        s.error = error_msg;
                    }
                }

                // Best-effort state snapshot.
                {
                    let statuses = session_statuses_arc.lock().unwrap().clone();
                    let gs_snap = gossip_status_arc.lock().unwrap().clone();
                    let snapshot = OrchestratorStatus {
                        run_id: run_id.clone(),
                        phase: OrchestratorPhase::Running,
                        failure_policy,
                        sessions: statuses,
                        started_at: started_at_run,
                        completed_at: None,
                        mesh_status: None,
                        gossip_status: Some(gs_snap),
                    };
                    let _ = persist_state(run_dir, &snapshot);
                }

                // Decrement active count and release semaphore.
                active_count.fetch_sub(1, Ordering::Release);

                {
                    let (lock, cvar) = &*semaphore;
                    let mut in_flight = lock.lock().unwrap();
                    *in_flight -= 1;
                    cvar.notify_all();
                }
            });
        }

        // Drop the parent's tx so coordinator exits when all worker clones drop.
        drop(tx);
    });

    // ── Build final status and outcomes ──────────────────────────────

    let wall_duration = wall_start.elapsed();
    let final_statuses = session_statuses_arc.lock().unwrap().clone();
    let final_gossip = gossip_status_arc.lock().unwrap().clone();

    let any_failed = final_statuses
        .iter()
        .any(|s| s.state == SessionRunState::Failed);

    let final_phase = if any_failed {
        OrchestratorPhase::PartialFailure
    } else {
        OrchestratorPhase::Completed
    };

    let final_status = OrchestratorStatus {
        run_id: run_id.clone(),
        phase: final_phase,
        failure_policy: config.failure_policy,
        sessions: final_statuses.clone(),
        started_at,
        completed_at: Some(Utc::now()),
        mesh_status: None,
        gossip_status: Some(final_gossip),
    };
    let _ = persist_state(&run_dir, &final_status);

    // Build outcomes vec (mirrors mesh.rs pattern).
    let outcomes: Vec<(String, SessionOutcome)> = cloned_sessions
        .iter()
        .zip(final_statuses.iter())
        .map(|((name, _session), status)| {
            let outcome = match status.state {
                SessionRunState::Completed => SessionOutcome::Completed {
                    result: Box::new(PipelineResult {
                        session_id: format!("gossip-{name}"),
                        spec_name: status.spec.clone(),
                        gate_summary: None,
                        merge_check: None,
                        stage_timings: vec![],
                        outcome: crate::pipeline::PipelineOutcome::Success,
                    }),
                    worktree_path: PathBuf::new(),
                    branch_name: String::new(),
                    changed_files: vec![],
                },
                _ => SessionOutcome::Failed {
                    error: PipelineError {
                        stage: PipelineStage::AgentLaunch,
                        message: status
                            .error
                            .clone()
                            .unwrap_or_else(|| "session failed".to_string()),
                        recovery: "check gossip session logs".to_string(),
                        elapsed: Duration::from_secs_f64(status.duration_secs.unwrap_or(0.0)),
                    },
                    stage: PipelineStage::AgentLaunch,
                },
            };
            (name.clone(), outcome)
        })
        .collect();

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
    use assay_types::{ManifestSession, OrchestratorMode, RunManifest};
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

    fn make_session(spec: &str, name: Option<&str>) -> ManifestSession {
        ManifestSession {
            spec: spec.to_string(),
            name: name.map(|n| n.to_string()),
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
        }
    }

    fn make_manifest(sessions: Vec<ManifestSession>) -> RunManifest {
        RunManifest {
            sessions,
            mode: OrchestratorMode::Gossip,
            mesh_config: None,
            gossip_config: None,
            state_backend: None,
        }
    }

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

    fn success_result(session: &ManifestSession) -> PipelineResult {
        PipelineResult {
            session_id: format!("sess-{}", session.spec),
            spec_name: session.spec.clone(),
            gate_summary: None,
            merge_check: None,
            stage_timings: vec![],
            outcome: crate::pipeline::PipelineOutcome::Success,
        }
    }

    #[test]
    fn run_gossip_calls_runner() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".assay/orchestrator")).unwrap();

        let manifest = make_manifest(vec![
            make_session("spec-a", Some("alpha")),
            make_session("spec-b", Some("beta")),
        ]);
        let config = OrchestratorConfig::default();
        let pipeline_config = make_pipeline_config(tmp.path());

        let runner_called = Arc::new(AtomicBool::new(false));
        let runner_called_inner = Arc::clone(&runner_called);

        let runner = move |session: &ManifestSession, _config: &PipelineConfig| {
            runner_called_inner.store(true, AtomicOrdering::SeqCst);
            Ok::<PipelineResult, PipelineError>(success_result(session))
        };

        let result = run_gossip(&manifest, &config, &pipeline_config, &runner).unwrap();
        assert!(
            runner_called.load(AtomicOrdering::SeqCst),
            "session_runner must be invoked by run_gossip"
        );
        assert_eq!(result.outcomes.len(), 2);
    }
}
