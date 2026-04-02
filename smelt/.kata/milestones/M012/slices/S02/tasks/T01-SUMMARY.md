---
id: T01
parent: S02
milestone: M012
provides:
  - SmeltError::Tracker variant with operation+message fields
  - SmeltError::tracker() convenience constructor
  - TrackerIssue struct (platform-agnostic issue representation)
  - TrackerState enum with 6 lifecycle variants and label_name() method
  - StateBackendConfig mirror enum (LocalFs, Linear, GitHub, Ssh, Custom)
  - JobManifest.state_backend optional field
key_files:
  - crates/smelt-core/src/tracker.rs
  - crates/smelt-core/src/error.rs
  - crates/smelt-core/src/manifest/mod.rs
  - crates/smelt-core/src/lib.rs
key_decisions:
  - "Used toml::Value for StateBackendConfig::Custom config payload (not serde_json::Value) since manifests are TOML"
patterns_established:
  - "tracker module at crates/smelt-core/src/tracker.rs houses all tracker foundation types"
  - "StateBackendConfig mirrors Assay schema without crate dependency (D002/D154)"
observability_surfaces:
  - "SmeltError::Tracker { operation, message } for structured tracker error reporting"
duration: 12min
verification_result: passed
completed_at: 2026-03-27T00:00:00Z
blocker_discovered: false
---

# T01: Core types — TrackerIssue, TrackerState, SmeltError::Tracker, StateBackendConfig

**Added tracker foundation types: TrackerIssue, TrackerState (6-variant lifecycle enum with label_name), StateBackendConfig (Assay mirror), SmeltError::Tracker, and JobManifest.state_backend field**

## What Happened

Created `crates/smelt-core/src/tracker.rs` with three primary types:

- `TrackerIssue` — platform-agnostic issue struct with id, title, body, source_url
- `TrackerState` — 6-variant enum (Ready, Queued, Running, PrCreated, Done, Failed) with `label_name(prefix)` producing `"{prefix}:{state}"` format, `Display` impl, `ALL` constant, and serde round-trip support
- `StateBackendConfig` — structural mirror of Assay's enum with `rename_all = "snake_case"` and explicit `#[serde(rename = "github")]` for the GitHub variant; uses `toml::Value` for Custom config payload

Extended `SmeltError` with a `Tracker { operation, message }` variant following the `Forge` pattern, plus a `tracker()` convenience constructor.

Added `state_backend: Option<StateBackendConfig>` to `JobManifest` with `#[serde(default)]` to ensure backward compatibility. Updated all manual `JobManifest` constructions in test files (compose.rs, docker_lifecycle, compose_lifecycle, k8s_lifecycle).

## Verification

- `cargo test -p smelt-core` — 169 passed, 0 failed (includes 12 new tracker tests + 4 new manifest state_backend tests)
- `cargo test --workspace` — all test suites pass (312+ tests), 0 failures
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

### Slice-level checks (T01 is first task — partial coverage expected):
- ✓ `cargo test -p smelt-core` — all core tests pass including StateBackendConfig serde tests
- ✓ `cargo clippy --workspace -- -D warnings` — zero warnings
- ✓ `cargo doc --workspace --no-deps` — zero warnings
- ◻ `cargo test -p smelt-cli -- tracker` — no CLI tracker tests yet (later tasks)
- ◻ `cargo test --workspace` 298+ tests — currently 312+, will grow with later tasks

## Diagnostics

- Match on `SmeltError::Tracker { operation, .. }` to identify tracker-specific failures
- `TrackerState::ALL` provides iterable access to all 6 lifecycle variants
- `TrackerState::label_name("smelt")` produces label strings like `"smelt:ready"` for external label-based state tracking

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/tracker.rs` — New file: TrackerIssue, TrackerState, StateBackendConfig, and unit tests
- `crates/smelt-core/src/error.rs` — Added Tracker variant and tracker() constructor
- `crates/smelt-core/src/manifest/mod.rs` — Added state_backend field to JobManifest
- `crates/smelt-core/src/lib.rs` — Exported pub mod tracker
- `crates/smelt-core/src/compose.rs` — Added state_backend: None to test helper
- `crates/smelt-core/src/manifest/tests/core.rs` — Added 4 state_backend integration tests
- `crates/smelt-cli/tests/docker_lifecycle.rs` — Added state_backend: None to test helper
- `crates/smelt-cli/tests/compose_lifecycle.rs` — Added state_backend: None to test helper
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — Added state_backend: None to test helper
