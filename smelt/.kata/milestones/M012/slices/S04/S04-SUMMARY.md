---
id: S04
parent: M012
milestone: M012
provides:
  - LinearClient trait with 5 async RPITIT methods (list_issues, add_label, remove_label, find_label, create_label)
  - ReqwestLinearClient production impl using async reqwest::Client with GraphQL variables
  - MockLinearClient VecDeque-based test double (mirrors MockGhClient pattern)
  - LinearTrackerSource<L: LinearClient> implementing TrackerSource trait
  - ensure_labels() with find-or-create logic and HashMap<String, String> UUID cache
  - TrackerConfig extended with api_key_env and team_id fields + Linear validation
  - reqwest promoted to production dependency in smelt-cli
requires:
  - slice: S02
    provides: TrackerSource trait, TrackerConfig, TrackerIssue/TrackerState, issue_to_manifest(), SmeltError::tracker()
affects:
  - S05
key_files:
  - crates/smelt-cli/src/serve/linear/mod.rs
  - crates/smelt-cli/src/serve/linear/client.rs
  - crates/smelt-cli/src/serve/linear/mock.rs
  - crates/smelt-cli/src/serve/linear/source.rs
  - crates/smelt-cli/src/serve/mod.rs
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/Cargo.toml
key_decisions:
  - "LinearClient trait uses RPITIT async methods with Send bounds (mirrors GhClient per D164)"
  - "ReqwestLinearClient uses GraphQL variables instead of string interpolation for injection safety"
  - "GraphQL error extraction checks errors array on HTTP 200 (Assay LinearBackend pattern)"
  - "TrackerIssue.id = Linear UUID (not identifier like KAT-42) — mutations need UUIDs, consumers treat id as opaque"
  - "Linear validation follows exact same match/None/Some-empty/Some-valid pattern as GitHub repo validation (D165)"
  - "LinearIssue.description defaults to empty string via serde(default) for null/missing fields"
patterns_established:
  - "serve/linear/ module structure mirrors serve/github/ exactly: mod.rs (trait+types), client.rs (production impl), mock.rs (test double), source.rs (TrackerSource impl)"
  - "MockLinearClient uses VecDeque per-method queues with Arc<Mutex<>> — same pattern as MockGhClient"
  - "Provider-specific validation blocks in validate() — each provider has its own if-block checking required fields"
  - "Label UUID caching via HashMap<String, String> populated by ensure_labels() — required before transition_state()"
observability_surfaces:
  - "tracing::debug! on every GraphQL request (operation name, URL, query preview first 80 chars)"
  - "tracing::warn! on GraphQL error responses (with error text) and non-200 HTTP status codes"
  - "tracing::info! on each label ensured (team_id, label_name, action: found/created)"
  - "tracing::info! on successful state transition (issue_id, from, to)"
  - "SmeltError::tracker(operation, message) with operation names matching trait method names"
  - "Auth header marked .set_sensitive(true) to prevent reqwest from logging credentials"
drill_down_paths:
  - .kata/milestones/M012/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M012/slices/S04/tasks/T02-SUMMARY.md
  - .kata/milestones/M012/slices/S04/tasks/T03-SUMMARY.md
duration: 30min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
---

# S04: Linear Tracker Backend

**Linear GraphQL tracker backend with trait abstraction, reqwest client, label UUID caching, and startup validation — 19 unit tests + 24 config tests, zero regressions**

## What Happened

Built the Linear tracker backend in three focused tasks, following the GitHub tracker backend's patterns exactly.

**T01** created the `serve/linear/` module as a direct mirror of `serve/github/`. The `LinearClient` trait defines 5 async RPITIT methods for GraphQL operations. `ReqwestLinearClient` sends POSTs to `{base_url}/graphql` using GraphQL variables (not string interpolation) and extracts errors from both the `errors` array on HTTP 200 (following Assay's `LinearBackend` pattern) and non-200 status codes. `MockLinearClient` uses the VecDeque-per-method-queue pattern from `MockGhClient`. Promoted `reqwest` from dev-dependency to production dependency.

**T02** implemented `LinearTrackerSource<L: LinearClient>` bridging the Linear client to the platform-agnostic `TrackerSource` trait. The key design decision: `TrackerIssue.id` uses the Linear UUID (not the human-readable identifier like "KAT-42") since mutations require UUIDs and the id is treated as opaque by consumers. `ensure_labels()` iterates `TrackerState::ALL`, calling `find_label()` first and only `create_label()` if not found, caching each UUID in `HashMap<String, String>`. `transition_state()` performs remove-old + add-new as two sequential mutations, failing with a clear error on cache miss. Wrote 10 unit tests covering all paths.

**T03** extended `TrackerConfig` with `api_key_env: Option<String>` and `team_id: Option<String>` fields and added a Linear-specific validation block in `ServerConfig::validate()` requiring both fields when `provider == "linear"`. Errors are collected D018-style (all errors in one message) and the validation follows the exact same `match`/None/Some-empty/Some-valid pattern as GitHub's repo validation. Fixed `make_tracker_config()` in tracker.rs tests to include the new fields. Wrote 7 new config validation tests.

## Verification

- `cargo test -p smelt-cli --lib -- serve::linear` — 19 tests pass (8 mock, 1 compile, 10 source)
- `cargo test -p smelt-cli --lib -- serve::config` — 24 tests pass (17 existing + 7 new)
- `cargo test --workspace` — 386 total tests pass across all crates, 0 failed, 11 ignored
- `cargo clippy --workspace -- -D warnings` — clean, zero warnings
- `cargo doc --workspace --no-deps` — clean, zero warnings

## Requirements Advanced

- R071 (Tracker-driven autonomous dispatch from Linear) — `LinearTrackerSource` now fully implements `TrackerSource` with poll, transition, and manifest generation. Waiting on S05 to wire into `dispatch_loop`.
- R072 (TrackerSource trait abstraction) — Linear backend proves the trait is implementable by a second concrete backend; abstract dispatch loop only needs `Arc<dyn TrackerSource>`.
- R074 (Label-based lifecycle state machine) — Linear label lifecycle (smelt:ready → smelt:queued → … → smelt:done/failed) is implemented and tested.

## Requirements Validated

None — R071, R072, R074 need S05 end-to-end wiring before they can be marked validated.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

- **GraphQL variables vs. string interpolation**: The plan mentioned "string interpolation" as a possible approach; implementation used GraphQL variables throughout. This is safer and more idiomatic — not a functional deviation.
- **T03 tracker.rs fix**: `make_tracker_config()` in tracker.rs tests required updating to include `api_key_env: None` and `team_id: None` (since `TrackerConfig` uses `deny_unknown_fields` and direct struct construction must list all fields). Also updated `test_tracker_linear_ignores_repo` to supply valid Linear fields since it now goes through full validation.

## Known Limitations

- `LinearTrackerSource` is implemented but not yet wired into `TrackerPoller` or `dispatch_loop` — that's S05's job.
- No integration tests against a live Linear project yet — env-gated integration tests were scoped to S05.
- `ensure_labels()` must be called before `transition_state()` — there's no auto-initialization guard; S05 should ensure this is called at poller startup.

## Follow-ups

- S05: Wire `LinearTrackerSource` into `TrackerPoller` in `dispatch_loop` via `tokio::select!`.
- S05: Call `ensure_labels()` at poller startup before the first poll cycle.
- S05: Add env-gated integration tests against a real Linear project (`LINEAR_API_KEY` env var).

## Files Created/Modified

- `crates/smelt-cli/src/serve/linear/mod.rs` — New: LinearClient trait, LinearIssue/LinearLabel types, compile-test, re-exports
- `crates/smelt-cli/src/serve/linear/client.rs` — New: ReqwestLinearClient with GraphQL helper and 5 method implementations
- `crates/smelt-cli/src/serve/linear/mock.rs` — New: MockLinearClient with VecDeque queues, builder methods, 8 unit tests
- `crates/smelt-cli/src/serve/linear/source.rs` — New: LinearTrackerSource struct, ensure_labels(), TrackerSource impl, 10 unit tests
- `crates/smelt-cli/src/serve/mod.rs` — Added `pub mod linear`
- `crates/smelt-cli/src/serve/config.rs` — Added api_key_env/team_id fields, Linear validation block, 7 new tests
- `crates/smelt-cli/src/serve/tracker.rs` — Fixed make_tracker_config() to include new fields; updated test_tracker_linear_ignores_repo
- `crates/smelt-cli/Cargo.toml` — Promoted reqwest from dev-dep to production dep

## Forward Intelligence

### What the next slice should know
- `LinearTrackerSource::new()` takes `LinearClient` + `TrackerConfig` + `team_id: String`. The `team_id` comes from `config.team_id.as_deref().unwrap_or_default()` — validation guarantees it's present for the Linear provider.
- The `api_key_env` field in `TrackerConfig` is the *name* of the env var (e.g. `"LINEAR_API_KEY"`), not the key itself — `ReqwestLinearClient::new()` calls `std::env::var(api_key_env)` at construction time.
- `ensure_labels()` is `async` and must be awaited before the first `transition_state()` call — it's not called automatically in `new()`.
- `TrackerState::ALL` is used in `ensure_labels()` to provision all lifecycle labels upfront — S05 should verify this slice is still up to date if new states are added.

### What's fragile
- Two-mutation `transition_state()` (remove + add) is not atomic — if `add_label()` fails after `remove_label()` succeeds, the issue is in a label-less limbo. S05 should document this as a known gap and add a recovery path or idempotent retry.
- `ensure_labels()` populates the cache from the Linear API at startup — if the API is unreachable at startup, the poller will fail hard. S05 should add a startup-health-check pattern.

### Authoritative diagnostics
- `SmeltError::tracker("transition", "label '...' not in cache — was ensure_labels() called?")` — definitive signal that ensure_labels() was not called before transition_state()
- `tracing::warn!` output with "GraphQL errors:" prefix shows Linear API-returned errors verbatim
- `tracing::debug!` shows GraphQL query previews — useful for diagnosing wrong query structure

### What assumptions changed
- Original plan said "pattern from Assay's `LinearBackend`" — confirmed correct; the `graphql()` helper's error extraction logic (check `json["errors"]` on 200) matches Assay's implementation exactly.
- `TrackerState::ALL` was assumed to exist and iterate all variants — confirmed it does, 6 variants.
