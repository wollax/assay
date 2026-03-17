---
estimated_steps: 4
estimated_files: 3
---

# T03: Cross-adapter consistency and just ready

**Slice:** S04 — Codex & OpenCode Adapters
**Milestone:** M002

## Description

Final validation pass ensuring all three adapters are consistent, the full workspace compiles and passes lint/test/deny, and snapshot counts are correct. This task catches any fmt/clippy/deny issues and ensures the slice is fully integrated before marking complete.

## Steps

1. Run `just fmt` and fix any formatting issues across new files.
2. Run `just lint` (clippy) and resolve any warnings in `codex.rs` and `opencode.rs`.
3. Run `just test` and verify total test counts: ~27 Claude + ~12 Codex + ~12 OpenCode = ~51+ in assay-harness.
4. Run `just ready` (fmt + lint + test + deny) and confirm green. Fix any `cargo deny` issues (new `toml` dep should already be allowed since it's an existing workspace dep).

## Must-Haves

- [ ] `just ready` passes clean (all four checks: fmt, lint, test, deny)
- [ ] All three adapter modules compile and pass tests
- [ ] No new clippy warnings
- [ ] Snapshot file count in `crates/assay-harness/src/snapshots/` is ≥ 28 (12 Claude + 8+ Codex + 8+ OpenCode)

## Verification

- `just ready` exits 0 with no failures
- `cargo test -p assay-harness 2>&1 | grep 'test result'` shows total ≥ 50 tests, 0 failures
- `ls crates/assay-harness/src/snapshots/ | wc -l` shows ≥ 28 snapshot files

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: `just ready` is the canonical health check
- Failure state exposed: None

## Inputs

- `crates/assay-harness/src/codex.rs` — from T01
- `crates/assay-harness/src/opencode.rs` — from T02
- All existing workspace code — regression check

## Expected Output

- Clean `just ready` run
- Any minor fixes to `codex.rs` or `opencode.rs` from clippy/fmt
- Slice ready for S04-SUMMARY and merge
