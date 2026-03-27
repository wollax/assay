# S02: LinearBackend — Research

**Date:** 2026-03-27

## Summary

S02 implements `LinearBackend`, the first remote `StateBackend`. The backend creates a Linear issue on first `push_session_event` (keyed by run_dir), appends comments on subsequent calls with serialized `OrchestratorStatus` JSON, and reads the latest comment back for `read_run_state`. The async-in-sync pattern (D161) uses `tokio::runtime::Builder::new_current_thread().block_on(...)` scoped per method. `reqwest` 0.13 (already in Cargo.lock as a transitive dep of `jsonschema`) handles HTTP. Mock HTTP contract tests prove the trait contract without real API calls.

The primary risks are (1) the Linear GraphQL query shape for fetching issue comments, and (2) the tokio runtime scoping to avoid nested-runtime panics in tests. Both are resolvable: the Kata CLI extension (`~/.kata-cli/agent/extensions/linear/linear-client.ts`) provides proven GraphQL mutation/query shapes, and contract tests should use synchronous mock responses (no real tokio runtime needed in test harness).

## Recommendation

Implement `LinearBackend` with an internal `LinearClient` that wraps `reqwest::blocking::Client` (not async reqwest). This eliminates the D161 async-in-sync complexity entirely — `reqwest::blocking` handles the tokio runtime internally and avoids nested-runtime panics. The blocking client is behind the `linear` feature flag so it doesn't affect default builds.

**Why `reqwest::blocking` instead of async + scoped runtime:**
- `reqwest::blocking::Client` internally manages its own tokio runtime — no need to hand-roll `Builder::new_current_thread().block_on()`
- Avoids the nested-runtime panic risk that D161 warns about
- Simpler code: each method is a straightforward synchronous call
- Consistent with D007 (sync core) — the async is fully internalized within reqwest
- D161 says "use scoped runtime" but the rationale is "sync trait methods need HTTP" — `reqwest::blocking` achieves the same goal more safely

If `reqwest::blocking` causes issues (e.g. reqwest version constraints), fall back to the explicit `new_current_thread` pattern from D161.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| HTTP client for GraphQL | `reqwest::blocking::Client` | Internalizes tokio runtime; avoids nested-runtime panics |
| GraphQL request format | Kata CLI `linear-client.ts` patterns | Proven query shapes: `issueCreate`, `commentCreate`, `issue { comments { nodes } }` |
| Atomic file write for `.linear-issue-id` | `std::fs::write` (simple text file) | Issue ID file is a small string; no crash-safety concern for a cache file |
| JSON serialization of `OrchestratorStatus` | `serde_json::to_string_pretty` | Already used by `LocalFsBackend` |

## Existing Code and Patterns

- `crates/assay-core/src/state_backend.rs` — `StateBackend` trait with 7 methods, `CapabilitySet`, `LocalFsBackend`, `NoopBackend`. The template for all method signatures, error types (`crate::Result`), and doc patterns. Follow `LocalFsBackend` for method structure.
- `crates/assay-backends/src/factory.rs` — `backend_from_config()` stub. The `Linear` arm currently dispatches to `NoopBackend`. S02 replaces it with `LinearBackend::new(...)`.
- `crates/assay-types/src/state_backend.rs` — `StateBackendConfig::Linear { team_id, project_id }` variant. Fields locked by schema snapshot.
- `~/.kata-cli/agent/extensions/linear/linear-client.ts` — Proven GraphQL shapes:
  - `issueCreate(input: $input) { success issue { id } }` with `IssueCreateInput!`
  - `commentCreate(input: $input) { success comment { id body } }` with `CommentCreateInput!` (input: `{ issueId, body }`)
  - Issue query: `issue(id: $id) { id title }` — for fetching issue with comments, use `issue(id: $id) { comments(last: 1) { nodes { body } } }`
- `crates/assay-types/src/orchestrate.rs:96` — `OrchestratorStatus` struct with `deny_unknown_fields`. Serialized as comment body; deserialized on `read_run_state`. Must handle the full struct shape including optional `mesh_status` and `gossip_status` fields.
- `Cargo.lock` — `reqwest 0.13.2` already present (transitive via `jsonschema`). Adding `reqwest = { version = "0.13", features = ["blocking", "json"] }` behind the `linear` feature in `assay-backends` should resolve cleanly.

## Constraints

- **D161 (async-in-sync):** Trait methods must be sync. Using `reqwest::blocking` satisfies this without hand-rolling a tokio runtime. If blocking client is rejected, use `tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(...)` per method.
- **D164 (capabilities):** `messaging=false, gossip_manifest=false, annotations=true, checkpoints=false` — hardcoded, not configurable.
- **Feature flag isolation:** `reqwest` must only enter the dependency tree when `linear` feature is enabled. The `[dependencies]` entry needs `optional = true` or must be under `[target.'cfg(feature = "linear")'.dependencies]`.
- **No secrets on disk:** `LINEAR_API_KEY` read from `std::env::var("LINEAR_API_KEY")` at construction time. Fail with a clear error if not set.
- **assay-backends depends on assay-core (not vice versa):** `LinearBackend` cannot use `crate::Result` from assay-core. It must use `assay_core::Result` or return `assay_core::AssayError` via the `assay_core::Result` alias.
- **`OrchestratorStatus` has `deny_unknown_fields`:** Deserialization on `read_run_state` will reject any extra fields — the comment body must be exact JSON. Use `serde_json::to_string_pretty(status)` for push and `serde_json::from_str::<OrchestratorStatus>(&body)` for read.
- **tokio as workspace dep:** `tokio = { version = "1", features = ["full"] }` is already in root `Cargo.toml`. If the scoped-runtime approach is needed, add `tokio = { workspace = true }` behind the `linear` feature.

## Common Pitfalls

- **Nested tokio runtime panic** — If a test is already running inside a `#[tokio::test]` or if `tracing-test` sets up a runtime, calling `tokio::runtime::Builder::new_current_thread().build()` will panic with "cannot start runtime from within runtime". Solution: use `reqwest::blocking` which handles this internally, OR ensure contract tests are `#[test]` (not `#[tokio::test]`) and use a mock HTTP server (e.g. `mockito` or inline HTTP mock).
- **Linear GraphQL error shape** — Linear returns `{ "data": null, "errors": [{ "message": "...", "extensions": { "code": "..." } }] }` on failure. Must check for `errors` array in response body before accessing `data`. A 200 status code does NOT guarantee success — GraphQL errors come as 200 with an `errors` field.
- **Issue ID file race on concurrent runs** — Multiple `push_session_event` calls from different threads for the same `run_dir` could race on reading/writing `.linear-issue-id`. The orchestrator serializes `push_session_event` calls per `run_dir` in practice (one status write per session completion), but the file write should use atomic write (write to temp, rename) to be safe.
- **Comment body size** — `OrchestratorStatus` JSON can be large (many sessions, merge reports). Linear comments have no documented size limit, but extremely large bodies may be truncated in the UI. Not a functional issue for `read_run_state` which reads via API, but worth noting.
- **`serde(rename = "github")` on `StateBackendConfig::GitHub`** — Don't accidentally change this when editing the file. The S01 round-trip test catches it, but worth knowing.
- **`annotate_run` vs `supports_annotations`** — LinearBackend sets `supports_annotations = true`. The `annotate_run` method posts a tagged comment. The tag prefix `[assay:manifest]` makes it identifiable but is NOT used by `read_run_state` — `read_run_state` reads the latest comment regardless of tag.

## Open Risks

- **reqwest::blocking + feature flag interaction** — `reqwest::blocking` internally creates a tokio runtime. If the `linear` feature is enabled alongside `telemetry` (M009/S05), both could try to create runtimes. `reqwest::blocking` manages its own; OTel uses `rt-tokio`. These should not conflict (different runtimes), but worth verifying with `cargo test --features linear,telemetry` after implementation.
- **Linear API rate limits** — Linear's API has rate limits (not documented precisely). Multiple rapid `push_session_event` calls during a fast orchestration run could hit rate limits. No retry logic is in scope (fail fast per S02 constraints), but this could affect UAT.
- **GraphQL schema evolution** — Linear may change field names or require new required fields. Contract tests with mock responses won't catch this. Real API validation is UAT only.
- **`read_run_state` fetching latest comment may return annotation instead of status** — If `annotate_run` is called after the last `push_session_event`, the latest comment contains a manifest annotation, not an `OrchestratorStatus` JSON. `serde_json::from_str` will fail. Solution: use `comments(last: 5)` and iterate backwards to find the first valid JSON parse, OR filter by comment body not starting with `[assay:`.

## Linear GraphQL API Shapes (Validated)

Based on the Kata CLI `linear-client.ts` (production-proven):

### Create Issue
```graphql
mutation CreateIssue($input: IssueCreateInput!) {
  issueCreate(input: $input) {
    success
    issue { id }
  }
}
```
Input: `{ teamId: "...", title: "...", description: "..." }`

### Create Comment
```graphql
mutation CreateComment($input: CommentCreateInput!) {
  commentCreate(input: $input) {
    success
    comment { id body }
  }
}
```
Input: `{ issueId: "...", body: "..." }`

### Get Latest Comment
```graphql
query GetIssueComments($id: String!) {
  issue(id: $id) {
    comments(last: 1) {
      nodes { body }
    }
  }
}
```
Returns: `data.issue.comments.nodes[0].body`

## Mock Testing Strategy

Use a lightweight mock HTTP approach (no external mock server crate):

1. `LinearClient` accepts a `base_url: String` parameter (defaults to `https://api.linear.app`)
2. In tests, spin up a `std::net::TcpListener` on `127.0.0.1:0` and handle one request at a time
3. Or simpler: inject a `Box<dyn Fn(request) -> response>` transport function into `LinearClient` for pure unit tests without any HTTP server
4. Recommended: use `mockito` crate (lightweight, sync-compatible, widely used) — add as dev-dependency behind `linear` feature

Best approach: make `LinearClient::new(api_key, base_url)` accept a configurable base URL. Tests point to a local mock server. Production uses `https://api.linear.app/graphql`.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Linear API | `sickn33/antigravity-awesome-skills@linear-claude-skill` (119 installs) | available — not relevant (Claude Code skill, not Rust implementation) |
| reqwest | none found | n/a |

No relevant skills discovered — this is a Rust implementation task using standard library patterns.

## Sources

- Linear GraphQL API shapes validated against Kata CLI `linear-client.ts` (source: `~/.kata-cli/agent/extensions/linear/linear-client.ts`)
- `reqwest::blocking` internals — creates its own tokio runtime, safe for sync contexts (source: reqwest crate documentation)
- Existing `LocalFsBackend` implementation (source: `crates/assay-core/src/state_backend.rs`)
- S01 summary and forward intelligence (source: `.kata/milestones/M011/slices/S01/S01-SUMMARY.md`)
