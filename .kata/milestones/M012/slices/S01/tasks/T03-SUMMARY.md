---
id: T03
parent: S01
milestone: M012
provides:
  - Flaky test timeout increased from 10s to 30s (R061 resolved)
  - Full slice verification sweep — all 5 checks green
key_files:
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions: []
patterns_established: []
observability_surfaces:
  - "Inspect timeout: `rg 'from_secs' crates/smelt-cli/tests/docker_lifecycle.rs`"
duration: 5min
verification_result: passed
completed_at: 2026-03-27T12:00:00Z
blocker_discovered: false
---

# T03: Fix flaky test timeout and final verification

**Increased `test_cli_run_invalid_manifest` subprocess timeout from 10s to 30s and ran full slice verification sweep — 298 tests pass, 0 clippy warnings, 0 doc warnings, eprintln! count correct**

## What Happened

Changed `Duration::from_secs(10)` to `Duration::from_secs(30)` at docker_lifecycle.rs line 813. This resolves R061 — the test was flaking when cargo needed to link the binary on cold/incremental rebuilds, where 10s was insufficient.

Then ran the complete slice verification suite to confirm all T01 (tracing subscriber) and T02 (eprintln! migration) work plus this fix are correct.

## Verification

All 5 verification commands passed:

| Check | Result |
|-------|--------|
| `rg 'from_secs(10)' crates/smelt-cli/tests/docker_lifecycle.rs` | 0 results ✓ |
| `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` | main.rs:1, serve/tui.rs:1 ✓ |
| `cargo clippy --workspace -- -D warnings` | 0 warnings ✓ |
| `cargo doc --workspace --no-deps` | 0 warnings ✓ |
| `cargo test --workspace` | 298 passed, 0 failed ✓ |

## Diagnostics

- Inspect timeout value: `rg 'from_secs' crates/smelt-cli/tests/docker_lifecycle.rs`

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/tests/docker_lifecycle.rs` — Changed subprocess timeout from 10s to 30s
