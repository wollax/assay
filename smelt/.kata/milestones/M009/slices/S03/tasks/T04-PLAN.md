---
estimated_steps: 4
estimated_files: 4
---

# T04: Final verification and clippy cleanup

**Slice:** S03 — Large file decomposition
**Milestone:** M009

## Description

Belt-and-suspenders final verification: confirm all size thresholds are met, all 286+ tests pass, and cargo doc is clean. Also address the 16 pre-existing `collapsible-if` clippy warnings in smelt-core's `compose.rs` and `k8s.rs` (flagged as S01 follow-up).

## Steps

1. Run `wc -l` on all three target module files (`run/mod.rs`, `ssh/mod.rs`, `tests/mod.rs`) and confirm they're under their thresholds (300, 400, 500 respectively).
2. Fix the pre-existing `collapsible-if` clippy warnings in `crates/smelt-core/src/compose.rs` and `crates/smelt-core/src/k8s.rs` — merge nested `if` blocks into combined conditions.
3. Run `cargo clippy --workspace -- -D warnings` and verify it's clean (or at minimum the 16 collapsible-if warnings are gone).
4. Run full verification suite: `cargo test --workspace` (286+ pass), `cargo doc --workspace --no-deps` (0 warnings), `cargo build --workspace` (clean).

## Must-Haves

- [ ] `run/mod.rs` < 300 lines, `ssh/mod.rs` < 400 lines, `tests/mod.rs` < 500 lines
- [ ] `cargo test --workspace` — 286+ pass, 0 failures
- [ ] `cargo doc --workspace --no-deps` — 0 warnings
- [ ] `cargo clippy --workspace` — collapsible-if warnings resolved
- [ ] R044 (Large file decomposition) validated

## Verification

- `wc -l crates/smelt-cli/src/commands/run/mod.rs crates/smelt-cli/src/serve/ssh/mod.rs crates/smelt-cli/src/serve/tests/mod.rs` — all under thresholds
- `cargo test --workspace 2>&1 | grep 'test result:'` — all 0 failures, total ≥ 286
- `cargo doc --workspace --no-deps 2>&1 | grep -c warning` — 0
- `cargo clippy --workspace -- -D warnings` — exit 0 or reduced warning count

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: `cargo clippy`, `cargo test`, `cargo doc`
- Failure state exposed: None

## Inputs

- T01, T02, T03 outputs — all three decompositions complete
- S01 summary — notes 16 pre-existing clippy warnings in compose.rs and k8s.rs

## Expected Output

- `crates/smelt-core/src/compose.rs` — collapsible-if warnings fixed
- `crates/smelt-core/src/k8s.rs` — collapsible-if warnings fixed
- Clean verification output across all checks
