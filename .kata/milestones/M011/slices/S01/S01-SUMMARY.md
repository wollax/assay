---
id: S01
parent: M011
milestone: M011
provides:
  - assay-backends crate with feature flags (linear, github, ssh) and backend_from_config() factory fn
  - StateBackendConfig Linear, GitHub, Ssh named variants with correct serde shapes
  - Schema snapshots updated and committed for state-backend-config-schema and run-manifest-orchestrate-schema
  - Serde round-trip tests (JSON + TOML) for all 5 StateBackendConfig variants
  - Factory dispatch tests proving LocalFs → CapabilitySet::all(), others → CapabilitySet::none()
requires: []
affects:
  - slice: S02
    provides: StateBackendConfig::Linear variant + backend_from_config() stub for LinearBackend to replace
  - slice: S03
    provides: StateBackendConfig::GitHub variant + backend_from_config() stub for GitHubBackend to replace
  - slice: S04
    provides: StateBackendConfig::Ssh variant + backend_from_config() stub for SshSyncBackend to replace + full factory wiring
key_files:
  - crates/assay-types/src/state_backend.rs
  - crates/assay-backends/Cargo.toml
  - crates/assay-backends/src/lib.rs
  - crates/assay-backends/src/factory.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap
  - crates/assay-core/tests/state_backend.rs
  - Cargo.toml
key_decisions:
  - D160 — assay-backends as new leaf crate (depends on assay-core + assay-types, not vice versa)
  - D165 — backend_from_config factory fn in assay_backends::factory
patterns_established:
  - backend_from_config() dispatches config enum to Arc<dyn StateBackend>; NoopBackend stubs pending S02–S04
  - #[serde(rename = "github")] on GitHub variant to override rename_all = "snake_case" (which would produce "git_hub")
observability_surfaces:
  - Schema snapshot files serve as locked contract; cargo insta review shows diffs on any shape change
  - Compilation errors are the primary diagnostic surface for this pure type/crate scaffolding slice
drill_down_paths:
  - .kata/milestones/M011/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M011/slices/S01/tasks/T02-SUMMARY.md
duration: ~10m
verification_result: passed
completed_at: 2026-03-27
---

# S01: assay-backends crate scaffold and StateBackendConfig variants

**New `assay-backends` leaf crate with `linear`/`github`/`ssh` feature flags and `backend_from_config()` factory fn; `StateBackendConfig` gains `Linear`, `GitHub`, `Ssh` named variants with schema snapshots updated and 1497 tests passing.**

## What Happened

**T01** created the structural foundation in two places:

1. `crates/assay-types/src/state_backend.rs` — added three new struct variants to `StateBackendConfig`:
   - `Linear { team_id: String, project_id: Option<String> }` — serde `rename_all = "snake_case"` produces correct `"linear"` tag
   - `GitHub { repo: String, label: Option<String> }` — explicit `#[serde(rename = "github")]` to override the default `"git_hub"` that `rename_all` would produce
   - `Ssh { host: String, remote_assay_dir: String, user: Option<String>, port: Option<u16> }` — serde produces correct `"ssh"` tag
   All `Option` fields use `#[serde(default, skip_serializing_if = "Option::is_none")]`.

2. Created `crates/assay-backends/` with:
   - `Cargo.toml` — workspace deps (`assay-core` with `orchestrate` feature, `assay-types`, `serde`, `serde_json`); feature flags `linear`, `github`, `ssh`; dev-dep on `tempfile`
   - `src/lib.rs` — module root exposing `factory`
   - `src/factory.rs` — `backend_from_config()` dispatching `LocalFs` → `LocalFsBackend`, all others → `NoopBackend` (stubs for S02–S04)

   Added `assay-backends` to workspace `[dependencies]` in root `Cargo.toml`.

**T02** locked the new shapes and proved zero regression:

1. Added 7 serde round-trip tests to `crates/assay-core/tests/state_backend.rs`:
   - JSON round-trips for Linear (full and minimal variants), GitHub, Ssh (full and minimal)
   - Explicit GitHub rename assertion (serializes as `"github"`, not `"git_hub"`)
   - TOML round-trip for `RunManifest` with a `Linear` backend

2. Added 5 factory dispatch tests as inline `#[cfg(test)]` module in `crates/assay-backends/src/factory.rs`:
   - `LocalFs` → `CapabilitySet::all()`
   - `Linear`, `GitHub`, `Ssh`, `Custom` → `CapabilitySet::none()`

3. Ran `cargo insta accept` to regenerate both schema snapshots (state-backend-config-schema and run-manifest-orchestrate-schema) with the new variants included.

4. `just ready` — 1497 tests, all passed, zero failures.

## Verification

- `cargo build -p assay-backends` — compiles clean ✓
- `cargo test -p assay-backends` — 5 factory dispatch tests pass ✓
- `cargo test -p assay-types --features orchestrate` — 70 schema tests pass (both updated snapshots) ✓
- `cargo test -p assay-core --features orchestrate` — all round-trip tests pass ✓
- `just ready` — 1497 tests, green, zero failures ✓

## Requirements Advanced

- R079 — `assay-backends` crate exists; `StateBackendConfig` has `Linear`, `GitHub`, `Ssh` variants; `backend_from_config()` factory fn dispatches all five variants; schema snapshots committed and locked

## Requirements Validated

- R079 — Fully validated: crate compiles, factory fn covers all variants, schema snapshots accepted, `just ready` green with 1497 tests. All S01 success criteria met.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

None — followed plan exactly.

## Known Limitations

- `backend_from_config()` dispatches `Linear`, `GitHub`, `Ssh` variants to `NoopBackend` stubs — these become real backends in S02, S03, S04 respectively
- `cargo test -p assay-types` (without `orchestrate` feature) fails to compile `schema_roundtrip.rs` because `state_backend` is feature-gated; this is pre-existing, not a regression

## Follow-ups

- S02: Replace `Linear` → `NoopBackend` in `backend_from_config()` with `LinearBackend::new(...)` after implementing `LinearBackend`
- S03: Replace `GitHub` → `NoopBackend` with `GitHubBackend::new(...)`
- S04: Replace `Ssh` → `NoopBackend` with `SshSyncBackend::new(...)`; wire `backend_from_config()` into CLI/MCP construction sites

## Files Created/Modified

- `crates/assay-types/src/state_backend.rs` — added `Linear`, `GitHub`, `Ssh` variants to `StateBackendConfig`
- `crates/assay-backends/Cargo.toml` — new crate manifest with `linear`/`github`/`ssh` feature flags
- `crates/assay-backends/src/lib.rs` — module root exposing `factory`
- `crates/assay-backends/src/factory.rs` — `backend_from_config()` factory fn + 5 dispatch tests
- `crates/assay-core/tests/state_backend.rs` — 7 new serde round-trip tests for new variants
- `crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap` — regenerated with new variants
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap` — regenerated with new variants in RunManifest
- `Cargo.toml` — added `assay-backends` to workspace `[dependencies]`

## Forward Intelligence

### What the next slice should know
- `backend_from_config()` is in `assay_backends::factory` — S02 replaces the `StateBackendConfig::Linear` arm with `Arc::new(LinearBackend::new(api_key, team_id, project_id))`
- The `assay-core` dep in `assay-backends/Cargo.toml` requires the `orchestrate` feature to access `LocalFsBackend`, `NoopBackend`, `CapabilitySet`, and `StateBackend` — don't forget this when adding `reqwest` behind the `linear` feature flag
- D150 applies: LinearBackend methods must be sync; use `tokio::runtime::Builder::new_current_thread()` scoped to each method body for async HTTP calls, never `tokio::runtime::Handle::current()`

### What's fragile
- The GitHub rename assertion test verifies `"github"` (not `"git_hub"`) — if anyone changes the variant name or removes `#[serde(rename = "github")]`, the test will catch it immediately, but it's worth knowing why the attribute exists

### Authoritative diagnostics
- Schema snapshot files in `crates/assay-types/tests/snapshots/` — any variant shape change produces an immediate deterministic snapshot mismatch
- `cargo test -p assay-backends` — factory dispatch tests are the fastest signal that backend_from_config() dispatches correctly

### What assumptions changed
- No assumptions changed — plan was straightforward and executed exactly as written
