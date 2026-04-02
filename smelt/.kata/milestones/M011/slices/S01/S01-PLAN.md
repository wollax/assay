# S01: Decompose manifest.rs and git/cli.rs

**Goal:** Both `manifest.rs` (1924L) and `git/cli.rs` (1365L) are decomposed into focused modules under 500 lines each, with all public API preserved via re-exports and all existing tests passing unchanged.
**Demo:** `manifest.rs` and `git/cli.rs` are each below 500 lines; `cargo test --workspace` passes with 290+ tests, 0 failures; `cargo clippy --workspace` and `cargo doc --workspace --no-deps` are clean.

## Must-Haves

- No file in `crates/smelt-core/src/manifest/` or `crates/smelt-core/src/git/cli/` exceeds 500 lines
- All existing import paths preserved: `smelt_core::manifest::JobManifest`, `smelt_core::manifest::resolve_repo_path`, `smelt_core::git::GitCli`, etc.
- All 290+ workspace tests pass with 0 failures
- `cargo clippy --workspace` clean
- `cargo doc --workspace --no-deps` zero warnings

## Proof Level

- This slice proves: contract (all public API signatures preserved, all tests pass unchanged)
- Real runtime required: no (compile + test verification)
- Human/UAT required: no

## Verification

- `find crates/smelt-core/src/manifest/ crates/smelt-core/src/git/cli/ -name '*.rs' -exec wc -l {} + | awk '$1 > 500 {found=1} END {exit found ? 1 : 0}'` — all files under 500L
- `cargo test --workspace` — 290+ tests, 0 failures
- `cargo clippy --workspace` — clean
- `cargo doc --workspace --no-deps` — zero warnings
- `cargo build --workspace` — confirms all import paths resolve

## Observability / Diagnostics

- Runtime signals: None (pure module restructuring, no runtime behavior change)
- Inspection surfaces: `cargo test`, `cargo clippy`, `cargo doc` — standard Rust toolchain
- Failure visibility: Compiler errors surface immediately on missing re-exports or broken paths
- Redaction constraints: None

## Integration Closure

- Upstream surfaces consumed: None (S01 is independent)
- New wiring introduced in this slice: `manifest/mod.rs` re-exports, `git/cli/mod.rs` re-exports — purely structural, no behavioral change
- What remains before the milestone is truly usable end-to-end: S02 (tracing migration + flaky test fix), S03 (health endpoint + final verification)

## Tasks

- [x] **T01: Decompose manifest.rs into directory module** `est:45m`
  - Why: `manifest.rs` is 1924L — the largest file in the codebase. Converts to `manifest/mod.rs` + child modules following D128 pattern. Implementation stays in mod.rs (~525L → split to ~275L mod.rs + ~250L validation.rs), tests distribute to submodules.
  - Files: `crates/smelt-core/src/manifest.rs` → `crates/smelt-core/src/manifest/mod.rs`, `crates/smelt-core/src/manifest/validation.rs`, `crates/smelt-core/src/manifest/tests/mod.rs`, `crates/smelt-core/src/manifest/tests/core.rs`, `crates/smelt-core/src/manifest/tests/forge.rs`, `crates/smelt-core/src/manifest/tests/compose.rs`, `crates/smelt-core/src/manifest/tests/kubernetes.rs`
  - Do: (1) Rename `manifest.rs` → `manifest/mod.rs`. (2) Extract `validate()` body + `detect_cycle()` to `manifest/validation.rs` as `pub(super)` functions; thin `validate()` wrapper stays in `mod.rs`. (3) Create `manifest/tests/mod.rs` with shared fixtures (`VALID_MANIFEST`, `VALID_COMPOSE_MANIFEST`, `load_from_str`) as `pub(super)` items. (4) Distribute 48 tests into 4 submodules by domain (core parsing/validation, forge, compose/services, kubernetes). (5) Re-export all pub items from `mod.rs`. (6) Verify `cargo build --workspace` and `cargo test --workspace`.
  - Verify: `cargo test --workspace` — all 290+ pass; `wc -l crates/smelt-core/src/manifest/*.rs crates/smelt-core/src/manifest/tests/*.rs` — all under 500L
  - Done when: `manifest.rs` no longer exists; all files in `manifest/` are under 500L; all workspace tests pass

- [x] **T02: Decompose git/cli.rs into directory module** `est:45m`
  - Why: `git/cli.rs` is 1365L. Implementation is compact (~330L) and stays together; the 29 tests (~1035L) distribute into submodules by operation family.
  - Files: `crates/smelt-core/src/git/cli.rs` → `crates/smelt-core/src/git/cli/mod.rs`, `crates/smelt-core/src/git/cli/tests/mod.rs`, `crates/smelt-core/src/git/cli/tests/basic.rs`, `crates/smelt-core/src/git/cli/tests/worktree.rs`, `crates/smelt-core/src/git/cli/tests/branch.rs`, `crates/smelt-core/src/git/cli/tests/commit.rs`, `crates/smelt-core/src/git/cli/tests/merge.rs`
  - Do: (1) Rename `git/cli.rs` → `git/cli/mod.rs`. (2) Create `git/cli/tests/mod.rs` with shared `setup_test_repo()` as `pub(super)`. (3) Distribute 29 tests into 5 submodules (basic ops, worktree, branch, commit/diff, merge/conflict+fetch). (4) `git/mod.rs` needs no changes — `mod cli;` resolves either `cli.rs` or `cli/mod.rs`. (5) Verify `cargo build --workspace` and `cargo test --workspace`.
  - Verify: `cargo test --workspace` — all 290+ pass; `wc -l crates/smelt-core/src/git/cli/*.rs crates/smelt-core/src/git/cli/tests/*.rs` — all under 500L
  - Done when: `git/cli.rs` no longer exists; all files in `git/cli/` are under 500L; all workspace tests pass

- [x] **T03: Final verification pass and cleanup** `est:15m`
  - Why: Ensures all quality gates pass in one clean sweep — clippy, doc, line counts, and full test suite. Catches any issues missed by individual task verification.
  - Files: no new files; verifies all outputs from T01 and T02
  - Do: (1) Run `cargo clippy --workspace` — fix any warnings. (2) Run `cargo doc --workspace --no-deps` — fix any warnings. (3) Verify no file exceeds 500L. (4) Run full `cargo test --workspace` confirming 290+ tests, 0 failures. (5) Verify `manifest.rs` and `git/cli.rs` no longer exist as flat files.
  - Verify: `cargo clippy --workspace` clean; `cargo doc --workspace --no-deps` zero warnings; `cargo test --workspace` 290+ pass; line count script confirms all under 500L
  - Done when: All milestone S01 success criteria verified in one pass

## Files Likely Touched

- `crates/smelt-core/src/manifest.rs` → `crates/smelt-core/src/manifest/mod.rs`
- `crates/smelt-core/src/manifest/validation.rs`
- `crates/smelt-core/src/manifest/tests/mod.rs`
- `crates/smelt-core/src/manifest/tests/core.rs`
- `crates/smelt-core/src/manifest/tests/forge.rs`
- `crates/smelt-core/src/manifest/tests/compose.rs`
- `crates/smelt-core/src/manifest/tests/kubernetes.rs`
- `crates/smelt-core/src/git/cli.rs` → `crates/smelt-core/src/git/cli/mod.rs`
- `crates/smelt-core/src/git/cli/tests/mod.rs`
- `crates/smelt-core/src/git/cli/tests/basic.rs`
- `crates/smelt-core/src/git/cli/tests/worktree.rs`
- `crates/smelt-core/src/git/cli/tests/branch.rs`
- `crates/smelt-core/src/git/cli/tests/commit.rs`
- `crates/smelt-core/src/git/cli/tests/merge.rs`
