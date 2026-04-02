# S03: Health endpoint + final verification

**Goal:** `GET /health` returns 200 without auth even when `[auth]` is configured; README documents it; all milestone success criteria verified in one pass.
**Demo:** `curl http://localhost:<port>/health` returns `{"status":"ok"}` with no auth header against an auth-configured `smelt serve` instance.

## Must-Haves

- `GET /health` returns 200 with `{"status":"ok"}` body without an `Authorization` header
- `GET /health` returns 200 even when `[auth]` is configured with valid tokens
- Health route is outside the auth middleware scope (uses `Router::merge()` per D140)
- Integration test exercises health endpoint with auth configured and no auth header
- README has a "Health Check" section documenting the endpoint
- Full milestone verification pass with results documented (S02-blocked criteria clearly separated)

## Proof Level

- This slice proves: integration
- Real runtime required: no (tests use in-process axum test server)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-cli health` — health endpoint integration test passes
- `cargo test --workspace` — all existing tests still pass, 0 failures
- `cargo clippy --workspace` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings
- `rg 'GET /health' README.md` — README documents the endpoint

## Observability / Diagnostics

- Runtime signals: health endpoint itself IS the observability surface — returns 200 when the server is alive
- Inspection surfaces: `GET /health` on any running `smelt serve` instance
- Failure visibility: non-200 response or connection refused indicates server is down
- Redaction constraints: none (health endpoint returns no sensitive data)

## Integration Closure

- Upstream surfaces consumed: `build_router()` in `http_api.rs`, `auth_middleware` in `http_api.rs`, `start_test_server_with_auth()` in `tests/mod.rs`
- New wiring introduced in this slice: `/health` route merged into axum router outside auth middleware layer
- What remains before the milestone is truly usable end-to-end: S02 code (eprintln→tracing migration, flaky test fix) was not merged — R061 and R062 remain unproven. S03-owned work (R063) is complete after this slice.

## Tasks

- [x] **T01: Implement health endpoint with auth-bypass and integration test** `est:20m`
  - Why: Core deliverable — adds the `GET /health` route outside the auth middleware and proves it works with auth configured
  - Files: `crates/smelt-cli/src/serve/http_api.rs`, `crates/smelt-cli/src/serve/tests/http.rs`
  - Do: Add `health_check` handler returning `Json(json!({"status": "ok"}))`. Restructure `build_router()` to use `Router::merge()` — authenticated API routes on one router, health route on a separate stateless router. Add integration test using `start_test_server_with_auth()` with auth configured, sending `GET /health` without `Authorization` header, asserting 200 and JSON body. Add doc comment on handler (D127).
  - Verify: `cargo test -p smelt-cli health` passes; `cargo test --workspace` all pass; `cargo clippy --workspace` clean
  - Done when: health test passes with auth configured and no auth header; all existing tests unbroken

- [x] **T02: README update + milestone verification pass** `est:15m`
  - Why: Documents the health endpoint for users and verifies all M011 success criteria in one pass, separating S02-blocked results
  - Files: `README.md`, `.kata/milestones/M011/slices/S03/S03-SUMMARY.md`
  - Do: Add "Health Check" section to README under `smelt serve` docs. Run full milestone verification: line counts, eprintln grep, cargo test/clippy/doc, health endpoint test. Document results with S02-blocked criteria clearly labeled.
  - Verify: `rg 'Health Check' README.md` finds the section; all S03-owned verification checks pass
  - Done when: README documents health endpoint; verification report written with clear pass/fail for each criterion

## Files Likely Touched

- `crates/smelt-cli/src/serve/http_api.rs`
- `crates/smelt-cli/src/serve/tests/http.rs`
- `README.md`
