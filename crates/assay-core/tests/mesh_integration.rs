//! Integration tests for the Mesh mode executor.
//!
//! These tests define the observable behavior of `run_mesh()` at the
//! filesystem level. Both tests **compile cleanly but fail** against the
//! current stub — the stub neither calls session runners nor writes `state.json`
//! nor populates `mesh_status`. Once T03 replaces the stub with a real
//! implementation, both tests should pass.
//!
//! No git repo is required — Mesh mode has no git operations.

#![cfg(feature = "orchestrate")]

use std::path::Path;
use std::time::Duration;

use assay_core::orchestrate::executor::OrchestratorConfig;
use assay_core::orchestrate::mesh::run_mesh;
use assay_core::pipeline::{PipelineConfig, PipelineError, PipelineResult};
use assay_types::orchestrate::{MeshMemberState, OrchestratorStatus};
use assay_types::{ManifestSession, OrchestratorMode, RunManifest};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Create a bare temp dir (no git repo needed for Mesh mode).
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

/// Build a `RunManifest` for Mesh mode from a list of (spec, name) pairs.
///
/// No `depends_on` — Mesh mode doesn't use DAG edges.
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

// ── Test 1: Message routing ───────────────────────────────────────────────────

/// Proves that the real `run_mesh()` implementation:
/// (a) injects a mesh-roster `PromptLayer` into each session (so the runner
///     can discover its outbox path),
/// (b) creates inbox/outbox directory trees before invoking the runner,
/// (c) runs a routing thread that moves files from `outbox/<target>/` to
///     the target's inbox,
/// (d) persists `mesh_status.messages_routed >= 1` to `state.json`.
///
/// The writer runner discovers its outbox dir by parsing the "mesh-roster"
/// prompt layer injected by the executor. The layer content contains a line
/// of the form `Outbox: <absolute-path>`. The runner writes
/// `<outbox>/reader/msg.txt` and then returns. The routing thread is expected
/// to move this file to the reader's inbox before `run_mesh()` returns.
#[test]
fn test_mesh_mode_message_routing() {
    let dir = setup_temp_dir();
    let tmp = dir.path();
    let pipeline_config = make_pipeline_config(tmp);

    let manifest = make_mesh_manifest(&[("writer-spec", "writer"), ("reader-spec", "reader")]);

    let config = OrchestratorConfig::default();

    // Writer runner: parse the mesh-roster layer to find its outbox path,
    // then write a message file targeting the "reader" session.
    // Reader runner: sleep briefly to give the routing thread a chance to
    // process the message before the run completes.
    let runner = |session: &ManifestSession, _config: &PipelineConfig| {
        let name = session.name.as_deref().unwrap_or(&session.spec);

        if name == "writer" {
            // Locate the mesh-roster prompt layer injected by the executor.
            let roster_layer = session
                .prompt_layers
                .iter()
                .find(|l| l.name == "mesh-roster");

            if let Some(layer) = roster_layer {
                // Parse the outbox path from "Outbox: <path>" line.
                if let Some(line) = layer.content.lines().find(|l| l.starts_with("Outbox: ")) {
                    let outbox_path = std::path::PathBuf::from(line.trim_start_matches("Outbox: "));
                    let target_dir = outbox_path.join("reader");
                    std::fs::create_dir_all(&target_dir).unwrap();
                    std::fs::write(target_dir.join("msg.txt"), b"hello from writer").unwrap();
                }
            }
            // Give the routing thread time to pick up the message.
            std::thread::sleep(Duration::from_millis(200));
        } else {
            // Reader: just wait a bit so the routing thread can run.
            std::thread::sleep(Duration::from_millis(300));
        }

        Ok(success_result(session))
    };

    let result = run_mesh(&manifest, &config, &pipeline_config, &runner).unwrap();

    // Assertion 1: state.json must exist under the run directory.
    let state_path = pipeline_config
        .assay_dir
        .join("orchestrator")
        .join(&result.run_id)
        .join("state.json");
    assert!(
        state_path.exists(),
        "state.json must exist at {state_path:?} — stub does not write it"
    );

    let raw = std::fs::read_to_string(&state_path).unwrap();
    let status: OrchestratorStatus = serde_json::from_str(&raw).unwrap();

    // Assertion 2: mesh_status must be populated.
    assert!(
        status.mesh_status.is_some(),
        "mesh_status should be Some — stub returns None"
    );

    let mesh = status.mesh_status.unwrap();

    // Assertion 3: at least one message was routed.
    assert!(
        mesh.messages_routed >= 1,
        "messages_routed should be >= 1, got {} — stub routes nothing",
        mesh.messages_routed
    );

    // Assertion 4: the reader's inbox must contain the routed file.
    let reader_inbox = pipeline_config
        .assay_dir
        .join("orchestrator")
        .join(&result.run_id)
        .join("mesh")
        .join("reader")
        .join("inbox");
    let inbox_entries: Vec<_> = std::fs::read_dir(&reader_inbox)
        .unwrap_or_else(|_| panic!("reader inbox dir should exist at {reader_inbox:?}"))
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        inbox_entries.len(),
        1,
        "reader inbox should contain exactly 1 file, found {} — routing thread didn't move the message",
        inbox_entries.len()
    );
}

// ── Test 2: Completed-not-dead membership classification ─────────────────────

/// Proves that sessions which exit normally (runner returns `Ok`) are
/// classified as `MeshMemberState::Completed` — not `Dead` — in the
/// persisted `mesh_status.members` list.
///
/// No file routing is involved — this purely tests membership state tracking.
#[test]
fn test_mesh_mode_completed_not_dead() {
    let dir = setup_temp_dir();
    let tmp = dir.path();
    let pipeline_config = make_pipeline_config(tmp);

    let manifest = make_mesh_manifest(&[("spec-alpha", "alpha"), ("spec-beta", "beta")]);

    let config = OrchestratorConfig::default();

    // Both runners return Ok immediately — no git, no file I/O.
    let runner = |session: &ManifestSession, _config: &PipelineConfig| {
        Ok::<PipelineResult, PipelineError>(success_result(session))
    };

    let result = run_mesh(&manifest, &config, &pipeline_config, &runner).unwrap();

    // Assertion 1: state.json must exist.
    let state_path = pipeline_config
        .assay_dir
        .join("orchestrator")
        .join(&result.run_id)
        .join("state.json");
    assert!(
        state_path.exists(),
        "state.json must exist at {state_path:?} — stub does not write it"
    );

    let raw = std::fs::read_to_string(&state_path).unwrap();
    let status: OrchestratorStatus = serde_json::from_str(&raw).unwrap();

    // Assertion 2: mesh_status must be populated.
    assert!(
        status.mesh_status.is_some(),
        "mesh_status should be Some — stub returns None"
    );

    let mesh = status.mesh_status.unwrap();

    // Assertion 3: both sessions should appear in the members list.
    assert_eq!(
        mesh.members.len(),
        2,
        "expected 2 mesh members, got {}",
        mesh.members.len()
    );

    // Assertion 4: all members must be Completed (not Dead).
    for member in &mesh.members {
        assert_eq!(
            member.state,
            MeshMemberState::Completed,
            "session '{}' should be Completed, got {:?} — stub never calls runners so no completion sentinel is written",
            member.name,
            member.state
        );
    }
}
