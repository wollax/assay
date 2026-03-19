---
id: T02
parent: S02
milestone: M004
provides:
  - crates/assay-core/tests/mesh_integration.rs with 2 failing integration tests
key_files:
  - crates/assay-core/tests/mesh_integration.rs
key_decisions:
  - Writer runner discovers its outbox path by parsing the "mesh-roster" prompt layer (looking for "Outbox: <path>" line) — this requires T03 to produce roster layers in exactly this format
  - First assertion in each test is `assert!(state_path.exists(), ...)` — fails cleanly against the stub since the stub writes no state.json; subsequent assertions (mesh_status, messages_routed, Completed state) are unreachable until T03 ships
  - Tests use bare tempdir (no git init) — only creates .assay/orchestrator/ subdirectory
patterns_established:
  - make_mesh_manifest(names: &[(&str, &str)]) helper builds Mesh-mode manifest with no depends_on
  - setup_temp_dir() creates a non-git temp dir with .assay/orchestrator/ structure (no git ops needed for Mesh)
  - success_result(session) builds a minimal PipelineResult::Success for mock runners
observability_surfaces:
  - "cargo test -p assay-core --features orchestrate -- mesh -- --nocapture" shows which assertion failed (state.json missing vs mesh_status None vs messages_routed==0 vs member.state!=Completed)
duration: ~15 minutes
verification_result: passed (tests compile, fail as expected)
completed_at: 2026-03-18
blocker_discovered: false
---

# T02: Write failing integration test for mesh mode

**Created `crates/assay-core/tests/mesh_integration.rs` with two integration tests that compile cleanly and fail against the stub with assertion errors.**

## What Happened

Created the integration test file with two tests:

1. **`test_mesh_mode_message_routing`** — 2 sessions ("writer", "reader"). The writer runner parses its mesh-roster prompt layer for an "Outbox: <path>" line, creates `outbox/reader/msg.txt`, then sleeps 200ms. The reader runner sleeps 300ms. After `run_mesh()`, asserts: state.json exists, `mesh_status.is_some()`, `messages_routed >= 1`, and the reader's inbox dir contains exactly 1 file.

2. **`test_mesh_mode_completed_not_dead`** — 2 sessions ("alpha", "beta"), both runners return `Ok` immediately. After `run_mesh()`, asserts: state.json exists, `mesh_status.is_some()`, 2 members present, all members have `state == MeshMemberState::Completed`.

Both tests fail against the current stub because the stub: (a) never calls `session_runner`, (b) never creates `state.json`, (c) never populates `mesh_status`. The first failing assertion in each test is the state.json existence check.

The `make_pipeline_config` pattern from `orchestrate_integration.rs` was followed. No git repo is needed — the helper only creates `.assay/orchestrator/` under a tempdir.

## Verification

```
cargo build -p assay-core --features orchestrate --tests
# → Finished (0 errors)

cargo test -p assay-core --features orchestrate -- mesh
# → 2 FAILED (test_mesh_mode_message_routing, test_mesh_mode_completed_not_dead)
# Failure: "state.json must exist at ... — stub does not write it"

cargo test -p assay-core --features orchestrate
# → 768 passed; 0 failed (existing tests); 2 failed (new mesh tests only)
```

## Diagnostics

Run `cargo test -p assay-core --features orchestrate -- mesh -- --nocapture` to see the full assertion message including the expected state.json path. After T03 implements `run_mesh()`, the failure will shift to whichever assertion the implementation doesn't yet satisfy.

## Deviations

- The task plan's "simplest approach" for writer-outbox discovery was followed exactly: parse "Outbox: <path>" from the mesh-roster prompt layer. T03 must produce roster layers with this exact line format.

## Known Issues

None — tests behave exactly as intended.

## Files Created/Modified

- `crates/assay-core/tests/mesh_integration.rs` — new file; 2 failing mesh integration tests
