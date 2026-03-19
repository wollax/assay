---
id: T03
parent: S04
milestone: M002
provides:
  - Clean workspace validation confirming all three adapters (Claude, Codex, OpenCode) are consistent and passing
key_files:
  - crates/assay-harness/src/codex.rs
  - crates/assay-harness/src/opencode.rs
  - crates/assay-harness/src/snapshots/
key_decisions: []
patterns_established: []
observability_surfaces:
  - "just ready is the canonical workspace health check"
duration: 5m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Cross-adapter consistency and just ready

**All three adapters pass clean with 49 harness tests, 30 snapshots, and `just ready` green.**

## What Happened

Ran the full validation pipeline (`just fmt`, `just lint`, `just test`, `just ready`) across the workspace. All checks passed on the first run with zero fixes needed — T01 and T02 left the codebase in a clean state.

Test breakdown for assay-harness:
- Claude adapter: 10 tests
- Codex adapter: 12 tests
- OpenCode adapter: 10 tests
- Prompt/settings: 17 tests
- Total: 49 passed, 0 failed

Snapshot files: 30 in `crates/assay-harness/src/snapshots/` (exceeds ≥28 requirement).

## Verification

- `just fmt` — no formatting changes needed
- `just lint` (clippy) — zero warnings, `Finished` clean
- `cargo test -p assay-harness` — 49 passed, 0 failed
- `cargo test -p assay-harness -- codex` — 12 passed
- `cargo test -p assay-harness -- opencode` — 10 passed
- `cargo test -p assay-harness -- claude` — 10 passed
- `just ready` — all four checks pass (fmt, lint, test, deny), "All checks passed."
- `ls crates/assay-harness/src/snapshots/ | wc -l` — 30 snapshot files

### Slice-level verification (all pass — final task):
- ✅ `cargo test -p assay-harness -- codex` — all Codex adapter tests pass
- ✅ `cargo test -p assay-harness -- opencode` — all OpenCode adapter tests pass
- ✅ `cargo test -p assay-harness` — all harness tests pass (49 total)
- ✅ `just ready` — full workspace passes

## Diagnostics

`just ready` is the canonical health check. Snapshot mismatches produce inline diffs on regression.

## Deviations

None. No fixes were needed — T01 and T02 delivered clean code.

## Known Issues

None.

## Files Created/Modified

No source files modified — this was a validation-only task.
