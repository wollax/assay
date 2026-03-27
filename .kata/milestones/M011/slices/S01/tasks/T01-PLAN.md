---
estimated_steps: 5
estimated_files: 6
---

# T01: Add StateBackendConfig variants and create assay-backends crate

**Slice:** S01 — assay-backends crate scaffold and StateBackendConfig variants
**Milestone:** M011

## Description

Add `Linear`, `GitHub`, and `Ssh` struct variants to `StateBackendConfig` in `assay-types`. Create the `assay-backends` crate with feature flags and a `backend_from_config()` factory function that dispatches all variants. This is the structural foundation that S02–S04 build on.

## Steps

1. **Add variants to `StateBackendConfig`** in `crates/assay-types/src/state_backend.rs`:
   - `Linear { team_id: String, project_id: Option<String> }` — serde `rename_all = "snake_case"` on the enum handles this correctly (`linear`)
   - `GitHub { repo: String, label: Option<String> }` — add `#[serde(rename = "github")]` because `rename_all = "snake_case"` produces `git_hub`
   - `Ssh { host: String, remote_assay_dir: String, user: Option<String>, port: Option<u16> }` — `rename_all` produces `ssh` (correct)
   - All `Option` fields use `#[serde(default, skip_serializing_if = "Option::is_none")]`

2. **Create `crates/assay-backends/Cargo.toml`**:
   - `[package]` with workspace version/edition/license/repository
   - Dependencies: `assay-core = { workspace = true, features = ["orchestrate"] }`, `assay-types = { workspace = true, features = ["orchestrate"] }`, `serde.workspace = true`, `serde_json.workspace = true`
   - `[features]`: `linear = []`, `github = []`, `ssh = []` (no deps behind them yet — S02–S04 add `reqwest` etc.)
   - Dev dependencies: `tempfile.workspace = true`

3. **Add `assay-backends` to workspace dependencies** in root `Cargo.toml`:
   - `assay-backends = { path = "crates/assay-backends" }`

4. **Create `crates/assay-backends/src/lib.rs`**:
   - `pub mod factory;`

5. **Create `crates/assay-backends/src/factory.rs`**:
   - `use std::path::PathBuf; use std::sync::Arc;`
   - `use assay_core::{LocalFsBackend, NoopBackend, StateBackend};`
   - `use assay_types::StateBackendConfig;`
   - Implement `pub fn backend_from_config(config: &StateBackendConfig, assay_dir: PathBuf) -> Arc<dyn StateBackend>`:
     - `StateBackendConfig::LocalFs => Arc::new(LocalFsBackend::new(assay_dir))`
     - `StateBackendConfig::Linear { .. } | StateBackendConfig::GitHub { .. } | StateBackendConfig::Ssh { .. } | StateBackendConfig::Custom { .. } => Arc::new(NoopBackend)` (stubs — S02–S04 replace)

## Must-Haves

- [ ] Three new struct variants on `StateBackendConfig` with correct serde names
- [ ] `#[serde(rename = "github")]` on `GitHub` variant
- [ ] `crates/assay-backends/` crate with `linear`, `github`, `ssh` feature flags
- [ ] `backend_from_config()` compiles and dispatches all five variants
- [ ] `assay-backends` added to workspace deps in root `Cargo.toml`

## Verification

- `cargo build -p assay-backends` compiles without errors
- `cargo test -p assay-types` compiles (schema snapshot tests will fail until T02 accepts them — that's expected)
- `cargo check --workspace` passes

## Observability Impact

- Signals added/changed: None — pure type/crate scaffolding
- How a future agent inspects this: `cargo build -p assay-backends` and `cargo doc -p assay-backends`
- Failure state exposed: Compilation errors are immediate and deterministic

## Inputs

- `crates/assay-types/src/state_backend.rs` — existing `StateBackendConfig` enum with `LocalFs` and `Custom` variants
- `crates/assay-core/src/state_backend.rs` — existing `LocalFsBackend`, `NoopBackend`, `StateBackend` trait
- M011-ROADMAP.md boundary map — exact field shapes for each variant

## Expected Output

- `crates/assay-types/src/state_backend.rs` — extended with 3 new variants
- `crates/assay-backends/Cargo.toml` — new crate manifest
- `crates/assay-backends/src/lib.rs` — module root
- `crates/assay-backends/src/factory.rs` — `backend_from_config()` implementation
- `Cargo.toml` — workspace deps updated
