---
estimated_steps: 5
estimated_files: 2
---

# T01: Implement health endpoint with auth-bypass and integration test

**Slice:** S03 — Health endpoint + final verification
**Milestone:** M011

## Description

Add a `GET /health` endpoint to the axum router in `http_api.rs` that bypasses the auth middleware entirely. The endpoint returns `{"status": "ok"}` with a 200 status code. An integration test proves the endpoint works when auth is configured and no `Authorization` header is sent.

## Steps

1. Read `crates/smelt-cli/src/serve/http_api.rs` — understand current `build_router()` structure and auth middleware attachment.
2. Add `health_check` handler: `async fn health_check() -> Json<serde_json::Value>` returning `json!({"status": "ok"})`. Add `/// Health check endpoint...` doc comment (D127).
3. Restructure `build_router()` using `Router::merge()`: authenticated API routes keep the auth middleware layer; health route on a separate `Router::new().route("/health", get(health_check))` merged after. The health router is `Router<()>` — axum 0.8 coerces stateless routes on merge.
4. Add integration test `test_health_endpoint_bypasses_auth` in `tests/http.rs`: use `start_test_server_with_auth()` with `Some(ResolvedAuth { ... })`, send `GET /health` with no auth header via reqwest, assert status 200, assert body contains `"status": "ok"`.
5. Run `cargo test -p smelt-cli health`, then `cargo test --workspace`, then `cargo clippy --workspace`.

## Must-Haves

- [ ] `health_check` handler exists with doc comment
- [ ] `build_router()` uses `Router::merge()` to place health route outside auth middleware
- [ ] Integration test sends `GET /health` without auth header against auth-configured server
- [ ] Test asserts 200 status and `{"status": "ok"}` JSON body
- [ ] All existing tests pass unchanged
- [ ] `cargo clippy --workspace` clean

## Verification

- `cargo test -p smelt-cli health` — new health test passes
- `cargo test --workspace` — all tests pass, 0 failures
- `cargo clippy --workspace` — zero warnings

## Observability Impact

- Signals added/changed: `/health` endpoint returns 200 as a liveness signal for load balancers and monitoring
- How a future agent inspects this: `curl http://localhost:<port>/health` on any running `smelt serve`
- Failure state exposed: non-200 or connection refused means server is not healthy

## Inputs

- `crates/smelt-cli/src/serve/http_api.rs` — current router assembly (build_router, auth_middleware)
- `crates/smelt-cli/src/serve/tests/mod.rs` — `start_test_server_with_auth()` helper
- `crates/smelt-cli/src/serve/tests/http.rs` — existing HTTP API test patterns
- S03-RESEARCH.md — Router::merge() pattern, pitfall notes on state type coercion

## Expected Output

- `crates/smelt-cli/src/serve/http_api.rs` — modified: `health_check` handler + restructured `build_router()` with `Router::merge()`
- `crates/smelt-cli/src/serve/tests/http.rs` — modified: new `test_health_endpoint_bypasses_auth` test
