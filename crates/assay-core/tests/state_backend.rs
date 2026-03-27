//! Contract tests verifying `StateBackend` object safety, `CapabilitySet`
//! constructor invariants, and `LocalFsBackend` trait conformance.

#![cfg(feature = "orchestrate")]

use assay_core::{CapabilitySet, LocalFsBackend, NoopBackend, StateBackend};
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

// ── Backward-compatible deserialization (RunManifest.state_backend) ──

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

// ── LocalFsBackend filesystem persistence ────────────────────────────

/// Verifies that push_session_event persists to disk and read_run_state
/// deserializes it back.
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

    let read_back = backend
        .read_run_state(dir.path())
        .expect("read_run_state should succeed");

    assert!(
        read_back.is_some(),
        "read_run_state should return the status that was just pushed"
    );
    let read_status = read_back.unwrap();
    assert_eq!(read_status.run_id, "integration-run");
    assert_eq!(read_status.phase, OrchestratorPhase::Running);
}

/// Verifies that save_checkpoint_summary writes checkpoints/latest.md to disk.
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

/// Verifies that send_message writes a file and poll_inbox reads it back.
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

    assert!(
        !messages.is_empty(),
        "poll_inbox should return the message that was just sent"
    );
    let (name, contents) = &messages[0];
    assert_eq!(name, "greeting");
    assert_eq!(contents, b"hello world");
}

// ── Serde round-trip tests for new StateBackendConfig variants ───────

#[test]
fn serde_json_round_trip_linear_variant() {
    let config = StateBackendConfig::Linear {
        team_id: "TEAM".into(),
        project_id: Some("PROJ".into()),
    };
    let json = serde_json::to_string(&config).unwrap();
    let back: StateBackendConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back, config);
}

#[test]
fn serde_json_round_trip_github_variant() {
    let config = StateBackendConfig::GitHub {
        repo: "user/repo".into(),
        label: Some("assay".into()),
    };
    let json = serde_json::to_string(&config).unwrap();
    let back: StateBackendConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back, config);
    // Verify the serde rename: must serialize as "github", not "git_hub"
    assert!(
        json.contains("\"github\""),
        "GitHub variant must serialize as 'github', got: {json}"
    );
    assert!(
        !json.contains("\"git_hub\""),
        "GitHub variant must NOT serialize as 'git_hub', got: {json}"
    );
}

#[test]
fn serde_json_round_trip_ssh_variant() {
    let config = StateBackendConfig::Ssh {
        host: "server.example.com".into(),
        remote_assay_dir: "/home/user/.assay".into(),
        user: Some("deploy".into()),
        port: Some(2222),
    };
    let json = serde_json::to_string(&config).unwrap();
    let back: StateBackendConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back, config);
}

#[test]
fn serde_json_round_trip_linear_variant_minimal() {
    // Without optional fields
    let config = StateBackendConfig::Linear {
        team_id: "TEAM".into(),
        project_id: None,
    };
    let json = serde_json::to_string(&config).unwrap();
    let back: StateBackendConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back, config);
    // project_id should be absent (skip_serializing_if)
    assert!(!json.contains("project_id"));
}

#[test]
fn serde_json_round_trip_ssh_variant_minimal() {
    // Without optional fields
    let config = StateBackendConfig::Ssh {
        host: "server.example.com".into(),
        remote_assay_dir: "/home/user/.assay".into(),
        user: None,
        port: None,
    };
    let json = serde_json::to_string(&config).unwrap();
    let back: StateBackendConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back, config);
}

#[test]
fn toml_round_trip_manifest_with_linear_backend() {
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
        state_backend: Some(StateBackendConfig::Linear {
            team_id: "TEAM".into(),
            project_id: Some("PROJ".into()),
        }),
    };
    let toml_out = toml::to_string(&manifest).unwrap();
    let back: RunManifest = toml::from_str(&toml_out).unwrap();
    assert_eq!(back.state_backend, manifest.state_backend);
}

/// Verifies that `NoopBackend` has all capabilities disabled.
#[test]
fn test_noop_backend_capabilities_all_false() {
    let backend = NoopBackend;
    let caps = backend.capabilities();
    assert!(!caps.supports_messaging);
    assert!(!caps.supports_gossip_manifest);
    assert!(!caps.supports_annotations);
    assert!(!caps.supports_checkpoints);
}

/// Verifies that `NoopBackend` can be used as a trait object via `Arc<dyn StateBackend>`.
#[test]
fn test_noop_backend_as_trait_object() {
    use std::sync::Arc;
    let backend: Arc<dyn StateBackend> = Arc::new(NoopBackend);
    let caps = backend.capabilities();
    assert!(
        !caps.supports_messaging,
        "NoopBackend should have no capabilities"
    );
}

/// Verifies that `NoopBackend` methods all return Ok without panicking.
#[test]
fn test_noop_backend_all_methods_succeed() {
    use std::path::PathBuf;
    let backend = NoopBackend;
    let dummy_dir = PathBuf::from("/tmp/nonexistent");

    let status = OrchestratorStatus {
        run_id: "test-noop".to_string(),
        phase: OrchestratorPhase::Running,
        failure_policy: FailurePolicy::SkipDependents,
        sessions: vec![],
        started_at: chrono::Utc::now(),
        completed_at: None,
        mesh_status: None,
        gossip_status: None,
    };

    assert!(backend.push_session_event(&dummy_dir, &status).is_ok());
    assert!(backend.read_run_state(&dummy_dir).unwrap().is_none());
    assert!(backend.send_message(&dummy_dir, "test", b"data").is_ok());
    assert!(backend.poll_inbox(&dummy_dir).unwrap().is_empty());
    assert!(backend.annotate_run(&dummy_dir, "manifest/path").is_ok());

    let checkpoint = TeamCheckpoint {
        version: 1,
        session_id: "noop-sess".to_string(),
        project: "/tmp/test".to_string(),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        trigger: "test".to_string(),
        agents: vec![],
        tasks: vec![],
        context_health: None,
    };
    assert!(
        backend
            .save_checkpoint_summary(&dummy_dir, &checkpoint)
            .is_ok()
    );
}
