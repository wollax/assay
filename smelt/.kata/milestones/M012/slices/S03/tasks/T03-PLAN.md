---
estimated_steps: 4
estimated_files: 3
---

# T03: TrackerConfig.repo field, validation, and integration tests

**Slice:** S03 — GitHub Issues Tracker Backend
**Milestone:** M012

## Description

Add `repo: Option<String>` to `TrackerConfig`, enforce it when `provider == "github"` (validated at startup per D018), and add gated integration tests that exercise `SubprocessGhClient` against a real GitHub repo. This closes the config→runtime loop: a `server.toml` with `[tracker] provider = "github"` now requires a `repo` field, and the full stack is provable against real `gh` CLI.

## Steps

1. Add `repo: Option<String>` with `#[serde(default)]` to `TrackerConfig` in `config.rs`. In `ServerConfig::validate()` tracker validation block, add:
   - When `provider == "github"`: require `repo` is `Some`, non-empty, and contains exactly one `/` (owner/repo format)
   - When `provider == "linear"`: `repo` is ignored (no validation)
   - Collect errors per D018 (add to existing `tracker_errors` vec)

2. Add config unit tests in `config.rs`:
   - `test_tracker_github_requires_repo` — github provider without repo → error
   - `test_tracker_github_invalid_repo_format` — repo without `/` → error; repo with multiple `/` → error
   - `test_tracker_github_valid_repo` — `"owner/repo"` accepted
   - `test_tracker_linear_ignores_repo` — linear provider without repo → ok
   - `test_tracker_github_repo_empty_string` — `repo = ""` → error
   - Update existing `tracker_section_parses_correctly` test to include `repo` field

3. Add `#[ignore]` integration tests in `crates/smelt-cli/src/serve/github/mod.rs` (or a separate `tests` block):
   - Gated by `SMELT_GH_TEST=1` and `SMELT_GH_REPO` env var
   - `test_gh_auth_status_integration` — `SubprocessGhClient.auth_status()` returns Ok
   - `test_gh_list_issues_integration` — `SubprocessGhClient.list_issues(repo, "nonexistent-label-xyz", 5)` returns Ok with empty vec (no issues match)
   - Both tests skip gracefully (eprintln + return) when env vars are not set

4. Run full verification: `cargo test --workspace` (all 337+ tests pass), `cargo clippy --workspace -- -D warnings`, `cargo doc --workspace --no-deps`

## Must-Haves

- [ ] `TrackerConfig` has `repo: Option<String>` field with `#[serde(default)]`
- [ ] `provider == "github"` requires repo in `owner/repo` format
- [ ] `provider == "linear"` ignores repo field
- [ ] Validation errors collected per D018 (not fail-fast)
- [ ] ≥5 config unit tests covering repo validation scenarios
- [ ] ≥2 gated integration tests (`SMELT_GH_TEST=1`) exercising real `gh` CLI
- [ ] All 337+ workspace tests pass (zero regressions)

## Verification

- `cargo test -p smelt-cli --lib -- serve::config` — all config tests pass including new repo validation
- `cargo test --workspace` — all tests pass, zero regressions
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings
- Manual: `SMELT_GH_TEST=1 SMELT_GH_REPO=owner/repo cargo test -p smelt-cli -- --ignored github` — integration tests pass

## Observability Impact

- Signals added/changed: Validation error messages include repo field context (missing, invalid format)
- How a future agent inspects this: check `ServerConfig::validate()` error output for `"repo must be"` or `"owner/repo format"` strings
- Failure state exposed: Invalid repo format produces a descriptive error at startup, not mid-dispatch

## Inputs

- `crates/smelt-cli/src/serve/config.rs` — existing TrackerConfig struct and validate() method
- `crates/smelt-cli/src/serve/github/client.rs` — SubprocessGhClient from T01
- S02 Summary — TrackerConfig field patterns, deny_unknown_fields

## Expected Output

- `crates/smelt-cli/src/serve/config.rs` — TrackerConfig with repo field + validation + tests
- `crates/smelt-cli/src/serve/github/mod.rs` — integration tests (gated)
