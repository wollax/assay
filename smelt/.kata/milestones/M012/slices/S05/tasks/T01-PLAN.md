---
estimated_steps: 4
estimated_files: 1
---

# T01: State backend passthrough in AssayInvoker

**Slice:** S05 — Dispatch Integration, State Backend Passthrough & Final Assembly
**Milestone:** M012

## Description

Add `state_backend` passthrough from `JobManifest` into the Assay `SmeltRunManifest` TOML. The `SmeltRunManifest` struct gains an `Option<StateBackendConfig>` field with serde defaults for backward compat (`deny_unknown_fields` constraint). `build_run_manifest_toml()` copies `manifest.state_backend` into the run manifest. This satisfies R075 independently of the tracker poller.

## Steps

1. Add `use crate::tracker::StateBackendConfig;` import to `assay.rs`
2. Add `#[serde(default, skip_serializing_if = "Option::is_none")] pub state_backend: Option<StateBackendConfig>` field to `SmeltRunManifest`
3. Update `build_run_manifest_toml()` to set `state_backend: manifest.state_backend.clone()` when constructing `SmeltRunManifest`
4. Add unit tests: (a) existing tests still pass (None → no section); (b) manifest with `state_backend: Some(StateBackendConfig::Linear { team_id, project_id })` produces TOML containing `[state_backend]` with `type = "linear"` and fields; (c) manifest with `StateBackendConfig::LocalFs` produces `state_backend = "local_fs"` line

## Must-Haves

- [ ] `SmeltRunManifest` has `state_backend: Option<StateBackendConfig>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`
- [ ] `build_run_manifest_toml()` includes `state_backend` from the input manifest
- [ ] Existing `build_run_manifest_toml` tests pass unchanged (backward compat — None produces no section)
- [ ] New test: Linear state_backend appears in TOML output as `[state_backend]` section
- [ ] New test: LocalFs state_backend appears as `state_backend = "local_fs"`

## Verification

- `cargo test -p smelt-core --lib -- assay::tests` — all tests pass including new ones
- `cargo test --workspace` — 387+ tests pass, 0 regressions
- `cargo clippy --workspace -- -D warnings` — zero warnings

## Observability Impact

- Signals added/changed: None — this is a serialization passthrough, not a runtime path
- How a future agent inspects this: Read the generated TOML string; `tracing::debug!` already logs full manifest content
- Failure state exposed: Serialization is infallible for valid data (existing `expect` covers this)

## Inputs

- `crates/smelt-core/src/assay.rs` — `SmeltRunManifest`, `build_run_manifest_toml()`
- `crates/smelt-core/src/tracker.rs` — `StateBackendConfig` enum with Serialize+Deserialize
- `crates/smelt-core/src/manifest/mod.rs` — `JobManifest.state_backend: Option<StateBackendConfig>` (already exists, D160)

## Expected Output

- `crates/smelt-core/src/assay.rs` — `SmeltRunManifest` with `state_backend` field; `build_run_manifest_toml()` passes it through; 2+ new unit tests
