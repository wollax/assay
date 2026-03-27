---
id: S02
milestone: M011
status: ready
---

# S02: LinearBackend — Context

## Goal

Implement `LinearBackend` — a `StateBackend` that creates a Linear issue on first `push_session_event` and appends comments on subsequent calls — so assay orchestrated runs produce visible state in Linear without SCP.

## Why this Slice

S02 proves the `StateBackend` abstraction delivers real value: an actual remote write path that a user with `LINEAR_API_KEY` can observe. It also retires the highest-risk M011 unknowns (async-in-sync runtime pattern, Linear GraphQL API shape) before the lower-risk S03 and S04.

## Scope

### In Scope

- `LinearBackend` struct in `assay-backends::linear` behind `cfg(feature = "linear")`
- `LinearBackend::new(api_key: String, team_id: String, project_id: Option<String>) -> Self` — reads `LINEAR_API_KEY` from environment and fails at construction time if not set; `Err` propagates through `backend_from_config()`
- `push_session_event(run_dir, status)`:
  - First call for a given `run_dir`: creates a Linear issue with title `"assay run: <run_dir_stem>"`; writes the returned Linear issue ID to `.linear-issue-id` under `run_dir`
  - Subsequent calls: reads `.linear-issue-id` from `run_dir`, appends a comment with the serialized status
  - API failure returns `Err` (hard fail — not silent degrade)
- `read_run_state(run_dir)`: reads `.linear-issue-id`, fetches the latest comment, deserializes as `OrchestratorStatus`; returns `Ok(None)` if no issue file exists
- `annotate_run(run_dir, manifest_path)`: reads `.linear-issue-id`, posts a comment with body starting with `"[assay:manifest] <manifest_path>"`
- `capabilities()`: messaging=false, gossip_manifest=false, annotations=true, checkpoints=false (D164)
- `send_message` / `poll_inbox` / `save_checkpoint_summary`: return `Ok(...)` no-ops (capabilities advertise false)
- `LinearClient` internal module: `create_issue(title, description) -> String`, `create_comment(issue_id, body)`, `get_latest_comment(issue_id) -> Option<String>` — all using `reqwest` + hand-rolled GraphQL against `api.linear.app/graphql`
- `reqwest` added to `assay-backends/Cargo.toml` behind `linear` feature flag
- Mock HTTP contract tests: `push_session_event` first-call creates issue + writes `.linear-issue-id`; subsequent call reads file + creates comment; `read_run_state` fetches latest comment; `annotate_run` posts tagged comment
- `backend_from_config()` updated: `Linear` variant → `Arc::new(LinearBackend::new(api_key, team_id, project_id))`
- `just ready` green with 1497+ tests

### Out of Scope

- `send_message` / `poll_inbox` implementation (messaging capability is false; Linear has no inbox/outbox)
- `save_checkpoint_summary` implementation (checkpoints capability is false)
- Token refresh / OAuth — personal API key only via `LINEAR_API_KEY` env var
- Real Linear API calls in automated tests — contract tests use mock HTTP; real API is UAT only
- Error recovery or retry logic on API failure — fail fast and return `Err`

## Constraints

- D161: async HTTP calls use `tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(...)` scoped to each method body. Never `tokio::runtime::Handle::current()` — tests may not run under a tokio runtime.
- D164: capabilities() shape is locked — messaging=false, gossip_manifest=false, annotations=true, checkpoints=false
- `LINEAR_API_KEY` must be read from environment at construction time, not stored in TOML/JSON on disk
- `reqwest` must be behind the `linear` feature flag in `assay-backends/Cargo.toml` — must not leak into binaries built without the feature
- `assay-core` dep in `assay-backends/Cargo.toml` already has `orchestrate` feature — do not remove it when adding `reqwest`
- Hard-fail semantics: API errors return `Err`; callers surface them as run failures

## Integration Points

### Consumes

- `assay-backends::factory::backend_from_config` (from S01) — the `Linear` arm currently dispatches to `NoopBackend`; S02 replaces it
- `StateBackendConfig::Linear { team_id, project_id }` (from S01) — field shapes locked
- `assay-core::StateBackend` trait — all 7 methods to implement
- `assay-core::OrchestratorStatus` — serialized as comment body on `push_session_event`; deserialized on `read_run_state`

### Produces

- `crates/assay-backends/src/linear.rs` — `LinearBackend` struct + `LinearClient` internal module
- Updated `crates/assay-backends/Cargo.toml` — `reqwest` dep behind `linear` feature
- Updated `crates/assay-backends/src/factory.rs` — `Linear` arm wired to real `LinearBackend`
- `.linear-issue-id` file written under `run_dir` on first `push_session_event` (runtime artifact, not a source file)
- Mock HTTP contract tests in `crates/assay-backends/tests/` or inline

## Open Questions

- **GraphQL query shape for `get_latest_comment`** — Linear's GraphQL schema uses `IssueCommentConnection`; exact field names (`comments { nodes { body } }`) need validation against Linear's public schema during research. This is the highest-risk unknown for S02.
- **reqwest version** — workspace has `reqwest 0.13` (via jsonschema dep). The `linear` feature flag in `assay-backends` should specify a version compatible with the workspace. Need to confirm no version conflict during planning.
