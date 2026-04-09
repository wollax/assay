//! Integration tests for orchestration span instrumentation.
//!
//! These tests define the orchestration span contract by asserting expected
//! span names appear in captured `tracing-test` subscriber output.
//! Each test uses mock session runners (instant success closures) with minimal
//! manifests — no real git repos or agent processes needed.
//!
//! Span assertions use a `{` suffix (e.g. `"orchestrate::dag{"`) to match only
//! named spans. Without it, `logs_contain("orchestrate::mesh")` would match
//! module paths in tracing-test output, producing false positives (D137).

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
            user_prompt: None,
            prompt_file: None,
        })
        .collect();
    RunManifest {
        sessions,
        mode: Default::default(),
        mesh_config: None,
        gossip_config: None,
        state_backend: None,
    }
}

/// Build a `PipelineConfig` backed by a tempdir. Creates `.assay` so
/// orchestrators can write state files without pre-flight failures.
fn mock_pipeline_config() -> (tempfile::TempDir, PipelineConfig) {
    let dir = tempfile::tempdir().expect("failed to create tempdir for mock_pipeline_config");
    let p = dir.path().to_path_buf();

    std::fs::create_dir_all(p.join(".assay")).expect("failed to create .assay dir in mock tempdir");

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

    // Empty vec triggers early return (no git operations, no sessions to merge),
    // but the root span should still be entered and emitted.
    let result = merge_completed_sessions(vec![], &config, |_, _, _, _| {
        unreachable!("no sessions to merge")
    });
    assert!(result.is_ok(), "empty merge should succeed: {result:?}");

    assert!(logs_contain("merge::run{"));
}
