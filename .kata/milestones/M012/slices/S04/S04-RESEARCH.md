# S04: Linear Tracker Backend — Research

**Date:** 2026-03-28

## Summary

S04 implements `LinearTrackerSource: TrackerSource` — the Linear equivalent of S03's `GithubTrackerSource`. It polls Linear for issues with a specific label, transitions label state via GraphQL, and delegates to `issue_to_manifest()` (already built in S02) for manifest generation. The architecture mirrors S03 exactly: a `LinearClient` trait with async RPITIT methods, a `ReqwestLinearClient` production impl using `reqwest::Client` for GraphQL HTTP calls, a `MockLinearClient` VecDeque test double, and `LinearTrackerSource<L: LinearClient>` bridging to `TrackerSource`.

The primary reference implementation is Assay's `LinearBackend` in `assay-backends/src/linear.rs`, which uses `reqwest::blocking::Client` for GraphQL against `https://api.linear.app/graphql`. Smelt's version uses async `reqwest::Client` (not blocking) since the tracker runs inside a tokio runtime. The GraphQL operations needed are simpler than Assay's — only issue listing with label filter, label CRUD, and issue label mutations.

## Recommendation

Follow the S03 `github/` directory module pattern exactly: `serve/linear/mod.rs` (trait + re-exports), `serve/linear/client.rs` (ReqwestLinearClient), `serve/linear/source.rs` (LinearTrackerSource), `serve/linear/mock.rs` (MockLinearClient). This preserves structural consistency and makes the codebase predictable.

Use `reqwest` (async, already a dev-dep in smelt-cli at v0.12) promoted to a production dependency. Do NOT use `reqwest::blocking` — Smelt's serve loop is async/tokio. Auth via `LINEAR_API_KEY` env var, resolved at config validation time.

TrackerConfig needs two new fields for Linear: `api_key_env: Option<String>` (name of env var holding the API key) and `team_id: Option<String>` (Linear team ID for issue scoping). Both required when `provider == "linear"`, ignored otherwise. Follows the `repo: Option<String>` pattern from GitHub (D165).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| GraphQL HTTP calls | `reqwest::Client` with `.json()` | Already proven by Assay's LinearBackend; async-native; `json` feature handles serde |
| Linear issue model | Linear GraphQL API — `issues(filter: {labels: {name: {eq: ...}}})` | Standard filter syntax; no need for custom search logic |
| Label lifecycle | `TrackerState::label_name(prefix)` from S02 | Canonical source of label strings; shared with GitHub backend |
| Issue-to-manifest injection | `issue_to_manifest()` from S02 | Complete and tested; S04 should NOT reimplement |
| Mock HTTP server for tests | VecDeque-based `MockLinearClient` (same as `MockGhClient`) | Lighter than `mockito`; consistent with S03 pattern; no new dev-dep needed |

## Existing Code and Patterns

- `crates/smelt-cli/src/serve/github/` — S03 reference architecture: `mod.rs` (trait + types), `client.rs` (subprocess impl), `source.rs` (TrackerSource bridge), `mock.rs` (VecDeque test double). S04 should mirror this exactly.
- `crates/smelt-cli/src/serve/tracker.rs` — `TrackerSource` trait, `issue_to_manifest()`, `load_template_manifest()`, `MockTrackerSource`. S04 calls `issue_to_manifest()` via the same path as S03.
- `crates/smelt-core/src/tracker.rs` — `TrackerIssue`, `TrackerState`, `StateBackendConfig`. All types S04 needs are already defined.
- `crates/smelt-cli/src/serve/config.rs` — `TrackerConfig` with `deny_unknown_fields`. Needs `api_key_env` and `team_id` fields added, validated when `provider == "linear"`.
- `../assay/crates/assay-backends/src/linear.rs` — Assay's `LinearClient` wraps `reqwest::blocking::Client` with `Authorization` header, GraphQL request/response handling, and error extraction from `errors` array. Port this pattern to async.

## Constraints

- **`reqwest` must become a production dep in smelt-cli** — currently only in `[dev-dependencies]`. Move to `[dependencies]` and add to workspace `[workspace.dependencies]`. The `json` feature is needed.
- **`deny_unknown_fields` on `TrackerConfig`** — new fields (`api_key_env`, `team_id`) must be added to the struct even though they're only used by Linear. Use `Option<String>` with `#[serde(default)]` (same pattern as `repo`).
- **D156: `reqwest` for Linear, not `reqwest::blocking`** — Smelt's serve loop is async/tokio. Using blocking client inside a tokio runtime requires `spawn_blocking` or risks panics. Async `reqwest::Client` is the correct choice.
- **D153: Labels, not workflow states** — Linear has both labels and workflow states. Per D153, use labels for consistency with GitHub. The `TrackerState::label_name()` pattern works identically.
- **D164 pattern: generic `<L: LinearClient>`, not `dyn LinearClient`** — RPITIT makes the trait non-object-safe. Same constraint as `GhClient`.
- **No crate dependency on Assay (D002)** — port the GraphQL patterns, don't import.

## Linear GraphQL API — Key Operations

The four operations needed for `LinearClient` trait methods:

### 1. List issues by label name

```graphql
query {
  issues(filter: { labels: { name: { eq: "smelt:ready" } }, team: { id: { eq: "TEAM_ID" } } }) {
    nodes { id identifier title description url }
  }
}
```

Returns `data.issues.nodes[]`. `identifier` is the human-readable ID (e.g. `"KAT-42"`), `id` is the UUID. Use `identifier` as `TrackerIssue.id` and `description` as `TrackerIssue.body`.

### 2. Add label to issue

```graphql
mutation {
  issueAddLabel(id: "ISSUE_UUID", labelId: "LABEL_UUID") {
    success
  }
}
```

### 3. Remove label from issue

```graphql
mutation {
  issueRemoveLabel(id: "ISSUE_UUID", labelId: "LABEL_UUID") {
    success
  }
}
```

### 4. Create label (for ensure_labels)

```graphql
mutation {
  issueLabelCreate(input: { name: "smelt:ready", teamId: "TEAM_ID" }) {
    success issueLabel { id name }
  }
}
```

### 5. Find label by name (needed to get label UUIDs for add/remove)

```graphql
query {
  issueLabels(filter: { name: { eq: "smelt:ready" }, team: { id: { eq: "TEAM_ID" } } }) {
    nodes { id name }
  }
}
```

**Key insight:** `edit_labels` on GitHub is a single CLI call (D166). On Linear, add and remove are separate mutations. The `transition_state()` impl must call remove-old + add-new as two mutations. This is slightly less atomic than GitHub's single-call approach but still sufficient for D157 (label transition is the first action before enqueueing).

**Label UUID resolution:** Linear label mutations require the label UUID, not the label name. `LinearTrackerSource` needs to resolve label names to UUIDs. Two approaches:
1. Resolve on every call (extra query per transition) — simple but adds latency
2. Cache label name→UUID mapping at startup via `ensure_labels()` — more efficient

Recommendation: Cache in a `HashMap<String, String>` populated during `ensure_labels()`. `ensure_labels()` creates labels if missing and returns their UUIDs. The cache is valid for the lifetime of the `LinearTrackerSource` instance. This avoids an extra query on every poll and transition.

## Common Pitfalls

- **GraphQL errors arrive as HTTP 200 with `errors` array** — always check for `json["errors"]` before treating the response as successful. Assay's LinearClient handles this; port the same pattern.
- **Linear issue `id` vs `identifier`** — `id` is a UUID, `identifier` is human-readable (e.g. `KAT-42`). Use `identifier` for TrackerIssue.id (consistent with how humans reference issues) and `id` (UUID) internally for mutations.
- **Label creation idempotency** — Linear's `issueLabelCreate` may error if the label already exists (unlike GitHub's `--force`). Handle by querying first, creating only if not found. `ensure_labels()` should be query-then-create, not create-and-handle-error.
- **`reqwest::Client` reuse** — create one `reqwest::Client` per `ReqwestLinearClient` instance and reuse it. Do not create per-request; connection pooling matters for repeated polling.
- **Rate limiting** — Linear has rate limits. At 30s poll intervals with ~2 queries per poll, this is well within limits. No special handling needed for MVP.
- **Team-scoped label queries** — Labels in Linear can be workspace-level or team-level. Always scope queries by team ID to avoid cross-team collisions.

## Open Risks

- **Linear label creation API may not support idempotent creation** — if `issueLabelCreate` fails on duplicates, the query-then-create pattern adds complexity. Mitigated by the ensure_labels() approach (query first).
- **Label UUID caching invalidation** — if a label is deleted externally between ensure_labels() and a transition call, the cached UUID becomes stale. Low risk for a daemon that runs continuously; labels are rarely deleted. Can add a retry-with-refresh if needed.
- **Integration test gating** — `SMELT_LINEAR_TEST=1` + `SMELT_LINEAR_API_KEY` + `SMELT_LINEAR_TEAM_ID` env vars needed. Tests must create/clean up labels in a real Linear project. Risk: test pollution if cleanup fails.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Linear GraphQL API | none | No relevant agent skill found — API is straightforward enough to implement directly from Assay's reference |
| reqwest | none | Standard Rust HTTP client — no skill needed |

## Sources

- `../assay/crates/assay-backends/src/linear.rs` — Assay's `LinearBackend` with `LinearClient` GraphQL wrapper (local codebase reference)
- `../assay/crates/assay-backends/tests/linear_backend.rs` — Assay's mock-based Linear tests using mockito (local codebase reference)
- `crates/smelt-cli/src/serve/github/` — S03 GithubTrackerSource architecture (local codebase reference)
- `crates/smelt-cli/src/serve/tracker.rs` — S02 TrackerSource trait and utilities (local codebase reference)
