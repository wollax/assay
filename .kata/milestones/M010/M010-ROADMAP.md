# M010: HTTP API Authentication & Code Quality

**Vision:** `smelt serve` gains bearer token authentication with a read/write permission split, securing the HTTP API for shared networks and CI deployments. Alongside, two items of code quality debt from M009's PR review cycle are cleaned up: silent teardown error discarding and SSH argument builder duplication.

## Success Criteria

- `smelt serve` with `[auth]` config rejects unauthenticated requests with 401
- Read-only token can GET but receives 403 on POST/DELETE
- Read-write token has full API access
- No `[auth]` config = current behavior (no auth, full access) — backward compatible
- `cargo test --workspace` passes with 286+ tests, 0 failures
- Teardown errors produce visible `eprintln!` warnings, not silent `let _ =`
- Error chain preserved via `.context()` instead of `anyhow!("{e}")`
- `build_ssh_args` / `build_scp_args` share a common helper — no duplicated flag logic
- `cargo doc --workspace --no-deps` zero warnings
- `cargo clippy --workspace` clean

## Key Risks / Unknowns

- **axum middleware layer ordering** — auth must run on API routes but not break health checks or future unauthenticated endpoints. Low risk — axum's layer system is well-documented and testable.

## Proof Strategy

- **Middleware correctness** → retire in S01 by testing all 4 token states (none configured, missing token, wrong token, valid token) × 2 permission levels (read, write) against real axum routes

## Verification Classes

- Contract verification: unit tests for auth middleware, teardown logging, SSH args; `cargo test/doc/clippy` all green
- Integration verification: HTTP request/response tests with auth headers against `build_router()`
- Operational verification: none
- UAT / human verification: manual curl test with auth headers

## Milestone Definition of Done

This milestone is complete only when all are true:

- `smelt serve` rejects unauthenticated requests when `[auth]` is configured
- Read-only vs read-write permission split works correctly
- No `[auth]` = full access (backward compat)
- All teardown `let _ =` replaced with logged warnings
- All `anyhow!("{e}")` replaced with `.context()`
- SSH arg builders share common helper
- `cargo test --workspace` ≥ 286 tests, 0 failures
- `cargo doc --workspace --no-deps` 0 warnings
- `cargo clippy --workspace` clean
- `examples/server.toml` documents `[auth]` section
- README.md server mode section updated

## Requirement Coverage

- Covers: R050, R051, R052, R053
- Partially covers: none
- Leaves for later: R022 (budget/cost tracking), R026 (tracker integration)
- Orphan risks: none

## Slices

- [x] **S01: Bearer token auth middleware with read/write split** `risk:high` `depends:[]`
  > After this: `smelt serve` with `[auth]` config enforces bearer tokens; read-only token can GET but not POST/DELETE; read-write token has full access; no config = no auth (backward compat); unit tests prove all token×permission combinations.

- [x] **S02: Teardown error handling + SSH DRY cleanup** `risk:low` `depends:[]`
  > After this: teardown failures produce visible warnings instead of silent `let _ =`; error chains preserved via `.context()`; SSH argument builders share a common helper; all existing tests still pass.

- [ ] **S03: Documentation and final verification** `risk:low` `depends:[S01,S02]`
  > After this: `examples/server.toml` documents `[auth]` section; README.md server mode updated with auth config; all milestone success criteria verified in one pass.

## Boundary Map

### S01 (independent)

Produces:
- `AuthConfig` struct in `config.rs` — `read_token_env`, `write_token_env` fields with env var resolution
- `ServerConfig.auth: Option<AuthConfig>` — backward-compatible serde
- `auth_middleware` axum layer — extracts `Authorization: Bearer <token>`, checks permission level (read vs write based on HTTP method), returns 401/403
- `build_router()` updated to apply auth layer when `AuthConfig` is present
- Unit tests covering: no auth config = pass-through; missing header = 401; invalid token = 401; read token on GET = 200; read token on POST = 403; write token on all = 200

Consumes:
- nothing (independent)

### S02 (independent)

Produces:
- `teardown_on_error()` helper in `phases.rs` — replaces 6× `let _ =` blocks with logged warnings
- `.context()` replacing `anyhow!("{e}")` in 3 monitor.write() call sites
- `build_common_ssh_args()` helper in `client.rs` — shared logic for `build_ssh_args`/`build_scp_args`
- All existing tests still passing

Consumes:
- nothing (independent)

### S01, S02 → S03

Produces:
- Updated `examples/server.toml` with `[auth]` section and comments
- Updated `README.md` server mode section with auth configuration
- Full milestone verification pass

Consumes from S01:
- `AuthConfig` struct and `[auth]` config format
Consumes from S02:
- Clean phases.rs and client.rs
