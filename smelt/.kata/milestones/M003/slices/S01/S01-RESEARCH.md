# S01: GitHub Forge Client — Research

**Researched:** 2026-03-21
**Domain:** GitHub API (octocrab), Rust async traits, HTTP mock testing
**Confidence:** HIGH

## Summary

S01 introduces `smelt_core::forge` — a `ForgeClient` trait and `GitHubForge` implementation behind an `octocrab`-gated `forge` feature flag. The work is entirely additive: new module, new Cargo feature, new types, new tests. No existing code is modified except adding `pub mod forge` to `lib.rs` (behind `#[cfg(feature = "forge")]`) and adding `octocrab` as an optional dep to `smelt-core/Cargo.toml`.

The technical surface is small and well-understood: `octocrab` provides a builder-pattern PR creation API and a `PullRequest` model that directly maps to all fields needed for `PrHandle` and `PrStatus`. CI check status requires one additional API call to the combined commit-status endpoint. Mock HTTP testing uses the `wiremock` crate — the same one already in octocrab's own dev-deps — combined with octocrab's `OctocrabBuilder::base_uri()` to redirect requests to the test server.

The primary non-obvious risk is the RPITIT constraint (D019): `ForgeClient` trait methods must use `impl Future<...> + Send` return syntax (not `async fn` directly in trait definitions, since that is `async_fn_in_trait` which requires nightly or a workaround). However, Rust 2024 edition (confirmed in workspace `Cargo.toml`) makes `async fn` in traits stable and equivalent to RPITIT — so `async fn create_pr(...)` in the trait body is legal and produces the right desugaring. The trait will NOT be object-safe (RPITIT precludes it), matching D031 precedent. Callers in `execute_run()` instantiate `GitHubForge` directly; no `dyn ForgeClient` is needed.

## Recommendation

Add `forge` feature flag to `smelt-core`, create `crates/smelt-core/src/forge.rs` with the trait, struct, and types. Use `octocrab = "0.49"` (latest stable; context mentions "0.41 or latest" — 0.49.5 is current). Use `wiremock = "0.6"` in dev-deps for unit tests against a mock HTTP server. Derive `PrState` directly from the PR model's `state` and `merged` fields. Derive `CiStatus` from a second call to the commit combined-status endpoint (or from `pr.mergeable_state` as a lightweight alternative for S01 — document the choice in S01-PLAN and lock it in a decision).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| GitHub API HTTP client | `octocrab = "0.49"` | Tokio-native, already decided (D052); handles auth headers, error parsing, pagination |
| Mock HTTP server for unit tests | `wiremock = "0.6"` | octocrab uses it in its own dev-deps; `MockServer::start().await`, then match on method/path |
| Conditional HTTP requests (ETag) | Built-in on `octocrab` HTTP layer | `_get()` with `If-None-Match` header; raw HTTP API supports custom headers — use in S03 poll loop |
| CI check aggregation | GitHub Commits Combined Status API | `GET /repos/{owner}/{repo}/commits/{sha}/status` returns `state: success|failure|pending|error` |

## Existing Code and Patterns

- `crates/smelt-core/src/provider.rs` — **RPITIT trait pattern to follow exactly**: `fn method(&self, ...) -> impl Future<Output = crate::Result<T>> + Send`. ForgeClient must follow the same structure.
- `crates/smelt-core/src/error.rs` — `SmeltError` enum. Add a `Forge { operation: String, message: String }` variant (mirrors `Provider` structure). octocrab errors implement `std::error::Error`; wrap via `provider_with_source` pattern or a new `forge_with_source` constructor.
- `crates/smelt-core/src/lib.rs` — Module registration. Add `#[cfg(feature = "forge")] pub mod forge;` and `#[cfg(feature = "forge")] pub use forge::{ForgeClient, GitHubForge, ...};` matching the existing `pub use` style.
- `crates/smelt-core/src/manifest.rs` — `#[serde(deny_unknown_fields)]` on every struct (D017). `ForgeConfig` must carry this attribute.
- `crates/smelt-core/src/collector.rs` — Generic struct pattern `ResultCollector<G: GitOps>` (D031). ForgeClient is the same — not object-safe, used with generics. No `dyn` needed in S01.
- `crates/smelt-core/src/docker.rs` — test skip pattern (D024): `#[ignore]` or env-gated skip when daemon unavailable. ForgeClient tests should similarly skip when `GITHUB_TOKEN` is absent (integration), but mock tests never need it.

## Constraints

- **Rust 2024 edition, workspace resolver = "2"** — `async fn` in traits is stable; RPITIT desugaring is automatic. `ForgeClient` trait can use bare `async fn` in the trait body. The trait is **not object-safe** — do not attempt `dyn ForgeClient`.
- **D019 firm** — No `async_trait` macro. Rust 2024 handles this natively.
- **D017 firm** — `#[serde(deny_unknown_fields)]` on `ForgeConfig`.
- **D014 firm** — `GITHUB_TOKEN` is read on the host from `token_env` env var, never passed to the container.
- **D052 firm** — octocrab only; no `gh` CLI shell-out.
- **Feature flag isolation** — `smelt-core` without `--features forge` must compile with zero octocrab deps. `lib.rs` must gate the entire `forge` module. `smelt-cli` in S02 will activate the feature when wiring Phase 9.
- **No `serde_json` in current smelt-core deps** — it comes in transitively via octocrab when feature is enabled. Fine; no explicit dep needed.
- **octocrab `PullRequest` is `#[non_exhaustive]`** — cannot construct in tests; must use API responses (real or mocked). Use wiremock to return crafted JSON rather than constructing the struct directly.
- **`octocrab::Error` is not `Send + Sync` (check before assuming)** — if it implements `std::error::Error + Send + Sync`, wrap with `SmeltError::forge_with_source`; otherwise map to string via `.to_string()`.

## Key octocrab API Surface

### PR Creation
```rust
let octocrab = octocrab::OctocrabBuilder::new()
    .personal_token(token)
    .build()?;

let pr = octocrab
    .pulls(owner, repo)
    .create(title, head, base)
    .body(body)
    .send()
    .await?;
// pr.html_url: Option<Url>
// pr.number: u64
```

### PR Status Poll
```rust
let pr = octocrab.pulls(owner, repo).get(pr_number).await?;
// pr.state: Option<IssueState>   (IssueState::Open / IssueState::Closed)
// pr.merged: Option<bool>
// pr.review_comments: Option<u64>

// PrState derivation:
// merged == Some(true)               → PrState::Merged
// state == Some(IssueState::Closed)  → PrState::Closed
// else                               → PrState::Open
```

### CI Status (additional call)
```rust
// HEAD SHA comes from pr.head.sha
let status = octocrab
    .repos(owner, repo)
    .get_combined_status(sha)  // if this method exists — verify at implementation time
    .await?;
// Fallback: use octocrab._get() raw HTTP if method not available:
// GET /repos/{owner}/{repo}/commits/{sha}/status
// Response: { "state": "success" | "failure" | "pending" | "error" | "unknown" }
```

**Note:** octocrab docs show 15% coverage. If `get_combined_status` isn't available in the typed API, use:
```rust
octocrab._get(format!("/repos/{owner}/{repo}/commits/{sha}/status")).await
// then deserialize manually with a small struct { state: String }
```

### Mock HTTP Server Setup (wiremock)
```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

let server = MockServer::start().await;
let octocrab = octocrab::OctocrabBuilder::new()
    .base_uri(server.uri())?
    .personal_token("fake-token")
    .build()?;

Mock::given(method("POST"))
    .and(path("/repos/owner/repo/pulls"))
    .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
        "number": 42,
        "html_url": "https://github.com/owner/repo/pull/42",
        "state": "open",
        "merged": false,
        // ... minimal valid response
    })))
    .mount(&server)
    .await;
```

## Common Pitfalls

- **`OctocrabBuilder::base_uri()` signature changed across versions** — In 0.49, it is `fn base_uri(self, base_uri: impl TryInto<Uri>) -> Result<Self>`, returning `Result`. Handle the `?` at builder call site, not just at `.build()`.
- **octocrab `PullRequest.state` field** — It's `Option<IssueState>` not a string. `IssueState` is defined in `octocrab::models::IssueState`. Import correctly; don't assume it's in `octocrab::models::pulls`.
- **PR creation 422 Unprocessable Entity** — Happens when head branch doesn't exist on GitHub, or when a PR between the same head/base already exists. The error message from GitHub's API is in the response body; octocrab surfaces it as `octocrab::Error::GitHub { .. }`. Unwrap and log the message verbatim (D014 context: error messages must be clear).
- **`async fn` in trait in Rust 2024 edition** — The compiler desugars to RPITIT automatically. The trait is not object-safe. Any attempt to store as `Box<dyn ForgeClient>` will fail with a clear error. Don't try it.
- **`review_comments` vs `reviews`** — `pr.review_comments` counts inline review comments, not submitted reviews (approvals/change requests). For `review_count` in `PrStatus`, `pr.review_comments` is the simplest no-extra-call field. If actual review count (approvals) is needed in S03, use `pulls.list_reviews(number)`. Document the choice as a decision.
- **Mock JSON must match octocrab's deserialization** — octocrab uses `serde_json` with `#[serde(deny_unknown_fields)]` on some models. The mock response must include all required fields. Start with minimal JSON and add fields as tests fail. Required for `PullRequest`: `url`, `id`, `node_id`, `html_url`, `number`, `state`, `locked`, `maintainer_can_modify`, `head`, `base`.
- **Feature flag in dev-deps** — `wiremock` should only appear in `[dev-dependencies]` of `smelt-core`. octocrab itself goes in `[dependencies]` as optional.

## Open Risks

- **octocrab `get_combined_status` may not exist in 0.49** — Check during T01/T02. Fallback is `_get()` with a manually-deserialized response struct. Low risk since the raw HTTP API always works.
- **Token validation at construction vs call time** — octocrab accepts any string as a personal token; auth errors surface at first API call (401 response). Tests must cover the 401 path with wiremock.
- **PAT scope requirements unclear** — For public repos, `public_repo` scope is sufficient for PR creation. For private repos, `repo` scope required. Fine-grained PATs need `pull_requests: write`. These constraints are GitHub-side; Smelt should surface the HTTP error verbatim. Not a code risk but a documentation note.
- **`octocrab::Error` Send + Sync status** — If octocrab's error type is not `Send + Sync`, it can't be wrapped in `SmeltError` via `Box<dyn Error + Send + Sync>`. Check at implementation: if `octocrab::Error: Send + Sync`, use the `provider_with_source` pattern; otherwise stringify.
- **RPITIT `+ Send` bound propagation** — The `impl Future<Output = ...> + Send` bound on trait methods means `GitHubForge::create_pr` must not capture any non-Send state. octocrab's `Octocrab` instance is `Clone + Send + Sync`, so this is safe. But if any closure or intermediate future isn't `Send`, the compiler error will be opaque. Design the impl to only capture `&self` references to `Octocrab` (which is `Send`).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| octocrab / GitHub API | none found | none found |
| wiremock | none found | none found |

## Sources

- octocrab 0.49.5 docs — PR creation builder, `PullRequestHandler::create()`, `PullRequest` model fields (source: docs.rs/octocrab)
- octocrab 0.49.5 deps — `wiremock = "0.6"` confirmed in octocrab dev-deps (source: docs.rs/octocrab dependency list)
- `crates/smelt-core/src/provider.rs` — RPITIT trait shape to replicate (source: local)
- `crates/smelt-core/src/error.rs` — `SmeltError` variants and constructors to extend (source: local)
- `crates/smelt-core/src/manifest.rs` — `deny_unknown_fields` + validation pattern to replicate for `ForgeConfig` (source: local)
- `crates/smelt-core/Cargo.toml` — Current dependencies; no `[features]` section exists yet (source: local)
- Workspace `Cargo.toml` — `edition = "2024"`, `resolver = "2"`, confirms `async fn` in trait stability (source: local)
