---
id: T03
parent: S04
milestone: M012
provides:
  - TrackerConfig.api_key_env field (Option<String>) for Linear API key env var name
  - TrackerConfig.team_id field (Option<String>) for Linear team ID
  - Validation requiring both fields when provider is "linear"
  - 7 new config tests covering all Linear validation paths
  - D018-compliant multi-error collection for Linear fields
key_files:
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/tracker.rs
key_decisions:
  - "Linear validation follows exact same match/None/Some-empty/Some-valid pattern as GitHub repo validation (D165)"
patterns_established:
  - "Provider-specific validation blocks in validate() — each provider has its own if-block checking required fields"
observability_surfaces:
  - "Startup error message includes all invalid fields in one multi-line message: 'invalid tracker configuration:\\n  api_key_env must be set...\\n  team_id must be set...'"
duration: 8min
verification_result: passed
completed_at: 2025-06-28T12:00:00Z
blocker_discovered: false
---

# T03: TrackerConfig Linear fields and validation

**Extended TrackerConfig with `api_key_env` and `team_id` fields, Linear-specific validation in `validate()`, and 7 unit tests covering all Linear config paths**

## What Happened

Added two new optional fields (`api_key_env: Option<String>`, `team_id: Option<String>`) to `TrackerConfig` with `#[serde(default)]`, following the existing `repo` field pattern. Extended `ServerConfig::validate()` with a Linear-specific validation block that requires both fields when `provider == "linear"`, using the same `match` pattern as GitHub's repo validation — checking for `None` (missing) and empty/whitespace strings separately with distinct error messages.

Also fixed `make_tracker_config()` in `tracker.rs` tests to include the two new fields as `None` (since it directly constructs `TrackerConfig`).

Wrote 7 new tests: missing api_key_env, missing team_id, empty api_key_env, empty team_id (whitespace), valid linear config, github ignores linear fields, and multiple errors collected. All errors are collected per D018 before returning.

## Verification

- `cargo test -p smelt-cli --lib -- serve::config` — 24 tests passed (17 existing + 7 new)
- `cargo test --workspace` — 175 tests passed, 0 failed
- `cargo clippy --workspace -- -D warnings` — clean
- `cargo doc --workspace --no-deps` — clean

## Diagnostics

Startup error messages clearly list all invalid fields when Linear provider is misconfigured. Example: `"invalid tracker configuration:\n  api_key_env must be set when provider is \"linear\"\n  team_id must be set when provider is \"linear\""`.

## Deviations

Fixed `make_tracker_config()` in `tracker.rs` to include `api_key_env: None` and `team_id: None` — required because TrackerConfig uses `deny_unknown_fields` and direct struct construction must include all fields. Updated existing `test_tracker_linear_ignores_repo` to supply valid `api_key_env`/`team_id` since it now needs them to pass validation.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/config.rs` — Added api_key_env/team_id fields, Linear validation block, 7 new tests
- `crates/smelt-cli/src/serve/tracker.rs` — Fixed make_tracker_config() to include new fields
