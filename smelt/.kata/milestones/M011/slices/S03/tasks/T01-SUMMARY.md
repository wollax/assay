---
id: T01
parent: S03
milestone: M011
provides:
  - GET /health endpoint returning {"status":"ok"} with 200
  - health_check handler with doc comment (D127)
  - Router::merge() structure separating auth and health routes
  - Integration test proving health bypasses auth middleware
key_files:
  - crates/smelt-cli/src/serve/http_api.rs
  - crates/smelt-cli/src/serve/tests/http.rs
key_decisions:
  - "health_check is a stateless handler — Router<()> merged into the stateful API router"
patterns_established:
  - "Router::merge() pattern for auth-bypass routes in build_router()"
observability_surfaces:
  - "GET /health returns 200 {\"status\":\"ok\"} — liveness probe for load balancers"
duration: 10min
verification_result: passed
completed_at: 2026-03-27T00:00:00Z
blocker_discovered: false
---

# T01: Implement health endpoint with auth-bypass and integration test

**Added `GET /health` endpoint outside auth middleware using `Router::merge()`, with integration test proving it returns 200 when auth is configured and no token is sent**

## What Happened

Added a `health_check` async handler returning `Json(json!({"status": "ok"}))` with a doc comment explaining it lives outside auth intentionally. Restructured `build_router()` to separate authenticated API routes (with auth middleware layer and shared state) from the health route on a stateless `Router::new()`, merged via `Router::merge()`. The health router is `Router<()>` — axum 0.8 coerces stateless routes on merge, so no state plumbing needed.

Added `test_health_endpoint_bypasses_auth` integration test in `tests/http.rs` that starts a server with `start_test_server_with_auth()` using both read and write tokens configured, sends `GET /health` with no `Authorization` header, and asserts 200 status with `{"status":"ok"}` body.

## Verification

- `cargo test -p smelt-cli health` — 1 passed (new health test), plus 1 compose healthcheck test (pre-existing)
- `cargo test --workspace` — 161 passed, 0 failed
- `cargo clippy --workspace` — zero warnings

Slice-level checks:
- ✅ `cargo test -p smelt-cli health` — passes
- ✅ `cargo test --workspace` — all pass, 0 failures
- ✅ `cargo clippy --workspace` — clean
- ⏳ `cargo doc --workspace --no-deps` — not yet run (T02 scope)
- ⏳ `rg 'GET /health' README.md` — README update is T02

## Diagnostics

`GET /health` on any running `smelt serve` instance returns 200 with `{"status":"ok"}`. Non-200 or connection refused means server is not healthy. No sensitive data exposed.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/http_api.rs` — Added `health_check` handler, restructured `build_router()` with `Router::merge()`
- `crates/smelt-cli/src/serve/tests/http.rs` — Added `test_health_endpoint_bypasses_auth` integration test
