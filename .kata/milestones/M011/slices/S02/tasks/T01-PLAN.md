---
estimated_steps: 4
estimated_files: 3
---

# T01: Create LinearBackend contract tests (red state)

**Slice:** S02 — LinearBackend
**Milestone:** M011

## Description

Test-first: define the full LinearBackend contract via integration tests using `mockito` as the mock HTTP server. These tests exercise every `StateBackend` method on `LinearBackend` and verify the GraphQL request shapes, the `.linear-issue-id` file lifecycle, and the capability flags. Tests will not compile until T02 implements the types — that's the correct red state.

## Steps

1. Add dependencies to `crates/assay-backends/Cargo.toml`:
   - `reqwest = { version = "0.13", features = ["blocking", "json"], optional = true }` in `[dependencies]`
   - `mockito = "1"` in `[dev-dependencies]`
   - Update `linear` feature: `linear = ["reqwest"]`

2. Create `crates/assay-backends/tests/linear_backend.rs` with `#![cfg(feature = "linear")]` gate and the following contract tests:
   - `test_capabilities_returns_d164_flags` — construct `LinearBackend`, assert `capabilities()` returns `messaging=false, gossip_manifest=false, annotations=true, checkpoints=false`
   - `test_push_first_event_creates_issue` — set up mockito to expect a `issueCreate` GraphQL mutation, call `push_session_event` on a fresh run_dir, assert `.linear-issue-id` file was written, assert the mock was called with correct GraphQL shape
   - `test_push_subsequent_event_creates_comment` — pre-write `.linear-issue-id` file, set up mockito to expect `commentCreate` mutation, call `push_session_event`, assert mock was called with the issue ID from the file
   - `test_read_run_state_deserializes_latest_comment` — pre-write `.linear-issue-id`, set up mockito to return a `GetIssueComments` response with an `OrchestratorStatus` JSON body, call `read_run_state`, assert returned status matches
   - `test_read_run_state_returns_none_when_no_issue` — call `read_run_state` on a run_dir with no `.linear-issue-id` file, assert `Ok(None)`
   - `test_annotate_run_posts_tagged_comment` — pre-write `.linear-issue-id`, set up mockito, call `annotate_run`, assert comment body starts with `[assay:manifest]`
   - `test_construction_fails_without_api_key` — temporarily unset `LINEAR_API_KEY`, attempt construction, assert error contains "LINEAR_API_KEY"
   - `test_push_handles_graphql_error_response` — set up mockito to return a 200 with `{"errors": [...]}`, call `push_session_event`, assert error is surfaced

3. Each test constructs a `LinearBackend` via a helper fn that creates it with the mockito server URL as `base_url` and a fake API key.

4. Verify that `cargo test -p assay-backends --features linear` fails to compile (expected — `LinearBackend` module doesn't exist yet).

## Must-Haves

- [ ] `Cargo.toml` has `reqwest` behind `linear` feature and `mockito` as dev-dep
- [ ] Test file has 8 test functions covering all contract points from S02-PLAN verification section
- [ ] Tests use `mockito::Server` for HTTP mocking (not real API)
- [ ] Tests are gated behind `#![cfg(feature = "linear")]`
- [ ] GraphQL request shapes in mocks match the validated shapes from S02-RESEARCH (issueCreate, commentCreate, issue comments query)

## Verification

- `cargo test -p assay-backends --features linear` fails to compile (red state — LinearBackend not yet implemented)
- `cargo test -p assay-backends` (without linear feature) still passes all existing factory tests
- `just fmt` passes on the new test file

## Observability Impact

- None — this task creates tests only, no runtime code

## Inputs

- `crates/assay-backends/Cargo.toml` — existing crate manifest to extend with deps
- `crates/assay-core/src/state_backend.rs` — `StateBackend` trait signatures (7 methods) and `CapabilitySet` shape
- S02-RESEARCH.md — validated GraphQL shapes for issue create, comment create, get latest comment
- D164 — capability flags: messaging=false, gossip_manifest=false, annotations=true, checkpoints=false

## Expected Output

- `crates/assay-backends/Cargo.toml` — updated with reqwest (optional, linear feature), mockito dev-dep
- `crates/assay-backends/tests/linear_backend.rs` — 8 contract test functions (red state, won't compile yet)
