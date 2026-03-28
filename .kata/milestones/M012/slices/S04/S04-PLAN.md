# S04: Linear Tracker Backend

**Goal:** `LinearTrackerSource` polls Linear via GraphQL API, transitions labels, generates manifests from templates — proven by unit tests with mock HTTP and integration tests gated by env var.
**Demo:** `cargo test -p smelt-cli --lib -- serve::linear` passes all unit tests; `cargo test --workspace` passes with zero regressions; Linear config validated at startup.

## Must-Haves

- `LinearClient` trait with async RPITIT methods: `list_issues`, `add_label`, `remove_label`, `find_label`, `create_label`
- `ReqwestLinearClient` production impl using async `reqwest::Client` for GraphQL against `https://api.linear.app/graphql`
- `MockLinearClient` VecDeque-based test double matching `MockGhClient` pattern
- `LinearTrackerSource<L: LinearClient>` bridging to `TrackerSource` trait
- Label UUID caching via `ensure_labels()` at startup — `HashMap<String, String>` mapping label names to UUIDs
- `TrackerConfig` gains `api_key_env: Option<String>` and `team_id: Option<String>` fields, validated when `provider == "linear"`
- `reqwest` promoted from dev-dep to production dep in smelt-cli (with `json` feature)
- All existing 337+ workspace tests pass with zero regressions
- `cargo clippy --workspace -- -D warnings` clean
- `cargo doc --workspace --no-deps` clean

## Proof Level

- This slice proves: contract + integration (mock-based unit tests, env-gated live integration)
- Real runtime required: no (mock tests); yes (env-gated integration tests)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-cli --lib -- serve::linear` — all LinearClient trait, mock, and source tests pass
- `cargo test -p smelt-cli --lib -- serve::config` — existing + new Linear config validation tests pass
- `cargo test --workspace` — all 337+ tests pass, zero regressions
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Observability / Diagnostics

- Runtime signals: `tracing::info!` on successful label ensure; `tracing::debug!` on GraphQL request/response; `tracing::warn!` on GraphQL errors in `errors` array
- Inspection surfaces: `SmeltError::Tracker { operation, message }` with operation names (`list_issues`, `find_label`, `create_label`, `add_label`, `remove_label`) for structured error reporting
- Failure visibility: GraphQL `errors` array contents surfaced in error messages; HTTP status code included on non-200; label-not-found produces clear error with label name and team ID
- Redaction constraints: `LINEAR_API_KEY` value never logged — only the env var name; API key resolved at runtime via `std::env::var`

## Integration Closure

- Upstream surfaces consumed: `TrackerSource` trait (`serve/tracker.rs`), `TrackerConfig` (`serve/config.rs`), `TrackerIssue`/`TrackerState` (`smelt-core/tracker.rs`), `issue_to_manifest()` (`serve/tracker.rs`), `SmeltError::tracker()` (`smelt-core/error.rs`)
- New wiring introduced in this slice: `serve/linear/` module registered in `serve/mod.rs`; `TrackerConfig` extended with Linear-specific fields; `reqwest` promoted to production dep
- What remains before the milestone is truly usable end-to-end: S05 wires `LinearTrackerSource` into `TrackerPoller` in `dispatch_loop`, `state_backend` passthrough, TUI, docs

## Tasks

- [x] **T01: LinearClient trait, ReqwestLinearClient, and MockLinearClient** `est:45m`
  - Why: Foundation — defines the GraphQL abstraction layer, production HTTP client, and test double. Everything else in S04 builds on this.
  - Files: `crates/smelt-cli/src/serve/linear/mod.rs`, `crates/smelt-cli/src/serve/linear/client.rs`, `crates/smelt-cli/src/serve/linear/mock.rs`, `crates/smelt-cli/src/serve/mod.rs`, `crates/smelt-cli/Cargo.toml`
  - Do: Define `LinearClient` trait with 5 async RPITIT methods (list_issues, add_label, remove_label, find_label, create_label). Implement `ReqwestLinearClient` using async `reqwest::Client` with `Authorization` header and GraphQL JSON payloads. Port error extraction from Assay's `LinearClient` pattern (check `json["errors"]` on HTTP 200). Implement `MockLinearClient` as VecDeque test double matching `MockGhClient`. Promote `reqwest` from dev-dep to production dep in smelt-cli. Register `linear` module in `serve/mod.rs`.
  - Verify: `cargo test -p smelt-cli --lib -- serve::linear::mock` passes; `cargo test --workspace` passes; `cargo clippy --workspace -- -D warnings` clean
  - Done when: `LinearClient` trait compiles, `ReqwestLinearClient` builds without errors, `MockLinearClient` tests pass, all workspace tests green

- [x] **T02: LinearTrackerSource bridging LinearClient to TrackerSource** `est:40m`
  - Why: Connects the Linear-specific client to the platform-agnostic `TrackerSource` trait. Includes label UUID caching via `ensure_labels()` and the `transition_state` implementation (remove-old + add-new as two mutations).
  - Files: `crates/smelt-cli/src/serve/linear/source.rs`, `crates/smelt-cli/src/serve/linear/mod.rs`
  - Do: Implement `LinearTrackerSource<L: LinearClient>` with `ensure_labels()` that queries/creates all lifecycle labels and caches name→UUID in `HashMap`. Implement `TrackerSource::poll_ready_issues()` using `find_label` + `list_issues`. Implement `TrackerSource::transition_state()` as remove-old-label + add-new-label (two mutations, using cached UUIDs). Map Linear `identifier` (e.g. "KAT-42") to `TrackerIssue.id` and `description` to `TrackerIssue.body`. Write comprehensive unit tests using `MockLinearClient`.
  - Verify: `cargo test -p smelt-cli --lib -- serve::linear::source` passes; `cargo test --workspace` passes
  - Done when: `LinearTrackerSource` implements `TrackerSource`, all poll/transition/ensure_labels paths tested, mock tests green

- [x] **T03: TrackerConfig Linear fields and validation** `est:30m`
  - Why: Extends config to support Linear provider — `api_key_env` and `team_id` required when `provider == "linear"`. Completes the startup validation path so bad Linear configs fail fast.
  - Files: `crates/smelt-cli/src/serve/config.rs`
  - Do: Add `api_key_env: Option<String>` and `team_id: Option<String>` to `TrackerConfig` with `#[serde(default)]`. Add validation in `ServerConfig::validate()`: when `provider == "linear"`, require both fields non-empty (collect errors per D018). Add unit tests for: Linear valid config, missing api_key_env rejected, missing team_id rejected, empty values rejected, fields ignored for github provider.
  - Verify: `cargo test -p smelt-cli --lib -- serve::config` passes; `cargo test --workspace` passes; `cargo clippy --workspace -- -D warnings` clean; `cargo doc --workspace --no-deps` clean
  - Done when: Linear config parses and validates correctly; all existing config tests still pass; new validation tests green

## Files Likely Touched

- `crates/smelt-cli/src/serve/linear/mod.rs` — New: trait, types, re-exports
- `crates/smelt-cli/src/serve/linear/client.rs` — New: ReqwestLinearClient
- `crates/smelt-cli/src/serve/linear/source.rs` — New: LinearTrackerSource
- `crates/smelt-cli/src/serve/linear/mock.rs` — New: MockLinearClient
- `crates/smelt-cli/src/serve/mod.rs` — Register linear module
- `crates/smelt-cli/src/serve/config.rs` — Add api_key_env, team_id fields + validation
- `crates/smelt-cli/Cargo.toml` — Promote reqwest to production dep
