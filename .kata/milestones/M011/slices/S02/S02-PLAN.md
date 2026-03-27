# S02: LinearBackend

**Goal:** `LinearBackend` implements all 7 `StateBackend` methods; `push_session_event` creates a Linear issue on first call and appends comments on subsequent calls; `read_run_state` fetches the latest comment and deserializes it; `capabilities()` returns the correct flags; contract tests with mock HTTP pass; `backend_from_config()` dispatches `Linear` to `LinearBackend`; `just ready` green.

**Demo:** `cargo test -p assay-backends --features linear` passes all contract tests proving issue creation, comment append, latest-comment read-back, and annotation; `backend_from_config()` returns a `LinearBackend` with the correct capabilities for the `Linear` variant; `just ready` green with 1497+ tests.

## Must-Haves

- `LinearBackend` struct implementing all 7 `StateBackend` methods behind `cfg(feature = "linear")`
- `LinearClient` internal module with `create_issue`, `create_comment`, `get_latest_comment` methods wrapping `reqwest::blocking::Client` GraphQL calls
- `push_session_event` creates issue on first call (persists issue ID to `.linear-issue-id` file in run_dir), appends comment on subsequent calls
- `read_run_state` fetches latest comment body and deserializes as `OrchestratorStatus`
- `annotate_run` posts a tagged comment (prefix `[assay:manifest]`)
- `capabilities()` returns `messaging=false, gossip_manifest=false, annotations=true, checkpoints=false` (D164)
- `LINEAR_API_KEY` read from env at construction; clear error if missing
- `reqwest` dep gated behind `linear` feature flag
- `backend_from_config()` updated: `Linear` variant → `Arc::new(LinearBackend::new(...))`
- Contract tests using `mockito` mock HTTP server (no real Linear API calls)
- `just ready` green

## Proof Level

- This slice proves: contract (mock HTTP transport; no real Linear API)
- Real runtime required: no (mock server only)
- Human/UAT required: yes — real Linear API validation requires `LINEAR_API_KEY` and a test project

## Verification

- `cargo test -p assay-backends --features linear` — all LinearBackend contract tests pass
- `cargo test -p assay-backends` — factory dispatch tests still pass (including updated Linear arm)
- `just ready` — 1497+ workspace tests green, zero regression
- At least one test proving `push_session_event` first call creates an issue (mock returns issue ID)
- At least one test proving `push_session_event` subsequent call creates a comment (reads `.linear-issue-id`)
- At least one test proving `read_run_state` deserializes latest comment body as `OrchestratorStatus`
- At least one test proving `annotate_run` posts a comment with `[assay:manifest]` prefix
- At least one test proving `capabilities()` returns the exact D164 flags
- At least one test proving `read_run_state` returns `Ok(None)` when no issue exists yet
- At least one test proving construction fails with clear error when `LINEAR_API_KEY` is unset

## Observability / Diagnostics

- Runtime signals: `tracing::info!` on issue creation (issue_id), comment creation; `tracing::warn!` on GraphQL error responses; `tracing::debug!` on HTTP request/response
- Inspection surfaces: `.linear-issue-id` file in run_dir maps each orchestration run to a Linear issue; standard `read_run_state` returns the latest persisted status
- Failure visibility: GraphQL errors surfaced as `AssayError::Io` with the Linear error message embedded; HTTP transport errors surfaced via reqwest error chain; missing `LINEAR_API_KEY` returns a clear `AssayError::Io` at construction
- Redaction constraints: `LINEAR_API_KEY` must never appear in logs or error messages; only the key name is referenced

## Integration Closure

- Upstream surfaces consumed: `StateBackend` trait and `CapabilitySet` from `assay-core`; `StateBackendConfig::Linear` variant from `assay-types`; `OrchestratorStatus` and `TeamCheckpoint` from `assay-types`; `backend_from_config()` stub from `assay-backends::factory`
- New wiring introduced in this slice: `LinearBackend` replaces `NoopBackend` in the `backend_from_config()` `Linear` arm; `reqwest` enters `assay-backends` dependency tree behind `linear` feature
- What remains before the milestone is truly usable end-to-end: S03 (GitHubBackend), S04 (SshSyncBackend + CLI/MCP factory wiring)

## Tasks

- [x] **T01: Create LinearBackend contract tests (red state)** `est:25m`
  - Why: Test-first — define the contract before implementation so all subsequent work has an unambiguous target
  - Files: `crates/assay-backends/tests/linear_backend.rs`, `crates/assay-backends/Cargo.toml`
  - Do: Add `mockito` and `reqwest` (with `blocking` + `json` features) as deps behind `linear` feature. Write contract tests: capabilities check, push first event creates issue, push subsequent event creates comment, read_run_state deserializes, read_run_state returns None when no issue, annotate_run posts tagged comment, construction fails without API key. Tests will not compile yet (red state).
  - Verify: `cargo test -p assay-backends --features linear` fails to compile (LinearBackend doesn't exist yet) — this is correct
  - Done when: Test file exists with 8+ test functions covering all contract points; `Cargo.toml` has reqwest and mockito deps behind linear feature

- [ ] **T02: Implement LinearClient and LinearBackend** `est:35m`
  - Why: Core implementation — the LinearClient wraps GraphQL HTTP calls; LinearBackend implements StateBackend using it
  - Files: `crates/assay-backends/src/linear.rs`, `crates/assay-backends/src/lib.rs`, `crates/assay-backends/src/factory.rs`
  - Do: Create `linear.rs` behind `cfg(feature = "linear")` with `LinearClient` (configurable base_url, `reqwest::blocking::Client`, create_issue/create_comment/get_latest_comment methods using validated GraphQL shapes) and `LinearBackend` struct implementing all 7 `StateBackend` methods. Update `factory.rs` to dispatch `Linear` → `LinearBackend`. Wire module in `lib.rs`.
  - Verify: `cargo test -p assay-backends --features linear` — all contract tests pass; `just ready` green
  - Done when: All T01 tests pass; `backend_from_config()` returns a `LinearBackend` for `Linear` config; `just ready` green with 1497+ tests

## Files Likely Touched

- `crates/assay-backends/Cargo.toml`
- `crates/assay-backends/src/lib.rs`
- `crates/assay-backends/src/linear.rs`
- `crates/assay-backends/src/factory.rs`
- `crates/assay-backends/tests/linear_backend.rs`
