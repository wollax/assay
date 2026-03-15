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

    let json = serde_json::to_string_pretty(session)
        .map_err(|e| AssayError::json("serializing work session", &sessions_dir, e))?;

    let mut tmpfile = NamedTempFile::new_in(&sessions_dir)
        .map_err(|e| AssayError::io("creating temp file for session", &sessions_dir, e))?;

    tmpfile
        .write_all(json.as_bytes())
        .map_err(|e| AssayError::io("writing work session", &sessions_dir, e))?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|e| AssayError::io("syncing work session", &sessions_dir, e))?;

    let final_path = sessions_dir.join(format!("{}.json", session.id));
    tmpfile
        .persist(&final_path)
        .map_err(|e| AssayError::io("persisting work session", &final_path, e.error))?;

    Ok(final_path)
}

/// Load a work session by ID from `.assay/sessions/<session_id>.json`.
///
/// Returns [`AssayError::WorkSessionNotFound`] if the file does not exist.
pub fn load_session(assay_dir: &Path, session_id: &str) -> Result<WorkSession> {
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
                eprintln!("Warning: skipping session entry: {e}");
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
}
