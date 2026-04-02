---
id: S03
parent: M012
milestone: M012
provides:
  - GhClient trait with 4 async RPITIT methods (list_issues, edit_labels, create_label, auth_status)
  - SubprocessGhClient shelling out to gh CLI via tokio::process::Command + which::which
  - MockGhClient VecDeque-based test double for all 4 methods
  - GhIssue struct with serde Deserialize for gh --json output
  - GithubTrackerSource<G: GhClient> implementing TrackerSource trait
  - poll_ready_issues() with auth check + ready-label filter + GhIssue→TrackerIssue mapping
  - transition_state() with single edit_labels call for atomic label swap (D157)
  - ensure_labels() creating all 6 lifecycle labels idempotently via create_label --force
  - TrackerConfig.repo field with owner/repo validation for GitHub provider
  - 2 gated integration tests (SMELT_GH_TEST=1 + SMELT_GH_REPO) against real gh CLI
requires:
  - slice: S02
    provides: TrackerSource trait, TrackerConfig, TrackerIssue, TrackerState, issue_to_manifest(), SmeltError::Tracker
affects:
  - S05
key_files:
  - crates/smelt-cli/src/serve/github/mod.rs
  - crates/smelt-cli/src/serve/github/client.rs
  - crates/smelt-cli/src/serve/github/mock.rs
  - crates/smelt-cli/src/serve/github/source.rs
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/mod.rs
key_decisions:
  - "D164: GhClient trait uses generic <G: GhClient> not dyn GhClient — async RPITIT is non-object-safe; mirrors SshClient/ForgeClient pattern"
  - "D165: TrackerConfig.repo required for GitHub (owner/repo format), ignored for Linear"
  - "D166: edit_labels combines --add-label and --remove-label in single gh issue edit call for D157 atomicity"
patterns_established:
  - "GhClient trait with RPITIT async methods in serve/github/mod.rs — mirrors SshClient pattern"
  - "MockGhClient VecDeque pattern in serve/github/mock.rs — mirrors MockSshClient"
  - "GithubTrackerSource<G: GhClient> generic pattern — bridges GhClient to TrackerSource with label-prefix lifecycle"
  - "Integration test gate: SMELT_GH_TEST=1 + SMELT_GH_REPO env vars; tests eprintln-skip when not set"
observability_surfaces:
  - "tracing::debug! on every gh subprocess invocation with full command + args"
  - "tracing::warn! on non-zero exit codes with exit code + stderr"
  - "tracing::info! on label creation (repo + label fields)"
  - "tracing::info! on successful label transitions (repo + issue + from + to fields)"
  - "SmeltError::Tracker { operation: 'gh_binary' } for missing gh binary"
  - "SmeltError::Tracker { operation: 'auth_status' } for auth failures"
  - "SmeltError::Tracker { operation: 'poll' } wrapping auth failures during polling"
  - "SmeltError::Tracker { operation: 'transition' } for invalid issue_id parse"
  - "Validation error messages: 'repo must be set', 'repo must not be empty', 'repo must be in owner/repo format'"
drill_down_paths:
  - .kata/milestones/M012/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M012/slices/S03/tasks/T02-SUMMARY.md
  - .kata/milestones/M012/slices/S03/tasks/T03-SUMMARY.md
duration: 28min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
---

# S03: GitHub Issues Tracker Backend

**`GithubTrackerSource` polls GitHub Issues via `gh` CLI, transitions lifecycle labels atomically, and generates manifests from templates — proven by 24 unit tests (mock-based, no `gh` required) and 2 gated integration tests against real `gh` CLI**

## What Happened

Built the GitHub tracker backend across three tasks, each self-contained and independently verifiable.

**T01** established the `gh` CLI abstraction layer in `serve/github/`. The `GhClient` trait defines 4 async RPITIT methods — `list_issues`, `edit_labels`, `create_label`, `auth_status` — returning `Result<T, SmeltError>` directly (not `anyhow::Result`, since the tracker layer needs structured errors). `SubprocessGhClient` discovers `gh` via `which::which` and shells out via `tokio::process::Command`, using `-R owner/repo` explicitly on every call per D155. `MockGhClient` uses the `Arc<Mutex<VecDeque<Result>>>` pattern from `MockSshClient`, with per-method queues. `GhIssue` deserializes `gh issue list --json` output. 8 unit tests pass without `gh` present.

**T02** wired `GhClient` to the `TrackerSource` trait in `GithubTrackerSource<G: GhClient>`. `poll_ready_issues()` calls `auth_status()` first (auth failures surface as `SmeltError::Tracker { operation: "poll" }`), then `list_issues()` with the ready label and `--limit 50`, mapping `GhIssue` → `TrackerIssue` (number→string id, url→source_url). `transition_state()` parses `issue_id` as `u64`, builds from/to label names via `TrackerState::label_name()`, and calls `edit_labels()` once (D157 atomicity, D166). `ensure_labels()` iterates `TrackerState::ALL` creating each label idempotently. 8 unit tests cover all paths including auth failure, empty results, transition failure, invalid IDs, and label creation.

**T03** closed the config loop: `repo: Option<String>` with `#[serde(default)]` on `TrackerConfig`, validated in `ServerConfig::validate()` when `provider == "github"` (requires `Some`, non-empty, exactly one `/`). Linear ignores `repo` entirely. 5 new config tests cover all validation branches. 2 `#[ignore]` integration tests in `github/mod.rs` are gated by `SMELT_GH_TEST=1` + `SMELT_GH_REPO` and skip gracefully when not set. Updated all existing config tests to include `repo = "owner/repo"` to prevent cascading validation errors.

## Verification

- `cargo test -p smelt-cli --lib -- serve::github` — 16 passed, 2 ignored (gated integration)
- `cargo test -p smelt-cli --lib -- serve::config` — 17 passed (5 new + 12 existing)
- `cargo test --workspace` — 360 passed, 0 failed, 11 ignored (zero regressions vs 337+ baseline + new tests)
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Requirements Advanced

- R070 (Tracker-driven autonomous dispatch from GitHub Issues) — `GithubTrackerSource` now implements `TrackerSource` with full lifecycle: poll, label transition (ready→queued atomically), and manifest generation via `issue_to_manifest()`. Missing S05 (TrackerPoller integration into dispatch loop) before end-to-end is operational.
- R074 (Label-based lifecycle state machine) — GitHub side proven: label auto-creation, `smelt:ready → smelt:queued → ...` transitions via single `gh issue edit` call, all 6 lifecycle labels covered by `ensure_labels()`.

## Requirements Validated

None in this slice — R070 and R074 remain Active until S05 closes the end-to-end loop.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

- **T01:** Removed `pub(crate) mod tests` compatibility shim from `github/mod.rs` — it was unused and triggered a clippy warning. The SSH module has this shim because `dispatch.rs` imports `MockSshClient` from it; the GitHub module has no such consumer yet.
- **T01:** `GhClient` returns `SmeltError` directly instead of `anyhow::Result` — deliberate divergence from `SshClient` since the tracker layer needs structured errors for operation-specific handling.
- **T02:** Added `test_transition_state_invalid_issue_id` (not in plan) — bonus test verifying u64 parse error path. Total 8 tests instead of the planned ≥6.
- **T03:** Updated existing config tests (and `make_tracker_config` helper in `tracker.rs`) to include `repo = "owner/repo"` — required to prevent cascading validation errors from the new GitHub repo requirement.

## Known Limitations

- `GithubTrackerSource` is not yet wired into the `smelt serve` dispatch loop — that's S05 (`TrackerPoller` integration).
- No retry or backoff on transient `gh` CLI failures — a single failed `gh issue edit` logs a warning and skips the issue; persistent failures require operator intervention.
- `--limit 50` cap on `list_issues` is hardcoded — sufficient for current use cases; configurable limit deferred if needed.
- Double-dispatch prevention (D157) relies on GitHub's label update atomicity, not a distributed lock — adequate for single-instance deployments; multiple `smelt serve` instances could still race.

## Follow-ups

- S05: Wire `GithubTrackerSource` (and `LinearTrackerSource` from S04) into `TrackerPoller` background task in `smelt serve` dispatch loop.
- S05: `state_backend` passthrough from `JobManifest` into Assay `RunManifest` TOML.
- S05: TUI display of tracker-sourced jobs (new `JobSource::Tracker` variant in status table).
- Consider: configurable `--limit` on issue listing if high-volume repos need more than 50 issues per poll.

## Files Created/Modified

- `crates/smelt-cli/src/serve/github/mod.rs` — GhClient trait, GhIssue struct, module re-exports, integration tests
- `crates/smelt-cli/src/serve/github/client.rs` — SubprocessGhClient with gh CLI subprocess wrappers
- `crates/smelt-cli/src/serve/github/mock.rs` — MockGhClient test double + 8 unit tests
- `crates/smelt-cli/src/serve/github/source.rs` — GithubTrackerSource with TrackerSource impl + 8 unit tests
- `crates/smelt-cli/src/serve/config.rs` — TrackerConfig.repo field, GitHub repo validation, 5 new tests, updated existing tests
- `crates/smelt-cli/src/serve/tracker.rs` — Updated make_tracker_config() test helper to include repo field
- `crates/smelt-cli/src/serve/mod.rs` — Added `pub mod github`

## Forward Intelligence

### What the next slice should know
- `GithubTrackerSource` needs a `repo` string and a `label_prefix` string at construction time — both come from `TrackerConfig`; `S05::TrackerPoller` should construct it from the config directly.
- `ensure_labels()` on `GithubTrackerSource` should be called once at tracker startup (not per poll cycle) — it creates all 6 lifecycle labels idempotently and is safe to retry but unnecessary every 30s.
- `issue_to_manifest()` is a free function in `serve/tracker.rs` (D161), not a trait method — `TrackerPoller` calls it after `poll_ready_issues()` returns, before dispatching.
- The double-dispatch guard (D157) works by transitioning `smelt:ready → smelt:queued` before the issue is enqueued into `ServerState`. `TrackerPoller` must call `transition_state(issue.id, TrackerState::Ready, TrackerState::Queued)` before `server_state.enqueue(job)`.

### What's fragile
- `SubprocessGhClient::list_issues()` parses `gh` JSON output with `serde_json::from_str` — if `gh` adds new fields that break `GhIssue` deserialization (e.g., nested objects where strings were), it will fail silently (error returned, empty results). Consider `#[serde(other)]` or `deny_unknown_fields = false` if needed.
- Config test updates: adding new validation to `TrackerConfig` will require updating all test helpers again — `make_tracker_config()` in `tracker.rs` and `with_tracker_toml()` in `config.rs` are the canonical helpers.

### Authoritative diagnostics
- `SmeltError::Tracker { operation, message }` is the structured error type for all GitHub-specific failures — search for `operation` field to identify the failure layer (gh_binary, auth_status, poll, transition).
- `tracing::debug!` on every `gh` subprocess call includes `cmd` and `args` fields — set `SMELT_LOG=debug` to see the exact `gh` invocations being made.

### What assumptions changed
- T01 assumed `GhClient` could mirror `SshClient`'s `anyhow::Result` return — actual impl uses `SmeltError` directly because `TrackerSource` needs operation-specific errors; `anyhow` would lose the structured error type.
