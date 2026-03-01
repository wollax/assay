---
phase: 03-error-types-and-domain-model
plan: 01
subsystem: types
tags: [serde, schemars, chrono, toml, domain-types]
dependency-graph:
  requires: []
  provides: [GateKind, GateResult, Criterion]
  affects: [phase-06-spec-files, phase-07-gate-evaluation, phase-08-mcp-server-tools, phase-09-cli-surface]
tech-stack:
  added: [chrono 0.4, toml 0.8]
  patterns: [serde-internal-tagging, skip-serializing-if, workspace-dep-features]
key-files:
  created:
    - crates/assay-types/src/gate.rs
    - crates/assay-types/src/criterion.rs
  modified:
    - Cargo.toml
    - crates/assay-types/Cargo.toml
    - crates/assay-types/src/lib.rs
decisions:
  - GateKind uses serde internal tagging (#[serde(tag = "kind")]) for TOML compatibility
  - GateResult does not derive PartialEq (DateTime equality is semantically questionable)
  - serde_json moved to dev-dependencies in assay-types (not used in non-test source)
  - schemars chrono04 feature enabled at workspace level for DateTime<Utc> JsonSchema support
metrics:
  duration: ~3 minutes
  completed: 2026-03-01
---

# Phase 03 Plan 01: Domain Types Summary

GateKind enum (Command, AlwaysPass) with serde internal tagging, GateResult struct with skip_serializing_if for empty stdout/stderr/exit_code and chrono DateTime timestamp, and Criterion struct with optional cmd field — all deriving Serialize, Deserialize, JsonSchema with 6 roundtrip tests.

## Tasks Completed

### Task 1: Add workspace dependencies and update assay-types Cargo.toml
- Added `chrono = { version = "0.4", features = ["serde"] }` to workspace dependencies
- Changed `schemars = "1"` to `schemars = { version = "1", features = ["chrono04"] }` for DateTime JsonSchema
- Added `toml = "0.8"` to workspace dependencies
- Added `chrono.workspace = true` to assay-types dependencies
- Moved `serde_json` from dependencies to dev-dependencies in assay-types
- Added `toml.workspace = true` to assay-types dev-dependencies

### Task 2: Implement domain types and roundtrip tests
- Created `gate.rs`: GateKind enum with `#[serde(tag = "kind")]` internal tagging (Command, AlwaysPass variants), GateResult struct with skip_serializing_if on stdout/stderr/exit_code, chrono::DateTime<Utc> timestamp
- Created `criterion.rs`: Criterion struct with optional cmd field (forward-compatible with future prompt field)
- Updated `lib.rs`: added `pub mod gate`, `pub mod criterion`, and re-exports for GateKind, GateResult, Criterion
- 6 tests: TOML roundtrip for GateKind::Command, GateKind::AlwaysPass, Criterion with/without cmd; JSON skip/include for GateResult

## Verification Results

- `cargo test -p assay-types`: 6/6 tests pass
- `cargo check -p assay-types`: compiles in isolation
- `just ready`: fmt-check, clippy, all workspace tests (9 total), cargo-deny all pass

## Decisions Made

1. **serde internal tagging on GateKind**: `#[serde(tag = "kind")]` produces `kind = "Command"` in TOML — compatible with all serialization formats and self-describing
2. **No PartialEq on GateResult**: DateTime equality comparison is semantically questionable; downstream code should compare specific fields
3. **serde_json as dev-dependency**: assay-types source files do not use serde_json; only tests need it
4. **schemars chrono04 at workspace level**: Ensures DateTime<Utc> derives JsonSchema without relying on rmcp transitive feature activation

## Deviations

None — plan executed exactly as written.
