---
id: T02
parent: S01
milestone: M001
provides:
  - JobManifest type system with strict TOML parsing (deny_unknown_fields on all structs)
  - JobManifest::load() and JobManifest::validate() for manifest pipeline
  - CredentialStatus resolution from environment variables
  - SmeltError::Manifest variant for parse/validation errors
  - Example manifests (valid and invalid) for CLI testing
key_files:
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-core/src/error.rs
  - examples/job-manifest.toml
  - examples/bad-manifest.toml
key_decisions:
  - Used serde deny_unknown_fields on all six manifest structs (JobManifest, JobMeta, Environment, CredentialConfig, SessionDef, MergeConfig) for strict schema enforcement
  - Validation collects all errors before returning (not fail-fast) so users see every issue at once
  - Cycle detection uses DFS with 3-state coloring (unvisited/in-progress/done) for O(V+E) correctness
  - Credential resolution returns CredentialStatus enum (Resolved/Missing) with source info — never exposes actual values
patterns_established:
  - Manifest validation pattern: from_str() for TOML deserialization, then validate() for semantic checks — two-phase pipeline
  - Error aggregation: ValidationErrors collects Vec<String> for multi-error reporting
observability_surfaces:
  - SmeltError::Manifest includes field path and specific constraint message for every validation failure
  - CredentialStatus::Display shows "env:VAR → resolved" or "env:VAR → MISSING" without exposing values
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Job manifest types with strict validation

**Created manifest type system with 6 serde structs, deny_unknown_fields enforcement, two-phase load+validate pipeline, and 17 unit tests covering every validation rule.**

## What Happened

Built `crates/smelt-core/src/manifest.rs` with the complete manifest type system:
- `JobManifest` (top-level), `JobMeta` (`[job]`), `Environment` (`[environment]`), `CredentialConfig` (`[credentials]`), `SessionDef` (`[[session]]`), `MergeConfig` (`[merge]`)
- All structs use `#[serde(deny_unknown_fields)]` for strict schema enforcement
- `JobManifest::load(path)` reads and parses TOML from disk
- `JobManifest::from_str(content, source)` parses from string (for testing)
- `JobManifest::validate()` checks: empty required fields, unique session names, timeout > 0, valid depends_on references, no self-dependencies, cycle detection (DFS), valid merge.order references
- `JobManifest::resolve_credentials()` checks env vars and returns `CredentialStatus` (Resolved/Missing) — never exposes actual credential values

Added `SmeltError::Manifest { field, message }` variant to the error enum. Updated `lib.rs` to export the manifest module and `JobManifest`. Added `serde` and `toml` dependencies to smelt-core.

Created two example manifests: `examples/job-manifest.toml` (valid, full schema) and `examples/bad-manifest.toml` (multiple validation errors).

## Verification

- `cargo test -p smelt-core -- manifest` — 17 tests passed ✅
- `cargo test --workspace` — 49 tests passed (32 git + 17 manifest), 0 failed ✅
- `cargo build --workspace` — zero errors, zero warnings ✅

### Slice-level checks (partial — T02 is intermediate):
- `cargo test -p smelt-core` — ✅ all manifest tests pass
- `cargo test -p smelt-cli` — ✅ (0 tests, no CLI integration tests yet — T04)
- `cargo build --workspace` — ✅ zero errors, zero warnings
- `cargo run -- run examples/job-manifest.toml --dry-run` — stub output (T04 wires this up)
- `cargo run -- run examples/bad-manifest.toml --dry-run` — stub output (T04 wires this up)

## Diagnostics

- `SmeltError::Manifest` reports the field path and specific constraint violated for every validation failure
- Validation is non-fail-fast: all errors collected and reported together
- `CredentialStatus` display format: `env:VARNAME → resolved` or `env:VARNAME → MISSING`

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/manifest.rs` — new: complete manifest type system with 17 unit tests
- `crates/smelt-core/src/error.rs` — added SmeltError::Manifest variant
- `crates/smelt-core/src/lib.rs` — added manifest module export
- `crates/smelt-core/Cargo.toml` — added serde and toml dependencies
- `examples/job-manifest.toml` — new: valid example manifest demonstrating full schema
- `examples/bad-manifest.toml` — new: invalid manifest for validation testing
