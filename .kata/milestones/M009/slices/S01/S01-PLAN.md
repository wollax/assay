# S01: cargo doc zero-warning + deny(missing_docs) on smelt-cli

**Goal:** `cargo doc --workspace --no-deps` exits 0 with zero warnings; `#![deny(missing_docs)]` enabled on smelt-cli and compiles clean; all stale `#[allow(dead_code)]` annotations in production code removed or justified.
**Demo:** Running `cargo doc --workspace --no-deps` and `cargo build -p smelt-cli` both succeed with zero warnings; `#![deny(missing_docs)]` is present in `crates/smelt-cli/src/lib.rs`.

## Must-Haves

- `cargo doc --workspace --no-deps` exits 0 with zero warnings
- `#![deny(missing_docs)]` present in `crates/smelt-cli/src/lib.rs` and compiles clean
- All ~37 production public items in smelt-cli have doc comments
- Broken intra-doc link in `ssh.rs:185` fixed
- 4 `#[allow(dead_code)]` annotations audited: stale ones removed, justified ones annotated with rationale
- `cargo test --workspace` passes with zero regressions (286+ tests)
- `cargo clippy --workspace` is clean

## Proof Level

- This slice proves: contract
- Real runtime required: no (compile-time lints + existing test suite)
- Human/UAT required: no

## Verification

- `cargo doc --workspace --no-deps 2>&1 | grep -c warning` outputs `0`
- `cargo build -p smelt-cli 2>&1 | grep -c warning` outputs `0`
- `grep -c 'deny(missing_docs)' crates/smelt-cli/src/lib.rs` outputs `1`
- `cargo test --workspace` passes (286+ tests, 0 failures)
- `cargo clippy --workspace -- -D warnings` exits 0

## Observability / Diagnostics

- Runtime signals: none (compile-time lint work, no runtime behavior change)
- Inspection surfaces: `cargo doc --workspace --no-deps` warnings are the primary diagnostic
- Failure visibility: compiler errors pinpoint exact file/line for any missing doc comment
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: none (first slice, no dependencies)
- New wiring introduced in this slice: `#![deny(missing_docs)]` lint gate on smelt-cli — all future public items must have docs
- What remains before the milestone is truly usable end-to-end: S02 (README + examples docs), S03 (large file decomposition)

## Tasks

- [x] **T01: Fix broken intra-doc link and add doc comments to serve/ module public items** `est:25m`
  - Why: Fixes the only `cargo doc` warning (ssh.rs:185 broken link) and documents the bulk of undocumented items (serve/config.rs, serve/queue.rs, serve/ssh.rs, serve/types.rs, serve/mod.rs — ~23 items)
  - Files: `crates/smelt-cli/src/serve/ssh.rs`, `crates/smelt-cli/src/serve/config.rs`, `crates/smelt-cli/src/serve/queue.rs`, `crates/smelt-cli/src/serve/types.rs`, `crates/smelt-cli/src/serve/mod.rs`
  - Do: Fix `[build_ssh_args]` link on ssh.rs:185 to backtick-only per D070; add `///` doc comments to all undocumented public items in the 5 serve/ files; audit and handle the 2 smelt-cli `#[allow(dead_code)]` annotations (config.rs:75, ssh.rs:532)
  - Verify: `cargo doc --workspace --no-deps 2>&1 | grep -c warning` outputs `0`; `cargo test --workspace` passes
  - Done when: zero cargo doc warnings; all serve/ public items documented; both `#[allow(dead_code)]` annotations resolved

- [x] **T02: Add doc comments to lib.rs, commands/, audit smelt-core annotations, and enable deny(missing_docs)** `est:20m`
  - Why: Documents remaining public items (lib.rs pub mod, commands/mod.rs 6 pub mods), audits the 2 smelt-core `#[allow(dead_code)]` annotations, and enables the lint gate as the final step
  - Files: `crates/smelt-cli/src/lib.rs`, `crates/smelt-cli/src/commands/mod.rs`, `crates/smelt-core/src/k8s.rs`, `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do: Add doc comments to `pub mod serve` and `pub mod commands` in lib.rs; add doc comments to all 6 `pub mod` re-exports in commands/mod.rs; audit smelt-core/k8s.rs:184 `#[allow(dead_code)]` on PodState (remove if fields are used); audit tests/docker_lifecycle.rs:133 (keep — test helper); add `#![deny(missing_docs)]` to lib.rs; run full verification suite
  - Verify: `cargo build -p smelt-cli` compiles clean; `cargo doc --workspace --no-deps 2>&1 | grep -c warning` outputs `0`; `cargo test --workspace` passes; `cargo clippy --workspace -- -D warnings` exits 0
  - Done when: `#![deny(missing_docs)]` in lib.rs compiles clean; all 4 `#[allow(dead_code)]` annotations resolved; full test suite green; zero clippy warnings

## Files Likely Touched

- `crates/smelt-cli/src/lib.rs`
- `crates/smelt-cli/src/serve/mod.rs`
- `crates/smelt-cli/src/serve/config.rs`
- `crates/smelt-cli/src/serve/queue.rs`
- `crates/smelt-cli/src/serve/ssh.rs`
- `crates/smelt-cli/src/serve/types.rs`
- `crates/smelt-cli/src/commands/mod.rs`
- `crates/smelt-core/src/k8s.rs`
