//! Work session persistence: create, transition, save, load, and list.
//!
//! Sessions are persisted as pretty-printed JSON under `.assay/sessions/<session-id>.json`.
//! All writes use the atomic tempfile-then-rename pattern proven in [`crate::history`].

use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use tempfile::NamedTempFile;

use assay_types::work_session::{AgentInvocation, PhaseTransition, SessionPhase, WorkSession};

use crate::error::{AssayError, Result};

/// Create a new work session in the [`SessionPhase::Created`] phase.
///
/// Generates a ULID-based identifier with a `created_at` timestamp.
/// The transitions list starts empty — the first entry is appended
/// by [`transition_session`] when the session advances.
pub fn create_work_session(
    spec_name: &str,
    worktree_path: PathBuf,
    agent_command: &str,
    agent_model: Option<&str>,
) -> WorkSession {
    let now = Utc::now();
    let id = ulid::Ulid::new().to_string();

    WorkSession {
        id,
        spec_name: spec_name.to_string(),
        worktree_path,
        phase: SessionPhase::Created,
        created_at: now,
        transitions: vec![],
        agent: AgentInvocation {
            command: agent_command.to_string(),
            model: agent_model.map(String::from),
        },
        gate_runs: vec![],
        assay_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Transition a session to the next phase, appending an audit trail entry.
///
/// Returns [`AssayError::WorkSessionTransition`] if the transition is invalid
/// (e.g., skipping phases or transitioning from a terminal phase).
pub fn transition_session(
    session: &mut WorkSession,
    next: SessionPhase,
    trigger: &str,
    notes: Option<&str>,
) -> Result<()> {
    if !session.phase.can_transition_to(next) {
        return Err(AssayError::WorkSessionTransition {
            session_id: session.id.clone(),
            from: session.phase,
            to: next,
        });
    }

    session.transitions.push(PhaseTransition {
        from: session.phase,
        to: next,
        timestamp: Utc::now(),
        trigger: trigger.to_string(),
        notes: notes.map(String::from),
    });
    session.phase = next;
    Ok(())
}

/// Persist a work session as atomic pretty-printed JSON.
///
/// Creates `.assay/sessions/` if it does not exist. Uses the tempfile-then-rename
/// pattern to guarantee the file is either fully written or absent.
///
/// Returns the final path on success.
pub fn save_session(assay_dir: &Path, session: &WorkSession) -> Result<PathBuf> {
    let sessions_dir = assay_dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir)
        .map_err(|e| AssayError::io("creating sessions directory", &sessions_dir, e))?;

    crate::history::validate_path_component(&session.id, "session ID")?;

    let final_path = sessions_dir.join(format!("{}.json", session.id));

    let json = serde_json::to_string_pretty(session).map_err(|e| {
        AssayError::json(
            format!("serializing work session {}", session.id),
            &final_path,
            e,
        )
    })?;

    let mut tmpfile = NamedTempFile::new_in(&sessions_dir)
        .map_err(|e| AssayError::io("creating temp file for session", &sessions_dir, e))?;

    tmpfile
        .write_all(json.as_bytes())
        .map_err(|e| AssayError::io("writing work session", &final_path, e))?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|e| AssayError::io("syncing work session", &final_path, e))?;
    tmpfile
        .persist(&final_path)
        .map_err(|e| AssayError::io("persisting work session", &final_path, e.error))?;

    Ok(final_path)
}

/// Load a work session by ID from `.assay/sessions/<session_id>.json`.
///
/// Returns [`AssayError::WorkSessionNotFound`] if the file does not exist.
/// Returns an error if `session_id` contains path traversal components.
pub fn load_session(assay_dir: &Path, session_id: &str) -> Result<WorkSession> {
    crate::history::validate_path_component(session_id, "session ID")?;

    let path = assay_dir
        .join("sessions")
        .join(format!("{session_id}.json"));

    let content = std::fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AssayError::WorkSessionNotFound {
                session_id: session_id.to_string(),
            }
        } else {
            AssayError::io("reading work session", &path, e)
        }
    })?;

    serde_json::from_str(&content)
        .map_err(|e| AssayError::json("deserializing work session", &path, e))
}

/// List session IDs in lexicographic (chronological) order.
///
/// Returns an empty vec if the sessions directory does not exist.
pub fn list_sessions(assay_dir: &Path) -> Result<Vec<String>> {
    let sessions_dir = assay_dir.join("sessions");
    if !sessions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut ids: Vec<String> = std::fs::read_dir(&sessions_dir)
        .map_err(|e| AssayError::io("listing sessions", &sessions_dir, e))?
        .filter_map(|entry| match entry {
            Ok(e) => Some(e),
            Err(e) => {
                // NOTE: entry-level read_dir errors are logged but not surfaced to callers.
                // Changing the return type to include warnings would be more invasive;
                // callers that need full error reporting should use the MCP session_list
                // which captures per-session load failures separately.
                tracing::warn!("skipping session entry: {e}");
                None
            }
        })
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                path.file_stem().and_then(|s| s.to_str()).map(String::from)
            } else {
                None
            }
        })
        .collect();

    // ULID string sort = chronological sort.
    ids.sort();
    Ok(ids)
}

/// Load a session, apply a mutation, and save atomically.
///
/// Returns the mutated session on success. If the closure returns an error,
/// the session is NOT saved — the on-disk state remains unchanged.
pub fn with_session<F>(assay_dir: &Path, session_id: &str, mutate: F) -> Result<WorkSession>
where
    F: FnOnce(&mut WorkSession) -> Result<()>,
{
    let mut session = load_session(assay_dir, session_id)?;
    mutate(&mut session)?;
    save_session(assay_dir, &session)?;
    Ok(session)
}

/// Create a session and immediately transition to [`SessionPhase::AgentRunning`].
///
/// Combines create + transition + save in a single atomic operation.
/// Returns the saved session.
pub fn start_session(
    assay_dir: &Path,
    spec_name: &str,
    worktree_path: PathBuf,
    agent_command: &str,
    agent_model: Option<&str>,
) -> Result<WorkSession> {
    let mut session = create_work_session(spec_name, worktree_path, agent_command, agent_model);
    transition_session(
        &mut session,
        SessionPhase::AgentRunning,
        "session_start",
        None,
    )?;
    save_session(assay_dir, &session)?;
    Ok(session)
}

/// Transition a session to GateEvaluated and link a gate run ID.
///
/// Convenience for the common gate_evaluate flow: load → transition → link run → save.
pub fn record_gate_result(
    assay_dir: &Path,
    session_id: &str,
    gate_run_id: &str,
    trigger: &str,
    notes: Option<&str>,
) -> Result<WorkSession> {
    with_session(assay_dir, session_id, |session| {
        transition_session(session, SessionPhase::GateEvaluated, trigger, notes)?;
        if !session.gate_runs.contains(&gate_run_id.to_string()) {
            session.gate_runs.push(gate_run_id.to_string());
        }
        Ok(())
    })
}

/// Mark a session as completed after successful evaluation.
pub fn complete_session(
    assay_dir: &Path,
    session_id: &str,
    notes: Option<&str>,
) -> Result<WorkSession> {
    with_session(assay_dir, session_id, |session| {
        transition_session(session, SessionPhase::Completed, "session_complete", notes)
    })
}

/// Mark a session as abandoned with a reason.
pub fn abandon_session(assay_dir: &Path, session_id: &str, reason: &str) -> Result<WorkSession> {
    with_session(assay_dir, session_id, |session| {
        transition_session(
            session,
            SessionPhase::Abandoned,
            "session_abandon",
            Some(reason),
        )
    })
}

// ── Recovery scan ────────────────────────────────────────────────────

/// Summary of a recovery scan for stale sessions.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RecoverySummary {
    /// Number of sessions recovered (transitioned to Abandoned).
    pub recovered: usize,
    /// Number of eligible sessions (AgentRunning and stale) that could not be recovered
    /// due to missing transition records or failed state transitions.
    pub skipped: usize,
    /// Number of errors encountered (corrupt files, save failures).
    pub errors: usize,
    /// Whether the scan was capped at the 100-session limit.
    ///
    /// When `true`, there may be additional sessions beyond the cap that were not scanned.
    /// Operators should investigate and recover any remaining stale sessions manually.
    pub truncated: bool,
}

/// Format a [`chrono::Duration`] as a human-readable string (e.g., "3h 12m" or "45m").
fn format_duration(d: chrono::Duration) -> String {
    let h = d.num_hours();
    let m = d.num_minutes() % 60;
    match (h, m) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// Build a recovery note with timing and host details.
///
/// Format: `"Recovered on startup: stale for 3h 12m (threshold: 1h). Host: macbook.local, PID: 12345"`
///
/// Duration formatting: "3h 12m" for mixed, "1h" for exact hours, "45m" for sub-hour.
fn build_recovery_note(
    stale_duration: chrono::Duration,
    threshold: chrono::Duration,
    hostname: &str,
    pid: u32,
) -> String {
    format!(
        "Recovered on startup: stale for {} (threshold: {}). Host: {}, PID: {}",
        format_duration(stale_duration),
        format_duration(threshold),
        hostname,
        pid
    )
}

/// Scan for stale `agent_running` sessions and mark them as abandoned.
///
/// Called on MCP server startup before any tool call. Sessions in `AgentRunning`
/// phase that are older than `stale_threshold_secs` (measured from the transition
/// timestamp into `AgentRunning`) are transitioned to `Abandoned` with a recovery
/// note containing hostname, PID, and timing details.
///
/// Corrupt session files are logged and skipped — one bad file does not block
/// recovery of other sessions. The scan is capped at 100 sessions (oldest first).
pub fn recover_stale_sessions(assay_dir: &Path, stale_threshold_secs: u64) -> RecoverySummary {
    let mut summary = RecoverySummary::default();

    let ids = match list_sessions(assay_dir) {
        Ok(ids) => ids,
        Err(e) => {
            tracing::warn!("recovery scan: cannot list sessions: {e}");
            return summary;
        }
    };

    let threshold = chrono::Duration::seconds(stale_threshold_secs as i64);
    let now = Utc::now();
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let pid = std::process::id();

    // Process oldest first (ULID sort = chronological), cap at 100.
    const RECOVERY_CAP: usize = 100;
    summary.truncated = ids.len() > RECOVERY_CAP;
    for id in ids.iter().take(RECOVERY_CAP) {
        let mut session = match load_session(assay_dir, id) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(session_id = %id, "recovery scan: skipping corrupt session: {e}");
                summary.errors += 1;
                continue;
            }
        };

        if session.phase != SessionPhase::AgentRunning {
            continue;
        }

        // Find the transition timestamp into AgentRunning.
        let entered_at = session
            .transitions
            .iter()
            .rev()
            .find(|t| t.to == SessionPhase::AgentRunning)
            .map(|t| t.timestamp);

        let Some(entered_at) = entered_at else {
            tracing::warn!(
                session_id = %id,
                "recovery scan: session in AgentRunning but no transition record, skipping"
            );
            summary.skipped += 1;
            continue;
        };

        let age = now - entered_at;
        if age < threshold {
            continue;
        }

        let note = build_recovery_note(age, threshold, &hostname, pid);
        if let Err(e) = transition_session(
            &mut session,
            SessionPhase::Abandoned,
            "startup_recovery",
            Some(&note),
        ) {
            tracing::warn!(session_id = %id, "recovery scan: transition failed: {e}");
            summary.errors += 1;
            continue;
        }

        match save_session(assay_dir, &session) {
            Ok(_) => {
                tracing::info!(
                    session_id = %id,
                    spec_name = %session.spec_name,
                    stale_duration = %format_duration(age),
                    "recovered stale session"
                );
                summary.recovered += 1;
            }
            Err(e) => {
                tracing::warn!(session_id = %id, "recovery scan: save failed: {e}");
                summary.errors += 1;
            }
        }
    }

    if summary.recovered > 0 {
        tracing::info!(
            recovered = summary.recovered,
            skipped = summary.skipped,
            errors = summary.errors,
            "recovery scan complete"
        );
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── create_work_session ──────────────────────────────────────

    #[test]
    fn create_session_has_ulid_id() {
        let session = create_work_session("test-spec", PathBuf::from("/tmp/wt"), "claude", None);
        assert!(!session.id.is_empty());
        assert_eq!(session.id.len(), 26, "ULID string is 26 characters");
    }

    #[test]
    fn create_session_starts_in_created_phase() {
        let session = create_work_session("test-spec", PathBuf::from("/tmp/wt"), "claude", None);
        assert_eq!(session.phase, SessionPhase::Created);
    }

    #[test]
    fn create_session_has_created_at_and_empty_transitions() {
        let before = Utc::now();
        let session = create_work_session("test-spec", PathBuf::from("/tmp/wt"), "claude", None);
        let after = Utc::now();
        assert!(session.transitions.is_empty(), "transitions start empty");
        assert!(session.created_at >= before && session.created_at <= after);
    }

    #[test]
    fn create_session_captures_agent_info() {
        let session = create_work_session(
            "auth-flow",
            PathBuf::from("/tmp/wt/auth"),
            "claude --spec auth-flow",
            Some("claude-sonnet-4-20250514"),
        );
        assert_eq!(session.agent.command, "claude --spec auth-flow");
        assert_eq!(
            session.agent.model.as_deref(),
            Some("claude-sonnet-4-20250514")
        );
        assert_eq!(session.spec_name, "auth-flow");
        assert_eq!(session.worktree_path, PathBuf::from("/tmp/wt/auth"));
        assert_eq!(session.assay_version, env!("CARGO_PKG_VERSION"));
    }

    // ── transition_session ───────────────────────────────────────

    #[test]
    fn transition_happy_path() {
        let mut session =
            create_work_session("test-spec", PathBuf::from("/tmp/wt"), "claude", None);
        transition_session(
            &mut session,
            SessionPhase::AgentRunning,
            "agent_started",
            None,
        )
        .expect("valid transition");

        assert_eq!(session.phase, SessionPhase::AgentRunning);
        assert_eq!(session.transitions.len(), 1);
    }

    #[test]
    fn transition_appends_audit_entry() {
        let mut session =
            create_work_session("test-spec", PathBuf::from("/tmp/wt"), "claude", None);
        transition_session(
            &mut session,
            SessionPhase::AgentRunning,
            "gate_run:abc123",
            Some("first run"),
        )
        .expect("valid transition");

        let entry = &session.transitions[0];
        assert_eq!(entry.from, SessionPhase::Created);
        assert_eq!(entry.to, SessionPhase::AgentRunning);
        assert_eq!(entry.trigger, "gate_run:abc123");
        assert_eq!(entry.notes.as_deref(), Some("first run"));
    }

    #[test]
    fn transition_rejects_invalid() {
        let mut session =
            create_work_session("test-spec", PathBuf::from("/tmp/wt"), "claude", None);
        let original_id = session.id.clone();
        let result = transition_session(&mut session, SessionPhase::Completed, "skip", None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                AssayError::WorkSessionTransition { ref session_id, .. }
                    if session_id == &original_id
            ),
            "expected WorkSessionTransition with matching session_id, got: {err:?}"
        );
        // Session state must not change on rejected transition
        assert_eq!(session.phase, SessionPhase::Created);
        assert!(session.transitions.is_empty());
    }

    #[test]
    fn transition_rejects_from_terminal() {
        let mut session =
            create_work_session("test-spec", PathBuf::from("/tmp/wt"), "claude", None);
        transition_session(
            &mut session,
            SessionPhase::Abandoned,
            "user_abandoned",
            None,
        )
        .expect("transition to Abandoned");

        let result = transition_session(&mut session, SessionPhase::Created, "resurrect", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AssayError::WorkSessionTransition { .. }
        ));
    }

    // ── save / load / list ───────────────────────────────────────

    #[test]
    fn save_and_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let session = create_work_session(
            "round-trip",
            PathBuf::from("/tmp/wt"),
            "claude",
            Some("sonnet"),
        );
        save_session(dir.path(), &session).unwrap();
        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(session, loaded);
    }

    #[test]
    fn save_creates_sessions_directory() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");
        assert!(!sessions_dir.exists());

        let session = create_work_session("dir-create", PathBuf::from("/tmp/wt"), "claude", None);
        save_session(dir.path(), &session).unwrap();
        assert!(sessions_dir.is_dir());
    }

    #[test]
    fn load_not_found() {
        let dir = TempDir::new().unwrap();
        let result = load_session(dir.path(), "01NONEXISTENT0000000000000");
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), AssayError::WorkSessionNotFound { .. }),
            "expected WorkSessionNotFound"
        );
    }

    #[test]
    fn list_empty_directory() {
        let dir = TempDir::new().unwrap();
        let ids = list_sessions(dir.path()).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn list_returns_sorted_ids() {
        let dir = TempDir::new().unwrap();
        let s1 = create_work_session("spec", PathBuf::from("/tmp/1"), "c", None);
        let s2 = create_work_session("spec", PathBuf::from("/tmp/2"), "c", None);
        let s3 = create_work_session("spec", PathBuf::from("/tmp/3"), "c", None);

        // Save in non-sorted order.
        save_session(dir.path(), &s3).unwrap();
        save_session(dir.path(), &s1).unwrap();
        save_session(dir.path(), &s2).unwrap();

        let ids = list_sessions(dir.path()).unwrap();
        assert_eq!(ids.len(), 3);

        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "list_sessions should return sorted IDs");

        // All three IDs present.
        assert!(ids.contains(&s1.id));
        assert!(ids.contains(&s2.id));
        assert!(ids.contains(&s3.id));
    }

    #[test]
    fn save_load_with_gate_runs() {
        let dir = TempDir::new().unwrap();
        let mut session =
            create_work_session("gate-runs", PathBuf::from("/tmp/wt"), "claude", None);
        session.gate_runs = vec!["run-001".to_string(), "run-002".to_string()];

        save_session(dir.path(), &session).unwrap();
        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded.gate_runs, vec!["run-001", "run-002"]);
    }

    #[test]
    fn full_lifecycle() {
        let dir = TempDir::new().unwrap();
        let mut session = create_work_session(
            "lifecycle",
            PathBuf::from("/tmp/wt"),
            "claude",
            Some("opus"),
        );

        // Created -> AgentRunning
        transition_session(
            &mut session,
            SessionPhase::AgentRunning,
            "agent_started",
            None,
        )
        .unwrap();

        // AgentRunning -> GateEvaluated
        transition_session(
            &mut session,
            SessionPhase::GateEvaluated,
            "gate_run:run-001",
            Some("all criteria passed"),
        )
        .unwrap();

        // GateEvaluated -> Completed
        transition_session(&mut session, SessionPhase::Completed, "auto_complete", None).unwrap();

        assert_eq!(session.phase, SessionPhase::Completed);
        // 3 transitions (no birth transition)
        assert_eq!(session.transitions.len(), 3);

        // Persist and reload.
        save_session(dir.path(), &session).unwrap();
        let loaded = load_session(dir.path(), &session.id).unwrap();

        assert_eq!(loaded.phase, SessionPhase::Completed);
        assert_eq!(loaded.transitions.len(), 3);
        assert_eq!(loaded, session);
    }

    #[test]
    fn save_rejects_path_traversal_id() {
        let dir = TempDir::new().unwrap();
        let mut session =
            create_work_session("test-spec", PathBuf::from("/tmp/wt"), "claude", None);
        session.id = "../evil".to_string();
        let result = save_session(dir.path(), &session);
        assert!(result.is_err(), "should reject path-traversal session ID");
    }

    // ── with_session ─────────────────────────────────────────────

    #[test]
    fn with_session_happy_path() {
        let dir = TempDir::new().unwrap();
        let session = create_work_session("test-spec", PathBuf::from("/tmp/wt"), "claude", None);
        save_session(dir.path(), &session).unwrap();

        let updated = with_session(dir.path(), &session.id, |s| {
            transition_session(s, SessionPhase::AgentRunning, "test", None)
        })
        .unwrap();

        assert_eq!(updated.phase, SessionPhase::AgentRunning);

        // Verify on-disk copy matches
        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded.phase, SessionPhase::AgentRunning);
        assert_eq!(loaded, updated);
    }

    #[test]
    fn with_session_aborts_on_closure_error() {
        let dir = TempDir::new().unwrap();
        let session = create_work_session("test-spec", PathBuf::from("/tmp/wt"), "claude", None);
        save_session(dir.path(), &session).unwrap();

        // Attempt invalid transition: Created -> Completed
        let result = with_session(dir.path(), &session.id, |s| {
            transition_session(s, SessionPhase::Completed, "skip", None)
        });

        assert!(result.is_err(), "invalid transition should return error");

        // On-disk session must still be in Created phase
        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(
            loaded.phase,
            SessionPhase::Created,
            "on-disk state should not change when closure fails"
        );
    }

    // ── start_session ────────────────────────────────────────────

    #[test]
    fn start_session_happy_path() {
        let dir = TempDir::new().unwrap();
        let session = start_session(
            dir.path(),
            "my-spec",
            PathBuf::from("/tmp/wt"),
            "claude",
            Some("sonnet"),
        )
        .unwrap();

        assert_eq!(session.phase, SessionPhase::AgentRunning);
        assert_eq!(session.transitions.len(), 1);
        assert_eq!(session.transitions[0].trigger, "session_start");
        assert_eq!(session.spec_name, "my-spec");

        // Verify persisted
        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded, session);
    }

    // ── record_gate_result ───────────────────────────────────────

    #[test]
    fn record_gate_result_happy_path() {
        let dir = TempDir::new().unwrap();
        let session =
            start_session(dir.path(), "spec", PathBuf::from("/tmp/wt"), "claude", None).unwrap();

        let updated = record_gate_result(
            dir.path(),
            &session.id,
            "run-001",
            "gate_eval",
            Some("passed"),
        )
        .unwrap();

        assert_eq!(updated.phase, SessionPhase::GateEvaluated);
        assert_eq!(updated.gate_runs, vec!["run-001"]);

        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded, updated);
    }

    #[test]
    fn record_gate_result_deduplicates() {
        let dir = TempDir::new().unwrap();
        let mut session = create_work_session("spec", PathBuf::from("/tmp/wt"), "claude", None);
        transition_session(&mut session, SessionPhase::AgentRunning, "start", None).unwrap();
        // Pre-populate gate_runs with an existing ID
        session.gate_runs.push("run-001".to_string());
        save_session(dir.path(), &session).unwrap();

        let updated =
            record_gate_result(dir.path(), &session.id, "run-001", "gate_eval", None).unwrap();

        assert_eq!(
            updated.gate_runs,
            vec!["run-001"],
            "duplicate gate_run_id should not be added"
        );
    }

    // ── complete_session ─────────────────────────────────────────

    #[test]
    fn complete_session_full_lifecycle() {
        let dir = TempDir::new().unwrap();
        let session =
            start_session(dir.path(), "spec", PathBuf::from("/tmp/wt"), "claude", None).unwrap();

        let evaluated =
            record_gate_result(dir.path(), &session.id, "run-001", "gate_eval", None).unwrap();

        let completed = complete_session(dir.path(), &evaluated.id, Some("all passed")).unwrap();

        assert_eq!(completed.phase, SessionPhase::Completed);
        let last_transition = completed.transitions.last().unwrap();
        assert_eq!(last_transition.trigger, "session_complete");
        assert_eq!(last_transition.notes.as_deref(), Some("all passed"));
    }

    // ── abandon_session ──────────────────────────────────────────

    #[test]
    fn abandon_session_from_agent_running() {
        let dir = TempDir::new().unwrap();
        let session =
            start_session(dir.path(), "spec", PathBuf::from("/tmp/wt"), "claude", None).unwrap();

        let abandoned = abandon_session(dir.path(), &session.id, "agent crashed").unwrap();

        assert_eq!(abandoned.phase, SessionPhase::Abandoned);
        let last_transition = abandoned.transitions.last().unwrap();
        assert_eq!(last_transition.trigger, "session_abandon");
        assert_eq!(last_transition.notes.as_deref(), Some("agent crashed"));
    }

    #[test]
    fn abandon_session_from_created() {
        let dir = TempDir::new().unwrap();
        let session = create_work_session("spec", PathBuf::from("/tmp/wt"), "claude", None);
        save_session(dir.path(), &session).unwrap();

        let abandoned = abandon_session(dir.path(), &session.id, "stale recovery sweep").unwrap();

        assert_eq!(abandoned.phase, SessionPhase::Abandoned);
        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded, abandoned);
    }

    // ── recovery scan ───────────────────────────────────────────────

    /// Helper: create a session in AgentRunning with a backdated transition timestamp.
    fn create_stale_session(dir: &Path, spec_name: &str, hours_ago: i64) -> WorkSession {
        let mut session = create_work_session(spec_name, PathBuf::from("/tmp/wt"), "claude", None);
        let entered_at = Utc::now() - chrono::Duration::hours(hours_ago);
        session.transitions.push(PhaseTransition {
            from: SessionPhase::Created,
            to: SessionPhase::AgentRunning,
            timestamp: entered_at,
            trigger: "session_start".to_string(),
            notes: None,
        });
        session.phase = SessionPhase::AgentRunning;
        save_session(dir, &session).unwrap();
        session
    }

    #[test]
    fn recover_no_sessions() {
        let dir = TempDir::new().unwrap();
        // Create sessions dir but leave it empty.
        std::fs::create_dir_all(dir.path().join("sessions")).unwrap();

        let summary = recover_stale_sessions(dir.path(), 3600);

        assert_eq!(summary.recovered, 0);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.errors, 0);
    }

    #[test]
    fn recover_no_sessions_dir() {
        let dir = TempDir::new().unwrap();
        // No sessions directory at all — should not panic.
        let summary = recover_stale_sessions(dir.path(), 3600);

        assert_eq!(summary.recovered, 0);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.errors, 0);
    }

    #[test]
    fn recover_stale_session() {
        let dir = TempDir::new().unwrap();
        let session = create_stale_session(dir.path(), "stale-spec", 2);

        let summary = recover_stale_sessions(dir.path(), 3600); // 1h threshold

        assert_eq!(summary.recovered, 1);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.errors, 0);

        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded.phase, SessionPhase::Abandoned);
        let last = loaded.transitions.last().unwrap();
        assert_eq!(last.trigger, "startup_recovery");
        assert!(
            last.notes
                .as_deref()
                .unwrap()
                .contains("Recovered on startup"),
            "recovery note should contain marker text, got: {:?}",
            last.notes
        );
    }

    #[test]
    fn recover_skips_non_agent_running() {
        let dir = TempDir::new().unwrap();

        // Created phase
        let s1 = create_work_session("created", PathBuf::from("/tmp/wt"), "claude", None);
        save_session(dir.path(), &s1).unwrap();

        // Completed phase
        let mut s2 = create_work_session("completed", PathBuf::from("/tmp/wt"), "claude", None);
        transition_session(&mut s2, SessionPhase::AgentRunning, "start", None).unwrap();
        transition_session(&mut s2, SessionPhase::GateEvaluated, "gate", None).unwrap();
        transition_session(&mut s2, SessionPhase::Completed, "done", None).unwrap();
        save_session(dir.path(), &s2).unwrap();

        // Abandoned phase
        let mut s3 = create_work_session("abandoned", PathBuf::from("/tmp/wt"), "claude", None);
        transition_session(&mut s3, SessionPhase::Abandoned, "abandon", None).unwrap();
        save_session(dir.path(), &s3).unwrap();

        // GateEvaluated phase
        let mut s4 = create_work_session("evaluated", PathBuf::from("/tmp/wt"), "claude", None);
        transition_session(&mut s4, SessionPhase::AgentRunning, "start", None).unwrap();
        transition_session(&mut s4, SessionPhase::GateEvaluated, "gate", None).unwrap();
        save_session(dir.path(), &s4).unwrap();

        let summary = recover_stale_sessions(dir.path(), 3600);

        assert_eq!(
            summary.recovered, 0,
            "no AgentRunning sessions should be recovered"
        );
    }

    #[test]
    fn recover_skips_fresh_session() {
        let dir = TempDir::new().unwrap();
        // Create an AgentRunning session with transition timestamp ~now.
        let mut session = create_work_session("fresh", PathBuf::from("/tmp/wt"), "claude", None);
        transition_session(&mut session, SessionPhase::AgentRunning, "start", None).unwrap();
        save_session(dir.path(), &session).unwrap();

        let summary = recover_stale_sessions(dir.path(), 3600); // 1h threshold

        assert_eq!(
            summary.recovered, 0,
            "fresh session should not be recovered"
        );

        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded.phase, SessionPhase::AgentRunning);
    }

    #[test]
    fn recover_corrupt_file() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();

        // Write corrupt JSON.
        std::fs::write(
            sessions_dir.join("00CORRUPT0000000000000000AB.json"),
            "not json",
        )
        .unwrap();

        // Create a valid stale session (will sort after corrupt due to ULID ordering).
        let session = create_stale_session(dir.path(), "valid-stale", 2);

        let summary = recover_stale_sessions(dir.path(), 3600);

        assert_eq!(summary.errors, 1, "corrupt file should be counted as error");
        assert_eq!(
            summary.recovered, 1,
            "valid stale session should still be recovered"
        );

        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded.phase, SessionPhase::Abandoned);
    }

    #[test]
    fn recover_idempotent() {
        let dir = TempDir::new().unwrap();
        create_stale_session(dir.path(), "stale", 2);

        let first = recover_stale_sessions(dir.path(), 3600);
        assert_eq!(first.recovered, 1);

        let second = recover_stale_sessions(dir.path(), 3600);
        assert_eq!(second.recovered, 0, "second run should recover nothing");
    }

    #[test]
    fn recover_missing_transition_record() {
        let dir = TempDir::new().unwrap();
        // Create session with AgentRunning phase but NO transition records.
        let mut session =
            create_work_session("inconsistent", PathBuf::from("/tmp/wt"), "claude", None);
        session.phase = SessionPhase::AgentRunning;
        // transitions is empty — data inconsistency.
        save_session(dir.path(), &session).unwrap();

        let summary = recover_stale_sessions(dir.path(), 3600);

        assert_eq!(
            summary.skipped, 1,
            "missing transition record should be skipped"
        );
        assert_eq!(summary.recovered, 0);

        // Session should remain in AgentRunning (untouched).
        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded.phase, SessionPhase::AgentRunning);
    }

    #[test]
    fn build_recovery_note_format() {
        let note = build_recovery_note(
            chrono::Duration::minutes(192), // 3h 12m
            chrono::Duration::minutes(60),  // 1h
            "testhost",
            12345,
        );

        assert_eq!(
            note,
            "Recovered on startup: stale for 3h 12m (threshold: 1h). Host: testhost, PID: 12345"
        );
    }

    #[test]
    fn build_recovery_note_minutes_only() {
        let note = build_recovery_note(
            chrono::Duration::minutes(45),
            chrono::Duration::minutes(30),
            "host",
            99,
        );

        assert_eq!(
            note,
            "Recovered on startup: stale for 45m (threshold: 30m). Host: host, PID: 99"
        );
        assert!(
            !note.contains("0h"),
            "should not contain '0h' prefix when under 1 hour"
        );
    }

    // ── load_session path validation ─────────────────────────────────

    #[test]
    fn load_session_rejects_path_traversal_id() {
        let dir = TempDir::new().unwrap();
        let result = load_session(dir.path(), "../evil");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("invalid session ID"),
            "should reject via path validation, got: {msg}"
        );
    }

    #[test]
    fn load_session_rejects_slash_in_id() {
        let dir = TempDir::new().unwrap();
        let result = load_session(dir.path(), "foo/bar");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("invalid session ID"),
            "should reject via path validation, got: {msg}"
        );
    }

    // ── list_sessions non-JSON filter ─────────────────────────────────

    #[test]
    fn list_sessions_filters_non_json_files() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();

        // Write some non-JSON files
        std::fs::write(sessions_dir.join("README.txt"), "not json").unwrap();
        std::fs::write(sessions_dir.join("notes"), "also not json").unwrap();
        std::fs::write(sessions_dir.join("backup.json.bak"), "partial backup").unwrap();

        // Also write a valid session
        let session = create_work_session("spec", PathBuf::from("/tmp/wt"), "claude", None);
        save_session(dir.path(), &session).unwrap();

        let ids = list_sessions(dir.path()).unwrap();
        assert_eq!(ids.len(), 1, "only the .json session should appear");
        assert_eq!(ids[0], session.id);
        assert!(!ids.iter().any(|id| id.contains("README")));
        assert!(!ids.iter().any(|id| id.contains("notes")));
        assert!(!ids.iter().any(|id| id.contains(".bak")));
    }

    // ── full_lifecycle per-transition field assertions ─────────────────

    #[test]
    fn full_lifecycle_transition_fields() {
        let dir = TempDir::new().unwrap();
        let mut session = create_work_session(
            "lifecycle-fields",
            PathBuf::from("/tmp/wt"),
            "claude",
            Some("opus"),
        );

        let before_t1 = Utc::now();
        transition_session(
            &mut session,
            SessionPhase::AgentRunning,
            "agent_started",
            None,
        )
        .unwrap();
        let after_t1 = Utc::now();

        let before_t2 = Utc::now();
        transition_session(
            &mut session,
            SessionPhase::GateEvaluated,
            "gate_run:run-001",
            Some("all criteria passed"),
        )
        .unwrap();
        let after_t2 = Utc::now();

        let before_t3 = Utc::now();
        transition_session(&mut session, SessionPhase::Completed, "auto_complete", None).unwrap();
        let after_t3 = Utc::now();

        // Transition 1: Created -> AgentRunning
        let t1 = &session.transitions[0];
        assert_eq!(t1.from, SessionPhase::Created);
        assert_eq!(t1.to, SessionPhase::AgentRunning);
        assert_eq!(t1.trigger, "agent_started");
        assert!(t1.notes.is_none());
        assert!(t1.timestamp >= before_t1 && t1.timestamp <= after_t1);

        // Transition 2: AgentRunning -> GateEvaluated (with notes)
        let t2 = &session.transitions[1];
        assert_eq!(t2.from, SessionPhase::AgentRunning);
        assert_eq!(t2.to, SessionPhase::GateEvaluated);
        assert_eq!(t2.trigger, "gate_run:run-001");
        assert_eq!(t2.notes.as_deref(), Some("all criteria passed"));
        assert!(t2.timestamp >= before_t2 && t2.timestamp <= after_t2);

        // Transition 3: GateEvaluated -> Completed
        let t3 = &session.transitions[2];
        assert_eq!(t3.from, SessionPhase::GateEvaluated);
        assert_eq!(t3.to, SessionPhase::Completed);
        assert_eq!(t3.trigger, "auto_complete");
        assert!(t3.notes.is_none());
        assert!(t3.timestamp >= before_t3 && t3.timestamp <= after_t3);

        // Timestamps are monotonically non-decreasing
        assert!(t1.timestamp <= t2.timestamp);
        assert!(t2.timestamp <= t3.timestamp);

        // Persist and reload — all fields survive round-trip
        save_session(dir.path(), &session).unwrap();
        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(
            loaded.transitions[1].notes.as_deref(),
            Some("all criteria passed")
        );
        assert_eq!(loaded.transitions[2].from, SessionPhase::GateEvaluated);
    }

    // ── convenience function error paths ─────────────────────────────

    #[test]
    fn record_gate_result_errors_on_wrong_phase() {
        let dir = TempDir::new().unwrap();
        // Completed session cannot transition to GateEvaluated
        let mut session = create_work_session("spec", PathBuf::from("/tmp/wt"), "claude", None);
        transition_session(&mut session, SessionPhase::AgentRunning, "start", None).unwrap();
        transition_session(&mut session, SessionPhase::GateEvaluated, "gate", None).unwrap();
        transition_session(&mut session, SessionPhase::Completed, "done", None).unwrap();
        save_session(dir.path(), &session).unwrap();

        let result = record_gate_result(dir.path(), &session.id, "run-001", "gate_eval", None);
        assert!(
            result.is_err(),
            "record_gate_result on completed session should fail"
        );
        assert!(
            matches!(
                result.unwrap_err(),
                AssayError::WorkSessionTransition { .. }
            ),
            "should be a WorkSessionTransition error"
        );

        // On-disk session remains Completed (no partial write)
        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded.phase, SessionPhase::Completed);
    }

    #[test]
    fn complete_session_errors_on_wrong_phase() {
        let dir = TempDir::new().unwrap();
        // AgentRunning cannot transition directly to Completed
        let session =
            start_session(dir.path(), "spec", PathBuf::from("/tmp/wt"), "claude", None).unwrap();

        let result = complete_session(dir.path(), &session.id, None);
        assert!(
            result.is_err(),
            "complete_session from AgentRunning should fail"
        );
        assert!(
            matches!(
                result.unwrap_err(),
                AssayError::WorkSessionTransition { .. }
            ),
            "should be a WorkSessionTransition error"
        );

        // On-disk session remains AgentRunning
        let loaded = load_session(dir.path(), &session.id).unwrap();
        assert_eq!(loaded.phase, SessionPhase::AgentRunning);
    }

    // ── recover_skips_non_agent_running assertion tightening ──────────

    #[test]
    fn recover_skips_non_agent_running_all_assertions() {
        let dir = TempDir::new().unwrap();

        // Created phase
        let s1 = create_work_session("created", PathBuf::from("/tmp/wt"), "claude", None);
        save_session(dir.path(), &s1).unwrap();

        // GateEvaluated phase
        let mut s2 = create_work_session("evaluated", PathBuf::from("/tmp/wt"), "claude", None);
        transition_session(&mut s2, SessionPhase::AgentRunning, "start", None).unwrap();
        transition_session(&mut s2, SessionPhase::GateEvaluated, "gate", None).unwrap();
        save_session(dir.path(), &s2).unwrap();

        let summary = recover_stale_sessions(dir.path(), 3600);

        assert_eq!(summary.recovered, 0, "no sessions should be recovered");
        assert_eq!(
            summary.skipped, 0,
            "non-AgentRunning sessions are bypassed, not skipped"
        );
        assert_eq!(summary.errors, 0, "no errors should occur");
        assert!(!summary.truncated, "scan was not truncated");
    }

    // ── recovery scan cap with truncated flag ────────────────────────

    #[test]
    fn recovery_scan_cap_truncates_at_100() {
        let dir = TempDir::new().unwrap();

        // Create 101 stale sessions
        for i in 0..101 {
            create_stale_session(dir.path(), &format!("spec-{i}"), 2);
        }

        let summary = recover_stale_sessions(dir.path(), 3600);

        assert_eq!(
            summary.recovered, 100,
            "exactly 100 sessions should be recovered"
        );
        assert!(
            summary.truncated,
            "truncated should be true when cap is reached"
        );
    }

    #[test]
    fn recovery_scan_not_truncated_below_cap() {
        let dir = TempDir::new().unwrap();

        // Create only 3 stale sessions — well below the 100-session cap
        for i in 0..3 {
            create_stale_session(dir.path(), &format!("spec-{i}"), 2);
        }

        let summary = recover_stale_sessions(dir.path(), 3600);

        assert_eq!(summary.recovered, 3);
        assert!(
            !summary.truncated,
            "truncated should be false when below cap"
        );
    }

    // ── stale threshold behavior ─────────────────────────────────────

    #[test]
    fn recover_respects_stale_threshold() {
        let dir = TempDir::new().unwrap();

        // Session that is 30 minutes old — fresh relative to 1h threshold
        let mut fresh = create_work_session("fresh", PathBuf::from("/tmp/wt"), "claude", None);
        let fresh_entered = Utc::now() - chrono::Duration::minutes(30);
        fresh.transitions.push(PhaseTransition {
            from: SessionPhase::Created,
            to: SessionPhase::AgentRunning,
            timestamp: fresh_entered,
            trigger: "session_start".to_string(),
            notes: None,
        });
        fresh.phase = SessionPhase::AgentRunning;
        save_session(dir.path(), &fresh).unwrap();

        // Session that is 2 hours old — stale relative to 1h threshold
        let stale = create_stale_session(dir.path(), "stale", 2);

        let summary = recover_stale_sessions(dir.path(), 3600); // 1h = 3600s

        assert_eq!(
            summary.recovered, 1,
            "only the stale session should be recovered"
        );
        assert_eq!(summary.errors, 0);

        // Fresh session untouched
        let loaded_fresh = load_session(dir.path(), &fresh.id).unwrap();
        assert_eq!(loaded_fresh.phase, SessionPhase::AgentRunning);

        // Stale session recovered
        let loaded_stale = load_session(dir.path(), &stale.id).unwrap();
        assert_eq!(loaded_stale.phase, SessionPhase::Abandoned);
    }
}
