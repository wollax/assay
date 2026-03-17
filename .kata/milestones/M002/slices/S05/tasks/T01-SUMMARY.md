---
id: T01
parent: S05
milestone: M002
provides:
  - ScopeViolationType enum and ScopeViolation struct in assay-types
  - file_scope and shared_files fields on ManifestSession
key_files:
  - crates/assay-types/src/harness.rs
  - crates/assay-types/src/manifest.rs
  - crates/assay-types/tests/schema_snapshots.rs
key_decisions:
  - ScopeViolationType uses kebab-case serde rename (out-of-scope, shared-file-conflict) consistent with other enums like PromptLayerKind and HookEvent
patterns_established:
  - Scope types follow existing harness.rs conventions: serde derives, deny_unknown_fields, inventory registration, schema snapshots
observability_surfaces:
  - ScopeViolation is the structured diagnostic type for scope enforcement — carries file path, violation_type, and pattern for actionable error messages
duration: 1
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add scope types to assay-types and ManifestSession fields

**Added ScopeViolationType enum, ScopeViolation struct, and file_scope/shared_files fields on ManifestSession with full serde support, schema snapshots, and round-trip tests.**

## What Happened

1. Added `ScopeViolationType` enum (OutOfScope, SharedFileConflict) and `ScopeViolation` struct (file, violation_type, pattern) to `harness.rs` with all standard derives, `deny_unknown_fields`, and inventory registration.
2. Added `file_scope: Vec<String>` and `shared_files: Vec<String>` to `ManifestSession` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.
3. Re-exported the new types from `lib.rs`.
4. Added 2 new schema snapshot tests and accepted 4 snapshots (2 new + 2 updated for manifest-session and run-manifest).
5. Added round-trip unit tests: JSON serialize/deserialize for ScopeViolation (both variants), deny_unknown_fields rejection, TOML round-trip for ManifestSession with and without scope fields, and serialization omission of empty vecs.
6. Updated all existing `ManifestSession` struct literals across the workspace (manifest.rs, pipeline.rs, dag.rs, executor.rs, schema_roundtrip.rs) to include the new fields.

## Verification

- `cargo test -p assay-types` — 40 tests pass (including 2 new schema snapshots, 3 harness round-trip tests, 3 manifest round-trip tests)
- `cargo test -p assay-types --test schema_snapshots` — all pass, no pending snapshots
- `just ready` — all checks passed (fmt, lint, test, deny)
- Backward compatibility confirmed: TOML manifests without file_scope/shared_files parse correctly via serde defaults

### Slice-level verification (partial — T01 is first task):
- `cargo test -p assay-types -- scope` — ✅ ScopeViolation round-trip and schema snapshot tests pass
- `cargo test -p assay-types -- schema_snapshots` — ✅ updated ManifestSession/RunManifest snapshots pass
- `cargo test -p assay-harness -- scope` — ⏳ not yet (T02 scope)
- `cargo test -p assay-cli -- harness` — ⏳ not yet (T03 scope)
- `just ready` — ✅ full suite green

## Diagnostics

Deserialize `ScopeViolation` from JSON; the `violation_type` field distinguishes `out-of-scope` vs `shared-file-conflict` violations. Types only — no runtime behavior yet.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/harness.rs` — Added ScopeViolationType and ScopeViolation with derives and inventory registration, plus round-trip tests
- `crates/assay-types/src/manifest.rs` — Added file_scope and shared_files fields to ManifestSession, plus round-trip tests
- `crates/assay-types/src/lib.rs` — Re-exported ScopeViolation and ScopeViolationType
- `crates/assay-types/tests/schema_snapshots.rs` — Added 2 new schema snapshot tests
- `crates/assay-types/tests/snapshots/` — 2 new + 2 updated snapshot files
- `crates/assay-core/src/manifest.rs` — Updated ManifestSession literals with new fields
- `crates/assay-core/src/pipeline.rs` — Updated ManifestSession literals with new fields
- `crates/assay-core/src/orchestrate/dag.rs` — Updated ManifestSession literal with new fields
- `crates/assay-core/src/orchestrate/executor.rs` — Updated ManifestSession literal with new fields
- `crates/assay-types/tests/schema_roundtrip.rs` — Fixed ManifestSession literals with new fields
