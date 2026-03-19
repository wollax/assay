---
estimated_steps: 5
estimated_files: 5
---

# T01: Add scope types to assay-types and ManifestSession fields

**Slice:** S05 — Harness CLI & Scope Enforcement
**Milestone:** M002

## Description

Add `ScopeViolation` and `ScopeViolationType` types to assay-types for scope enforcement results, and add `file_scope` and `shared_files` fields to `ManifestSession` for user-authored scope declarations. These are the foundation types consumed by scope enforcement (T02) and CLI dispatch (T03). All new types follow existing conventions: serde derives, deny_unknown_fields, inventory registration, schema snapshots.

## Steps

1. Add `ScopeViolationType` enum (OutOfScope, SharedFileConflict) and `ScopeViolation` struct (file, violation_type, pattern) to `crates/assay-types/src/harness.rs` with `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]`, `#[serde(deny_unknown_fields)]`, and inventory registration.
2. Add `file_scope: Vec<String>` and `shared_files: Vec<String>` fields to `ManifestSession` in `crates/assay-types/src/manifest.rs` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` — same pattern as `depends_on`.
3. Add schema snapshot tests for `ScopeViolationType` and `ScopeViolation` in `crates/assay-types/tests/schema_snapshots.rs`.
4. Run `cargo test -p assay-types` and accept updated snapshots for `manifest-session-schema` and `run-manifest-schema` via `cargo insta review`.
5. Verify round-trip: add a unit test in harness.rs tests that serializes/deserializes ScopeViolation, and a test in manifest.rs (or schema_snapshots.rs) that parses a ManifestSession TOML with file_scope/shared_files fields and one without (backward compat).

## Must-Haves

- [ ] `ScopeViolationType` enum with OutOfScope and SharedFileConflict variants
- [ ] `ScopeViolation` struct with file, violation_type, and pattern fields
- [ ] Both types have serde derives, deny_unknown_fields, inventory registration
- [ ] `file_scope` and `shared_files` on ManifestSession with serde defaults
- [ ] Schema snapshots for new types locked
- [ ] Updated ManifestSession/RunManifest snapshots accepted
- [ ] Backward compatibility: manifests without scope fields still parse

## Verification

- `cargo test -p assay-types` — all tests pass including new snapshot tests
- `cargo test -p assay-types -- schema_snapshots` — no pending snapshots
- Existing manifest round-trip tests still pass (backward compat)

## Observability Impact

- Signals added/changed: `ScopeViolation` is the structured diagnostic type for scope enforcement — carries file path, violation type, and matching pattern for actionable error messages
- How a future agent inspects this: deserialize `ScopeViolation` from JSON; `violation_type` field distinguishes out-of-scope vs shared-file violations
- Failure state exposed: None (types only, no runtime behavior)

## Inputs

- `crates/assay-types/src/harness.rs` — existing HarnessProfile, PromptLayer types to follow conventions
- `crates/assay-types/src/manifest.rs` — existing ManifestSession with depends_on pattern
- `crates/assay-types/tests/schema_snapshots.rs` — existing snapshot test pattern

## Expected Output

- `crates/assay-types/src/harness.rs` — ScopeViolationType enum and ScopeViolation struct added
- `crates/assay-types/src/manifest.rs` — file_scope and shared_files fields on ManifestSession
- `crates/assay-types/tests/schema_snapshots.rs` — 2 new snapshot tests
- `crates/assay-types/tests/snapshots/` — 2 new snapshot files + 2 updated snapshots
