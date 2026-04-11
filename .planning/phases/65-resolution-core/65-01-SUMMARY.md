---
phase: 65-resolution-core
plan: "01"
subsystem: type-foundation
tags: [resolution, types, slug-validation, criteria-library, error-variants]
dependency_graph:
  requires: []
  provides: [ResolvedGate, ResolvedCriterion, CriterionSource, validate_slug, load_library, save_library, scan_libraries, load_library_by_slug]
  affects: [assay-types, assay-core]
tech_stack:
  added: []
  patterns: [atomic-write-NamedTempFile, inventory-schema-registration, tdd]
key_files:
  created:
    - crates/assay-types/src/resolved_gate.rs
    - crates/assay-core/src/spec/compose.rs
  modified:
    - crates/assay-types/src/lib.rs
    - crates/assay-core/src/error.rs
    - crates/assay-core/src/spec/mod.rs
decisions:
  - "ResolvedCriterion uses named field (not flatten) to avoid serde deny_unknown_fields + flatten pitfall"
  - "ResolvedGate and ResolvedCriterion do NOT use deny_unknown_fields (runtime output types, not TOML-authored)"
  - "CriterionSource enum uses rename_all snake_case for consistent JSON representation"
  - "save_library validates slug before any I/O to preserve fail-fast semantics"
  - "scan_libraries silently skips parse errors (consistent with scan() in spec/mod.rs)"
  - "format_library_not_found extracted as private fn to satisfy thiserror Display pattern with complex formatting"
metrics:
  duration_minutes: 4
  completed_date: "2026-04-11"
  tasks_completed: 2
  files_changed: 5
---

# Phase 65 Plan 01: Resolution Foundation Summary

**One-liner:** ResolvedGate/ResolvedCriterion types with inventory registration, 5 new AssayError variants, and validated criteria library I/O (load/save/scan/find) with atomic writes and fuzzy suggestions.

## What Was Built

### Resolution Types (assay-types)

`crates/assay-types/src/resolved_gate.rs` provides three types that represent a fully expanded gate post-composition:

- `CriterionSource` — enum with `Own`, `Parent { gate_slug }`, `Library { slug }` variants; `rename_all = "snake_case"`
- `ResolvedCriterion` — wraps a `Criterion` with its `CriterionSource`, no `deny_unknown_fields`
- `ResolvedGate` — full expanded gate with `gate_slug`, `parent_slug`, `included_libraries`, `criteria`
- Schema registered via `inventory::submit!` as `"resolved-gate"`

### Error Variants (assay-core)

Five new `AssayError` variants added to `crates/assay-core/src/error.rs`:

| Variant | Purpose |
|---------|---------|
| `LibraryParse` | TOML parse failure for a criteria library file |
| `LibraryNotFound` | Library slug not found, with fuzzy suggestion |
| `ParentGateNotFound` | Missing parent referenced by `extends` |
| `CycleDetected` | Circular `extends` chain |
| `InvalidSlug` | Slug fails regex `^[a-z0-9][a-z0-9_-]{0,63}$` |

`LibraryNotFound` formatting extracted to a private `format_library_not_found` fn to satisfy thiserror's Display requirements with conditional suggestion text.

### Criteria Library I/O (assay-core)

`crates/assay-core/src/spec/compose.rs` provides:

- `validate_slug(value: &str) -> Result<()>` — enforces `^[a-z0-9][a-z0-9_-]*$` up to 64 chars
- `load_library(path: &Path) -> Result<CriteriaLibrary>` — mirrors `load_gates` pattern with `format_toml_error`
- `save_library(assay_dir: &Path, lib: &CriteriaLibrary) -> Result<PathBuf>` — atomic write via `NamedTempFile`, slug validated first
- `scan_libraries(assay_dir: &Path) -> Result<Vec<CriteriaLibrary>>` — scans `.assay/criteria/*.toml`, sorted by name, silently skips errors
- `load_library_by_slug(assay_dir: &Path, slug: &str) -> Result<CriteriaLibrary>` — validates slug, resolves path, provides `find_fuzzy_match` suggestion on not-found

## Test Coverage

| Suite | Tests | Location |
|-------|-------|----------|
| ResolvedGate types | 7 | `assay-types::resolved_gate::tests` |
| validate_slug | 9 | `assay-core::spec::compose::tests` |
| Library I/O | 11 | `assay-core::spec::compose::tests` |
| **Total** | **27** | |

## Commits

| Hash | Description |
|------|-------------|
| 2228795 | feat(65-01): resolution types, error variants, and slug validation |
| b4ec3ec | feat(65-01): criteria library I/O (load, save, scan, load_by_slug) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Borrow-after-partial-move in test**
- **Found during:** Task 1 — RED phase test compilation
- **Issue:** `matches!(err, AssayError::InvalidSlug { slug, .. } if slug == "...")` moved `slug` then tried to use `err` in the assert message
- **Fix:** Changed to `ref slug` to borrow instead of move
- **Files modified:** `crates/assay-core/src/spec/compose.rs`
- **Commit:** 2228795

**2. [Rule 3 - Formatting] Pre-commit hook required cargo fmt**
- **Found during:** Task 1 commit
- **Issue:** cargo fmt --check failed on long lines in compose.rs and assert macros in resolved_gate.rs
- **Fix:** Ran `cargo fmt --all`, re-staged
- **Files modified:** `crates/assay-core/src/spec/compose.rs`, `crates/assay-types/src/resolved_gate.rs`
- **Commit:** 2228795 (same commit after re-stage)

## Self-Check: PASSED

- `crates/assay-types/src/resolved_gate.rs` — FOUND
- `crates/assay-core/src/spec/compose.rs` — FOUND
- Commit `2228795` (Task 1) — FOUND
- Commit `b4ec3ec` (Task 2) — FOUND
