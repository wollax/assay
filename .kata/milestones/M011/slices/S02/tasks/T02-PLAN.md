---
estimated_steps: 5
estimated_files: 4
---

# T02: Implement LinearClient and LinearBackend

**Slice:** S02 — LinearBackend
**Milestone:** M011

## Description

Implement the `LinearBackend` struct and its internal `LinearClient` HTTP wrapper. `LinearClient` uses `reqwest::blocking::Client` (not async — the blocking client internalizes its own tokio runtime, avoiding the nested-runtime panic risk from D161). `LinearBackend` implements all 7 `StateBackend` methods. The `backend_from_config()` factory is updated to dispatch `Linear` → `LinearBackend`. All T01 contract tests must pass.

## Steps

1. Create `crates/assay-backends/src/linear.rs` behind `#[cfg(feature = "linear")]`:
   - Define `LinearClient` struct with `client: reqwest::blocking::Client`, `api_key: String`, `base_url: String`
   - `LinearClient::new(api_key: String, base_url: String)` — constructs client with `Authorization: <api_key>` default header
   - `LinearClient::create_issue(&self, team_id: &str, title: &str, description: &str) -> assay_core::Result<String>` — sends `issueCreate` GraphQL mutation, returns issue ID. Checks for `errors` array in response (GraphQL errors come as 200).
   - `LinearClient::create_comment(&self, issue_id: &str, body: &str) -> assay_core::Result<()>` — sends `commentCreate` GraphQL mutation
   - `LinearClient::get_latest_comment(&self, issue_id: &str) -> assay_core::Result<Option<String>>` — sends issue comments query (`last: 1`), returns body text or None

2. Define `LinearBackend` struct with `client: LinearClient`, `team_id: String`, `project_id: Option<String>`:
   - `LinearBackend::new(api_key: String, team_id: String, project_id: Option<String>, base_url: String) -> Self`
   - Implement `StateBackend` for `LinearBackend`:
     - `capabilities()` → D164 flags (messaging=false, gossip_manifest=false, annotations=true, checkpoints=false)
     - `push_session_event(run_dir, status)` → read `.linear-issue-id` from run_dir; if missing, create issue (title = run_dir basename, description = session summary), write issue ID to file; if present, create comment with `serde_json::to_string_pretty(status)`
     - `read_run_state(run_dir)` → read `.linear-issue-id`; if missing return `Ok(None)`; if present, fetch latest comment, deserialize as `OrchestratorStatus`
     - `send_message` → return error (messaging unsupported)
     - `poll_inbox` → return error (messaging unsupported)
     - `annotate_run(run_dir, manifest_path)` → read `.linear-issue-id`; create comment with body `[assay:manifest] {manifest_path}`
     - `save_checkpoint_summary` → return error (checkpoints unsupported)

3. Wire the module in `crates/assay-backends/src/lib.rs`:
   - Add `#[cfg(feature = "linear")] pub mod linear;`

4. Update `crates/assay-backends/src/factory.rs`:
   - In the `Linear` arm: read `LINEAR_API_KEY` from `std::env::var`, construct `LinearBackend::new(api_key, team_id, project_id, "https://api.linear.app/graphql".to_string())`, return `Arc::new(backend)`
   - Keep `tracing::warn!` fallback if `LINEAR_API_KEY` is not set (fall back to `NoopBackend` with a warning instead of panicking in the factory)
   - Update factory test for Linear: when `LINEAR_API_KEY` is set, verify `CapabilitySet` matches D164 flags

5. Run `just ready` to confirm all tests pass including the new contract tests.

## Must-Haves

- [ ] `LinearClient` wraps `reqwest::blocking::Client` with configurable `base_url` for testability
- [ ] GraphQL mutations/queries match validated shapes from S02-RESEARCH (issueCreate, commentCreate, issue comments query)
- [ ] GraphQL error responses (200 with `errors` array) are detected and surfaced as `AssayError`
- [ ] `.linear-issue-id` file written atomically (write to run_dir, read back on subsequent calls)
- [ ] `capabilities()` returns exactly D164 flags
- [ ] `send_message`, `poll_inbox`, `save_checkpoint_summary` return errors when called (unsupported capabilities)
- [ ] `LINEAR_API_KEY` missing → clear error at construction time (or graceful NoopBackend fallback in factory)
- [ ] All 8 T01 contract tests pass
- [ ] `just ready` green with 1497+ tests

## Verification

- `cargo test -p assay-backends --features linear` — all contract tests pass
- `cargo test -p assay-backends` — factory tests still pass
- `just ready` — green, zero regression
- `cargo clippy -p assay-backends --features linear` — no warnings

## Observability Impact

- Signals added/changed: `tracing::info!` when issue created (logs issue_id); `tracing::debug!` on GraphQL requests; `tracing::warn!` on GraphQL error responses and missing LINEAR_API_KEY
- How a future agent inspects this: `.linear-issue-id` file in run_dir maps run to Linear issue; `read_run_state` surfaces the latest status; error messages include "LINEAR_API_KEY" or "GraphQL error" for diagnosis
- Failure state exposed: `AssayError::Io` with operation label ("creating Linear issue", "creating comment", "fetching comments", "LINEAR_API_KEY not set") and embedded HTTP/GraphQL error details

## Inputs

- `crates/assay-backends/tests/linear_backend.rs` — T01 contract tests (the target to make pass)
- `crates/assay-backends/src/factory.rs` — existing factory with `Linear` → `NoopBackend` stub to replace
- `crates/assay-core/src/state_backend.rs` — `StateBackend` trait with 7 method signatures, `CapabilitySet`, `AssayError`
- S02-RESEARCH.md — validated GraphQL shapes, `reqwest::blocking` recommendation, pitfalls (error shape, issue ID race)

## Expected Output

- `crates/assay-backends/src/linear.rs` — `LinearClient` + `LinearBackend` implementation (~200-250 lines)
- `crates/assay-backends/src/lib.rs` — `#[cfg(feature = "linear")] pub mod linear;` added
- `crates/assay-backends/src/factory.rs` — `Linear` arm dispatches to `LinearBackend::new(...)` with env-var API key
- All 8+ contract tests green; `just ready` green
