---
id: T02
parent: S04
milestone: M002
provides:
  - test_collect_after_merge_commit in collector.rs — verifies collect() handles --no-ff merge commits correctly
  - test_job_phase_gates_failed_serde in monitor.rs — verifies GatesFailed round-trips through TOML serde
key_files:
  - crates/smelt-core/src/collector.rs
  - crates/smelt-core/src/monitor.rs
key_decisions:
  - toml::to_string cannot serialize a bare enum value (TOML requires a top-level key-value structure); wrapped JobPhase in a local Wrapper struct for the serde round-trip test
patterns_established:
  - Use a local Wrapper struct in serde tests when the subject is a bare enum value and toml is the serialization target
observability_surfaces:
  - test_job_phase_gates_failed_serde makes "gates_failed" serialization an explicit tested contract — future enum changes break this test before silently corrupting run-state.toml
  - test_collect_after_merge_commit surfaces collect() failures on merge-commit scenarios with --nocapture git output
duration: short
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Add merge-commit and serde unit tests to collector.rs and monitor.rs

**Added two unit tests — `test_collect_after_merge_commit` and `test_job_phase_gates_failed_serde` — bringing smelt-core to 112 total passing tests with no regressions.**

## What Happened

Added both tests as specified in the task plan with one minor deviation in the serde test (see Deviations).

**`test_collect_after_merge_commit`** in `collector.rs`: creates a feature branch with one commit, merges it back to the default branch with `git merge --no-ff`, then calls `ResultCollector::collect()` and asserts `commit_count == 2`, `!no_changes`, non-empty `files_changed`, and that the target branch points at the merge commit HEAD. Tests passed on first compile.

**`test_job_phase_gates_failed_serde`** in `monitor.rs`: wraps `JobPhase::GatesFailed` in a local `Wrapper { phase: JobPhase }` struct, serializes with `toml::to_string`, asserts the output contains `"gates_failed"`, then deserializes back and asserts round-trip equality.

## Verification

```
cargo test -p smelt-core test_collect_after_merge_commit -- --nocapture
# test result: ok. 1 passed

cargo test -p smelt-core test_job_phase_gates_failed_serde -- --nocapture
# test result: ok. 1 passed

cargo test --workspace 2>&1 | grep "test result"
# smelt-core: test result: ok. 112 passed; 0 failed
# All other crates: ok, 0 failed
```

## Diagnostics

- `grep test_collect_after_merge_commit crates/smelt-core/src/collector.rs` — confirms test presence
- `grep test_job_phase_gates_failed_serde crates/smelt-core/src/monitor.rs` — confirms test presence
- Run with `--nocapture` to see git command output on failure

## Deviations

The task plan's serde test used `toml::to_string(&JobPhase::GatesFailed)` directly, but `toml` returns `Err(UnsupportedType(Some("JobPhase")))` for bare enum values — TOML does not support top-level non-table values. Fixed by wrapping in a local `Wrapper { phase: JobPhase }` struct, which accurately exercises the same serde path used when `JobPhase` appears in `RunState` (a struct). The test intent — verifying `"gates_failed"` serializes and deserializes correctly — is fully preserved.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/collector.rs` — added `test_collect_after_merge_commit` to `#[cfg(test)]` module
- `crates/smelt-core/src/monitor.rs` — added `test_job_phase_gates_failed_serde` to existing `#[cfg(test)]` module
