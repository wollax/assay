//! Integration tests for the Gossip mode executor.
//!
//! These tests define the observable behavior of `run_gossip()` at the
//! filesystem level. Both tests **compile cleanly but fail** against the
//! current stub — the stub neither calls session runners nor writes
//! `knowledge.json` nor populates `gossip_status`. Once T03 replaces the stub
//! with a real implementation, both tests should pass.
//!
//! No git repo is required — Gossip mode has no git operations.

#![cfg(feature = "orchestrate")]

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use assay_core::NoopBackend;
use assay_core::orchestrate::executor::OrchestratorConfig;
use assay_core::orchestrate::gossip::run_gossip;
use assay_core::pipeline::{PipelineConfig, PipelineError, PipelineResult};
use assay_types::orchestrate::{GossipStatus, KnowledgeManifest, OrchestratorStatus};
use assay_types::{ManifestSession, OrchestratorMode, RunManifest};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Create a bare temp dir with the orchestrator subdirectory pre-created.
fn setup_temp_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let assay_dir = dir.path().join(".assay");
    std::fs::create_dir_all(assay_dir.join("orchestrator")).unwrap();
    dir
}

/// Build a `PipelineConfig` pointing at the temp dir.
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

/// Build a `RunManifest` for Gossip mode from a list of (spec, name) pairs.
///
/// No `depends_on` — Gossip mode doesn't use DAG edges.
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

// ── Test 1: Knowledge manifest written with all session entries ───────────────

/// Proves that the real `run_gossip()` implementation:
/// (a) invokes all session runners,
/// (b) synthesizes their results into `gossip/knowledge.json`,
/// (c) persists `gossip_status.sessions_synthesized == 3` to `state.json`,
/// (d) sets `gossip_status.knowledge_manifest_path` pointing at `knowledge.json`.
///
/// The test uses 3 mock sessions ("alpha", "beta", "gamma") that each sleep
/// briefly to simulate real work. After `run_gossip()` returns, we verify
/// both the on-disk manifest and the persisted status.
#[test]
fn test_gossip_mode_knowledge_manifest() {
    let dir = setup_temp_dir();
    let tmp = dir.path();
    let pipeline_config = make_pipeline_config(tmp);

    let manifest = make_gossip_manifest(&[
        ("alpha-spec", "alpha"),
        ("beta-spec", "beta"),
        ("gamma-spec", "gamma"),
    ]);

    let config = OrchestratorConfig::default();

    // All runners succeed after a brief pause to simulate real work.
    let runner = |session: &ManifestSession, _config: &PipelineConfig| {
        std::thread::sleep(Duration::from_millis(50));
        Ok::<PipelineResult, PipelineError>(success_result(session))
    };

    let result = run_gossip(&manifest, &config, &pipeline_config, &runner).unwrap();

    // Assertion 1: knowledge.json must exist under the run's gossip directory.
    let knowledge_path = pipeline_config
        .assay_dir
        .join("orchestrator")
        .join(&result.run_id)
        .join("gossip")
        .join("knowledge.json");
    assert!(
        knowledge_path.exists(),
        "knowledge.json must exist at {knowledge_path:?} — stub does not write it"
    );

    // Assertion 2: knowledge.json must deserialize as KnowledgeManifest.
    let raw = std::fs::read_to_string(&knowledge_path)
        .unwrap_or_else(|e| panic!("failed to read knowledge.json at {knowledge_path:?}: {e}"));
    let manifest_data: KnowledgeManifest = serde_json::from_str(&raw).unwrap_or_else(|e| {
        panic!("failed to deserialize KnowledgeManifest from {knowledge_path:?}: {e}\nraw: {raw}")
    });

    // Assertion 3: all 3 sessions must appear as entries.
    assert_eq!(
        manifest_data.entries.len(),
        3,
        "expected 3 entries in knowledge.json, got {} — stub synthesizes nothing\nentries: {:?}",
        manifest_data.entries.len(),
        manifest_data
            .entries
            .iter()
            .map(|e| &e.session_name)
            .collect::<Vec<_>>()
    );

    // Assertion 4: each expected session name appears in the entries.
    for expected_name in &["alpha", "beta", "gamma"] {
        let found = manifest_data
            .entries
            .iter()
            .any(|e| e.session_name == *expected_name);
        assert!(
            found,
            "session '{expected_name}' missing from knowledge.json entries — stub writes no entries\nactual entries: {:?}",
            manifest_data
                .entries
                .iter()
                .map(|e| &e.session_name)
                .collect::<Vec<_>>()
        );
    }

    // Assertion 5: state.json must exist.
    let state_path = pipeline_config
        .assay_dir
        .join("orchestrator")
        .join(&result.run_id)
        .join("state.json");
    assert!(
        state_path.exists(),
        "state.json must exist at {state_path:?} — stub does not write it"
    );

    let state_raw = std::fs::read_to_string(&state_path)
        .unwrap_or_else(|e| panic!("failed to read state.json at {state_path:?}: {e}"));
    let status: OrchestratorStatus = serde_json::from_str(&state_raw).unwrap_or_else(|e| {
        panic!(
            "failed to deserialize OrchestratorStatus from {state_path:?}: {e}\nraw: {state_raw}"
        )
    });

    // Assertion 6: gossip_status must be populated.
    assert!(
        status.gossip_status.is_some(),
        "gossip_status should be Some in state.json at {state_path:?} — stub returns None"
    );

    let gossip: GossipStatus = status.gossip_status.unwrap();

    // Assertion 7: sessions_synthesized must equal 3.
    assert_eq!(
        gossip.sessions_synthesized, 3,
        "expected sessions_synthesized == 3, got {} — stub synthesizes nothing",
        gossip.sessions_synthesized
    );

    // Assertion 8: knowledge_manifest_path must end with gossip/knowledge.json.
    assert!(
        gossip
            .knowledge_manifest_path
            .ends_with("gossip/knowledge.json"),
        "knowledge_manifest_path should end with 'gossip/knowledge.json', got {:?}",
        gossip.knowledge_manifest_path
    );
}

// ── Test 2: Gossip knowledge manifest path injected into prompt layers ────────

/// Proves that the real `run_gossip()` implementation injects a
/// `"gossip-knowledge-manifest"` `PromptLayer` into each session before
/// invoking its runner.
///
/// The layer content must contain a line of the form
/// `"Knowledge manifest: <absolute-path>"` where the path is under the run's
/// assay directory. This lets the running agent locate the manifest for
/// cross-session knowledge sharing.
///
/// The test uses 2 mock sessions ("s1", "s2"). Each runner asserts:
/// - a `PromptLayer` named `"gossip-knowledge-manifest"` is present
/// - its content contains a `"Knowledge manifest: "` line
/// - the path after the prefix is under the assay directory
///
/// The shared `Arc<Mutex<Option<String>>>` captures the assay-dir path for
/// cross-thread verification.
#[test]
fn test_gossip_mode_manifest_path_in_prompt_layer() {
    let dir = setup_temp_dir();
    let tmp = dir.path();
    let pipeline_config = make_pipeline_config(tmp);

    let manifest = make_gossip_manifest(&[("spec-s1", "s1"), ("spec-s2", "s2")]);

    let config = OrchestratorConfig::default();

    // Capture the assay_dir path for use inside runners.
    let assay_dir = pipeline_config.assay_dir.clone();
    // Track any prompt-layer assertion failure messages from inside the runner.
    let layer_errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let layer_errors_runner = Arc::clone(&layer_errors);
    // Count runner invocations so we can assert all sessions were dispatched.
    let runner_call_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let runner_call_count_inner = Arc::clone(&runner_call_count);

    let runner = move |session: &ManifestSession, _config: &PipelineConfig| {
        *runner_call_count_inner.lock().unwrap() += 1;
        let name = session.name.as_deref().unwrap_or(&session.spec);

        // Assertion A: the gossip-knowledge-manifest prompt layer must be present.
        let gossip_layer = session
            .prompt_layers
            .iter()
            .find(|l| l.name == "gossip-knowledge-manifest");

        if gossip_layer.is_none() {
            layer_errors_runner.lock().unwrap().push(format!(
                "session '{}': no 'gossip-knowledge-manifest' PromptLayer found. \
                 Layers present: {:?}",
                name,
                session
                    .prompt_layers
                    .iter()
                    .map(|l| &l.name)
                    .collect::<Vec<_>>()
            ));
            return Ok(success_result(session));
        }

        let layer = gossip_layer.unwrap();

        // Assertion B: content must contain a "Knowledge manifest: <path>" line.
        let manifest_line = layer
            .content
            .lines()
            .find(|l| l.starts_with("Knowledge manifest: "));

        if manifest_line.is_none() {
            layer_errors_runner.lock().unwrap().push(format!(
                "session '{}': 'gossip-knowledge-manifest' layer has no \
                 'Knowledge manifest: ' line.\nLayer content:\n{}",
                name, layer.content
            ));
            return Ok(success_result(session));
        }

        let manifest_path_str = manifest_line
            .unwrap()
            .trim_start_matches("Knowledge manifest: ");

        // Assertion C: the extracted path must be under the assay directory.
        let manifest_path = std::path::Path::new(manifest_path_str);
        if !manifest_path.starts_with(&assay_dir) {
            layer_errors_runner.lock().unwrap().push(format!(
                "session '{}': manifest path '{manifest_path_str}' is not under \
                 assay_dir '{assay_dir:?}'",
                name
            ));
        }

        Ok(success_result(session))
    };

    let result = run_gossip(&manifest, &config, &pipeline_config, &runner);

    // Assertion 1: runner must have been called for every session.
    let calls = *runner_call_count.lock().unwrap();
    assert_eq!(
        calls, 2,
        "expected runner to be called 2 times (once per session), got {calls} — stub never calls runners"
    );

    // Assertion 2: surface any runner-internal prompt-layer failures.
    let errors = layer_errors.lock().unwrap();
    assert!(
        errors.is_empty(),
        "prompt-layer assertion failures inside runner:\n{}",
        errors.join("\n---\n")
    );

    assert!(
        result.is_ok(),
        "run_gossip returned Err: {:?}",
        result.err()
    );

    // Final check: assay_dir capture still valid (temp dir not dropped early).
    let _ = result.unwrap();
}

// ── Test 3: Capability degradation with NoopBackend ──────────────────────────

/// Proves that `run_gossip()` degrades gracefully when the backend has no
/// gossip-manifest capability.
///
/// With `NoopBackend` (all capabilities disabled), the gossip executor:
/// - Omits the `"gossip-knowledge-manifest"` PromptLayer from all sessions
/// - Skips all three `persist_knowledge_manifest` callsites (initial, per-completion, final flush)
/// - Still runs all sessions to completion (Ok result)
/// - All sessions present in outcomes
/// - No error or panic from the absent manifest support
#[test]
fn test_gossip_degrades_gracefully_without_manifest() {
    let dir = setup_temp_dir();
    let tmp = dir.path();
    let pipeline_config = make_pipeline_config(tmp);

    let manifest = make_gossip_manifest(&[("spec-alpha", "alpha"), ("spec-beta", "beta")]);

    let config = OrchestratorConfig {
        backend: std::sync::Arc::new(NoopBackend),
        ..OrchestratorConfig::default()
    };

    // Track whether any session receives a gossip-knowledge-manifest PromptLayer.
    let got_gossip_layer: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let got_gossip_layer_runner = Arc::clone(&got_gossip_layer);

    let runner = move |session: &ManifestSession, _config: &PipelineConfig| {
        let name = session.name.as_deref().unwrap_or(&session.spec);

        // Check if the gossip-knowledge-manifest layer was injected.
        let has_gossip_layer = session
            .prompt_layers
            .iter()
            .any(|l| l.name == "gossip-knowledge-manifest");

        if has_gossip_layer {
            got_gossip_layer_runner
                .lock()
                .unwrap()
                .push(name.to_string());
        }

        Ok::<PipelineResult, PipelineError>(success_result(session))
    };

    let result = run_gossip(&manifest, &config, &pipeline_config, &runner);

    // The run should complete without error.
    assert!(
        result.is_ok(),
        "run_gossip with NoopBackend should succeed, got: {:?}",
        result.err()
    );

    let result = result.unwrap();

    // No session should have received the gossip-knowledge-manifest layer
    // when the backend doesn't support gossip manifest.
    let sessions_with_layer = got_gossip_layer.lock().unwrap();
    assert!(
        sessions_with_layer.is_empty(),
        "no session should receive 'gossip-knowledge-manifest' PromptLayer \
         when gossip_manifest capability is disabled, but these did: {:?}",
        *sessions_with_layer
    );

    // All sessions should be present in the outcomes.
    assert_eq!(
        result.outcomes.len(),
        2,
        "expected 2 session outcomes, got {}",
        result.outcomes.len()
    );

    // The run_id should be a valid non-empty string.
    assert!(
        !result.run_id.is_empty(),
        "run_id should be set even with NoopBackend"
    );
}
