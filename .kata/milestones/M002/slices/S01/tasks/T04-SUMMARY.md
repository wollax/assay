---
id: T04
parent: S01
milestone: M002
provides:
  - orchestrate feature enabled in assay-cli and assay-mcp
  - dependency-aware structural validation in manifest::validate()
key_files:
  - crates/assay-cli/Cargo.toml
  - crates/assay-mcp/Cargo.toml
  - crates/assay-core/src/manifest.rs
key_decisions:
  - "Dependency validation in validate() mirrors DependencyGraph::from_manifest() checks for unknown refs, self-deps, and duplicate names — lightweight structural pre-checks that don't require the feature gate"
patterns_established:
  - "Dependency-aware checks only activate when at least one session has non-empty depends_on — preserves backward compat for manifests without deps"
observability_surfaces:
  - "manifest::validate() surfaces dependency errors with field paths like sessions[2].depends_on[0] alongside existing ManifestError entries"
duration: ~10min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T04: Manifest validation integration and just ready

**Wired orchestrate feature gate into downstream crates and added dependency-aware structural validation to manifest::validate(), proving S01 composes correctly across the full workspace.**

## What Happened

1. Enabled `orchestrate` feature on assay-core dependency in both `crates/assay-cli/Cargo.toml` and `crates/assay-mcp/Cargo.toml`.
2. Extended `manifest::validate()` with dependency-aware pre-checks that activate only when at least one session has non-empty `depends_on`:
   - Duplicate effective names are rejected (with field path showing the conflicting session index)
   - Self-dependencies are rejected (field path includes the specific depends_on entry)
   - Unknown session references are rejected (names the unknown target)
   - When duplicates exist, reference checks are skipped since resolution is ambiguous
3. Added 7 unit tests covering all dependency validation paths.
4. Ran `just fmt` to fix formatting, then `just ready` passed clean.

## Verification

- `cargo test -p assay-core --features orchestrate` — 700 tests passed
- `cargo test -p assay-mcp` — 27 tests passed, no snapshot updates needed
- `cargo test -p assay-cli` — 4 tests passed
- `just ready` — all checks passed (fmt, lint, test, deny)

### Slice-level verification (all pass — this is the final task):
- ✅ `cargo test -p assay-core --features orchestrate` — all DAG unit tests pass
- ✅ `cargo test -p assay-core` (without feature) — existing tests pass, orchestrate module absent
- ✅ `cargo test -p assay-types` — schema snapshots match (verified via full workspace test)
- ✅ `cargo insta test --review` — no pending snapshot changes
- ✅ `just ready` — full suite green

## Diagnostics

Validation errors from dependency checks appear in the same `Vec<ManifestError>` as existing validation errors, surfaced through `AssayError::ManifestValidation`. Field paths use the pattern `sessions[i].depends_on[j]` with messages naming the specific problematic reference.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/Cargo.toml` — enabled orchestrate feature on assay-core dependency
- `crates/assay-mcp/Cargo.toml` — enabled orchestrate feature on assay-core dependency
- `crates/assay-core/src/manifest.rs` — added dependency-aware validation checks and 7 unit tests
