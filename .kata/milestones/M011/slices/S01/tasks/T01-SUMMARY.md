---
id: T01
parent: S01
milestone: M011
provides:
  - manifest/ directory module structure replacing flat manifest.rs
  - manifest/validation.rs with extracted validate_manifest() and detect_cycle()
  - manifest/tests/ with 48 tests split across 4 domain submodules
  - shared test fixtures in tests/mod.rs (VALID_MANIFEST, VALID_COMPOSE_MANIFEST, load_from_str)
  - all public API paths preserved via re-exports and thin wrappers
key_files:
  - crates/smelt-core/src/manifest/mod.rs
  - crates/smelt-core/src/manifest/validation.rs
  - crates/smelt-core/src/manifest/tests/mod.rs
  - crates/smelt-core/src/manifest/tests/core.rs
  - crates/smelt-core/src/manifest/tests/forge.rs
  - crates/smelt-core/src/manifest/tests/compose.rs
  - crates/smelt-core/src/manifest/tests/kubernetes.rs
key_decisions:
  - "ValidationErrors struct moved to validation.rs (co-located with validation logic)"
  - "Test helper minimal_toml() reduces TOML boilerplate in core tests, keeping core.rs under 500L"
  - "test_validate_runtime_unknown_rejected and test_validate_runtime_compose_valid placed in kubernetes.rs (runtime validation domain)"
patterns_established:
  - "File-to-directory module conversion: mod.rs keeps structs+thin wrappers, submodules own logic (D128)"
  - "Test directory with shared fixtures in tests/mod.rs, domain submodules use super::* (D129)"
observability_surfaces:
  - none — pure structural refactor
duration: 15min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T01: Decompose manifest.rs into directory module

**Converted 1924-line manifest.rs into 7-file directory module with validation extraction and domain-split tests, all under 500 lines**

## What Happened

Converted the flat `manifest.rs` file into a `manifest/` directory module:

1. Previous attempt had already moved `manifest.rs` → `manifest/mod.rs` via git mv.
2. Extracted `ValidationErrors`, `validate_manifest()`, and `detect_cycle()` into `manifest/validation.rs` (237 lines). The `validate()` method on `JobManifest` now delegates to `validation::validate_manifest(self)`.
3. Created `manifest/tests/mod.rs` with shared fixtures (`VALID_MANIFEST`, `VALID_COMPOSE_MANIFEST`, `load_from_str`) and 4 submodule declarations.
4. Distributed all 48 tests across domain submodules: `core.rs` (26 tests, 305L), `forge.rs` (5 tests, 171L), `compose.rs` (8 tests, 290L), `kubernetes.rs` (9 tests, 265L).
5. Removed unused imports (`HashSet`, `std::collections`) from `mod.rs` since validation logic moved out. Kept only `HashMap` needed by `resolve_credentials()`.

The `minimal_toml()` helper in `core.rs` reduces inline TOML boilerplate, keeping that file well under the 500-line limit.

## Verification

- `test ! -f crates/smelt-core/src/manifest.rs` — flat file gone ✓
- `ls manifest/{mod,validation}.rs manifest/tests/{mod,core,forge,compose,kubernetes}.rs` — all exist ✓
- `wc -l` — all files under 500 lines (max: 307L for mod.rs) ✓
- `cargo build --workspace` — clean ✓
- `cargo test --workspace` — 290 tests, 0 failures ✓
- `cargo test -p smelt-core --lib manifest` — 48 manifest tests pass (51 with 3 unrelated `assay` matches) ✓
- `cargo clippy --workspace` — clean ✓
- `cargo doc --workspace --no-deps` — clean ✓

### Slice-level checks
- All files under 500L: PASS
- cargo test --workspace 290+ tests: PASS
- cargo clippy --workspace: PASS
- cargo doc --workspace --no-deps: PASS
- cargo build --workspace: PASS

## Diagnostics

None — pure structural refactor. `cargo test -p smelt-core --lib manifest` lists all manifest tests in their new submodule paths for future inspection.

## Deviations

- `mod.rs` is 307 lines vs estimated 275 — doc comments and the `resolve_repo_path` function (with its URL_PREFIXES constant) take more space than estimated. Still well under 500L.
- `tests/core.rs` needed a `minimal_toml()` helper to stay under 500L — without it, inline TOML strings pushed it to 583 lines.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/manifest/mod.rs` — struct definitions, load/from_str/validate wrapper, resolve_credentials, CredentialStatus, resolve_repo_path (307L)
- `crates/smelt-core/src/manifest/validation.rs` — validate_manifest() + detect_cycle() + ValidationErrors (237L)
- `crates/smelt-core/src/manifest/tests/mod.rs` — shared fixtures and submodule declarations (95L)
- `crates/smelt-core/src/manifest/tests/core.rs` — 26 core parsing/validation/credential/repo-path tests (305L)
- `crates/smelt-core/src/manifest/tests/forge.rs` — 5 forge config tests (171L)
- `crates/smelt-core/src/manifest/tests/compose.rs` — 8 compose/services tests (290L)
- `crates/smelt-core/src/manifest/tests/kubernetes.rs` — 9 kubernetes runtime tests (265L)
