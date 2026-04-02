---
id: T01
parent: S04
milestone: M012
provides:
  - LinearClient trait with 5 async RPITIT methods (list_issues, add_label, remove_label, find_label, create_label)
  - ReqwestLinearClient production impl using async reqwest::Client with GraphQL
  - MockLinearClient VecDeque-based test double with builder methods
  - LinearIssue and LinearLabel data types with serde Deserialize
  - reqwest promoted to production dependency in smelt-cli
key_files:
  - crates/smelt-cli/src/serve/linear/mod.rs
  - crates/smelt-cli/src/serve/linear/client.rs
  - crates/smelt-cli/src/serve/linear/mock.rs
  - crates/smelt-cli/src/serve/mod.rs
  - crates/smelt-cli/Cargo.toml
key_decisions:
  - "LinearClient trait uses RPITIT async methods with Send bounds (mirrors GhClient per D164)"
  - "ReqwestLinearClient uses GraphQL variables instead of string interpolation for safety"
  - "GraphQL error extraction checks errors array on HTTP 200 (Assay pattern)"
  - "LinearIssue.description defaults to empty string via serde(default) for null/missing fields"
patterns_established:
  - "serve/linear/ module structure mirrors serve/github/ exactly: mod.rs (trait+types), client.rs (production impl), mock.rs (test double)"
  - "MockLinearClient uses VecDeque per-method queues with Arc<Mutex<>> — same pattern as MockGhClient"
observability_surfaces:
  - "tracing::debug! on every GraphQL request (operation name, URL, query preview)"
  - "tracing::warn! on GraphQL error responses and non-200 HTTP status"
  - "SmeltError::tracker(operation, message) with operation names matching trait method names"
duration: 12min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T01: LinearClient trait, ReqwestLinearClient, and MockLinearClient

**Async Linear GraphQL client trait with 5 methods, reqwest-based production impl with error extraction, and VecDeque mock test double — mirroring the GhClient pattern exactly**

## What Happened

Created the `serve/linear/` module following the exact same structure as `serve/github/`. The `LinearClient` trait defines 5 async RPITIT methods for Linear GraphQL operations: `list_issues`, `add_label`, `remove_label`, `find_label`, and `create_label`. All return `Result<T, SmeltError>` with Send futures.

`ReqwestLinearClient` builds an async `reqwest::Client` with `Authorization` and `Content-Type` headers set at construction. Each method sends a POST to `{base_url}/graphql` with GraphQL variables (not string interpolation). The `graphql()` helper extracts errors from the `errors` array on HTTP 200 responses (matching Assay's pattern) and also checks for non-200 status codes.

`MockLinearClient` uses the VecDeque pattern from `MockGhClient` with per-method queues, builder methods, and Arc<Mutex<>> for Clone support. Unit tests cover queued results, exhausted queues, and JSON deserialization of both `LinearIssue` and `LinearLabel`.

Promoted `reqwest` from dev-dependency to production dependency. Registered `pub mod linear` in `serve/mod.rs`.

## Verification

- `cargo test -p smelt-cli --lib -- serve::linear::mock` — 8 tests pass
- `cargo test -p smelt-cli --lib -- serve::linear::compile_tests` — 1 test passes (trait implementability)
- `cargo test --workspace` — 366 passed, 11 ignored, 0 failed
- `cargo clippy --workspace -- -D warnings` — clean
- `grep 'reqwest' crates/smelt-cli/Cargo.toml` — shows under `[dependencies]` only

### Slice-level checks status (T01 of 4):
- ✅ `cargo test -p smelt-cli --lib -- serve::linear::mock` — passes (9 tests)
- ⏳ `cargo test -p smelt-cli --lib -- serve::linear` — partial (no source tests yet, T02)
- ⏳ `cargo test -p smelt-cli --lib -- serve::config` — not yet modified (T03)
- ✅ `cargo test --workspace` — all pass
- ✅ `cargo clippy --workspace -- -D warnings` — clean

## Diagnostics

- `SmeltError::tracker("list_issues"|"add_label"|"remove_label"|"find_label"|"create_label", message)` — structured errors with operation names
- `tracing::debug!` logs on every GraphQL request showing operation, URL, and query preview (first 80 chars)
- `tracing::warn!` logs on GraphQL errors (with error text from API) and non-200 HTTP status codes
- Auth header is marked `.set_sensitive(true)` to prevent accidental logging by reqwest

## Deviations

- Used GraphQL variables (`$teamId`, `$labelName`, etc.) instead of string-interpolated queries as shown in research. This is safer against injection and is standard GraphQL practice.
- `description` field on `LinearIssue` uses `#[serde(default)]` to handle null/missing values from the API gracefully.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/linear/mod.rs` — LinearClient trait, LinearIssue/LinearLabel types, re-exports, compile-test
- `crates/smelt-cli/src/serve/linear/client.rs` — ReqwestLinearClient with GraphQL helper and 5 method implementations
- `crates/smelt-cli/src/serve/linear/mock.rs` — MockLinearClient with VecDeque queues, builders, and 10 unit tests
- `crates/smelt-cli/src/serve/mod.rs` — Added `pub mod linear`
- `crates/smelt-cli/Cargo.toml` — Promoted reqwest from dev-dep to production dep
