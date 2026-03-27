# S03: Health endpoint + final verification — Research

**Date:** 2026-03-27
**Domain:** axum routing, health checks, milestone verification
**Confidence:** HIGH

## Summary

S03 has three deliverables: (1) a `GET /health` endpoint that bypasses auth middleware, (2) README update documenting the endpoint, and (3) a full milestone verification pass confirming all M011 success criteria.

**Critical finding: S02 was merged with only docs/planning artifacts — no code changes.** The squash merge commit `13794d76` contains only 4 files (all `.kata/` docs). The 52 `eprintln!` calls remain across smelt-cli, and the flaky test was not fixed. S03 depends on S02 for "all tracing migration complete, zero eprintln" — this dependency is **unmet**. The milestone's success criterion "Zero `eprintln!` calls in `crates/smelt-cli/src/` (except the error handler in `main.rs`)" cannot be verified as passing. S03 must scope its own deliverables (health endpoint, README, verification) and clearly document the S02 gap in the verification report.

The health endpoint itself is straightforward. The axum router in `http_api.rs` uses `from_fn_with_state` middleware for auth (D135). To bypass auth for `/health`, the standard axum pattern is to use `Router::merge()` — define the health route on a separate router without the auth layer, then merge it with the authenticated router. Alternatively, add `/health` before the middleware layer using axum's route layering. Both approaches are clean; the merge approach is more explicit.

## Recommendation

**Health endpoint implementation:**

1. In `build_router()`, create the health route on a separate `Router` that does NOT go through the auth middleware layer. Use `Router::merge()` to combine the unauthenticated health route with the authenticated API routes.

2. Handler: `async fn health_check() -> Json<serde_json::Value>` returning `{"status": "ok"}` with implicit 200.

3. Test: integration test using `start_test_server_with_auth()` with auth configured, sending `GET /health` without an `Authorization` header, asserting 200 and `{"status": "ok"}` body.

**Auth bypass pattern (axum 0.8):**

```rust
pub(crate) fn build_router(state: SharedState, auth: Option<ResolvedAuth>) -> Router {
    let api_routes = Router::new()
        .route("/api/v1/jobs", post(post_job))
        .route("/api/v1/jobs", get(list_jobs))
        .route("/api/v1/jobs/{id}", get(get_job))
        .route("/api/v1/jobs/{id}", delete(delete_job))
        .layer(axum::middleware::from_fn_with_state(auth, auth_middleware))
        .with_state(state.clone());

    let health_routes = Router::new()
        .route("/health", get(health_check));

    api_routes.merge(health_routes)
}
```

This places `/health` outside the auth middleware scope entirely.

**README update:** Add a "Health Check" section under the `smelt serve` documentation describing the endpoint, its unauthenticated nature, and expected response.

**Verification pass:** Run all milestone success criteria checks and document results. S02-dependent criteria (eprintln migration, tracing output) will be documented as blocked/failing.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Auth bypass for specific routes | axum `Router::merge()` (built-in) | Standard axum pattern for mixed-auth route groups |
| JSON responses | `axum::Json` + `serde_json::json!` (already in deps) | Same pattern as all existing API handlers |
| Integration test HTTP client | `reqwest` (already in dev-deps) | Same pattern as existing auth integration tests |

## Existing Code and Patterns

- `crates/smelt-cli/src/serve/http_api.rs:184-193` — `build_router()` is the single assembly point for the axum router. Health route goes here.
- `crates/smelt-cli/src/serve/http_api.rs:118-177` — `auth_middleware()` uses `from_fn_with_state(auth, ...)`. Routes outside this layer bypass auth automatically.
- `crates/smelt-cli/src/serve/tests/mod.rs:49-62` — `start_test_server_with_auth()` helper already supports auth in tests. Health test uses this with `Some(ResolvedAuth { ... })`.
- `crates/smelt-cli/src/serve/tests/http.rs` — Existing HTTP API tests (POST, GET, DELETE, auth). Health test follows same pattern.
- `README.md` — 356 lines, covers all subcommands. Health endpoint docs go in the `smelt serve` section.

## Constraints

- **D140 (health unauthenticated):** `GET /health` bypasses auth middleware, returns 200 even when `[auth]` is configured.
- **D127 (deny(missing_docs)):** All new public items need doc comments. The health handler is `pub(crate)` so this applies.
- **http_api.rs is 315 lines** — Well under 500L. Adding ~15 lines for the health handler and router restructure keeps it there.
- **axum 0.8** — Uses `Router::merge()` for combining route groups (stable API).
- **S02 dependency unmet** — 52 `eprintln!` calls remain; tracing migration did not land. Milestone success criteria for R061 and R062 cannot be verified as passing.

## Common Pitfalls

- **Health route accidentally placed inside auth middleware scope** — If `/health` is added to the existing `Router::new()` chain before `.layer(auth_middleware)`, it WILL be covered by auth. The fix is to use a separate `Router` and merge. The code structure must make this visually obvious.
- **`with_state` type mismatch on merge** — The health route doesn't need `SharedState`, but `Router::merge()` requires compatible state types or using `Router<()>` for the health side. Since `health_check` takes no state parameter, `Router::new().route("/health", get(health_check))` produces a `Router<()>` which merges cleanly with a `Router<SharedState>` because axum 0.8 coerces stateless routes. Verify at compile time.
- **Forgetting the README update** — The milestone success criteria don't explicitly list README, but the roadmap says "README updated" and R063 notes describe it. Include it.

## Open Risks

- **S02 code not merged** — The milestone verification pass will show failures for eprintln migration (R062) and flaky test fix (R061). These are pre-existing gaps from S02's incomplete merge, not S03 issues. The verification report must clearly separate S03-owned results from S02-blocked results.
- **No existing health check tests to regress** — This is net-new code. Risk of missing edge cases is low given the simplicity (static JSON response, no state access).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| axum | No directly relevant skill found | axum routing is standard; no specialized skill needed |
| Rust | `apollographql/skills@rust-best-practices` (4.8K installs) | Available but not relevant to this narrow scope |

## Sources

- Codebase exploration: `http_api.rs` (315L), `serve.rs` (107L), `tests/mod.rs` (62L), `config.rs`
- axum 0.8 `Router::merge()` — standard pattern for combining authenticated and unauthenticated route groups
- S02-RESEARCH.md — tracing migration context (research complete but code not merged)
- D135, D140 in DECISIONS.md — auth middleware design and health bypass decision
