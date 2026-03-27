# S01: assay-backends crate scaffold and StateBackendConfig variants

**Goal:** `assay-backends` crate exists in workspace with feature flags; `StateBackendConfig` has `Linear`, `GitHub`, `Ssh` named variants with schema snapshots updated; `backend_from_config()` factory fn dispatches all variants; `just ready` green.
**Demo:** `cargo build -p assay-backends` compiles; serde round-trip tests pass for all five `StateBackendConfig` variants; `backend_from_config()` returns `LocalFsBackend` for `LocalFs` and `NoopBackend` for the three new variants; schema snapshots accepted and committed; `just ready` green with 1488+ tests.

## Must-Haves

- `crates/assay-backends/` crate exists with `Cargo.toml` declaring `linear`, `github`, `ssh` feature flags (no deps behind them yet)
- `StateBackendConfig` has `Linear { team_id, project_id }`, `GitHub { repo, label }`, `Ssh { host, remote_assay_dir, user, port }` named variants
- `#[serde(rename = "github")]` on the `GitHub` variant (serde `rename_all = "snake_case"` would produce `git_hub`)
- Schema snapshots updated for `state-backend-config-schema` and `run-manifest-orchestrate-schema`
- `backend_from_config(config: &StateBackendConfig, assay_dir: PathBuf) -> Arc<dyn StateBackend>` in `assay_backends::factory`
- Serde round-trip tests for `LocalFs`, `Custom`, `Linear`, `GitHub`, `Ssh` variants
- TOML round-trip test for `RunManifest` with each new variant
- `just ready` green — zero regression

## Proof Level

- This slice proves: contract (serde round-trip, schema snapshot, factory dispatch, compilation)
- Real runtime required: no
- Human/UAT required: no

## Verification

- `cargo build -p assay-backends` — crate compiles
- `cargo test -p assay-backends` — factory dispatch tests pass
- `cargo test -p assay-types` — `state-backend-config-schema` snapshot passes
- `cargo test -p assay-types --features orchestrate` — `run-manifest-orchestrate-schema` snapshot passes
- `cargo test -p assay-core --features orchestrate` — state_backend round-trip tests pass (including new variants)
- `just ready` — full workspace green with 1488+ tests

## Observability / Diagnostics

- Runtime signals: None — this slice is pure type/crate scaffolding, no runtime behavior
- Inspection surfaces: Schema snapshots serve as the locked contract; `cargo insta review` shows diffs
- Failure visibility: Compilation errors, serde deserialization errors, and snapshot mismatches are all immediate and deterministic
- Redaction constraints: None

## Integration Closure

- Upstream surfaces consumed: `assay_core::state_backend::{StateBackend, LocalFsBackend, NoopBackend, CapabilitySet}`, `assay_types::StateBackendConfig`
- New wiring introduced in this slice: `assay_backends::factory::backend_from_config()` — a factory fn that S02–S04 will replace stub paths in; three new `StateBackendConfig` variants that S02–S04 will implement backends for
- What remains before the milestone is truly usable end-to-end: S02 (LinearBackend impl), S03 (GitHubBackend impl), S04 (SshSyncBackend impl + CLI/MCP factory wiring)

## Tasks

- [x] **T01: Add StateBackendConfig variants and create assay-backends crate** `est:45m`
  - Why: Creates the foundational types and crate structure that all other tasks and slices depend on
  - Files: `crates/assay-types/src/state_backend.rs`, `crates/assay-backends/Cargo.toml`, `crates/assay-backends/src/lib.rs`, `crates/assay-backends/src/factory.rs`
  - Do: Add `Linear`, `GitHub`, `Ssh` struct variants to `StateBackendConfig` with `#[serde(rename = "github")]` on GitHub variant. Create `crates/assay-backends/` with Cargo.toml (workspace deps: `assay-core`, `assay-types`, `serde`, `serde_json`; features: `linear`, `github`, `ssh`). Implement `backend_from_config()` dispatching `LocalFs` → `LocalFsBackend`, others → `NoopBackend`. Add `assay-backends` to workspace deps in root Cargo.toml.
  - Verify: `cargo build -p assay-backends` compiles; `cargo test -p assay-backends` passes
  - Done when: Crate compiles, factory fn exists and dispatches all five variants

- [x] **T02: Write tests, regenerate schema snapshots, and pass `just ready`** `est:30m`
  - Why: Locks the new variant shapes via schema snapshots and proves serde round-trips; ensures zero regression across all 1488+ tests
  - Files: `crates/assay-core/tests/state_backend.rs`, `crates/assay-backends/src/factory.rs` (tests), `crates/assay-types/tests/snapshots/` (updated snapshots)
  - Do: Add serde round-trip tests for all five `StateBackendConfig` variants (JSON and TOML) in `state_backend.rs`. Add factory dispatch tests in `assay-backends` (verify `LocalFs` returns `CapabilitySet::all()`, others return `CapabilitySet::none()`). Run `cargo insta review` for both feature-flag states to accept updated snapshots. Run `just ready`.
  - Verify: `just ready` green with 1488+ tests
  - Done when: All new tests pass, schema snapshots committed, `just ready` green

## Files Likely Touched

- `crates/assay-types/src/state_backend.rs`
- `crates/assay-backends/Cargo.toml`
- `crates/assay-backends/src/lib.rs`
- `crates/assay-backends/src/factory.rs`
- `crates/assay-core/tests/state_backend.rs`
- `crates/assay-types/tests/snapshots/` (schema snapshot files)
- `Cargo.toml` (workspace deps)
