---
id: T01
parent: S04
milestone: M004
provides:
  - Real HarnessWriter session runner wired into execute_mesh() and execute_gossip()
  - Outcomes from orch_result populated into OrchestrationResponse (not hardcoded zeros)
  - stderr mode labels: "mode: mesh — N session(s)" / "mode: gossip — N session(s)"
  - Two guard unit tests for Mesh/Gossip mode enum preservation
key_files:
  - crates/assay-cli/src/commands/run.rs
key_decisions:
  - Followed execute_orchestrated() template verbatim for session runner closure and outcome-iteration loop; dropped 3-phase checkout+merge logic (mesh/gossip don't merge)
  - Exit code 1 on any failure or skip (matches DAG path behavior)
patterns_established:
  - session_runner closure construction (HarnessWriter via assay_harness::claude) is now shared across all three mode paths (DAG, Mesh, Gossip)
observability_surfaces:
  - "stderr: mode: mesh — N session(s) / mode: gossip — N session(s) on entry"
  - "stderr per-session: [✓]/[✗]/[−] lines (same format as DAG)"
  - "JSON: --json flag produces OrchestrationResponse with .sessions array populated from outcomes"
  - "jq: assay run manifest.toml --json | jq '.sessions | length' — non-zero for executed sessions"
duration: ~15 min
verification_result: passed
completed_at: 2026-03-18
blocker_discovered: false
---

# T01: Rewrite execute_mesh() and execute_gossip() CLI Stubs with Real Session Runners

**Replaced `unreachable!()` stubs in `execute_mesh()` and `execute_gossip()` with the real HarnessWriter session runner closure and outcome-population loop from `execute_orchestrated()`.**

## What Happened

Both `execute_mesh()` and `execute_gossip()` previously contained `unreachable!()` as their session runner, guaranteeing a panic on any real mesh/gossip manifest. The fix was straightforward: copy the HarnessWriter session runner closure from `execute_orchestrated()` (unchanged), call `run_mesh()` / `run_gossip()` with it, then iterate `orch_result.outcomes` to populate `OrchestrationResponse.sessions` and `OrchestrationSummary` with real counts.

The 3-phase checkout+merge logic from `execute_orchestrated()` was intentionally omitted — mesh/gossip don't merge worktrees back, so `merge_report` remains the empty sentinel value.

Two lightweight guard tests (`execute_mesh_output_mode_label`, `execute_gossip_output_mode_label`) were added to verify the mode enum is preserved through `RunManifest` construction.

## Verification

- `cargo test -p assay-cli` — 32/32 pass (including 2 new tests)
- `cargo clippy -p assay-cli --all-targets -- -D warnings` — 0 warnings, exits 0
- `grep -c "unreachable" crates/assay-cli/src/commands/run.rs` — returns 0

## Diagnostics

- `assay run manifest.toml 2>&1 | grep "mode:"` — confirms mode label printed ("mode: mesh — N session(s)" or "mode: gossip — N session(s)")
- `assay run manifest.toml --json | jq '.sessions | length'` — non-zero after real sessions execute
- `assay run manifest.toml --json | jq '.sessions[] | select(.outcome=="failed") | .error'` — failure messages surfaced in JSON

## Deviations

None. The task plan was followed exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/commands/run.rs` — `execute_mesh()` and `execute_gossip()` rewritten with real runner; 2 guard tests added
