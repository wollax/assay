//! Integration tests for orchestration span instrumentation.
//!
//! These tests define the orchestration span contract by asserting expected
//! span names appear in captured `tracing-test` subscriber output.
//! Each test uses mock session runners (instant success closures) with minimal
//! manifests — no real git repos or agent processes needed.
//!
//! All tests should compile and FAIL initially, proving the assertions are
//! real before any instrumentation code is written.

#![cfg(feature = "orchestrate")]

use assay_core::orchestrate::executor::{OrchestratorConfig, run_orchestrated};
use assay_core::orchestrate::gossip::run_gossip;
use assay_core::orchestrate::merge_runner::{MergeRunnerConfig, merge_completed_sessions};
use assay_core::orchestrate::mesh::run_mesh;
use assay_core::pipeline::{PipelineConfig, PipelineError, PipelineOutcome, PipelineResult};
use assay_types::{ManifestSession, MergeStrategy, RunManifest};
use std::path::PathBuf;

// ── Helpers ──────────────────────────────────────────────────────────

/// Build a `RunManifest` with `n` independent sessions (no `depends_on`).
fn mock_manifest(n: usize) -> RunManifest {
    let sessions: Vec<ManifestSession> = (0..n)
        .map(|i| ManifestSession {
            spec: format!("spec-{i}"),
            name: Some(format!("session-{i}")),
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
        })
        .collect();
    RunManifest {
        sessions,
        #[cfg(feature = "orchestrate")]
        mode: Default::default(),
        #[cfg(feature = "orchestrate")]
        mesh_config: None,
        #[cfg(feature = "orchestrate")]
        gossip_config: None,
    }
}

/// Build a `PipelineConfig` with tempdir paths (dirs are created by the
/// orchestrators, so we just need plausible paths).
fn mock_pipeline_config() -> (tempfile::TempDir, PipelineConfig) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().to_path_buf();

    // Create .assay directory so orchestrators can write state
    std::fs::create_dir_all(p.join(".assay")).unwrap();

    let config = PipelineConfig {
        project_root: p.clone(),
        assay_dir: p.join(".assay"),
        specs_dir: p.join(".assay/specs"),
        worktree_base: p.clone(),
        timeout_secs: 10,
        base_branch: Some("main".to_string()),
    };
    (dir, config)
}

/// An instant session runner that returns `Ok(PipelineResult)` with minimal
/// fields — no real git repos or agent processes involved.
fn instant_runner(
    session: &ManifestSession,
    _config: &PipelineConfig,
) -> Result<PipelineResult, PipelineError> {
    Ok(PipelineResult {
        session_id: format!("test-{}", session.spec),
        spec_name: session.spec.clone(),
        gate_summary: None,
        merge_check: None,
        stage_timings: vec![],
        outcome: PipelineOutcome::Success,
    })
}

// ── Orchestration root span tests ────────────────────────────────────

#[tracing_test::traced_test]
#[test]
fn test_dag_root_span_emitted() {
    let manifest = mock_manifest(2);
    let (_dir, config) = mock_pipeline_config();
    let orch_config = OrchestratorConfig::default();

    let _ = run_orchestrated(&manifest, orch_config, &config, &instant_runner);

    // Span name must appear with field braces — distinguishes from module paths.
    assert!(logs_contain("orchestrate::dag{"));
}

#[tracing_test::traced_test]
#[test]
fn test_dag_session_span_emitted() {
    let manifest = mock_manifest(2);
    let (_dir, config) = mock_pipeline_config();
    let orch_config = OrchestratorConfig::default();

    let _ = run_orchestrated(&manifest, orch_config, &config, &instant_runner);

    assert!(logs_contain("orchestrate::dag::session{"));
}

#[tracing_test::traced_test]
#[test]
fn test_mesh_root_span_emitted() {
    let manifest = mock_manifest(2);
    let (_dir, config) = mock_pipeline_config();
    let orch_config = OrchestratorConfig::default();

    let _ = run_mesh(&manifest, &orch_config, &config, &instant_runner);

    // Use `{` suffix to match only named spans, not module paths like
    // `assay_core::orchestrate::mesh:`.
    assert!(logs_contain("orchestrate::mesh{"));
}

#[tracing_test::traced_test]
#[test]
fn test_gossip_root_span_emitted() {
    let manifest = mock_manifest(2);
    let (_dir, config) = mock_pipeline_config();
    let orch_config = OrchestratorConfig::default();

    let _ = run_gossip(&manifest, &orch_config, &config, &instant_runner);

    assert!(logs_contain("orchestrate::gossip{"));
}

#[tracing_test::traced_test]
#[test]
fn test_merge_root_span_emitted() {
    let config = MergeRunnerConfig {
        strategy: MergeStrategy::CompletionTime,
        project_root: PathBuf::from("/tmp/assay-span-test-nonexistent"),
        base_branch: "main".to_string(),
        conflict_resolution_enabled: false,
    };

    // Empty vec → merge runner returns immediately (no sessions to merge),
    // but the root span should still be entered.
    let _ = merge_completed_sessions(vec![], &config, |_, _, _, _| {
        unreachable!("no sessions to merge")
    });

    assert!(logs_contain("merge::run{"));
}
