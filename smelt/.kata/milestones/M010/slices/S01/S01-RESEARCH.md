# S01: Bearer token auth middleware with read/write split — Research

**Date:** 2026-03-25
**Domain:** axum HTTP middleware, bearer token authentication
**Confidence:** HIGH

## Summary

This slice adds optional bearer token authentication to the `smelt serve` HTTP API. The implementation touches three files: `config.rs` (new `AuthConfig` struct), `http_api.rs` (middleware layer on the router), and `tests/http.rs` (auth test coverage).

Axum 0.8 provides `axum::middleware::from_fn_with_state` which is the correct tool — it lets us write a plain `async fn` that extracts the `Authorization` header, checks the token against resolved env var values, and calls `next.run(request).await` on success or returns 401/403 on failure. The middleware function receives the full `Request` object, so it can inspect `request.method()` to decide read vs write permission. No tower dependency beyond what axum already re-exports.

The auth config follows the established D014/D112 env var passthrough pattern: `server.toml` stores env var *names* (e.g. `read_token_env = "SMELT_READ_TOKEN"`), and the actual token values are resolved from the environment at server startup — never stored in config. This is identical to the `key_env` pattern used in `WorkerConfig` and `ForgeConfig`.

## Recommendation

Use `axum::middleware::from_fn_with_state` with an `AuthState` struct (resolved tokens) injected as middleware state. The middleware function:
1. If no `AuthConfig` present → pass through (backward compat)
2. Extract `Authorization: Bearer <token>` header
3. Missing/malformed → 401
4. Check method: GET = read, POST/DELETE = write
5. For read: accept read token OR write token
6. For write: accept only write token
7. Invalid token for permission level → 403

Resolve env vars at startup (in `execute()`) and pass resolved values to the router builder. This avoids repeated `std::env::var()` on every request.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| HTTP middleware | `axum::middleware::from_fn_with_state` | Already in axum 0.8; no new dep needed; type-safe state injection |
| Header extraction | `axum::http::HeaderMap` / `Request::headers()` | Part of axum/http; correct parsing of Authorization header |
| Token comparison | `subtle::ConstantTimeEq` or direct `==` | Constant-time compare prevents timing attacks; however for a local daemon with pre-shared tokens, `==` is acceptable for MVP — timing attack surface is minimal |

## Existing Code and Patterns

- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig` with `deny_unknown_fields`. New `AuthConfig` goes here as `#[serde(default)] pub auth: Option<AuthConfig>`. Follow `WorkerConfig` pattern for the struct.
- `crates/smelt-cli/src/serve/http_api.rs` — `build_router()` constructs the Router with 4 routes and `SharedState`. Middleware layer goes on the router via `.layer(axum::middleware::from_fn_with_state(auth_state, auth_middleware))`.
- `crates/smelt-cli/src/commands/serve.rs` — `execute()` loads config, builds router. Env var resolution for auth tokens happens here, between config load and router build.
- `crates/smelt-cli/src/serve/tests/http.rs` — All HTTP API tests use `start_test_server()` which calls `build_router(state)`. Tests need to be updated to pass auth state, or `build_router` signature changes to accept optional auth config.
- `crates/smelt-cli/src/serve/tests/mod.rs` — `start_test_server()` helper needs updating to support auth.

## Constraints

- `ServerConfig` uses `#[serde(deny_unknown_fields)]` — new `[auth]` section must be explicitly declared on the struct as `Option<AuthConfig>` with `#[serde(default)]`.
- `#![deny(missing_docs)]` is enforced on smelt-cli (D127) — all new public items need doc comments.
- Auth must be **opt-in** (D134) — `None` means no auth, all requests pass through.
- Env var *names* in config, never raw tokens (D014/D112).
- Existing tests must pass **unchanged** when no auth is configured — the middleware must be a no-op when `AuthConfig` is `None`.
- `build_router()` is `pub(crate)` — signature change is internal only, but `start_test_server()` in tests and `execute()` in `serve.rs` both call it.

## Common Pitfalls

- **Middleware state vs app state** — axum's `from_fn_with_state` injects its own state separate from the router's `with_state()`. The auth middleware needs its own `AuthState` (resolved tokens), not the `SharedState` (job queue). Using `from_fn` (no state) and closing over resolved tokens also works but `from_fn_with_state` is cleaner.
- **Method matching for permission split** — `DELETE` is a write operation. Must check `Method::GET` → read, everything else → write. Don't accidentally treat `HEAD` as write (it's read-equivalent).
- **Read token also accepted as write when only one token configured** — If only `read_token_env` is set but no `write_token_env`, what happens? Design decision: both fields should be required when `[auth]` is present. Simplest model. Or: `write_token_env` is required, `read_token_env` is optional (write token always has read access). Either way, needs explicit design in the plan.
- **Empty env var** — `std::env::var("FOO")` returns `Ok("")` if FOO is set but empty. Must treat empty as missing/error at startup.
- **`test_serve_http_responds_while_running` uses hardcoded port 18765** — This test calls `execute()` directly, which reads `ServerConfig`. It will need a config without `[auth]` to remain passing. Since the test writes its own config TOML, this should work with `#[serde(default)]` on the auth field.

## Open Risks

- **axum 0.8 `from_fn_with_state` exact API** — I'm confident this exists in axum 0.8 based on axum's stable middleware API since 0.6. If the exact function name changed in 0.8, the fallback is `from_fn` with a closure capturing the auth state. Very low risk.
- **`reqwest` test client and Authorization header** — Tests use `reqwest::Client`. Adding `.header("Authorization", "Bearer xxx")` is trivial, but existing tests must NOT send auth headers (to prove backward compat). Low risk.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| axum | `bobmatnyc/claude-mpm-skills@axum` (159 installs) | available — not installed |
| axum | `manutej/luxor-claude-marketplace@axum-web-framework` (79 installs) | available — not installed |

Neither skill is needed — the middleware pattern is straightforward and well-understood. axum's `from_fn_with_state` is a single function call with clear types.

## Sources

- Direct codebase exploration: `http_api.rs`, `config.rs`, `serve.rs`, `tests/http.rs`
- axum 0.8 middleware docs (known from prior experience — `axum::middleware::from_fn_with_state` stable since 0.6)
- Existing project decisions: D014, D112, D127, D132, D133, D134
