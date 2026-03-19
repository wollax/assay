# S06: RunManifest Type & Parsing — UAT

**Milestone:** M001
**Written:** 2026-03-16

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: This slice delivers types and parsing only — no runtime pipeline, no side effects, no user-facing CLI commands. All behavior is verified by unit tests and schema snapshots.

## Preconditions

- Rust toolchain installed
- `cargo insta` available for snapshot review

## Smoke Test

Run `cargo test -p assay-core -- manifest::tests::from_str_valid_minimal` — a single-session TOML manifest parses successfully.

## Test Cases

### 1. Schema snapshots lock type contract

1. Run `cargo test -p assay-types -- manifest`
2. **Expected:** 2 tests pass (run_manifest_schema_snapshot, manifest_session_schema_snapshot)

### 2. Round-trip TOML parsing

1. Run `cargo test -p assay-core -- manifest::tests::from_str_round_trip`
2. **Expected:** Parse → serialize → parse produces identical RunManifest

### 3. Full manifest with all optional fields

1. Run `cargo test -p assay-core -- manifest::tests::from_str_valid_full`
2. **Expected:** ManifestSession with settings, hooks, and prompt_layers all populated

### 4. Validation catches empty sessions

1. Run `cargo test -p assay-core -- manifest::tests::validate_empty_sessions`
2. **Expected:** ManifestValidation error with "sessions: at least one session required"

### 5. Validation catches blank spec

1. Run `cargo test -p assay-core -- manifest::tests::validate_missing_spec`
2. **Expected:** ManifestValidation error with "sessions[0].spec: must not be empty"

### 6. Unknown fields rejected

1. Run `cargo test -p assay-core -- manifest::tests::from_str_unknown_field`
2. **Expected:** TOML parse error (deny_unknown_fields rejects extra keys)

### 7. File loading with caret-pointer errors

1. Run `cargo test -p assay-core -- manifest::tests::load_parse_error`
2. **Expected:** ManifestParse error with file path, line/column, and source line display

## Edge Cases

### Missing required `spec` field

1. Run `cargo test -p assay-core -- manifest::tests::from_str_missing_spec`
2. **Expected:** TOML deserialization error (spec is not Option)

### Load nonexistent file

1. Run `cargo test -p assay-core -- manifest::tests::load_nonexistent_file`
2. **Expected:** ManifestParse error with file path in message

## Failure Signals

- Schema snapshot test failure indicates type contract drift — run `cargo insta review` to inspect
- ManifestValidation tests failing means validation logic has regressed
- `just ready` failure after changes indicates clippy, fmt, or deny regression

## Requirements Proved By This UAT

- R014 — RunManifest type exists with `[[sessions]]` array format, locked by schema snapshots
- R015 — Manifest parsing and validation produce actionable errors (caret pointers, field paths, collect-all semantics)
- R016 — Forward compatibility via `[[sessions]]` array verified by all test fixtures using array syntax

## Not Proven By This UAT

- Runtime pipeline consumption of manifests (S07)
- ManifestSession override fields wired into HarnessProfile construction (S07)
- Real TOML manifest files authored by users in production workflows
- Error message quality under adversarial/unusual TOML inputs beyond the test fixtures

## Notes for Tester

No manual testing needed — this is a pure types-and-parsing slice. All behavior is covered by the 15 automated tests (2 schema snapshots + 13 core tests). If you want to manually test, create a `.toml` file and call `assay_core::manifest::load()` from a scratch binary.
