---
estimated_steps: 6
estimated_files: 8
---

# T01: Decompose manifest.rs into directory module

**Slice:** S01 — Decompose manifest.rs and git/cli.rs
**Milestone:** M011

## Description

Convert `manifest.rs` (1924L) from a flat file into a directory module: `manifest/mod.rs` with struct definitions and thin methods, `manifest/validation.rs` for the `validate()` body and `detect_cycle()` helper, and `manifest/tests/` with tests distributed across 4 domain-specific submodules. All public API paths must be preserved via re-exports in `mod.rs`.

## Steps

1. Create `crates/smelt-core/src/manifest/` directory. Move `manifest.rs` to `manifest/mod.rs` (via `git mv`).
2. Verify `cargo build --workspace` succeeds immediately — the rename is transparent to Rust's module system.
3. Extract `detect_cycle()` and the body of `validate()` into `manifest/validation.rs`. Define `pub(super) fn validate_manifest(manifest: &JobManifest) -> crate::Result<()>` and `pub(super) fn detect_cycle(sessions: &[SessionDef]) -> Option<String>`. Update `validate()` in `mod.rs` to delegate to `validation::validate_manifest(self)`.
4. Create `manifest/tests/mod.rs` with shared test fixtures (`VALID_MANIFEST`, `VALID_COMPOSE_MANIFEST`, `load_from_str`) as `pub(super)` items, and `mod` declarations for 4 submodules: `core`, `forge`, `compose`, `kubernetes`.
5. Distribute the 48 tests into submodules: (a) `tests/core.rs` — ~26 core parsing/validation tests (parse_valid_manifest through resolve_repo_path_*). (b) `tests/forge.rs` — 5 forge tests (test_parse_manifest_with/without_forge, validate_forge_*, forge_deny_unknown_fields). (c) `tests/compose.rs` — 8 compose/services tests (test_compose_*, test_validate_compose_*, test_validate_services_*). (d) `tests/kubernetes.rs` — 9 k8s tests (test_kubernetes_*, test_validate_kubernetes_*, test_validate_runtime_*). Each submodule uses `use super::*;` to access shared fixtures plus `use crate::manifest::*;` for types.
6. Verify `cargo test -p smelt-core --lib` runs all 48 manifest tests, then `cargo test --workspace` for full 290+ suite.

## Must-Haves

- [ ] `manifest.rs` flat file no longer exists; replaced by `manifest/mod.rs`
- [ ] `manifest/validation.rs` contains `validate_manifest()` and `detect_cycle()` as `pub(super)` functions
- [ ] `manifest/tests/mod.rs` contains shared fixtures accessible to all submodules
- [ ] 48 manifest tests distributed across 4 test submodules by domain
- [ ] All files in `manifest/` and `manifest/tests/` are under 500 lines
- [ ] All existing import paths preserved (`smelt_core::manifest::JobManifest`, `smelt_core::manifest::resolve_repo_path`, etc.)
- [ ] `cargo test --workspace` passes with 290+ tests, 0 failures

## Verification

- `test ! -f crates/smelt-core/src/manifest.rs` — flat file gone
- `ls crates/smelt-core/src/manifest/mod.rs crates/smelt-core/src/manifest/validation.rs crates/smelt-core/src/manifest/tests/mod.rs` — all exist
- `wc -l crates/smelt-core/src/manifest/*.rs crates/smelt-core/src/manifest/tests/*.rs` — all under 500L
- `cargo test --workspace` — 290+ pass, 0 failures
- `cargo build --workspace` — clean (no broken imports)

## Observability Impact

- Signals added/changed: None — pure structural refactor
- How a future agent inspects this: `cargo test -p smelt-core --lib manifest` lists all manifest tests in their new submodule paths
- Failure state exposed: Compiler errors immediately surface missing re-exports or broken paths

## Inputs

- `crates/smelt-core/src/manifest.rs` — the 1924L file being decomposed
- D128 (file-to-directory module conversion), D129 (tests follow implementation), D130 (re-export shim pattern)
- M009/S03 decomposition exemplar: `crates/smelt-cli/src/commands/run/mod.rs`, `crates/smelt-cli/src/serve/ssh/mod.rs`

## Expected Output

- `crates/smelt-core/src/manifest/mod.rs` — struct definitions, `load()`, `from_str()`, thin `validate()` wrapper, `resolve_credentials()`, `resolve_repo_path()`, re-exports (~275L)
- `crates/smelt-core/src/manifest/validation.rs` — `validate_manifest()` + `detect_cycle()` (~250L)
- `crates/smelt-core/src/manifest/tests/mod.rs` — shared fixtures + submodule declarations (~80L)
- `crates/smelt-core/src/manifest/tests/core.rs` — core parsing/validation tests (~400L)
- `crates/smelt-core/src/manifest/tests/forge.rs` — forge-related tests (~200L)
- `crates/smelt-core/src/manifest/tests/compose.rs` — compose/services tests (~300L)
- `crates/smelt-core/src/manifest/tests/kubernetes.rs` — kubernetes tests (~280L)
