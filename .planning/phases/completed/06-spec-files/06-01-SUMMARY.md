# Phase 6 Plan 01: Spec Type Updates, Error Variants, and Spec Module Summary

**One-liner:** TOML spec parsing with [[criteria]] arrays, deny_unknown_fields validation, and directory scanning via from_str/validate/load/scan free functions mirroring config module pattern.

## Execution Details

| Field | Value |
|-------|-------|
| Phase | 06-spec-files |
| Plan | 01 |
| Type | TDD |
| Duration | ~10 minutes |
| Completed | 2026-03-02 |
| Tasks | 4/4 |
| Tests Added | 24 |
| Tests Total (workspace) | 78 |

## What Was Built

### Type Changes (assay-types)

- **Spec** gained `criteria: Vec<Criterion>` field, `#[serde(deny_unknown_fields)]`, and `description` became optional (defaults to empty string, skipped when empty in serialized output)
- **Criterion** gained `#[serde(deny_unknown_fields)]`
- Updated `schema_roundtrip.rs` tests to construct Spec with criteria
- Updated insta snapshots for spec, criterion, and workflow schemas

### Error Variants (assay-core/error.rs)

- `AssayError::SpecParse { path, message }` — TOML parse failure with file path context
- `AssayError::SpecValidation { path, errors: Vec<SpecError> }` — semantic validation failures
- `AssayError::SpecScan { path, source }` — directory I/O failure

### Spec Module (assay-core/spec/mod.rs)

| Function | Signature | Purpose |
|----------|-----------|---------|
| `from_str` | `&str -> Result<Spec, toml::de::Error>` | Parse TOML to Spec without validation |
| `validate` | `&Spec -> Result<(), Vec<SpecError>>` | Collect all semantic errors at once |
| `load` | `&Path -> Result<Spec>` | Read file + parse + validate with path context |
| `scan` | `&Path -> Result<ScanResult>` | Flat dir scan, sort by filename, detect duplicates |

Supporting types:
- `SpecError` — mirrors `ConfigError` (field + message + Display)
- `ScanResult` — `specs: Vec<(String, Spec)>` + `errors: Vec<AssayError>`

### Schema Regeneration

- `spec.schema.json` — now includes `criteria` array with Criterion `$ref`, `additionalProperties: false`
- `criterion.schema.json` — now includes `additionalProperties: false`
- `workflow.schema.json` — updated Spec definition within Workflow type

## Commits

| Hash | Type | Description |
|------|------|-------------|
| `2790c18` | feat | Update Spec and Criterion types for spec module |
| `317452a` | test | Add failing tests for spec module (RED phase) |
| `1d1770a` | feat | Implement spec module with from_str, validate, load, scan (GREEN phase) |
| `96c631a` | chore | Regenerate schemas for updated Spec and Criterion types |

## Deviations from Plan

None — plan executed exactly as written.

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| SpecError in spec/mod.rs, not error.rs | Mirrors ConfigError pattern — spec-specific validation output stays with spec concerns |
| Duplicate spec names: remove later file from specs, add to errors | First-seen wins; the later file with the duplicate name becomes an error entry |
| Criterion name validation: empty names checked before duplicate detection | Prevents confusing "duplicate empty name" errors |

## Test Coverage

24 new tests across 4 categories:
- **from_str** (7): valid minimal, with description/cmd, multiple criteria, description default, invalid TOML, unknown spec field, unknown criterion field
- **validate** (7): valid spec, empty name, whitespace name, zero criteria, duplicate criterion names, empty criterion name, collect all errors
- **load** (4): valid spec, missing file, invalid TOML parse, valid TOML invalid semantics
- **scan** (6): valid specs, mixed valid/invalid, ignore non-toml, sorted by filename, duplicate spec names, nonexistent directory

## Requirements Satisfied

- **SPEC-01**: TOML spec file parsing via `spec::load()` and `from_str()` free functions
- **SPEC-02**: Spec struct with `name`, `description`, `criteria: Vec<Criterion>`
- **SPEC-03**: Criteria with optional `cmd` field (present = executable, absent = descriptive)
- **SPEC-04**: Spec validation — name required, non-empty after trim, unique criteria names
- **SPEC-05**: Spec directory scanning — find all `.toml` files in directory

## Key Files

### Created
- `crates/assay-core/src/spec/mod.rs` (full implementation, replaced stub)

### Modified
- `crates/assay-types/src/lib.rs` (Spec type updated)
- `crates/assay-types/src/criterion.rs` (deny_unknown_fields added)
- `crates/assay-core/src/error.rs` (3 new variants)
- `crates/assay-types/tests/schema_roundtrip.rs` (Spec construction updated)
- `crates/assay-types/tests/snapshots/schema_snapshots__spec-schema.snap` (updated)
- `crates/assay-types/tests/snapshots/schema_snapshots__criterion-schema.snap` (updated)
- `crates/assay-types/tests/snapshots/schema_snapshots__workflow-schema.snap` (updated)
- `schemas/spec.schema.json` (regenerated)
- `schemas/criterion.schema.json` (regenerated)
- `schemas/workflow.schema.json` (regenerated)

## Next Phase Readiness

Plan 06-02 (CLI spec show/list subcommands) can proceed immediately. The spec module exports everything needed:
- `spec::from_str`, `spec::validate`, `spec::load`, `spec::scan`
- `spec::SpecError`, `spec::ScanResult`
- Updated `Spec` and `Criterion` types with criteria field

No blockers or concerns for Plan 02.
