//! Contract tests for `StateBackend`, `CapabilitySet`, and `LocalFsBackend`.
//!
//! These tests verify the API surface locked in S01. All `LocalFsBackend`
//! method bodies are stubs in S01 — the tests confirm construction, capability
//! reporting, object-safety, and stub behavior only. Real I/O is tested in S02.

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
    let backend: Box<dyn StateBackend> = Box::new(LocalFsBackend::new(dir.path().to_path_buf()));
    let caps = backend.capabilities();
    assert!(caps.supports_messaging);
}

#[test]
fn test_local_fs_backend_push_session_event_noop() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
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
    let result = backend.push_session_event(dir.path(), &status);
    assert!(result.is_ok());
}

#[test]
fn test_local_fs_backend_read_run_state_returns_none() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    let result = backend.read_run_state(dir.path());
    assert!(matches!(result, Ok(None)));
}
