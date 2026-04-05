//! Integration tests for pipeline span instrumentation.
//!
//! These tests use `tracing-test` to capture subscriber output and assert
//! that expected span names appear when pipeline functions are called.
//! Each test triggers an early `SpecLoad` failure to exercise the span
//! entry path without requiring a real git repository or agent binary.

use std::path::PathBuf;

use assay_core::pipeline::{self, PipelineConfig};
use assay_types::{ManifestSession, RunManifest};

/// Helper: build a `PipelineConfig` pointing at a non-existent specs dir
/// so that `setup_session` fails at `SpecLoad` — but only after entering
/// the function-level span.
fn config_with_missing_specs() -> PipelineConfig {
    PipelineConfig {
        project_root: PathBuf::from("/tmp/assay-span-test"),
        assay_dir: PathBuf::from("/tmp/assay-span-test/.assay"),
        specs_dir: PathBuf::from("/tmp/assay-span-test/nonexistent-specs"),
        worktree_base: PathBuf::from("/tmp/assay-span-test/worktrees"),
        timeout_secs: 10,
        base_branch: None,
    }
}

/// Helper: build a `ManifestSession` referencing a spec that does not exist.
fn session_for_missing_spec() -> ManifestSession {
    ManifestSession {
        spec: "nonexistent-spec".into(),
        name: None,
        settings: None,
        hooks: vec![],
        prompt_layers: vec![],
        file_scope: vec![],
        shared_files: vec![],
        depends_on: vec![],
    }
}

/// A no-op provider — never reached because `setup_session` fails first.
use assay_types::NullProvider;

// ── Function-level span tests ────────────────────────────────────────

#[tracing_test::traced_test]
#[test]
fn test_setup_session_emits_span() {
    let config = config_with_missing_specs();
    let session = session_for_missing_spec();

    // Will fail at SpecLoad, but the function-level span should be entered.
    let _ = pipeline::setup_session(&session, &config);

    assert!(logs_contain("pipeline::setup_session"));
}

#[tracing_test::traced_test]
#[test]
fn test_run_session_emits_span() {
    let config = config_with_missing_specs();
    let session = session_for_missing_spec();

    let _ = pipeline::run_session(&session, &config, &NullProvider);

    assert!(logs_contain("pipeline::run_session"));
}

#[tracing_test::traced_test]
#[test]
fn test_run_manifest_emits_span() {
    let manifest = RunManifest {
        sessions: vec![session_for_missing_spec()],
        #[cfg(feature = "orchestrate")]
        mode: Default::default(),
        #[cfg(feature = "orchestrate")]
        mesh_config: None,
        #[cfg(feature = "orchestrate")]
        gossip_config: None,
        #[cfg(feature = "orchestrate")]
        state_backend: None,
    };
    let config = config_with_missing_specs();

    let _ = pipeline::run_manifest(&manifest, &config, &NullProvider);

    assert!(logs_contain("pipeline::run_manifest"));
}

// ── Stage-level span tests ───────────────────────────────────────────

#[tracing_test::traced_test]
#[test]
fn test_setup_session_emits_spec_load_span() {
    let config = config_with_missing_specs();
    let session = session_for_missing_spec();

    // SpecLoad is the first stage inside setup_session — even on failure
    // the span should be entered before the error is returned.
    let _ = pipeline::setup_session(&session, &config);

    assert!(logs_contain("spec_load"));
}
