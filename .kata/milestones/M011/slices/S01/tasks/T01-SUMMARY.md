---
id: T01
parent: S01
milestone: M011
provides:
  - StateBackendConfig with Linear, GitHub, Ssh variants
  - assay-backends crate with factory module and feature flags
  - backend_from_config() dispatching all five variants
key_files:
  - crates/assay-types/src/state_backend.rs
  - crates/assay-backends/Cargo.toml
  - crates/assay-backends/src/lib.rs
  - crates/assay-backends/src/factory.rs
  - Cargo.toml
key_decisions:
  - none — followed plan exactly
patterns_established:
  - backend_from_config() factory pattern dispatching config enum to Arc<dyn StateBackend>
observability_surfaces:
  - none — pure type/crate scaffolding
duration: 5m
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T01: Add StateBackendConfig variants and create assay-backends crate

**Added Linear, GitHub, Ssh struct variants to StateBackendConfig and created assay-backends crate with backend_from_config() factory function.**

## What Happened

Added three new struct variants to `StateBackendConfig` in `assay-types`:
- `Linear { team_id, project_id? }` — serde `rename_all` produces correct `linear` tag
- `GitHub { repo, label? }` — explicit `#[serde(rename = "github")]` since `rename_all = "snake_case"` would produce `git_hub`
- `Ssh { host, remote_assay_dir, user?, port? }` — serde `rename_all` produces correct `ssh` tag

All `Option` fields use `#[serde(default, skip_serializing_if = "Option::is_none")]`.

Created `crates/assay-backends/` with:
- `Cargo.toml` — workspace deps (`assay-core`, `assay-types` with `orchestrate` feature, `serde`, `serde_json`), feature flags (`linear`, `github`, `ssh`), dev-dep on `tempfile`
- `src/lib.rs` — module root exposing `factory`
- `src/factory.rs` — `backend_from_config()` that dispatches `LocalFs` → `LocalFsBackend`, all others → `NoopBackend`

Added `assay-backends` to workspace dependencies in root `Cargo.toml`.

## Verification

- `cargo build -p assay-backends` — compiles clean ✅
- `cargo check --workspace` — passes (only pre-existing warnings in assay-mcp) ✅
- `cargo test -p assay-types` — has pre-existing compilation errors in test file (unrelated to this task, T02 will address schema snapshots) — expected per plan ✅

## Diagnostics

None — pure type/crate scaffolding. Compilation errors are the diagnostic surface.

## Deviations

None.

## Known Issues

- `cargo test -p assay-types` has pre-existing compilation errors in `schema_roundtrip.rs` (fields `gossip_config` and `state_backend` removed from `RunManifest` in a prior change). T02 will update tests and snapshots.

## Files Created/Modified

- `crates/assay-types/src/state_backend.rs` — added Linear, GitHub, Ssh variants to StateBackendConfig enum
- `crates/assay-backends/Cargo.toml` — new crate manifest with feature flags
- `crates/assay-backends/src/lib.rs` — module root
- `crates/assay-backends/src/factory.rs` — backend_from_config() factory function
- `Cargo.toml` — added assay-backends to workspace dependencies
