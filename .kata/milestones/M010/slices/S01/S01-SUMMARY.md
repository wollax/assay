---
id: S01
parent: M010
milestone: M010
provides:
  - AuthConfig struct with deny_unknown_fields for TOML parsing
  - ServerConfig.auth optional field (backward-compatible serde)
  - ResolvedAuth struct with resolved token values
  - resolve_auth() fails fast on missing/empty env vars at startup
  - auth_middleware() enforces bearer token with read/write split
  - build_router() accepts Option<ResolvedAuth> and conditionally applies middleware
  - start_test_server_with_auth() helper for auth-enabled integration tests
  - 4 integration tests covering all token×permission combinations (9+ cases)
requires:
  - slice: none
    provides: independent (no upstream dependencies)
affects:
  - S03
key_files:
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/http_api.rs
  - crates/smelt-cli/src/commands/serve.rs
  - crates/smelt-cli/src/serve/tests/mod.rs
  - crates/smelt-cli/src/serve/tests/http.rs
key_decisions:
  - "D132: Bearer token auth via pre-shared tokens in Authorization header"
  - "D133: Read/write permission split — two token levels"
  - "D134: Auth is opt-in (off by default) for backward compatibility"
  - "D135: Auth middleware uses Option<ResolvedAuth> as state, always applied"
  - "D136: write_token_env required, read_token_env optional"
  - "D137: GET/HEAD = read, all other methods = write"
  - "D138: ResolvedAuth fields pub(crate) for direct test construction"
patterns_established:
  - "Bearer token extraction: parse Authorization header, strip 'Bearer ' prefix"
  - "Permission check: GET/HEAD = read (accept read OR write token), else = write (accept only write token)"
  - "start_test_server_with_auth() layering: backward-compat wrapper delegates to auth variant with None"
  - "Auth test factories: auth_both_tokens()/auth_write_only() + start_auth_server() wrapper"
observability_surfaces:
  - "tracing::warn on 401/403 with method + path (never token values)"
  - "tracing::info at startup listing configured auth env var names"
  - "JSON error bodies on 401 ({error: 'missing or malformed...'}) and 403 ({error: 'token does not have...'})"
  - "Startup abort with descriptive error naming missing/empty env var"
drill_down_paths:
  - .kata/milestones/M010/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M010/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M010/slices/S01/tasks/T03-SUMMARY.md
duration: 30min
verification_result: passed
completed_at: 2026-03-24T00:00:00Z
---

# S01: Bearer token auth middleware with read/write split

**Bearer token auth infrastructure for `smelt serve` HTTP API: opt-in `[auth]` config with env var resolution, read/write permission split middleware, and 4 integration tests proving all token×permission combinations**

## What Happened

T01 built the full auth infrastructure in a single task: `AuthConfig` struct in `config.rs` with `write_token_env` (required) and `read_token_env` (optional), `ResolvedAuth` with env var resolution that fails fast on empty/missing vars, and `auth_middleware()` implementing the read/write split (GET/HEAD = read, everything else = write). The middleware is always applied via `from_fn_with_state` with `Option<ResolvedAuth>` — `None` means no-op pass-through for backward compatibility. `build_router()` was updated to accept the auth state, and `commands/serve.rs` resolves auth at startup with tracing.

T02 verified that T01 had proactively completed all wiring — `execute()` already resolves auth, the test helper already has `start_test_server_with_auth()`, and all 286 existing tests pass unchanged.

T03 added 4 integration tests covering every token×permission combination: missing header → 401 (GET/POST/DELETE), invalid token → 403 (valid Bearer format but unrecognized token), read token split (GET→200, POST→403, DELETE→403, write token all→200), and write-only mode (no read token configured, write token has full access). All tests verify JSON error bodies with descriptive messages.

## Verification

- `cargo test --workspace`: 290 tests passed, 0 failed
- `cargo test -p smelt-cli serve::tests::http::test_auth`: 4/4 auth tests pass
- `cargo clippy --workspace`: clean, no warnings
- `cargo doc --workspace --no-deps`: zero warnings
- All 9+ token×permission combinations verified programmatically
- All existing tests pass unchanged (backward compat confirmed)
- 401 responses include `{"error": "missing or malformed Authorization: Bearer <token> header"}`
- 403 responses include `{"error": "token does not have {read|write} permission"}`

## Requirements Advanced

- R050 (Bearer token authentication) — `[auth]` config enforces bearer tokens; 401 on missing/malformed header; env var resolution at startup
- R051 (Read/write permission split) — Read token GET→200, POST→403, DELETE→403; write token has full access; write-only mode works

## Requirements Validated

- R050 — Proven by 4 integration tests: missing header→401, invalid token→403, valid read token works for GET, valid write token works for all methods; startup fails fast on missing env vars
- R051 — Proven by test_auth_read_token_permission_split and test_auth_write_only_mode covering all method×token combinations

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T03 plan specified "invalid token → 401" but implementation correctly returns 403 for valid Bearer format with unrecognized token value (401 is only for missing/malformed headers). Tests written to match actual middleware semantics — this is the correct behavior.
- T01 proactively completed all T02 work (serve wiring and test helper), making T02 a pure verification task with no code changes.

## Known Limitations

- Auth applies globally to all API routes — no per-route exclusions (e.g. health check endpoint). Not needed currently but would require router restructuring if unauthenticated endpoints are added.
- No rate limiting or brute-force protection on token validation.
- Token comparison is constant-time only if the string comparison implementation is (Rust's `==` on strings is not guaranteed constant-time). Acceptable for pre-shared tokens in trusted networks.

## Follow-ups

- S03 must document `[auth]` section in `examples/server.toml` and README.md
- S02 handles unrelated code quality items (teardown error handling, SSH DRY cleanup)

## Files Created/Modified

- `crates/smelt-cli/src/serve/config.rs` — Added `AuthConfig` struct, `ServerConfig.auth` field
- `crates/smelt-cli/src/serve/http_api.rs` — Added `ResolvedAuth`, `resolve_auth()`, `auth_middleware()`, updated `build_router()` signature
- `crates/smelt-cli/src/commands/serve.rs` — Auth resolution at startup with tracing
- `crates/smelt-cli/src/serve/tests/mod.rs` — Added `start_test_server_with_auth()` helper
- `crates/smelt-cli/src/serve/tests/http.rs` — Added 4 auth integration tests and helper functions

## Forward Intelligence

### What the next slice should know
- Auth middleware is fully wired and tested. S02 can work independently without touching auth code.
- S03 needs to document the `[auth]` section format: `write_token_env` (required string), `read_token_env` (optional string), both are env var names not raw token values.

### What's fragile
- `ResolvedAuth` fields are `pub(crate)` (D138) — if the struct moves to smelt-core, test construction would need updating.
- The middleware always runs (even when `None`) — this is by design but means every request pays the function call overhead of checking `Option::is_none()`.

### Authoritative diagnostics
- Auth test failures → check `crates/smelt-cli/src/serve/tests/http.rs` — all 4 test functions exercise the middleware end-to-end through the real axum router
- Startup auth failures → check `resolve_auth()` in `http_api.rs` — it names the exact env var that's missing/empty

### What assumptions changed
- Original plan assumed 3 tasks of roughly equal size — T01 ended up doing all implementation and wiring, making T02 a verification-only pass. This is fine but means the task estimates were skewed.
