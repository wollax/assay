# S01: Bearer token auth middleware with read/write split

**Goal:** `smelt serve` with `[auth]` config enforces bearer token authentication with a read/write permission split on the HTTP API. No config = no auth (backward compat).
**Demo:** Requests without a valid `Authorization: Bearer <token>` header return 401; a read-only token can GET but gets 403 on POST/DELETE; a read-write token has full access; no `[auth]` section = current behavior.

## Must-Haves

- `AuthConfig` struct in `config.rs` with `read_token_env` (optional) and `write_token_env` (required) fields, env var names resolved at startup
- `ServerConfig.auth: Option<AuthConfig>` with `#[serde(default)]` — backward compatible with existing `server.toml` files
- Auth middleware using `axum::middleware::from_fn_with_state` that checks `Authorization: Bearer <token>` header
- Permission logic: GET/HEAD = read (accept read or write token), POST/DELETE = write (accept only write token)
- No `[auth]` section → middleware is no-op (all requests pass through)
- Missing/malformed Authorization header → 401
- Valid token but wrong permission level → 403
- Empty env var at startup → error (fail fast, don't start server)
- All existing tests pass unchanged (no auth configured = pass-through)
- Unit tests covering all token×permission combinations
- `#![deny(missing_docs)]` compiles clean with all new public items documented

## Proof Level

- This slice proves: contract + integration (axum router-level tests with real HTTP requests)
- Real runtime required: no (test server on localhost, no Docker)
- Human/UAT required: no (all combinations tested programmatically)

## Verification

- `cargo test --workspace` passes with 0 failures, existing 286+ tests still green
- New auth tests in `crates/smelt-cli/src/serve/tests/http.rs` cover:
  - No auth config → all requests pass (existing tests prove this implicitly)
  - Auth config + missing header → 401 on GET, POST, DELETE
  - Auth config + invalid token → 401
  - Auth config + read token on GET → 200
  - Auth config + read token on POST → 403
  - Auth config + read token on DELETE → 403
  - Auth config + write token on GET → 200
  - Auth config + write token on POST → 200
  - Auth config + write token on DELETE → 200
  - Auth config + write-only (no read token) → write token works for both read and write
- `cargo clippy --workspace` clean
- `cargo doc --workspace --no-deps` zero warnings

## Observability / Diagnostics

- Runtime signals: `tracing::warn!` on rejected requests (401/403) with method + path for debugging; `tracing::info!` at startup when auth is enabled listing which env vars are configured
- Inspection surfaces: HTTP response status codes (401/403) with JSON error bodies containing reason strings
- Failure visibility: Startup fails fast with clear error message if configured env vars are unset or empty
- Redaction constraints: Never log token values; log only env var names and whether they resolved successfully

## Integration Closure

- Upstream surfaces consumed: `ServerConfig` (config.rs), `build_router()` (http_api.rs), `start_test_server()` (tests/mod.rs), `execute()` (serve.rs)
- New wiring introduced in this slice: `AuthConfig` parsed from `[auth]` TOML → env vars resolved in `execute()` → `AuthState` injected as middleware state on the router → middleware checks every request
- What remains before the milestone is truly usable end-to-end: S03 documents `[auth]` in `examples/server.toml` and README; S02 handles unrelated code quality items

## Tasks

- [x] **T01: AuthConfig struct, env var resolution, and auth middleware function** `est:45m`
  - Why: Core implementation — the config struct, env resolution logic, and middleware function are the foundation everything else builds on
  - Files: `crates/smelt-cli/src/serve/config.rs`, `crates/smelt-cli/src/serve/http_api.rs`
  - Do: Add `AuthConfig` struct with `write_token_env` (required) and `read_token_env` (optional) to config.rs; add `auth: Option<AuthConfig>` to `ServerConfig`; create `ResolvedAuth` struct holding resolved token values; write `resolve_auth()` that reads env vars and fails on empty; write `auth_middleware` async fn using `from_fn_with_state`; update `build_router()` to accept `Option<ResolvedAuth>` and conditionally apply the middleware layer; honor D127 `deny(missing_docs)` on all new items
  - Verify: `cargo check --workspace` compiles; `cargo doc --workspace --no-deps` zero warnings
  - Done when: `AuthConfig` parses from TOML, env vars resolve, middleware compiles, `build_router` accepts auth state

- [x] **T02: Wire auth into serve startup and update test helper** `est:30m`
  - Why: The middleware must be wired into the real `execute()` path and the test helper must support auth for T03's tests
  - Files: `crates/smelt-cli/src/commands/serve.rs`, `crates/smelt-cli/src/serve/tests/mod.rs`, `crates/smelt-cli/src/serve/mod.rs`
  - Do: Update `execute()` to resolve auth env vars between config load and router build, pass `Option<ResolvedAuth>` to `build_router()`; update `start_test_server()` to accept optional auth config; update `mod.rs` re-exports if needed; verify all existing tests compile and pass (they use `start_test_server(state)` without auth → backward compat)
  - Verify: `cargo test --workspace` all existing tests pass unchanged (286+); `test_serve_http_responds_while_running` still passes
  - Done when: `execute()` resolves auth; test helper supports auth; all existing tests green

- [x] **T03: Auth integration tests covering all token×permission combinations** `est:45m`
  - Why: Proves the middleware actually works — this is the verification that closes the slice
  - Files: `crates/smelt-cli/src/serve/tests/http.rs`
  - Do: Write tests using `start_test_server` with auth config: no-header→401, bad-token→401, read-token-GET→200, read-token-POST→403, read-token-DELETE→403, write-token-GET→200, write-token-POST→200, write-token-DELETE→200, write-only-mode (no read token)→write-token-works-for-read-and-write; verify 401/403 responses include meaningful JSON error bodies
  - Verify: `cargo test --workspace` all pass; `cargo clippy --workspace` clean; `cargo doc --workspace --no-deps` zero warnings
  - Done when: All 9+ auth test cases pass; full workspace test suite green; clippy and doc clean

## Files Likely Touched

- `crates/smelt-cli/src/serve/config.rs`
- `crates/smelt-cli/src/serve/http_api.rs`
- `crates/smelt-cli/src/serve/mod.rs`
- `crates/smelt-cli/src/commands/serve.rs`
- `crates/smelt-cli/src/serve/tests/mod.rs`
- `crates/smelt-cli/src/serve/tests/http.rs`
