---
id: T02
parent: S03
milestone: M004
provides:
  - crates/assay-core/tests/gossip_integration.rs with 2 failing integration tests that define the observable contract for run_gossip()
key_files:
  - crates/assay-core/tests/gossip_integration.rs
key_decisions:
  - Added runner_call_count Arc<Mutex<usize>> to test_gossip_mode_manifest_path_in_prompt_layer to ensure it fails against the stub (which never calls runners), rather than passing vacuously when layer_errors is empty
patterns_established:
  - Gossip integration tests mirror mesh_integration.rs: setup_temp_dir + make_pipeline_config + make_gossip_manifest + success_result helpers; post-run assertions on knowledge.json and state.json
  - When stub never calls runners, runner-internal assertions are vacuous — add a post-run runner_call_count assertion as the primary failure gate
observability_surfaces:
  - Run `cargo test -p assay-core --features orchestrate --test gossip_integration -- --nocapture` to see failure details with paths
duration: ~10m
verification_result: passed
completed_at: 2026-03-18
blocker_discovered: false
---

# T02: Write failing gossip integration tests

**Created `crates/assay-core/tests/gossip_integration.rs` with 2 integration tests that define the observable contract for `run_gossip()` — both compile cleanly and fail against the current stub.**

## What Happened

Created `crates/assay-core/tests/gossip_integration.rs` following the `mesh_integration.rs` pattern. The file includes:

- `#![cfg(feature = "orchestrate")]` gate
- Four helpers: `setup_temp_dir()`, `make_pipeline_config()`, `make_gossip_manifest()`, `success_result()`
- `test_gossip_mode_knowledge_manifest`: 3 mock sessions ("alpha", "beta", "gamma"); asserts `gossip/knowledge.json` exists, deserializes as `KnowledgeManifest` with 3 entries, and `state.json` has `gossip_status.sessions_synthesized == 3` with a path ending `gossip/knowledge.json`
- `test_gossip_mode_manifest_path_in_prompt_layer`: 2 mock sessions ("s1", "s2"); runner captures call count and checks for a `"gossip-knowledge-manifest"` `PromptLayer` with a `"Knowledge manifest: "` line under the assay dir; post-run asserts runner was called exactly 2 times

One deviation was needed: the second test initially passed vacuously because the stub never calls runners (so `layer_errors` stayed empty). Fixed by adding a `runner_call_count` counter and asserting it equals 2 after `run_gossip()` returns — this is the primary failure gate against the stub.

## Verification

```
cargo test -p assay-core --features orchestrate --test gossip_integration 2>&1
# → running 2 tests
# → test test_gossip_mode_knowledge_manifest ... FAILED
# → test test_gossip_mode_manifest_path_in_prompt_layer ... FAILED
# → test result: FAILED. 0 passed; 2 failed

cargo test -p assay-core --features orchestrate --test gossip_integration 2>&1 | grep 'error\[E'
# → (empty — no compile errors)
```

Failure messages:
- `test_gossip_mode_knowledge_manifest`: "knowledge.json must exist at `<path>` — stub does not write it"
- `test_gossip_mode_manifest_path_in_prompt_layer`: "expected runner to be called 2 times, got 0 — stub never calls runners"

## Diagnostics

Run with `--nocapture` to see the full expected paths:
```bash
cargo test -p assay-core --features orchestrate --test gossip_integration -- --nocapture
```

The `test_gossip_mode_knowledge_manifest` failure message includes the full expected path to `knowledge.json` so T03 implementer knows exactly what to produce.

## Deviations

- `test_gossip_mode_manifest_path_in_prompt_layer`: Added `Arc<Mutex<usize>>` runner call counter + post-run `assert_eq!(calls, 2, ...)` to ensure the test fails against the stub rather than passing vacuously. The task plan's approach of runner-internal `layer_errors` is insufficient when the stub never calls runners at all.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/tests/gossip_integration.rs` — new file with 2 failing integration tests defining the `run_gossip()` observable contract
