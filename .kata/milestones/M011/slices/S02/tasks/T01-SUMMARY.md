---
id: T01
parent: S02
milestone: M011
provides:
  - 8 contract tests defining the full LinearBackend StateBackend interface
  - reqwest (optional, linear feature) and mockito dev-dependencies in assay-backends
key_files:
  - crates/assay-backends/tests/linear_backend.rs
  - crates/assay-backends/Cargo.toml
key_decisions:
  - Used mockito::Server for mock HTTP (sync-compatible, no runtime conflicts)
  - Tests import `assay_backends::linear::LinearBackend` — T02 must expose this path
  - Constructor helper `make_backend(server)` takes api_key, base_url, team_id, project_id; T02 must match this signature
  - Added `from_env` constructor variant for env-based construction (tested in API key validation test)
patterns_established:
  - Contract test pattern: mockito mock → call backend method → assert mock called + side effects
  - `.linear-issue-id` file lifecycle tested via tempdir
observability_surfaces:
  - none (test-only task)
duration: 10m
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T01: Create LinearBackend contract tests (red state)

**Defined the full LinearBackend contract via 8 integration tests using mockito mock HTTP server.**

## What Happened

Added `reqwest` (optional, gated behind `linear` feature with `blocking` + `json` features) and `mockito` (dev-dependency) to `assay-backends/Cargo.toml`. Created `tests/linear_backend.rs` with `#![cfg(feature = "linear")]` containing 8 test functions that cover every contract point from the S02-PLAN verification section:

1. `test_capabilities_returns_d164_flags` — D164 capability flags
2. `test_push_first_event_creates_issue` — issueCreate mutation, .linear-issue-id written
3. `test_push_subsequent_event_creates_comment` — commentCreate mutation using stored issue ID
4. `test_read_run_state_deserializes_latest_comment` — issue comments query, JSON deserialization
5. `test_read_run_state_returns_none_when_no_issue` — no .linear-issue-id → Ok(None)
6. `test_annotate_run_posts_tagged_comment` — comment with `[assay:manifest]` prefix
7. `test_construction_fails_without_api_key` — from_env without LINEAR_API_KEY errors
8. `test_push_handles_graphql_error_response` — 200 + errors array surfaced as error

GraphQL request shapes in mock matchers align with validated shapes from S02-RESEARCH (issueCreate, commentCreate, issue comments query).

## Verification

- `cargo test -p assay-backends --features linear` fails to compile with `unresolved import assay_backends::linear` — correct red state
- `cargo test -p assay-backends` (no linear feature) passes all 5 factory tests — no regression
- `cargo fmt -p assay-backends -- --check` passes clean

### Slice-level verification (T01 — partial):
- ✅ `cargo test -p assay-backends` — factory dispatch tests pass
- ⬜ `cargo test -p assay-backends --features linear` — expected to fail (red state, T02 needed)
- ⬜ `just ready` — deferred to T02 (red state blocks compilation)

## Diagnostics

None — test-only task. Tests will compile and pass once T02 implements `LinearBackend`.

## Deviations

- Added `chrono = { workspace = true }` as dev-dependency (needed for `chrono::Utc::now()` in `sample_status()` helper)
- Removed unused `use std::path::Path` import after fmt

## Known Issues

None.

## Files Created/Modified

- `crates/assay-backends/Cargo.toml` — added reqwest (optional, linear feature), mockito + chrono dev-deps, updated linear feature definition
- `crates/assay-backends/tests/linear_backend.rs` — 8 contract test functions (red state)
