---
id: S01
milestone: M011
title: Decompose manifest.rs and git/cli.rs
status: done
started_at: 2026-03-24T12:00:00Z
completed_at: 2026-03-25T00:00:00Z
tasks: [T01, T02, T03]
requirements_proved: [R060]
requirements_updated: []
new_candidates: []
---

# S01: Decompose manifest.rs and git/cli.rs — Summary

**Both `manifest.rs` (1924L) and `git/cli.rs` (1365L) decomposed into focused directory modules under 500 lines each; all public API preserved via re-exports; 290 tests passing unchanged.**

## Outcome

All three tasks completed cleanly with no code fixes required at the final verification pass:

- `manifest.rs` → 7-file `manifest/` directory module (mod.rs 307L, validation.rs 237L, tests/ 4 domain submodules)
- `git/cli.rs` → 7-file `git/cli/` directory module (mod.rs 332L, tests/ 5 domain submodules)
- All 14 new files under 500 lines (max: 337L)
- All 290 workspace tests pass unchanged
- `cargo clippy --workspace` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings
- Neither `manifest.rs` nor `git/cli.rs` exist as flat files

## Task Outcomes

### T01: Decompose manifest.rs into directory module (15min, passed)

Converted 1924-line `manifest.rs` into a `manifest/` directory module:
- `mod.rs` (307L): struct definitions, load/from_str, validate wrapper, resolve_credentials, resolve_repo_path
- `validation.rs` (237L): `ValidationErrors`, `validate_manifest()`, `detect_cycle()`
- `tests/mod.rs` (95L): shared fixtures (`VALID_MANIFEST`, `VALID_COMPOSE_MANIFEST`, `load_from_str`)
- `tests/core.rs` (305L): 26 core parsing/validation/credential/repo-path tests + `minimal_toml()` helper
- `tests/forge.rs` (171L): 5 forge config tests
- `tests/compose.rs` (290L): 8 compose/services tests
- `tests/kubernetes.rs` (265L): 9 kubernetes runtime tests (incl. runtime validation domain tests)

Key decision: `ValidationErrors` co-located with validation logic in `validation.rs`, not in `mod.rs`. The `minimal_toml()` helper in `core.rs` reduced TOML boilerplate and kept `core.rs` under 500L.

### T02: Decompose git/cli.rs into directory module (10min, passed)

Moved `git/cli.rs` → `git/cli/mod.rs` via `git mv` — Rust's module system resolved transparently; `git/mod.rs` required zero changes. The `#[cfg(test)] mod tests` block replaced with `mod tests;`.

- `mod.rs` (332L): GitCli struct + full GitOps impl
- `tests/mod.rs` (49L): shared `setup_test_repo()` (pub(super)) + 5 submodule declarations
- `tests/basic.rs` (53L): 5 basic operation tests
- `tests/worktree.rs` (119L): 4 worktree tests
- `tests/branch.rs` (200L): 5 branch tests
- `tests/commit.rs` (310L): 10 commit/diff/log tests
- `tests/merge.rs` (319L): 5 merge/conflict/fetch tests

### T03: Final verification pass (3min, passed)

No code changes required. All checks passed on first run: clippy clean, doc clean, 290 tests pass, all 14 files under 500L, flat files confirmed absent.

## Key Decisions Made

| Decision | Rationale |
|----------|-----------|
| `ValidationErrors` moved to `validation.rs` | Co-location with validation logic is cleaner than keeping in `mod.rs` |
| `minimal_toml()` helper in `tests/core.rs` | Reduces inline TOML boilerplate; keeps core.rs under 500L without losing test clarity |
| Test submodule grouping by domain | manifest: core/forge/compose/kubernetes; git/cli: basic/worktree/branch/commit/merge |
| `git mv` for both conversions | Preserves git history; Rust resolves `mod cli;` to either `cli.rs` or `cli/mod.rs` transparently |
| `git/mod.rs` zero changes needed | Rust module system handles file-to-directory conversion without any `mod.rs` update |

## Verification Results

| Check | Result | Evidence |
|-------|--------|----------|
| All files under 500L | ✓ PASS | max 337L (tests/core.rs), 14 files checked |
| cargo test --workspace | ✓ PASS | 290 passed, 0 failed, 9 ignored |
| cargo clippy --workspace | ✓ PASS | zero warnings |
| cargo doc --workspace --no-deps | ✓ PASS | zero warnings |
| cargo build --workspace | ✓ PASS | all import paths resolve |
| manifest.rs absent | ✓ PASS | flat file removed |
| git/cli.rs absent | ✓ PASS | flat file removed |
| All public API preserved | ✓ PASS | zero compilation errors in consumers |

## Observability / Diagnostic Surfaces

No new runtime observability introduced — this is a pure structural refactor with no behavioral changes. Standard Rust toolchain serves as the inspection surface:

- `cargo test -p smelt-core --lib manifest` — lists all 48 manifest tests in domain submodule paths
- `cargo test -p smelt-core --lib git::cli` — lists all 29 git/cli tests in domain submodule paths
- `cargo clippy --workspace` / `cargo doc --workspace --no-deps` — lint and doc quality gates

## Requirements Updated

- **R060** (Large file decomposition round 2): advanced from `active` → `validated`. Both target files (`manifest.rs`, `git/cli.rs`) decomposed below 500L. All public API signatures preserved. All 290 tests pass unchanged.

## New Candidate Requirements

None discovered.

## Files Created/Modified

### manifest/ module (7 files)
- `crates/smelt-core/src/manifest/mod.rs` — 307L (was manifest.rs, 1924L)
- `crates/smelt-core/src/manifest/validation.rs` — 237L (extracted)
- `crates/smelt-core/src/manifest/tests/mod.rs` — 95L
- `crates/smelt-core/src/manifest/tests/core.rs` — 305L
- `crates/smelt-core/src/manifest/tests/forge.rs` — 171L
- `crates/smelt-core/src/manifest/tests/compose.rs` — 290L
- `crates/smelt-core/src/manifest/tests/kubernetes.rs` — 265L

### git/cli/ module (7 files)
- `crates/smelt-core/src/git/cli/mod.rs` — 332L (was git/cli.rs, 1365L)
- `crates/smelt-core/src/git/cli/tests/mod.rs` — 49L
- `crates/smelt-core/src/git/cli/tests/basic.rs` — 53L
- `crates/smelt-core/src/git/cli/tests/worktree.rs` — 119L
- `crates/smelt-core/src/git/cli/tests/branch.rs` — 200L
- `crates/smelt-core/src/git/cli/tests/commit.rs` — 310L
- `crates/smelt-core/src/git/cli/tests/merge.rs` — 319L
