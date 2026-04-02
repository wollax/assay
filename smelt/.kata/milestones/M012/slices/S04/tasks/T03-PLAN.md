---
estimated_steps: 4
estimated_files: 2
---

# T03: TrackerConfig Linear fields and validation

**Slice:** S04 — Linear Tracker Backend
**Milestone:** M012

## Description

Extend `TrackerConfig` with `api_key_env` and `team_id` fields required for the Linear provider. Add validation rules to `ServerConfig::validate()` that require both fields when `provider == "linear"`. Add integration test for Linear config end-to-end (gated by env var). Ensure all existing config tests pass without modification.

## Steps

1. **Add fields to `TrackerConfig`** in `serve/config.rs`:
   - `api_key_env: Option<String>` with `#[serde(default)]`
   - `team_id: Option<String>` with `#[serde(default)]`
   - Both follow the `repo: Option<String>` pattern (D165)

2. **Add Linear validation** to `ServerConfig::validate()`:
   - When `provider == "linear"`, require `api_key_env` is `Some` and non-empty
   - When `provider == "linear"`, require `team_id` is `Some` and non-empty
   - Collect errors per D018 (alongside existing tracker validation)

3. **Write unit tests** for Linear config:
   - `test_tracker_linear_requires_api_key_env` — missing → error
   - `test_tracker_linear_requires_team_id` — missing → error
   - `test_tracker_linear_empty_api_key_env_rejected` — empty string → error
   - `test_tracker_linear_empty_team_id_rejected` — empty string → error
   - `test_tracker_linear_valid_config` — both present → success (needs template file)
   - `test_tracker_github_ignores_linear_fields` — GitHub provider with api_key_env/team_id absent → success
   - `test_tracker_linear_multiple_errors_collected` — both missing → both errors in one message

4. **Verify no regressions**: Run full workspace tests, clippy, docs.

## Must-Haves

- [ ] `TrackerConfig` has `api_key_env: Option<String>` and `team_id: Option<String>` fields
- [ ] Validation requires both fields when `provider == "linear"`
- [ ] Validation ignores both fields when `provider == "github"`
- [ ] Error messages are collected (D018) — multiple missing fields produce a single multi-error message
- [ ] All existing config tests pass unchanged
- [ ] 7 new config tests pass

## Verification

- `cargo test -p smelt-cli --lib -- serve::config` — all config tests pass (existing + 7 new)
- `cargo test --workspace` — all tests pass, zero regressions
- `cargo clippy --workspace -- -D warnings` — clean
- `cargo doc --workspace --no-deps` — clean

## Observability Impact

- Signals added/changed: None (validation is startup-time only)
- How a future agent inspects this: startup error message includes all invalid fields with clear labels
- Failure state exposed: `"invalid tracker configuration:\n  api_key_env must be set when provider is \"linear\"\n  team_id must be set when provider is \"linear\""` — multi-line error with all issues listed

## Inputs

- `crates/smelt-cli/src/serve/config.rs` — existing TrackerConfig and validation logic
- D165 pattern: `repo: Option<String>` validated only for GitHub provider
- D018 pattern: error collection before returning

## Expected Output

- `crates/smelt-cli/src/serve/config.rs` — Updated TrackerConfig with 2 new fields, extended validation, 7 new tests
