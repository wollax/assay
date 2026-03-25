---
estimated_steps: 4
estimated_files: 4
---

# T02: Wire auth into serve startup and update test helper

**Slice:** S01 — Bearer token auth middleware with read/write split
**Milestone:** M010

## Description

Connect the auth middleware to the real `smelt serve` execution path and update the test infrastructure to support auth-enabled tests. The `execute()` function resolves auth env vars between config load and router build. The `start_test_server()` helper gains an auth parameter so T03 can write auth integration tests. All existing tests must pass unchanged.

## Steps

1. Update `execute()` in `crates/smelt-cli/src/commands/serve.rs`: after `ServerConfig::load()`, if `config.auth.is_some()` call `resolve_auth(&config.auth.unwrap())` to get `Option<ResolvedAuth>`. Add a `tracing::info!` when auth is enabled. Pass `resolved_auth` to `build_router()`.
2. Update `start_test_server()` in `crates/smelt-cli/src/serve/tests/mod.rs` to accept an `Option<ResolvedAuth>` parameter and pass it to `build_router()`. Update all existing callsites to pass `None` (no auth = backward compat).
3. Update `crates/smelt-cli/src/serve/mod.rs` re-exports: ensure `ResolvedAuth` and `resolve_auth` are accessible from `crate::serve::` for the serve command and tests.
4. Run `cargo test --workspace` — all 286+ existing tests must pass. Specifically verify `test_serve_http_responds_while_running` still passes (it calls `execute()` with a config that has no `[auth]` section).

## Must-Haves

- [ ] `execute()` resolves auth env vars and passes resolved auth to `build_router()`
- [ ] `start_test_server()` accepts `Option<ResolvedAuth>` parameter
- [ ] All existing test callsites updated to pass `None` for auth
- [ ] All 286+ existing tests pass unchanged
- [ ] `test_serve_http_responds_while_running` passes (no `[auth]` in its config)

## Verification

- `cargo test --workspace` all pass, 0 failures
- `cargo clippy --workspace` clean

## Observability Impact

- Signals added: `tracing::info!("Auth enabled: write_token_env={}, read_token_env={:?}", ...)` at startup (env var names only, never values)
- How a future agent inspects this: Check serve log for auth-enabled line
- Failure state exposed: `resolve_auth()` returns error with env var name before server starts

## Inputs

- T01 output: `AuthConfig`, `ResolvedAuth`, `resolve_auth()`, updated `build_router()` signature
- `crates/smelt-cli/src/commands/serve.rs` — existing `execute()` function
- `crates/smelt-cli/src/serve/tests/mod.rs` — existing `start_test_server()` helper

## Expected Output

- `crates/smelt-cli/src/commands/serve.rs` — `execute()` resolves auth and passes to router
- `crates/smelt-cli/src/serve/tests/mod.rs` — `start_test_server()` accepts auth param
- `crates/smelt-cli/src/serve/mod.rs` — re-exports updated
- All existing tests green
