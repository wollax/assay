//! Contract tests verifying `StateBackend` object safety, `CapabilitySet`
//! constructor invariants, and `LocalFsBackend` trait conformance.

#![cfg(feature = "orchestrate")]

use assay_core::{CapabilitySet, LocalFsBackend, StateBackend};
use assay_types::{
    FailurePolicy, OrchestratorPhase, OrchestratorStatus, StateBackendConfig, TeamCheckpoint,
};
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

// ── Backward-compat round-trip tests (T01/S02) ──────────────────────

#[test]
fn backward_compat_manifest_without_state_backend_deserializes_to_none() {
    use assay_types::RunManifest;
    let toml_str = r#"
[[sessions]]
spec = "auth"
"#;
    let manifest: RunManifest = toml::from_str(toml_str).unwrap();
    assert!(
        manifest.state_backend.is_none(),
        "manifest without state_backend should deserialize to None"
    );
}

#[test]
fn backward_compat_manifest_with_state_backend_round_trips() {
    use assay_types::RunManifest;
    let manifest = RunManifest {
        sessions: vec![assay_types::ManifestSession {
            spec: "auth".to_string(),
            name: None,
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
        }],
        mode: Default::default(),
        mesh_config: None,
        gossip_config: None,
        state_backend: Some(StateBackendConfig::LocalFs),
    };
    let toml_out = toml::to_string(&manifest).unwrap();
    let back: RunManifest = toml::from_str(&toml_out).unwrap();
    assert_eq!(
        back.state_backend,
        Some(StateBackendConfig::LocalFs),
        "state_backend should survive TOML round-trip"
    );
}

// ── Integration tests for LocalFsBackend real method bodies (T01/S02) ─

/// Red-state test: push_session_event should write state, read_run_state should
/// return it. Currently stubs — T02 makes this green.
#[test]
fn test_local_fs_backend_push_and_read_state() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    let status = OrchestratorStatus {
        run_id: "integration-run".to_string(),
        phase: OrchestratorPhase::Running,
        failure_policy: FailurePolicy::SkipDependents,
        sessions: vec![],
        started_at: chrono::Utc::now(),
        completed_at: None,
        mesh_status: None,
        gossip_status: None,
    };

    // Push state
    backend
        .push_session_event(dir.path(), &status)
        .expect("push_session_event should succeed");

    // Read it back — once implemented, this should return Some(status)
    let read_back = backend
        .read_run_state(dir.path())
        .expect("read_run_state should succeed");

    // T02 will make this assertion pass by implementing real persistence.
    // For now the stub returns None, so this test is expected to fail.
    assert!(
        read_back.is_some(),
        "read_run_state should return the status that was just pushed"
    );
    let read_status = read_back.unwrap();
    assert_eq!(read_status.run_id, "integration-run");
    assert_eq!(read_status.phase, OrchestratorPhase::Running);
}

/// Red-state test: save_checkpoint_summary should write a checkpoint file to disk.
/// Currently a stub — T02 makes this green.
#[test]
fn test_local_fs_backend_save_checkpoint_summary() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    let checkpoint = TeamCheckpoint {
        version: 1,
        session_id: "sess-checkpoint".to_string(),
        project: dir.path().display().to_string(),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        trigger: "gate-pass".to_string(),
        agents: vec![],
        tasks: vec![],
        context_health: None,
    };

    backend
        .save_checkpoint_summary(dir.path(), &checkpoint)
        .expect("save_checkpoint_summary should succeed");

    // save_checkpoint writes to checkpoints/latest.md (via checkpoint::persistence).
    let checkpoint_path = dir.path().join("checkpoints").join("latest.md");
    assert!(
        checkpoint_path.exists(),
        "save_checkpoint_summary should create checkpoints/latest.md at {:?}",
        checkpoint_path
    );
}

/// Red-state test: send_message then poll_inbox should return the message.
/// Currently stubs — T02 makes this green.
#[test]
fn test_local_fs_backend_send_and_poll_messages() {
    let dir = tempdir().unwrap();
    let backend = LocalFsBackend::new(dir.path().to_path_buf());
    let inbox_path = dir.path().join("inbox");

    backend
        .send_message(&inbox_path, "greeting", b"hello world")
        .expect("send_message should succeed");

    let messages = backend
        .poll_inbox(&inbox_path)
        .expect("poll_inbox should succeed");

    // T02 will make this assertion pass by implementing real messaging.
    assert!(
        !messages.is_empty(),
        "poll_inbox should return the message that was just sent"
    );
    let (name, contents) = &messages[0];
    assert_eq!(name, "greeting");
    assert_eq!(contents, b"hello world");
}
