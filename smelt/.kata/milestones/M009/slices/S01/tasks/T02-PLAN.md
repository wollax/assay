---
estimated_steps: 5
estimated_files: 4
---

# T02: Add doc comments to lib.rs, commands/, audit smelt-core annotations, and enable deny(missing_docs)

**Slice:** S01 — cargo doc zero-warning + deny(missing_docs) on smelt-cli
**Milestone:** M009

## Description

This task completes the documentation coverage by adding doc comments to the remaining undocumented public items in lib.rs and commands/mod.rs (~7 items), audits the 2 `#[allow(dead_code)]` annotations outside smelt-cli (smelt-core/k8s.rs and tests/docker_lifecycle.rs), and enables `#![deny(missing_docs)]` on smelt-cli as the lint gate. This is the final task — after it, the slice verification criteria are fully met.

## Steps

1. Add `///` doc comments to `pub mod commands` and `pub mod serve` in `crates/smelt-cli/src/lib.rs`.
2. Add `///` doc comments to all 6 `pub mod` re-exports in `crates/smelt-cli/src/commands/mod.rs` (init, list, run, serve, status, watch — verify exact list by reading file).
3. Audit smelt-core `#[allow(dead_code)]` on `PodState` in `crates/smelt-core/src/k8s.rs:184` — research indicates the struct and fields ARE used now (constructed at line 430, read via Mutex). Remove the `#[allow]` and compile-check. If the compiler warns, investigate and decide (keep with updated comment or remove the dead field). Audit `tests/docker_lifecycle.rs:133` — this is a test helper used in an `#[ignore]` test; keep the `#[allow]` as-is (legitimate).
4. Add `#![deny(missing_docs)]` to `crates/smelt-cli/src/lib.rs`. Compile with `cargo build -p smelt-cli` — if any items are still missing docs, the compiler will enumerate them. Fix any remaining gaps.
5. Run full verification: `cargo doc --workspace --no-deps` (zero warnings), `cargo build -p smelt-cli` (clean), `cargo test --workspace` (286+ tests, 0 failures), `cargo clippy --workspace -- -D warnings` (clean).

## Must-Haves

- [ ] `pub mod commands` and `pub mod serve` in lib.rs have doc comments
- [ ] All `pub mod` re-exports in commands/mod.rs have doc comments
- [ ] `#[allow(dead_code)]` on smelt-core/k8s.rs:184 (PodState) resolved (removed or justified)
- [ ] `#[allow(dead_code)]` on tests/docker_lifecycle.rs:133 confirmed legitimate (kept)
- [ ] `#![deny(missing_docs)]` added to `crates/smelt-cli/src/lib.rs`
- [ ] `cargo build -p smelt-cli` compiles clean with the deny lint active
- [ ] `cargo doc --workspace --no-deps` exits 0 with zero warnings
- [ ] `cargo test --workspace` passes with zero regressions
- [ ] `cargo clippy --workspace -- -D warnings` exits 0

## Verification

- `grep 'deny(missing_docs)' crates/smelt-cli/src/lib.rs` matches
- `cargo build -p smelt-cli 2>&1 | grep -c warning` outputs `0`
- `cargo doc --workspace --no-deps 2>&1 | grep -c warning` outputs `0`
- `cargo test --workspace` passes (286+ tests, 0 failures)
- `cargo clippy --workspace -- -D warnings` exits 0
- `grep -rn '#\[allow(dead_code)\]' crates/smelt-cli/src/ --include="*.rs"` returns zero matches in production code (test-only is OK)

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: `cargo build -p smelt-cli` will fail on any future undocumented public item — the lint is self-enforcing
- Failure state exposed: None

## Inputs

- T01 completed — all serve/ module items documented, broken doc link fixed, smelt-cli `#[allow(dead_code)]` annotations resolved
- S01-RESEARCH.md — gap analysis for remaining items and annotation audit findings

## Expected Output

- `crates/smelt-cli/src/lib.rs` — `#![deny(missing_docs)]` added, pub mod re-exports documented
- `crates/smelt-cli/src/commands/mod.rs` — all pub mod re-exports documented
- `crates/smelt-core/src/k8s.rs` — `#[allow(dead_code)]` removed or annotated with updated rationale
- Zero warnings across `cargo doc`, `cargo build`, `cargo clippy` for the entire workspace
