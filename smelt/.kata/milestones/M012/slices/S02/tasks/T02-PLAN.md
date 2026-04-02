---
estimated_steps: 5
estimated_files: 5
---

# T02: TrackerConfig, ServerConfig integration, and TrackerSource trait

**Slice:** S02 — TrackerSource Trait, Config, & Template Manifest
**Milestone:** M012

## Description

Add `TrackerConfig` to `ServerConfig` so `server.toml` can declare a `[tracker]` section. Define the `TrackerSource` async trait using RPITIT (D019). Add `JobSource::Tracker` variant. Wire validation: provider must be "github" or "linear", manifest_template path existence is deferred to T03's startup-time load, poll_interval_secs > 0, default_timeout > 0.

## Steps

1. Add `TrackerConfig` struct to `serve/config.rs` with `#[serde(deny_unknown_fields)]`: `provider: String`, `manifest_template: PathBuf`, `poll_interval_secs: u64` (default 30), `label_prefix: String` (default `"smelt"`), `default_harness: String`, `default_timeout: u64`. Add `tracker: Option<TrackerConfig>` to `ServerConfig` with `#[serde(default)]`.
2. Extend `ServerConfig::validate()` to collect tracker-specific errors (D018): provider must be `"github"` or `"linear"`; `poll_interval_secs > 0`; `default_timeout > 0`; `default_harness` non-empty; `label_prefix` non-empty.
3. Create `crates/smelt-cli/src/serve/tracker.rs` with the `TrackerSource` trait: two async methods using RPITIT — `poll_ready_issues(&self) -> impl Future<Output = anyhow::Result<Vec<TrackerIssue>>> + Send` and `transition_state(&self, issue_id: &str, from: TrackerState, to: TrackerState) -> impl Future<Output = anyhow::Result<()>> + Send`. Import `TrackerIssue` and `TrackerState` from `smelt_core::tracker`.
4. Add `JobSource::Tracker` variant to the `JobSource` enum in `serve/types.rs` with doc comment.
5. Register `pub mod tracker;` in `serve/mod.rs`. Write unit tests in `serve/config.rs` (or inline module): `[tracker]` section parses correctly; missing `[tracker]` still works; invalid provider rejected; zero poll_interval rejected; zero default_timeout rejected. Write a compile-test function in tracker.rs verifying `TrackerSource` trait compiles with a trivial impl.

## Must-Haves

- [ ] `TrackerConfig` struct with `deny_unknown_fields` and all required fields
- [ ] `ServerConfig` accepts `[tracker]` section; configs without it still parse
- [ ] `ServerConfig::validate()` collects tracker-specific validation errors (D018)
- [ ] `TrackerSource` trait with RPITIT async methods `poll_ready_issues` and `transition_state`
- [ ] `JobSource::Tracker` variant exists in `types.rs`
- [ ] Unit tests for config parsing (valid/invalid) and trait compilation

## Verification

- `cargo test -p smelt-cli -- tracker` — new tracker config and trait tests pass
- `cargo test --workspace` — all 298+ tests pass
- `cargo clippy --workspace -- -D warnings` — zero warnings

## Observability Impact

- Signals added/changed: Config validation errors for `[tracker]` are collected and reported at startup via `ServerConfig::validate()`
- How a future agent inspects this: Bad `[tracker]` config fails `ServerConfig::load()` with all errors listed
- Failure state exposed: Validation errors list each invalid field with its constraint violation

## Inputs

- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig` struct to extend
- `crates/smelt-cli/src/serve/types.rs` — `JobSource` enum to extend
- `crates/smelt-core/src/tracker.rs` — `TrackerIssue`, `TrackerState` from T01
- D019 (RPITIT), D150 (polling), D151 (one tracker per instance), D018 (collected errors)

## Expected Output

- `crates/smelt-cli/src/serve/config.rs` — gains `TrackerConfig` struct + validation in `validate()`
- `crates/smelt-cli/src/serve/tracker.rs` — new file with `TrackerSource` trait
- `crates/smelt-cli/src/serve/mod.rs` — exports tracker module
- `crates/smelt-cli/src/serve/types.rs` — `JobSource::Tracker` variant added
