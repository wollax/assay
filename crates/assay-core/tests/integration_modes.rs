//! All-modes regression suite: DAG + Mesh + Gossip in a single test file.
//!
//! Proves the S04 milestone criterion: "all three modes have end-to-end coverage."
//! Each test uses a simple 2-session manifest with mock session runners and
//! verifies that:
//!   - all sessions are executed (outcomes.len() == 2),
//!   - both sessions complete successfully,
//!   - mode-specific side effects (state.json, knowledge.json) are written.

#![cfg(feature = "orchestrate")]

use std::path::Path;
use std::process::Command;

use assay_core::orchestrate::executor::{OrchestratorConfig, SessionOutcome, run_orchestrated};
use assay_core::orchestrate::gossip::run_gossip;
use assay_core::orchestrate::mesh::run_mesh;
use assay_core::pipeline::{PipelineConfig, PipelineError, PipelineResult};
use assay_types::orchestrate::KnowledgeManifest;
use assay_types::{ManifestSession, OrchestratorMode, RunManifest};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Create a temp dir with `.assay/orchestrator` pre-created (no git repo).
/// Suitable for Mesh and Gossip tests.
fn setup_temp_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let assay_dir = dir.path().join(".assay");
    std::fs::create_dir_all(assay_dir.join("orchestrator")).unwrap();
    dir
}

/// Create a temp git repo with an initial commit on `main` and `.assay` dir.
/// Required for DAG tests (even with mock runners, `run_orchestrated` creates
/// worktree paths that the executor stores in outcomes).
fn setup_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();

    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(p)
        .output()
        .unwrap();

    let assay_dir = p.join(".assay");
    std::fs::create_dir_all(assay_dir.join("specs")).unwrap();
    std::fs::create_dir_all(assay_dir.join("orchestrator")).unwrap();
    std::fs::write(
        assay_dir.join("config.toml"),
        r#"project_name = "integration-modes""#,
    )
    .unwrap();
    std::fs::write(assay_dir.join(".gitignore"), "orchestrator/\n").unwrap();

    std::fs::write(p.join("readme.md"), "# integration modes test\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(p)
        .output()
        .unwrap();

    dir
}

/// Build a `PipelineConfig` pointing at the given directory.
fn make_pipeline_config(tmp: &Path) -> PipelineConfig {
    PipelineConfig {
        project_root: tmp.to_path_buf(),
        assay_dir: tmp.join(".assay"),
        specs_dir: tmp.join(".assay/specs"),
        worktree_base: tmp.to_path_buf(),
        timeout_secs: 60,
        base_branch: None,
    }
}

/// Build a DAG manifest (OrchestratorMode::Dag) with no dependencies.
fn make_dag_manifest(names: &[(&str, &str)]) -> RunManifest {
    RunManifest {
        sessions: names
            .iter()
            .map(|(spec, name)| ManifestSession {
                spec: spec.to_string(),
                name: Some(name.to_string()),
                settings: None,
                hooks: vec![],
                prompt_layers: vec![],
                file_scope: vec![],
                shared_files: vec![],
                depends_on: vec![],
            })
            .collect(),
        mode: OrchestratorMode::Dag,
        mesh_config: None,
        gossip_config: None,
        state_backend: None,
    }
}

/// Build a Mesh manifest (OrchestratorMode::Mesh) with no dependencies.
fn make_mesh_manifest(names: &[(&str, &str)]) -> RunManifest {
    RunManifest {
        sessions: names
            .iter()
            .map(|(spec, name)| ManifestSession {
                spec: spec.to_string(),
                name: Some(name.to_string()),
                settings: None,
                hooks: vec![],
                prompt_layers: vec![],
                file_scope: vec![],
                shared_files: vec![],
                depends_on: vec![],
            })
            .collect(),
        mode: OrchestratorMode::Mesh,
        mesh_config: None,
        gossip_config: None,
        state_backend: None,
    }
}

/// Build a Gossip manifest (OrchestratorMode::Gossip) with no dependencies.
fn make_gossip_manifest(names: &[(&str, &str)]) -> RunManifest {
    RunManifest {
        sessions: names
            .iter()
            .map(|(spec, name)| ManifestSession {
                spec: spec.to_string(),
                name: Some(name.to_string()),
                settings: None,
                hooks: vec![],
                prompt_layers: vec![],
                file_scope: vec![],
                shared_files: vec![],
                depends_on: vec![],
            })
            .collect(),
        mode: OrchestratorMode::Gossip,
        mesh_config: None,
        gossip_config: None,
        state_backend: None,
    }
}

/// Build a simple success `PipelineResult` for a session.
fn success_result(session: &ManifestSession) -> PipelineResult {
    let name = session.name.clone().unwrap_or_else(|| session.spec.clone());
    PipelineResult {
        session_id: format!("sess-{name}"),
        spec_name: session.spec.clone(),
        gate_summary: None,
        merge_check: None,
        stage_timings: vec![],
        outcome: assay_core::pipeline::PipelineOutcome::Success,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// DAG mode: 2-session manifest with no dependencies, both complete successfully.
///
/// Proves that `run_orchestrated()` dispatches all sessions and populates
/// outcomes when mode is `OrchestratorMode::Dag`.
#[test]
fn test_all_modes_dag_executes_two_sessions() {
    let dir = setup_git_repo();
    let tmp = dir.path();
    let pipeline_config = make_pipeline_config(tmp);

    let manifest = make_dag_manifest(&[("spec-alpha", "alpha"), ("spec-beta", "beta")]);
    let config = OrchestratorConfig::default();

    let runner = |session: &ManifestSession, _cfg: &PipelineConfig| {
        Ok::<PipelineResult, PipelineError>(success_result(session))
    };

    let result = run_orchestrated(&manifest, config, &pipeline_config, &runner).unwrap();

    assert_eq!(
        result.outcomes.len(),
        2,
        "DAG: expected 2 outcomes, got {}",
        result.outcomes.len()
    );

    for (name, outcome) in &result.outcomes {
        assert!(
            matches!(outcome, SessionOutcome::Completed { .. }),
            "DAG: session '{name}' should be Completed, got {outcome:?}"
        );
    }
}

/// Mesh mode: 2-session manifest, both complete successfully, state.json written.
///
/// Proves that `run_mesh()` dispatches all sessions and persists state.json
/// under `.assay/orchestrator/<run_id>/state.json`.
#[test]
fn test_all_modes_mesh_executes_two_sessions() {
    let dir = setup_temp_dir();
    let tmp = dir.path();
    let pipeline_config = make_pipeline_config(tmp);

    let manifest = make_mesh_manifest(&[("spec-alpha", "alpha"), ("spec-beta", "beta")]);
    let config = OrchestratorConfig::default();

    let runner = |session: &ManifestSession, _cfg: &PipelineConfig| {
        Ok::<PipelineResult, PipelineError>(success_result(session))
    };

    let result = run_mesh(&manifest, &config, &pipeline_config, &runner).unwrap();

    assert_eq!(
        result.outcomes.len(),
        2,
        "Mesh: expected 2 outcomes, got {}",
        result.outcomes.len()
    );

    for (name, outcome) in &result.outcomes {
        assert!(
            matches!(outcome, SessionOutcome::Completed { .. }),
            "Mesh: session '{name}' should be Completed, got {outcome:?}"
        );
    }

    // state.json must have been written by run_mesh()
    let state_path = pipeline_config
        .assay_dir
        .join("orchestrator")
        .join(&result.run_id)
        .join("state.json");
    assert!(
        state_path.exists(),
        "Mesh: state.json should exist at {state_path:?}"
    );
}

/// Gossip mode: 2-session manifest, both complete successfully,
/// knowledge.json has 2 entries.
///
/// Proves that `run_gossip()` dispatches all sessions, synthesizes their
/// results into `gossip/knowledge.json`, and that the manifest contains
/// exactly as many entries as there were sessions.
#[test]
fn test_all_modes_gossip_executes_two_sessions() {
    let dir = setup_temp_dir();
    let tmp = dir.path();
    let pipeline_config = make_pipeline_config(tmp);

    let manifest = make_gossip_manifest(&[("spec-alpha", "alpha"), ("spec-beta", "beta")]);
    let config = OrchestratorConfig::default();

    let runner = |session: &ManifestSession, _cfg: &PipelineConfig| {
        Ok::<PipelineResult, PipelineError>(success_result(session))
    };

    let result = run_gossip(&manifest, &config, &pipeline_config, &runner).unwrap();

    assert_eq!(
        result.outcomes.len(),
        2,
        "Gossip: expected 2 outcomes, got {}",
        result.outcomes.len()
    );

    for (name, outcome) in &result.outcomes {
        assert!(
            matches!(outcome, SessionOutcome::Completed { .. }),
            "Gossip: session '{name}' should be Completed, got {outcome:?}"
        );
    }

    // knowledge.json must have 2 entries
    let knowledge_path = pipeline_config
        .assay_dir
        .join("orchestrator")
        .join(&result.run_id)
        .join("gossip")
        .join("knowledge.json");
    assert!(
        knowledge_path.exists(),
        "Gossip: knowledge.json should exist at {knowledge_path:?}"
    );

    let knowledge_bytes = std::fs::read(&knowledge_path).unwrap();
    let knowledge: KnowledgeManifest = serde_json::from_slice(&knowledge_bytes).unwrap();
    assert_eq!(
        knowledge.entries.len(),
        2,
        "Gossip: knowledge.json should have 2 entries, got {}",
        knowledge.entries.len()
    );
}
