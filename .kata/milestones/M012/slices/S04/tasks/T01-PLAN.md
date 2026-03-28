---
estimated_steps: 5
estimated_files: 6
---

# T01: LinearClient trait, ReqwestLinearClient, and MockLinearClient

**Slice:** S04 — Linear Tracker Backend
**Milestone:** M012

## Description

Define the `LinearClient` trait with 5 async RPITIT methods for Linear GraphQL operations, implement `ReqwestLinearClient` using async `reqwest::Client`, and create `MockLinearClient` as a VecDeque-based test double. Promote `reqwest` from dev-dependency to production dependency. Register the `linear` module in `serve/mod.rs`.

The trait mirrors the `GhClient` pattern (D164): generic `<L: LinearClient>` at callsites, not `dyn LinearClient`. The production client follows Assay's `LinearBackend` GraphQL patterns but uses async `reqwest::Client` (not blocking, per D156 — although research initially said blocking, the serve loop is async/tokio so async is correct).

## Steps

1. **Promote `reqwest` to production dep** in `crates/smelt-cli/Cargo.toml`: move from `[dev-dependencies]` to `[dependencies]` with `features = ["json"]`.

2. **Create `serve/linear/mod.rs`** with:
   - `LinearClient` trait: 5 async RPITIT methods (`list_issues`, `add_label`, `remove_label`, `find_label`, `create_label`) all returning `Result<T, SmeltError>` + Send futures
   - `LinearIssue` struct: `id` (UUID), `identifier` (human-readable e.g. "KAT-42"), `title`, `description`, `url`
   - `LinearLabel` struct: `id` (UUID), `name`
   - Re-exports of `ReqwestLinearClient` and `LinearTrackerSource`
   - Compile-test for trait implementability

3. **Create `serve/linear/client.rs`** with `ReqwestLinearClient`:
   - Constructor takes `api_key: String`, `base_url: String` (default `https://api.linear.app`)
   - Builds `reqwest::Client` with `Authorization: {api_key}` and `Content-Type: application/json` headers
   - Each method sends POST to `{base_url}/graphql` with JSON body `{ "query": "..." }`
   - Error extraction: check `json["errors"]` array on HTTP 200 (Assay pattern)
   - `list_issues`: GraphQL query filtering by label name and team ID
   - `find_label`: GraphQL query for `issueLabels` filtered by name and team
   - `create_label`: GraphQL mutation `issueLabelCreate`
   - `add_label`: GraphQL mutation `issueAddLabel`
   - `remove_label`: GraphQL mutation `issueRemoveLabel`
   - `tracing::debug!` on every request, `tracing::warn!` on errors

4. **Create `serve/linear/mock.rs`** with `MockLinearClient`:
   - VecDeque queues for each method (matching `MockGhClient` pattern exactly)
   - Builder methods: `with_list_result`, `with_find_label_result`, `with_create_label_result`, `with_add_label_result`, `with_remove_label_result`
   - Unit tests: queued results return correctly, exhausted queue returns error, `LinearIssue` deserialize from JSON

5. **Register module** in `serve/mod.rs`: add `pub mod linear;`

## Must-Haves

- [ ] `LinearClient` trait with 5 async methods compiles and is implementable (compile-test)
- [ ] `ReqwestLinearClient` builds without errors, constructs `reqwest::Client` with auth headers
- [ ] `ReqwestLinearClient` extracts GraphQL errors from `errors` array on HTTP 200
- [ ] `MockLinearClient` implements `LinearClient` and returns pre-queued results
- [ ] `reqwest` is a production dependency in smelt-cli (not just dev-dep)
- [ ] All existing workspace tests pass (zero regressions)

## Verification

- `cargo test -p smelt-cli --lib -- serve::linear::mock` — mock tests pass
- `cargo test --workspace` — all 337+ tests pass
- `cargo clippy --workspace -- -D warnings` — clean
- `grep 'reqwest' crates/smelt-cli/Cargo.toml` shows it under `[dependencies]`

## Observability Impact

- Signals added/changed: `tracing::debug!` on GraphQL requests (operation name, query preview); `tracing::warn!` on GraphQL error responses (error messages from API)
- How a future agent inspects this: `SmeltError::tracker("operation", "message")` with operation names matching trait method names
- Failure state exposed: GraphQL errors surfaced with full error text from Linear API; HTTP non-200 status codes included in error messages

## Inputs

- `crates/smelt-cli/src/serve/github/mod.rs` — GhClient trait pattern to mirror
- `crates/smelt-cli/src/serve/github/client.rs` — SubprocessGhClient pattern (adapted for HTTP)
- `crates/smelt-cli/src/serve/github/mock.rs` — MockGhClient VecDeque pattern to mirror exactly
- S04-RESEARCH.md — GraphQL query/mutation syntax for Linear API
- `../assay/crates/assay-backends/src/linear.rs` — Assay's LinearClient error extraction pattern

## Expected Output

- `crates/smelt-cli/src/serve/linear/mod.rs` — LinearClient trait, types, re-exports
- `crates/smelt-cli/src/serve/linear/client.rs` — ReqwestLinearClient production impl
- `crates/smelt-cli/src/serve/linear/mock.rs` — MockLinearClient test double + tests
- `crates/smelt-cli/src/serve/mod.rs` — Updated with `pub mod linear`
- `crates/smelt-cli/Cargo.toml` — reqwest promoted to production dep
