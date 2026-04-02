---
id: S02
parent: M012
milestone: M012
provides:
  - TrackerIssue struct (platform-agnostic issue representation)
  - TrackerState enum (6-variant lifecycle with label_name() method)
  - StateBackendConfig mirror enum (LocalFs, Linear, GitHub, Ssh, Custom) in smelt-core
  - SmeltError::Tracker { operation, message } variant and tracker() constructor
  - JobManifest.state_backend optional field with serde(default)
  - TrackerConfig struct with deny_unknown_fields and all required fields
  - ServerConfig.tracker Optional<TrackerConfig> field with validation (D018)
  - TrackerSource trait (RPITIT, D019) with poll_ready_issues and transition_state
  - JobSource::Tracker variant
  - load_template_manifest() — validates zero-session constraint at startup (D017)
  - issue_to_manifest() — clones template, injects sanitized session from TrackerIssue
  - sanitize() helper for session name normalization
  - MockTrackerSource — VecDeque-based test double following MockSshClient pattern
  - Clone derives on all JobManifest types (enabling template cloning)
  - serde(default) on JobManifest.session (enabling zero-session templates)
requires:
  - slice: S01
    provides: Clean tracing infrastructure (no eprintln conflicts); stable test suite
affects:
  - S03
  - S04
  - S05
key_files:
  - crates/smelt-core/src/tracker.rs
  - crates/smelt-core/src/error.rs
  - crates/smelt-core/src/manifest/mod.rs
  - crates/smelt-core/src/lib.rs
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/tracker.rs
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/mod.rs
key_decisions:
  - "D160: state_backend added to JobManifest in S02 (not deferred to S05) — deny_unknown_fields would reject template manifests with [state_backend] otherwise"
  - "D161: issue_to_manifest() is a free function, not a trait method — logic is identical for all backends so putting it on the trait would force duplication"
  - "D162: Template manifest must have zero [[session]] entries — unambiguous requirement validated at startup per D017"
  - "D163: StateBackendConfig uses toml::Value for Custom variant (not serde_json::Value) — manifests are TOML; avoids conversion step"
patterns_established:
  - "TrackerSource trait uses RPITIT (D019) — no #[async_trait] macro, edition 2024 native"
  - "MockTrackerSource follows VecDeque<Result> pattern matching MockSshClient (new(), with_poll_result(), with_transition_result())"
  - "Template manifest = normal manifest with zero sessions, loaded via load_template_manifest(), sessions injected dynamically by issue_to_manifest()"
  - "Tracker validation errors collected per D018 — all errors reported at startup, never fail-fast"
  - "serde(default) on JobManifest.session allows zero-session templates; existing validate() still catches missing sessions in normal use"
observability_surfaces:
  - "SmeltError::Tracker { operation, message } for structured tracker error reporting"
  - "tracing::info! on successful template manifest load at startup (file path logged)"
  - "ServerConfig::load() fails fast with descriptive error on bad [tracker] config (D017)"
  - "All validation errors collected and reported together (D018) — multiple failures surface in one message"
drill_down_paths:
  - .kata/milestones/M012/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M012/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M012/slices/S02/tasks/T03-SUMMARY.md
duration: ~40min
verification_result: passed
completed_at: 2026-03-28T00:00:00Z
---

# S02: TrackerSource Trait, Config, & Template Manifest

**Complete tracker foundation: TrackerSource trait, TrackerConfig, template manifest loading with zero-session validation, issue injection, MockTrackerSource, and all core types — proven by 25+ unit tests across smelt-core and smelt-cli**

## What Happened

Three tasks delivered the full contract-level foundation for tracker-driven dispatch.

**T01** established the core types in smelt-core: `TrackerIssue` (platform-agnostic issue struct), `TrackerState` (6-variant lifecycle enum with `label_name(prefix)` producing `"{prefix}:{state}"` strings), and `StateBackendConfig` (structural mirror of Assay's enum using `toml::Value` for the Custom config payload, per D163). Extended `SmeltError` with a `Tracker { operation, message }` variant and `tracker()` constructor following the Forge pattern. Added `state_backend: Option<StateBackendConfig>` to `JobManifest` with `#[serde(default)]` — this was deliberately done in S02 rather than S05 (D160) because `deny_unknown_fields` on `JobManifest` would cause template manifests containing `[state_backend]` to fail to parse. Updated all manual `JobManifest` constructions in test helpers.

**T02** added `TrackerConfig` to `serve/config.rs` with `deny_unknown_fields` and six fields (`provider`, `manifest_template`, `poll_interval_secs`, `label_prefix`, `default_harness`, `default_timeout`), wired into `ServerConfig` as `tracker: Option<TrackerConfig>`. Extended `ServerConfig::validate()` with tracker-specific error collection per D018. Created `serve/tracker.rs` with the `TrackerSource` trait using RPITIT (D019) — two methods: `poll_ready_issues` and `transition_state`. Added `JobSource::Tracker` variant to `types.rs`.

**T03** completed the template system: `load_template_manifest()` loads and rejects manifests with `[[session]]` entries (D162), `issue_to_manifest()` clones a template and injects a session with a sanitized name, and `MockTrackerSource` provides a VecDeque-based test double matching the `MockSshClient` pattern. Wired template validation into `ServerConfig::load()` at startup. Added `Clone` derives to all manifest types (required for template cloning) and `#[serde(default)]` on `JobManifest.session` (required for zero-session templates; existing `validate()` still catches missing sessions in normal use). Template loading intentionally skips `JobManifest::validate()` since that validator requires ≥1 session, which contradicts the template requirement.

## Verification

- `cargo test -p smelt-cli --lib -- serve::tracker` — 14 tests pass
- `cargo test -p smelt-cli --lib -- serve::config` — 11 tests pass (including template-at-startup tests)
- `cargo test -p smelt-core` — 175 tests pass including TrackerState label round-trip, StateBackendConfig serde, and state_backend manifest tests
- `cargo test --workspace` — 337 tests pass, 0 failures, 0 regressions
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Requirements Advanced

- R072 (TrackerSource trait abstraction) — trait defined with RPITIT, proven by compile-test and MockTrackerSource
- R073 (Template manifest with issue injection) — load_template_manifest() + issue_to_manifest() proven by unit tests
- R074 (Label-based lifecycle state machine) — TrackerState enum with label_name() proven by unit tests

## Requirements Validated

None from this slice alone — R072/R073/R074 require S03/S04 backends to be fully validated.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

- **`#[serde(default)]` on `JobManifest.session`** — not in original plan but necessary for zero-session template manifests to parse. Existing `validate()` still enforces ≥1 session for non-template use.
- **`Clone` derives on all manifest types** — not in original plan but required for `issue_to_manifest()` template cloning.
- **`load_template_manifest()` skips `JobManifest::validate()`** — validator requires ≥1 session which contradicts templates. `deny_unknown_fields` TOML parsing provides sufficient structural validation.
- **`state_backend` added to `JobManifest` in S02 not S05** — D160 explains why: `deny_unknown_fields` would reject template manifests containing `[state_backend]` without this field; backward-compatible `Option` makes early addition safe.
- **`validate_no_sessions` test in smelt-core updated** — test expected parsing to fail; after adding `#[serde(default)]` on session, parsing now succeeds and validation catches the error at validate() time.

## Known Limitations

- `TrackerSource` has no `issue_to_manifest()` method on the trait — it's a free function. This is intentional (D161) but means backends don't express manifest generation as a first-class trait capability.
- No concrete backend implementations exist yet — S03 (GitHub) and S04 (Linear) are separate slices.
- `MockTrackerSource` is `pub(crate)` inside `#[cfg(test)]` — it's available for intra-crate tests but S03/S04 will need to replicate or re-export the pattern for their own test doubles.
- No runtime hookup — all types and functions exist at the contract level; `TrackerPoller` integration into `smelt serve`'s dispatch loop is S05.

## Follow-ups

- S03 should reuse `issue_to_manifest()` as a free function — the entire injection logic is already written and tested.
- S03/S04 will need to handle `MockTrackerSource` visibility if integration tests span crates — consider whether `pub` export is needed at that point.
- S05 must add `state_backend` serialization into AssayInvoker's RunManifest generation — the field is already on `JobManifest` and `StateBackendConfig` has full serde support.

## Files Created/Modified

- `crates/smelt-core/src/tracker.rs` — New file: TrackerIssue, TrackerState, StateBackendConfig, unit tests
- `crates/smelt-core/src/error.rs` — Added Tracker variant and tracker() constructor
- `crates/smelt-core/src/manifest/mod.rs` — Added state_backend field, Clone derives, serde(default) on session
- `crates/smelt-core/src/lib.rs` — Exported pub mod tracker
- `crates/smelt-core/src/compose.rs` — Added state_backend: None to test helper
- `crates/smelt-core/src/manifest/tests/core.rs` — Updated validate_no_sessions, added state_backend tests
- `crates/smelt-cli/src/serve/config.rs` — Added TrackerConfig, ServerConfig.tracker, validation, template startup check
- `crates/smelt-cli/src/serve/tracker.rs` — New file: TrackerSource trait, load_template_manifest, issue_to_manifest, sanitize, MockTrackerSource, 14 unit tests
- `crates/smelt-cli/src/serve/types.rs` — Added JobSource::Tracker variant
- `crates/smelt-cli/src/serve/mod.rs` — Registered tracker module
- `crates/smelt-cli/tests/docker_lifecycle.rs` — Added state_backend: None to test helper
- `crates/smelt-cli/tests/compose_lifecycle.rs` — Added state_backend: None to test helper
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — Added state_backend: None to test helper

## Forward Intelligence

### What the next slice should know
- `issue_to_manifest()` in `serve/tracker.rs` is the complete injection function — S03/S04 should call it directly, not reimplement it
- `TrackerState::label_name(prefix)` produces `"{prefix}:{state}"` strings; `label_prefix` in `TrackerConfig` defaults to `"smelt"` — use these together for GitHub label management
- `JobManifest` now has `Clone` — template cloning via `.clone()` works without any special handling
- `MockTrackerSource` is in a `#[cfg(test)] pub(crate) mod mock` block inside `serve/tracker.rs` — import as `crate::serve::tracker::mock::MockTrackerSource` in smelt-cli tests

### What's fragile
- `#[serde(default)]` on `session` field means a template manifest without `[[session]]` parses to an empty Vec — the zero-session check is in `load_template_manifest()`, not at parse time. Any code path that calls `JobManifest::load()` directly (not `load_template_manifest()`) will accept zero-session manifests at parse time; `validate()` catches them but only when called explicitly.
- `StateBackendConfig::Custom` uses `toml::Value` — this is fine for TOML round-trips but requires a conversion step if JSON serialization is ever needed (D163).

### Authoritative diagnostics
- Template load failures: `ServerConfig::load()` error message includes file path and reason (file not found OR "must not contain [[session]] entries")
- Tracker config validation: `ServerConfig::validate()` collects all errors; look for newline-separated items in the error message
- Label lifecycle: `TrackerState::label_name("smelt")` is the canonical source of label strings — grep for this in S03/S04 implementations

### What assumptions changed
- Original plan assumed `JobManifest::validate()` could be called for template validation — it cannot because validate() requires ≥1 session. Template validation relies on TOML parsing with `deny_unknown_fields` plus an explicit zero-session check.
- Original plan had `state_backend` passthrough as S05 work — moved to S02 because `deny_unknown_fields` would break template parsing otherwise.
