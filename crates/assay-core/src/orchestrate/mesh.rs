//! Mesh-mode executor for peer-coordinated multi-session runs.
//!
//! `run_mesh()` launches all sessions in parallel, injects a roster
//! `PromptLayer` into each session clone (listing peers and inbox/outbox
//! paths), runs a background routing thread that polls each session's outbox
//! and moves files to the target session's inbox, and tracks SWIM-inspired
//! membership state — persisted to `state.json` after each completion.

use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use ulid::Ulid;

use assay_types::orchestrate::{
    MeshMemberState, MeshMemberStatus, MeshStatus, OrchestratorPhase, OrchestratorStatus,
    SessionRunState, SessionStatus,
};
use assay_types::{ManifestSession, PromptLayer, PromptLayerKind};

use tracing::{Span, info, info_span};

use crate::error::AssayError;
use crate::orchestrate::executor::{OrchestratorConfig, OrchestratorResult, SessionOutcome};
use crate::pipeline::{PipelineConfig, PipelineError, PipelineResult};

/// Run a mesh-mode multi-session execution.
///
/// All sessions are launched in parallel (subject to `max_concurrency`).
/// A roster `PromptLayer` is injected into each session clone so runners
/// can discover peer inbox/outbox paths. A background routing thread polls
/// each session's outbox subdirectories and moves message files to the
/// target session's inbox. `OrchestratorStatus` (including `mesh_status`)
/// is persisted to `state.json` after each session completes.
pub fn run_mesh<F>(
    manifest: &assay_types::RunManifest,
    config: &OrchestratorConfig,
    pipeline_config: &PipelineConfig,
    session_runner: &F,
) -> Result<OrchestratorResult, AssayError>
where
    F: Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError> + Sync,
{
    run_mesh_with_id(manifest, config, pipeline_config, session_runner, None)
}

/// Like [`run_mesh`] but accepts an optional pre-generated `run_id`.
///
/// When `run_id` is `Some`, uses the provided value so callers can register
/// sessions in a `RunRegistry` before the run starts. When `None`, generates
/// a fresh ULID (existing behavior).
pub fn run_mesh_with_id<F>(
    manifest: &assay_types::RunManifest,
    config: &OrchestratorConfig,
    pipeline_config: &PipelineConfig,
    session_runner: &F,
    run_id: Option<String>,
) -> Result<OrchestratorResult, AssayError>
where
    F: Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError> + Sync,
{
    let session_count = manifest.sessions.len();

    let _root_span = info_span!(
        "orchestrate::mesh",
        session_count = session_count,
        mode = "mesh"
    )
    .entered();
    info!("starting Mesh orchestration");

    let run_id = run_id.unwrap_or_else(|| Ulid::new().to_string());
    let started_at = Utc::now();
    let wall_start = Instant::now();

    // Create run directory: .assay/orchestrator/<run_id>/
    let run_dir = pipeline_config.assay_dir.join("orchestrator").join(&run_id);
    std::fs::create_dir_all(&run_dir)
        .map_err(|e| AssayError::io("creating orchestrator run directory", &run_dir, e))?;

    // ── Per-session directory setup ──────────────────────────────────

    // Build per-session name, inbox, and outbox paths.
    let mut session_names: Vec<String> = Vec::with_capacity(session_count);
    let mut name_to_inbox: HashMap<String, PathBuf> = HashMap::new();
    let mut name_to_outbox: HashMap<String, PathBuf> = HashMap::new();
    let mut session_dirs: Vec<(String, PathBuf)> = Vec::with_capacity(session_count);

    for session in &manifest.sessions {
        let name = session.name.as_deref().unwrap_or(&session.spec).to_string();

        if !session.depends_on.is_empty() {
            tracing::warn!(
                session = %name,
                "depends_on is ignored in Mesh mode"
            );
        }

        let mesh_dir = run_dir.join("mesh").join(&name);
        let inbox_path = mesh_dir.join("inbox");
        let outbox_path = mesh_dir.join("outbox");

        std::fs::create_dir_all(&inbox_path)
            .map_err(|e| AssayError::io("creating session inbox directory", &inbox_path, e))?;
        std::fs::create_dir_all(&outbox_path)
            .map_err(|e| AssayError::io("creating session outbox directory", &outbox_path, e))?;

        tracing::info!(
            session = %name,
            inbox = %inbox_path.display(),
            "mesh session inbox created"
        );

        name_to_inbox.insert(name.clone(), inbox_path);
        name_to_outbox.insert(name.clone(), outbox_path);
        session_dirs.push((name.clone(), mesh_dir));
        session_names.push(name);
    }

    // ── Build cloned sessions with roster PromptLayer ────────────────

    // For each session, build a roster listing all peers and its own outbox.
    let mut cloned_sessions: Vec<(String, ManifestSession)> = Vec::with_capacity(session_count);

    for (i, session) in manifest.sessions.iter().enumerate() {
        let name = &session_names[i];
        let own_outbox = &name_to_outbox[name];

        // Roster content: own outbox + peer inboxes.
        let mut roster_lines = Vec::new();
        roster_lines.push(format!("# Mesh Roster for session: {name}"));
        roster_lines.push(format!("Outbox: {}", own_outbox.display()));
        roster_lines.push(String::new());
        roster_lines.push("# Peers".to_string());
        for peer_name in &session_names {
            if peer_name == name {
                continue;
            }
            let peer_inbox = &name_to_inbox[peer_name];
            roster_lines.push(format!(
                "Peer: {peer_name}  Inbox: {}",
                peer_inbox.display()
            ));
        }
        let roster_content = roster_lines.join("\n");

        let mut session_clone = session.clone();
        session_clone.prompt_layers.push(PromptLayer {
            kind: PromptLayerKind::System,
            name: "mesh-roster".to_string(),
            content: roster_content,
            priority: -5,
        });

        cloned_sessions.push((name.clone(), session_clone));
    }

    // ── Initialize status ────────────────────────────────────────────

    let initial_session_statuses: Vec<SessionStatus> = session_names
        .iter()
        .zip(manifest.sessions.iter())
        .map(|(name, session)| SessionStatus {
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

    let initial_mesh_status = MeshStatus {
        members: session_names
            .iter()
            .map(|name| MeshMemberStatus {
                name: name.clone(),
                state: MeshMemberState::Alive,
                last_heartbeat_at: Some(Utc::now()),
            })
            .collect(),
        messages_routed: 0,
    };

    let initial_status = OrchestratorStatus {
        run_id: run_id.clone(),
        phase: OrchestratorPhase::Running,
        failure_policy: config.failure_policy,
        sessions: initial_session_statuses.clone(),
        started_at,
        completed_at: None,
        mesh_status: Some(initial_mesh_status),
        gossip_status: None,
    };

    config
        .backend
        .push_session_event(&run_dir, &initial_status)?;

    // ── Shared state ─────────────────────────────────────────────────

    let active_count = Arc::new(AtomicUsize::new(session_count));
    let mesh_status_arc = Arc::new(Mutex::new(MeshStatus {
        members: session_names
            .iter()
            .map(|name| MeshMemberStatus {
                name: name.clone(),
                state: MeshMemberState::Alive,
                last_heartbeat_at: Some(Utc::now()),
            })
            .collect(),
        messages_routed: 0,
    }));
    let session_statuses_arc = Arc::new(Mutex::new(initial_session_statuses));

    // Bounded concurrency semaphore: (Mutex<usize>, Condvar) — slot counter.
    let effective_concurrency = config.max_concurrency.min(session_count);
    let semaphore = Arc::new((Mutex::new(0usize), Condvar::new()));

    // ── thread::scope ────────────────────────────────────────────────

    // Capture current span for cross-thread parenting.
    let parent_span = Span::current();

    // ── Capability check for messaging ───────────────────────────────
    let messaging_supported = config.backend.capabilities().supports_messaging;
    if !messaging_supported {
        tracing::warn!(
            capability = "messaging",
            mode = "mesh",
            "backend does not support messaging — mesh routing thread will not be spawned; \
             sessions execute in parallel but peer-to-peer message passing is unavailable"
        );
    }

    std::thread::scope(|scope| {
        // ── Routing thread (only spawned when messaging is supported) ─
        let active_count_ref = &active_count;
        let name_to_inbox_ref = &name_to_inbox;
        let mesh_status_arc_ref = &mesh_status_arc;
        let session_dirs_ref = &session_dirs;
        let routing_parent = parent_span.clone();

        if messaging_supported {
            scope.spawn(move || {
                let _parent_guard = routing_parent.enter();
                let _routing_span = info_span!("orchestrate::mesh::routing").entered();

                while active_count_ref.load(Ordering::Acquire) > 0 {
                    for (source_name, session_dir) in session_dirs_ref {
                        let outbox = session_dir.join("outbox");
                        let targets = match std::fs::read_dir(&outbox) {
                            Ok(rd) => rd,
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                            Err(e) => {
                                tracing::warn!(
                                    session = %source_name,
                                    outbox = %outbox.display(),
                                    error = %e,
                                    "failed to read outbox directory — messages from this session may not be routed"
                                );
                                continue;
                            }
                        };
                        for target_entry in targets.flatten() {
                            let target_name =
                                target_entry.file_name().to_string_lossy().to_string();
                            if let Some(inbox) = name_to_inbox_ref.get(&target_name) {
                                let msgs = match std::fs::read_dir(target_entry.path()) {
                                    Ok(rd) => rd,
                                    Err(e) => {
                                        tracing::warn!(
                                            source = %source_name,
                                            target = %target_name,
                                            error = %e,
                                            "failed to read target outbox subdirectory — messages may be lost"
                                        );
                                        continue;
                                    }
                                };
                                for msg in msgs.flatten() {
                                    let dst = inbox.join(msg.file_name());
                                    match std::fs::rename(msg.path(), &dst) {
                                        Ok(()) => {
                                            let mut ms = mesh_status_arc_ref.lock().unwrap();
                                            ms.messages_routed += 1;
                                            tracing::debug!(
                                                from = %source_name,
                                                to = %target_name,
                                                file = ?msg.file_name(),
                                                "routed message"
                                            );
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                from = %source_name,
                                                to = %target_name,
                                                file = ?msg.file_name(),
                                                error = %e,
                                                "failed to route mesh message — message left in outbox"
                                            );
                                        }
                                    }
                                }
                            } else {
                                tracing::warn!(
                                    target = %target_name,
                                    source = %source_name,
                                    "unknown outbox target — leaving file in place"
                                );
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            });
        } // end if messaging_supported

        // ── Session workers ───────────────────────────────────────────
        for (name, session_clone) in &cloned_sessions {
            let name = name.clone();
            let active_count = Arc::clone(&active_count);
            let mesh_status_arc = Arc::clone(&mesh_status_arc);
            let session_statuses_arc = Arc::clone(&session_statuses_arc);
            let semaphore = Arc::clone(&semaphore);
            let run_dir = &run_dir;
            let run_id = &run_id;
            let started_at_run = started_at;
            let failure_policy = config.failure_policy;
            let backend = Arc::clone(&config.backend);
            let worker_parent = parent_span.clone();

            scope.spawn(move || {
                // Re-enter the parent (orchestrate::mesh) span so session
                // spans are properly parented across the thread boundary.
                let _parent_guard = worker_parent.enter();
                let _session_span =
                    info_span!("orchestrate::mesh::session", session_name = %name).entered();
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

                tracing::info!(
                    session = %name,
                    "mesh session starting"
                );

                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    session_runner(session_clone, pipeline_config)
                }));

                let session_duration = session_start.elapsed();
                let completed_at = Utc::now();

                // Write completed sentinel regardless of outcome.
                let session_dir = run_dir.join("mesh").join(&name);
                if let Err(e) = std::fs::write(session_dir.join("completed"), b"") {
                    tracing::error!(
                        session = %name,
                        path = %session_dir.join("completed").display(),
                        error = %e,
                        "failed to write mesh session completion sentinel — routing thread may stall"
                    );
                }

                // Determine member state and session run state.
                let (member_state, run_state, error_msg) = match &result {
                    Ok(Ok(_)) => (MeshMemberState::Completed, SessionRunState::Completed, None),
                    Ok(Err(e)) => (
                        MeshMemberState::Dead,
                        SessionRunState::Failed,
                        Some(e.message.clone()),
                    ),
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
                            "mesh session worker panicked"
                        );
                        (MeshMemberState::Dead, SessionRunState::Failed, Some(msg))
                    }
                };

                // Update mesh member state.
                {
                    let mut ms = mesh_status_arc.lock().unwrap();
                    if let Some(member) = ms.members.iter_mut().find(|m| m.name == name) {
                        member.state = member_state;
                        member.last_heartbeat_at = Some(completed_at);
                    }
                }

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

                // Persist state snapshot (best-effort).
                {
                    let statuses = session_statuses_arc.lock().unwrap().clone();
                    let mesh = mesh_status_arc.lock().unwrap().clone();
                    let snapshot = OrchestratorStatus {
                        run_id: run_id.clone(),
                        phase: OrchestratorPhase::Running,
                        failure_policy,
                        sessions: statuses,
                        started_at: started_at_run,
                        completed_at: None,
                        mesh_status: Some(mesh),
                        gossip_status: None,
                    };
                    if let Err(e) = backend.push_session_event(run_dir, &snapshot) {
                        tracing::warn!(error = %e, "best-effort state snapshot failed — run status may be stale");
                    }
                }

                // Decrement active count — routing thread exits when 0.
                active_count.fetch_sub(1, Ordering::Release);

                // Release semaphore slot.
                {
                    let (lock, cvar) = &*semaphore;
                    let mut in_flight = lock.lock().unwrap();
                    *in_flight -= 1;
                    cvar.notify_all();
                }
            });
        }
    });

    // ── Build outcomes ────────────────────────────────────────────────

    let wall_duration = wall_start.elapsed();
    let final_statuses = session_statuses_arc.lock().unwrap().clone();
    let final_mesh = mesh_status_arc.lock().unwrap().clone();

    let any_failed = final_statuses
        .iter()
        .any(|s| s.state == SessionRunState::Failed);

    let final_phase = if any_failed {
        OrchestratorPhase::PartialFailure
    } else {
        OrchestratorPhase::Completed
    };

    // Persist final state.
    let final_status = OrchestratorStatus {
        run_id: run_id.clone(),
        phase: final_phase,
        failure_policy: config.failure_policy,
        sessions: final_statuses.clone(),
        started_at,
        completed_at: Some(Utc::now()),
        mesh_status: Some(final_mesh),
        gossip_status: None,
    };
    if let Err(e) = config.backend.push_session_event(&run_dir, &final_status) {
        tracing::error!(
            run_id = %run_id,
            error = %e,
            "failed to persist final orchestrator state — run status is not durable"
        );
    }

    // Build outcomes vec.
    let outcomes: Vec<(String, SessionOutcome)> = cloned_sessions
        .iter()
        .zip(final_statuses.iter())
        .map(|((name, _session), status)| {
            let outcome = match status.state {
                SessionRunState::Completed => SessionOutcome::Completed {
                    result: Box::new(PipelineResult {
                        session_id: format!("mesh-{name}"),
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
                        stage: crate::pipeline::PipelineStage::AgentLaunch,
                        message: status
                            .error
                            .clone()
                            .unwrap_or_else(|| "session failed".to_string()),
                        recovery: "check mesh session logs".to_string(),
                        elapsed: Duration::from_secs_f64(status.duration_secs.unwrap_or(0.0)),
                    },
                    stage: crate::pipeline::PipelineStage::AgentLaunch,
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
    use std::sync::atomic::AtomicUsize;

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
            user_prompt: None,
            prompt_file: None,
        }
    }

    fn make_manifest(sessions: Vec<ManifestSession>) -> RunManifest {
        RunManifest {
            sessions,
            mode: OrchestratorMode::Mesh,
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
    fn run_mesh_calls_all_runners() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".assay/orchestrator")).unwrap();
        let manifest = make_manifest(vec![
            make_session("spec-a", Some("alpha")),
            make_session("spec-b", Some("beta")),
        ]);
        let config = OrchestratorConfig::default();
        let pipeline_config = make_pipeline_config(tmp.path());

        let call_count = AtomicUsize::new(0);
        let runner = |session: &ManifestSession, _config: &PipelineConfig| {
            call_count.fetch_add(1, Ordering::SeqCst);
            Ok(success_result(session))
        };

        let result = run_mesh(&manifest, &config, &pipeline_config, &runner).unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
        assert_eq!(result.outcomes.len(), 2);
    }

    #[test]
    fn run_mesh_roster_layer_injected() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".assay/orchestrator")).unwrap();
        let manifest = make_manifest(vec![
            make_session("spec-a", Some("alpha")),
            make_session("spec-b", Some("beta")),
        ]);
        let config = OrchestratorConfig::default();
        let pipeline_config = make_pipeline_config(tmp.path());

        let runner = |session: &ManifestSession, _config: &PipelineConfig| {
            // Every session must have a mesh-roster layer.
            let roster = session
                .prompt_layers
                .iter()
                .find(|l| l.name == "mesh-roster");
            assert!(
                roster.is_some(),
                "session '{}' missing mesh-roster layer",
                session.name.as_deref().unwrap_or(&session.spec)
            );
            let content = &roster.unwrap().content;
            assert!(
                content.contains("Outbox:"),
                "roster must contain Outbox path"
            );
            Ok(success_result(session))
        };

        run_mesh(&manifest, &config, &pipeline_config, &runner).unwrap();
    }

    #[test]
    fn run_mesh_persists_state_json() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".assay/orchestrator")).unwrap();
        let manifest = make_manifest(vec![make_session("spec-solo", Some("solo"))]);
        let config = OrchestratorConfig::default();
        let pipeline_config = make_pipeline_config(tmp.path());

        let runner =
            |session: &ManifestSession, _config: &PipelineConfig| Ok(success_result(session));
        let result = run_mesh(&manifest, &config, &pipeline_config, &runner).unwrap();

        let state_path = pipeline_config
            .assay_dir
            .join("orchestrator")
            .join(&result.run_id)
            .join("state.json");
        assert!(state_path.exists(), "state.json should be persisted");

        let raw = std::fs::read_to_string(&state_path).unwrap();
        let status: OrchestratorStatus = serde_json::from_str(&raw).unwrap();
        assert!(status.mesh_status.is_some());
    }

    #[test]
    fn run_mesh_emits_warn_for_depends_on() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".assay/orchestrator")).unwrap();
        let mut session = make_session("some-spec", None);
        session.depends_on = vec!["other".to_string()];
        let manifest = make_manifest(vec![session]);
        let config = OrchestratorConfig::default();
        let pipeline_config = make_pipeline_config(tmp.path());

        // Should succeed despite depends_on (just warns and ignores).
        let runner =
            |session: &ManifestSession, _config: &PipelineConfig| Ok(success_result(session));
        let result = run_mesh(&manifest, &config, &pipeline_config, &runner).unwrap();
        assert_eq!(result.outcomes.len(), 1);
    }
}
