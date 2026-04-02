---
id: T03
parent: S02
milestone: M012
provides:
  - load_template_manifest() — loads and validates template manifests at startup (D017)
  - issue_to_manifest() — clones template, injects sanitized session from TrackerIssue
  - sanitize() helper — lowercases, replaces non-alnum with hyphens, collapses/trims
  - MockTrackerSource — VecDeque-based test double for TrackerSource trait
  - ServerConfig::load() validates template manifest at startup (D017)
  - Clone derives on all JobManifest types (JobManifest, JobMeta, Environment, CredentialConfig, SessionDef, MergeConfig, ComposeService, KubernetesConfig)
  - serde(default) on JobManifest.session to support zero-session template manifests
key_files:
  - crates/smelt-cli/src/serve/tracker.rs
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-core/src/manifest/mod.rs
key_decisions:
  - "Skipped JobManifest::validate() in load_template_manifest because validate() requires ≥1 session — templates intentionally have zero. TOML parse via deny_unknown_fields is sufficient structural validation."
  - "Added #[serde(default)] to JobManifest.session field so templates can omit [[session]] entirely. Existing validate() still catches zero-session manifests during normal (non-template) use."
  - "Added Clone derive to all manifest types (JobManifest, JobMeta, Environment, CredentialConfig, SessionDef, MergeConfig, ComposeService, KubernetesConfig) to enable template cloning in issue_to_manifest."
patterns_established:
  - "MockTrackerSource follows same VecDeque<Result> pattern as MockSshClient — new(), with_poll_result(), with_transition_result() builder methods"
  - "Template manifest = normal manifest with zero sessions, loaded via load_template_manifest(), sessions injected dynamically by issue_to_manifest()"
observability_surfaces:
  - "tracing::info! on successful template manifest load at startup"
  - "Descriptive anyhow errors on template load failure, session-present rejection, and file-not-found"
  - "ServerConfig::load() fails fast with clear message when template is invalid (D017)"
duration: 20min
verification_result: passed
completed_at: 2026-03-27T12:00:00Z
blocker_discovered: false
---

# T03: Template manifest loading, issue injection, and MockTrackerSource

**Added load_template_manifest, issue_to_manifest with title sanitization, MockTrackerSource test double, and wired template validation into ServerConfig::load() at startup (D017)**

## What Happened

Implemented three core functions in `serve/tracker.rs`: `load_template_manifest()` loads and validates template manifests (rejecting those with `[[session]]` entries), `issue_to_manifest()` clones a template and injects a session from a `TrackerIssue` with a sanitized name, and a private `sanitize()` helper that lowercases and replaces non-alphanumeric chars with hyphens.

Wired template loading into `ServerConfig::load()` — when a `[tracker]` section is present, the template manifest is validated at startup (D017), failing fast with a descriptive error.

Added `MockTrackerSource` in a `#[cfg(test)] pub(crate) mod mock` block following the `MockSshClient` VecDeque pattern. It implements `TrackerSource` with configurable poll and transition result queues.

To support `issue_to_manifest`'s template cloning, added `Clone` derives to all manifest types in `smelt-core`. Also added `#[serde(default)]` to `JobManifest.session` so template manifests can omit the `[[session]]` array entirely while the existing `validate()` still catches zero-session manifests in normal use.

Updated the `validate_no_sessions` test in smelt-core to reflect that parsing now succeeds (session defaults to empty) and validation catches the error.

## Verification

- `cargo test -p smelt-cli --lib -- serve::tracker` — 14 tests pass (template load, session rejection, injection, sanitize, mock exercises, trait compile-test)
- `cargo test -p smelt-cli --lib -- serve::config` — 11 tests pass (including new template-at-startup tests)
- `cargo test -p smelt-core` — 169 tests pass (updated validate_no_sessions test)
- `cargo test --workspace` — 337 tests pass, zero failures
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Diagnostics

- Failed template load at startup: `ServerConfig::load()` returns descriptive error including file path and reason
- Template with sessions: error says "template manifest must not contain [[session]] entries"
- Nonexistent template file: error says "failed to load template manifest <path>"
- `tracing::info!` logged on successful template load with file path

## Deviations

- Did not call `JobManifest::validate()` in `load_template_manifest()` — the validator requires at least one session, which contradicts the template requirement of zero sessions. The TOML parser with `deny_unknown_fields` provides sufficient structural validation.
- Added `#[serde(default)]` on `JobManifest.session` and `Clone` on all manifest types — not in original plan but necessary for the implementation to work.
- Removed `#[allow(dead_code)]` from `TrackerConfig.manifest_template` since the field is now read by `ServerConfig::load()`.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/tracker.rs` — Added load_template_manifest(), issue_to_manifest(), sanitize(), MockTrackerSource, and 14 unit tests
- `crates/smelt-cli/src/serve/config.rs` — Wired template validation into ServerConfig::load(), updated tests to use real template files, added 3 new tests
- `crates/smelt-core/src/manifest/mod.rs` — Added Clone derives to all manifest types, added serde(default) on session field
- `crates/smelt-core/src/manifest/tests/core.rs` — Updated validate_no_sessions test for new parse behavior
