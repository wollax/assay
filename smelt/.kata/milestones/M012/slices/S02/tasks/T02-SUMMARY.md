---
id: T02
parent: S02
milestone: M012
provides:
  - TrackerConfig struct with deny_unknown_fields and all required fields (provider, manifest_template, poll_interval_secs, label_prefix, default_harness, default_timeout)
  - ServerConfig.tracker Optional<TrackerConfig> field with serde(default)
  - Tracker-specific validation in ServerConfig::validate() collecting all errors per D018
  - TrackerSource trait with RPITIT async methods poll_ready_issues and transition_state
  - JobSource::Tracker variant for tracker-originated jobs
key_files:
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/tracker.rs
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/mod.rs
key_decisions:
  - "Used #[allow(dead_code)] on TrackerConfig.manifest_template — field is read at runtime by T03's tracker poll loop but not yet consumed by any code path"
patterns_established:
  - "TrackerSource trait uses RPITIT (D019) — no #[async_trait] macro, edition 2024 native"
  - "Tracker validation errors collected into Vec<String> then joined, matching worker validation pattern (D018)"
observability_surfaces:
  - "ServerConfig::validate() reports all tracker config violations in a single error message with newline-separated items"
duration: 8min
verification_result: passed
completed_at: 2026-03-27T00:00:00Z
blocker_discovered: false
---

# T02: TrackerConfig, ServerConfig integration, and TrackerSource trait

**Added TrackerConfig with 6-field schema, ServerConfig integration with collected validation errors, TrackerSource RPITIT async trait, and JobSource::Tracker variant — all proven by 9 unit tests**

## What Happened

Added `TrackerConfig` struct to `serve/config.rs` with `deny_unknown_fields` and fields: `provider`, `manifest_template`, `poll_interval_secs` (default 30), `label_prefix` (default "smelt"), `default_harness`, `default_timeout`. Wired it into `ServerConfig` as `tracker: Option<TrackerConfig>` with `serde(default)` so existing configs without a `[tracker]` section continue to parse.

Extended `ServerConfig::validate()` to collect tracker-specific errors following the D018 pattern: provider must be "github" or "linear", poll_interval_secs > 0, default_timeout > 0, default_harness non-empty, label_prefix non-empty. All errors are collected before returning.

Created `serve/tracker.rs` with the `TrackerSource` trait using RPITIT (D019) — two methods: `poll_ready_issues` returning `impl Future<Output = Result<Vec<TrackerIssue>>> + Send` and `transition_state` returning `impl Future<Output = Result<()>> + Send`. Includes a compile-test with `DummySource` proving the trait works.

Added `JobSource::Tracker` variant to `types.rs` and registered the tracker module in `mod.rs`.

## Verification

- `cargo test -p smelt-cli -- tracker` — 9 tests pass (7 config, 1 trait compile-test, 1 async exercise)
- `cargo test --workspace` — 175+ tests pass, zero failures
- `cargo clippy --workspace -- -D warnings` — zero warnings

## Diagnostics

- Bad `[tracker]` config in `server.toml` → `ServerConfig::load()` fails with all validation errors listed in one message
- Invalid provider → error message includes the actual value and the two allowed values
- Multiple simultaneous validation failures → all collected and reported together (D018)

## Deviations

- Added `#[allow(dead_code)]` on `manifest_template` field — clippy flags it as unread since T03 will wire the template loading. The field is structurally required for config parsing and serde.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/config.rs` — Added TrackerConfig struct, ServerConfig.tracker field, validation, and 7 unit tests
- `crates/smelt-cli/src/serve/tracker.rs` — New file: TrackerSource trait with RPITIT and compile-test
- `crates/smelt-cli/src/serve/types.rs` — Added JobSource::Tracker variant
- `crates/smelt-cli/src/serve/mod.rs` — Registered tracker module
