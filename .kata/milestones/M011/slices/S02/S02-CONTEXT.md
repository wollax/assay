---
id: S02
milestone: M011
status: ready
---

# S02: LinearBackend — Context

## Goal

Implement `LinearBackend` in `assay_backends::linear` behind the `linear` feature flag, with all 7 `StateBackend` methods, mock HTTP contract tests, and `backend_from_config()` updated to dispatch the `Linear` variant to a real backend.

## Why this Slice

S01 scaffolded the crate and variant types but all three remote backends fall back to `NoopBackend` with a warning. S02 makes the first remote backend real, proving the async-in-sync runtime pattern (D161) and the Linear GraphQL API integration. It unblocks S03/S04 by establishing the testing patterns (mock HTTP server, `new_current_thread` runtime scoping) that those slices replicate.

## Scope

### In Scope

- `assay_backends::linear::LinearBackend` implementing all 7 `StateBackend` methods
- `LinearBackend::new(api_key: String, team_id: String, project_id: Option<String>) -> Self`
- `LinearClient` internal module: `create_issue`, `create_comment`, `get_latest_comment`
- `push_session_event` behavior: first call creates a Linear issue (title `"Assay run <run_id>"`), subsequent calls append a comment — issue id tracked in `.assay/orchestrator/<run_id>/linear_issue_id`
- `read_run_state` behavior: fetch the latest comment on the tracked issue, deserialize its fenced JSON block back to `OrchestratorStatus`
- Comment format: full `OrchestratorStatus` JSON as a fenced code block (machine-readable; `read_run_state` deserializes it directly)
- API key sourced from `LinearBackend` constructor (caller reads `LINEAR_API_KEY` env var — not the backend's responsibility)
- Error handling: API errors and missing-key scenarios return `Err` — fail the orchestration run, no silent degradation
- `annotate_run`: no-op returning `Ok(())` — `supports_gossip_manifest = false` so orchestrator won't call it in gossip mode
- `send_message` / `poll_inbox`: no-op returning `Ok(())` / `Ok(vec![])` — `supports_messaging = false`
- `save_checkpoint_summary`: no-op returning `Ok(())` — `supports_checkpoints = false`
- `capabilities()`: `messaging=false, gossip_manifest=false, annotations=true, checkpoints=false`
- `reqwest` added to `assay-backends` Cargo.toml behind `linear` feature flag
- Mock HTTP contract tests proving first-call issue creation and subsequent-call comment appending
- `backend_from_config()` Linear arm updated: `StateBackendConfig::Linear { .. }` → `Arc::new(LinearBackend::new(...))`; still logs a warn when `linear` feature is not enabled (falls back to NoopBackend)
- `just ready` green with 1499+ tests

### Out of Scope

- `annotate_run` writing anything meaningful to Linear (capability is false; no-op only)
- Inbox/outbox semantics via Linear custom issue types (messaging deferred to M012+)
- `save_checkpoint_summary` persisting to Linear (TeamCheckpoint format doesn't map to Linear structures; deferred)
- Token refresh or OAuth — personal API key (`LINEAR_API_KEY`) only
- CLI/MCP construction site wiring to use `backend_from_config()` — that is S04
- Real Linear API validation in automated tests — UAT only

## Constraints

- D161: async HTTP wrapped in `tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(...)` scoped per method body. Do NOT use `tokio::runtime::Handle::current()` — tests may not run under a tokio runtime.
- D150: all trait methods are sync; async is internalized. No async fn on `StateBackend`.
- D149: `StateBackend` is the deliberate sole exception to D001 (zero-trait convention).
- D160: `assay-backends` is a leaf crate; `reqwest` behind the `linear` feature flag only — must not leak into `assay-core`.
- Issue ID persistence uses the existing run-dir file layout (`.assay/orchestrator/<run_id>/linear_issue_id`) — no new I/O abstractions.
- No secrets in artifacts on disk — API key is passed via constructor, never written to `.assay/`.

## Integration Points

### Consumes

- `assay_core::state_backend::{StateBackend, CapabilitySet}` — trait to implement
- `assay_types::orchestrate::OrchestratorStatus` — the data shape serialized into issue comments
- `assay_types::StateBackendConfig::Linear { team_id, project_id }` — config variant (from S01)
- `assay_backends::factory::backend_from_config` stub — the `Linear` arm to replace
- Linear GraphQL API at `https://api.linear.app/graphql` — `createIssue`, `createComment`, `issueComments` operations

### Produces

- `crates/assay-backends/src/linear.rs` — `LinearBackend` + `LinearClient` internal module
- `LINEAR_API_KEY`-driven issue creation: one issue per run_id, comments per `push_session_event` call
- `.assay/orchestrator/<run_id>/linear_issue_id` — persisted issue ID for comment routing
- Mock HTTP contract tests for `push_session_event` and `read_run_state`
- Updated `backend_from_config()` Linear arm

## Open Questions

- Linear GraphQL query shape for `issueComments` — specifically which fields are needed to fetch the latest comment body. Must validate against Linear's public schema during research. Current thinking: use `issue(id: $id) { comments(first: 50, orderBy: createdAt) { nodes { body } } }` and take the last node.
- Whether `new_current_thread` runtime creation panics when tests are run under `cargo nextest` with concurrency. Current thinking: mock the HTTP layer so no real runtime is needed in tests — contract tests use a mock server or a stub that returns pre-canned JSON without requiring async dispatch at all.
