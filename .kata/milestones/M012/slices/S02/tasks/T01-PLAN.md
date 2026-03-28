---
estimated_steps: 5
estimated_files: 4
---

# T01: Core types — TrackerIssue, TrackerState, SmeltError::Tracker, StateBackendConfig

**Slice:** S02 — TrackerSource Trait, Config, & Template Manifest
**Milestone:** M012

## Description

Establish the foundation types in `smelt-core` that all tracker functionality depends on: `TrackerIssue` (the platform-agnostic issue representation), `TrackerState` (the label-based lifecycle enum), `SmeltError::Tracker` (structured error variant), and `StateBackendConfig` (Assay mirror enum per D154). Also add the `state_backend` optional field to `JobManifest`.

## Steps

1. Add `SmeltError::Tracker { operation, message }` variant to `error.rs` following the `Forge` pattern, plus a `tracker()` convenience constructor.
2. Create `crates/smelt-core/src/tracker.rs` with: `TrackerIssue` struct (`id: String`, `title: String`, `body: String`, `source_url: String`), `TrackerState` enum (`Ready`, `Queued`, `Running`, `PrCreated`, `Done`, `Failed`) with `serde(rename_all = "snake_case")`, and a `label_name(&self, prefix: &str) -> String` method that returns e.g. `"smelt:ready"`.
3. In the same file, add `StateBackendConfig` mirror enum matching Assay's schema: `LocalFs`, `Linear { team_id, project_id? }`, `GitHub { repo, label? }`, `Ssh { host, remote_assay_dir, user?, port? }`, `Custom { name, config: toml::Value }` — with `rename_all = "snake_case"` and explicit `#[serde(rename = "github")]` for the GitHub variant. Use `toml::Value` (not `serde_json::Value`) since this will be serialized to TOML.
4. Add `state_backend: Option<StateBackendConfig>` to `JobManifest` in `manifest/mod.rs` with `#[serde(default)]`. Export `tracker` module from `lib.rs`.
5. Write unit tests in `tracker.rs`: `TrackerState` label round-trip for all 6 variants, `StateBackendConfig` TOML serde round-trips for `local_fs`, `linear`, `github`, and `ssh` variants, and `TrackerIssue` construction.

## Must-Haves

- [ ] `SmeltError::Tracker { operation, message }` variant exists with `tracker()` convenience constructor
- [ ] `TrackerIssue` struct with `id`, `title`, `body`, `source_url` fields
- [ ] `TrackerState` enum with 6 variants, `label_name()` method producing `"{prefix}:{state}"` format
- [ ] `StateBackendConfig` mirror enum with `rename_all = "snake_case"` and explicit GitHub rename
- [ ] `JobManifest` accepts optional `[state_backend]` section without breaking existing manifests
- [ ] Unit tests for `TrackerState` label names, `StateBackendConfig` serde, and `JobManifest` with `state_backend`

## Verification

- `cargo test -p smelt-core` — all tests pass including new tracker tests
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings (all new pub items documented)

## Observability Impact

- Signals added/changed: `SmeltError::Tracker` carries `operation` + `message` fields for structured error reporting
- How a future agent inspects this: Match on `SmeltError::Tracker { operation, .. }` to identify tracker-specific failures
- Failure state exposed: Operation name (e.g. "poll", "transition") distinguishes failure context

## Inputs

- `crates/smelt-core/src/error.rs` — existing `SmeltError` enum to extend
- `crates/smelt-core/src/manifest/mod.rs` — existing `JobManifest` struct
- `../assay/crates/assay-types/src/state_backend.rs` — Assay's `StateBackendConfig` schema to mirror
- D002 (no Assay crate dep), D017 (strict parsing), D154 (state backend passthrough)

## Expected Output

- `crates/smelt-core/src/error.rs` — gains `Tracker { operation, message }` variant + `tracker()` constructor
- `crates/smelt-core/src/tracker.rs` — new file with `TrackerIssue`, `TrackerState`, `StateBackendConfig`, and tests
- `crates/smelt-core/src/lib.rs` — exports `pub mod tracker`
- `crates/smelt-core/src/manifest/mod.rs` — `JobManifest` gains `state_backend: Option<StateBackendConfig>`
