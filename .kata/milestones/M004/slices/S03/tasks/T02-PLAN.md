---
estimated_steps: 7
estimated_files: 1
---

# T02: Write failing gossip integration tests

**Slice:** S03 — Gossip Mode
**Milestone:** M004

## Description

Create `crates/assay-core/tests/gossip_integration.rs` with two integration tests that define the observable contract for `run_gossip()`. Both tests must compile cleanly against the T01 types but fail against the current stub (which calls no runners and writes no files). Writing tests first gives T03 an unambiguous implementation target and proves the assertions are substantive (not vacuously passing).

## Steps

1. Create `crates/assay-core/tests/gossip_integration.rs` with `#![cfg(feature = "orchestrate")]` gate and the following imports: `std::path::Path`, `std::time::Duration`, `assay_core::orchestrate::executor::OrchestratorConfig`, `assay_core::orchestrate::gossip::run_gossip`, `assay_core::pipeline::{PipelineConfig, PipelineError, PipelineResult}`, `assay_types::orchestrate::{GossipStatus, KnowledgeManifest, OrchestratorStatus}`, `assay_types::{ManifestSession, OrchestratorMode, RunManifest}`.

2. Add helper functions matching the `mesh_integration.rs` pattern:
   - `setup_temp_dir() -> tempfile::TempDir` — creates bare temp dir with `.assay/orchestrator/`
   - `make_pipeline_config(tmp: &Path) -> PipelineConfig`
   - `make_gossip_manifest(names: &[(&str, &str)]) -> RunManifest` — `mode: OrchestratorMode::Gossip`, no `depends_on`
   - `success_result(session: &ManifestSession) -> PipelineResult`

3. Write `test_gossip_mode_knowledge_manifest`:
   - 3 mock sessions with names "alpha", "beta", "gamma"
   - Runner: all return `Ok(success_result(session))` after a brief `thread::sleep(Duration::from_millis(50))`
   - After `run_gossip()` returns, read `result.run_id`
   - Assert `knowledge.json` exists at `<assay_dir>/orchestrator/<run_id>/gossip/knowledge.json`
   - Deserialize as `KnowledgeManifest` (assert no deserialization error)
   - Assert `manifest.entries.len() == 3`
   - Assert all session names ("alpha", "beta", "gamma") appear in `manifest.entries` (by `session_name` field)
   - Read `state.json`, deserialize as `OrchestratorStatus`
   - Assert `status.gossip_status.is_some()`
   - Assert `status.gossip_status.unwrap().sessions_synthesized == 3`
   - Assert `status.gossip_status.unwrap().knowledge_manifest_path` ends with `gossip/knowledge.json`

4. Write `test_gossip_mode_manifest_path_in_prompt_layer`:
   - 2 mock sessions with names "s1", "s2"
   - Runner: for each session, locate a `PromptLayer` named `"gossip-knowledge-manifest"`; assert layer exists; find a line in `layer.content` starting with `"Knowledge manifest: "`; parse the path after the prefix; assert the path contains the run directory segment; return `Ok(success_result(session))`
   - Note: to access `run_id` inside the runner, capture a shared `Arc<Mutex<Option<String>>>` and set it on first call; or alternatively capture just the `assay_dir` path and verify the extracted path is under it
   - After `run_gossip()` returns: assert result is `Ok`

5. Run `cargo test -p assay-core --features orchestrate --test gossip_integration` and confirm:
   - Both tests compile without errors
   - Both tests fail (not panic/crash) — expected failures are assertions about `knowledge.json` existence or `gossip_status.is_some()` failing against the stub

6. Adjust any assertion messages to be maximally diagnostic (include file paths, actual vs expected values) so T03 implementer can immediately see what's missing.

7. Verify no feature flags are missing by checking `Cargo.toml` for `assay-core` — confirm the test integration file will be compiled under `--features orchestrate` (it should be, since `mesh_integration.rs` works the same way; no extra Cargo config needed).

## Must-Haves

- [ ] `gossip_integration.rs` created with `#![cfg(feature = "orchestrate")]` gate
- [ ] Both test functions compile without errors
- [ ] `test_gossip_mode_knowledge_manifest` fails against the stub (knowledge.json does not exist)
- [ ] `test_gossip_mode_manifest_path_in_prompt_layer` fails against the stub (no gossip-knowledge-manifest PromptLayer injected)
- [ ] Assertion messages are diagnostic (include expected path, actual value)

## Verification

- `cargo test -p assay-core --features orchestrate --test gossip_integration 2>&1 | grep -c "FAILED"` returns 2
- `cargo test -p assay-core --features orchestrate --test gossip_integration 2>&1 | grep "error\[E"` returns empty (no compile errors)

## Observability Impact

- Signals added/changed: None (tests only)
- How a future agent inspects this: Run `cargo test -p assay-core --features orchestrate --test gossip_integration -- --nocapture` to see failure details with paths
- Failure state exposed: Assertion messages will name the missing `knowledge.json` path so the T03 implementer knows exactly what to produce

## Inputs

- `crates/assay-core/tests/mesh_integration.rs` — helper structure and test pattern to mirror
- `crates/assay-types/src/orchestrate.rs` — `KnowledgeManifest`, `GossipStatus`, `OrchestratorStatus` types (from T01)
- S03-RESEARCH.md — integration test plan section with exact assertion list

## Expected Output

- `crates/assay-core/tests/gossip_integration.rs` — new file with 2 failing integration tests that compile cleanly
