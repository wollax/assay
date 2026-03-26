//! Contract tests verifying `StateBackend` object safety, `CapabilitySet`
//! constructor invariants, and `LocalFsBackend` trait conformance.

#![cfg(feature = "orchestrate")]

use assay_core::{CapabilitySet, LocalFsBackend, StateBackend};
use assay_types::{FailurePolicy, OrchestratorPhase, OrchestratorStatus};
use tempfile::tempdir;

#[test]
fn test_capability_set_all() {
    let caps = CapabilitySet::all();
    assert!(caps.supports_messaging);
    assert!(caps.supports_gossip_manifest);
    assert!(caps.supports_annotations);
    assert!(caps.supports_checkpoints);
}

#[test]
fn test_capability_set_none() {
    let caps = CapabilitySet::none();
    assert!(!caps.supports_messaging);
    assert!(!caps.supports_gossip_manifest);
    assert!(!caps.supports_annotations);
    assert!(!caps.supports_checkpoints);
}

#[test]
fn test_local_fs_backend_capabilities_all_true() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    let caps = backend.capabilities();
    assert!(caps.supports_messaging);
    assert!(caps.supports_gossip_manifest);
    assert!(caps.supports_annotations);
    assert!(caps.supports_checkpoints);
}

#[test]
fn test_local_fs_backend_as_trait_object() {
    let dir = tempdir().unwrap();
    // Arc (not Box) matches the ownership model used in OrchestratorConfig,
    // which derives Clone. Box<dyn StateBackend> is not Clone; Arc<dyn StateBackend> is.
    let backend: std::sync::Arc<dyn StateBackend> =
        std::sync::Arc::new(LocalFsBackend::new(dir.path().to_path_buf()));
    let caps = backend.capabilities();
    assert!(caps.supports_messaging);
}

#[test]
fn test_local_fs_backend_push_session_event_noop() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    // Explicit field construction — OrchestratorStatus does not implement Default.
    let status = OrchestratorStatus {
        run_id: "test-run".to_string(),
        phase: OrchestratorPhase::Running,
        failure_policy: FailurePolicy::SkipDependents,
        sessions: vec![],
        started_at: chrono::Utc::now(),
        completed_at: None,
        mesh_status: None,
        gossip_status: None,
    };
    assert!(backend.push_session_event(dir.path(), &status).is_ok());
}

#[test]
fn test_local_fs_backend_read_run_state_returns_none() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    assert!(matches!(backend.read_run_state(dir.path()), Ok(None)));
}

#[test]
fn test_local_fs_backend_send_message_noop() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    assert!(
        backend
            .send_message(dir.path(), "test-msg", b"hello")
            .is_ok()
    );
}

#[test]
fn test_local_fs_backend_poll_inbox_returns_empty() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    let result = backend.poll_inbox(dir.path());
    assert!(matches!(result, Ok(v) if v.is_empty()));
}

#[test]
fn test_local_fs_backend_annotate_run_noop() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    assert!(backend.annotate_run(dir.path(), "knowledge.json").is_ok());
}

#[test]
fn test_local_fs_backend_save_checkpoint_summary_noop() {
    use assay_types::TeamCheckpoint;
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    let checkpoint = TeamCheckpoint {
        version: 1,
        session_id: "sess-1".to_string(),
        project: dir.path().display().to_string(),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        trigger: "test".to_string(),
        agents: vec![],
        tasks: vec![],
        context_health: None,
    };
    assert!(
        backend
            .save_checkpoint_summary(dir.path(), &checkpoint)
            .is_ok()
    );
}

#[test]
fn test_local_fs_backend_assay_dir_accessor() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    assert_eq!(backend.assay_dir(), dir.path());
}
