# M010: HTTP API Authentication & Code Quality — Context

**Gathered:** 2026-03-25
**Status:** Ready for planning

## Project Description

Smelt's `smelt serve` HTTP API (`/api/v1/jobs`) currently has no authentication — any client on the network can enqueue, query, and cancel jobs. This was acceptable for trusted local networks during initial development (M006), but is a prerequisite for any deployment beyond localhost.

Additionally, two items of code quality debt from the M009 PR review cycle remain unaddressed: silent teardown error discarding in `phases.rs` and duplicated SSH argument building logic.

## Why This Milestone

The API is the programmatic entry point for CI integration, scripted batch runs, and future tracker integrations (R026). Without authentication, deploying `smelt serve` on a shared network or exposing it to CI runners is a security risk — any client can submit arbitrary manifests. The code quality items are small but real debt that compounds if left unfixed.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Configure bearer tokens in `server.toml` via env var references to secure the API
- Use separate read-only and read-write tokens for monitoring vs mutation
- Continue using `smelt serve` without auth config (backward compatible — auth is opt-in)
- See clear 401/403 errors when credentials are missing or insufficient
- See actionable warnings (not silent success) when container teardown fails

### Entry point / environment

- Entry point: `smelt serve --config server.toml` + HTTP clients (curl, CI scripts)
- Environment: local dev, shared dev network, CI runners
- Live dependencies involved: none

## Completion Class

- Contract complete means: unit tests prove auth middleware rejects/accepts correctly for all token combinations; teardown errors are logged not discarded; SSH args are DRY
- Integration complete means: `smelt serve` HTTP endpoints return 401/403 with auth configured and 200 without
- Operational complete means: none (no service lifecycle changes)

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- `smelt serve` with `[auth]` config rejects unauthenticated requests with 401
- Read-only token can GET but not POST/DELETE (403)
- Read-write token has full access
- No `[auth]` config = current behavior (no auth, full access)
- All 286+ tests pass, cargo doc/clippy/build clean
- Teardown errors produce visible warnings, not silent success

## Risks and Unknowns

- **axum middleware ordering** — auth middleware must run before route handlers but after health/readiness endpoints (if any). Low risk — axum layer ordering is well-documented.
- **Backward compatibility** — existing `server.toml` files must continue to work without `[auth]` section. Low risk — serde defaults handle this.

## Existing Codebase / Prior Art

- `crates/smelt-cli/src/serve/http_api.rs` — 196 lines, `build_router()` constructs the axum Router with 4 routes and SharedState
- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig` with `deny_unknown_fields`, loaded from `server.toml`
- `crates/smelt-cli/src/commands/run/phases.rs` — 359 lines, 6× `let _ = provider.teardown(...)` and 3× `anyhow!("{e}")` error chain loss
- `crates/smelt-cli/src/serve/ssh/client.rs` — 318 lines, `build_ssh_args`/`build_scp_args` share ~90% logic
- D014 / D112 — env var passthrough pattern for credentials (never store secrets in config)
- D102 — `ServerConfig` is separate from job manifests

> See `.kata/DECISIONS.md` for all architectural and pattern decisions — it is an append-only register; read it during planning, append to it during execution.

## Relevant Requirements

- R050 — Bearer token authentication on `smelt serve` HTTP API (new)
- R051 — Read/write permission split for API tokens (new)
- R052 — Teardown error visibility (new, from PR review backlog)
- R053 — SSH argument builder DRY cleanup (new, from PR review backlog)

## Scope

### In Scope

- Bearer token auth on the HTTP API via axum middleware
- Read-only vs read-write token split (GET = read, POST/DELETE = write)
- Token configuration via env var names in `server.toml` `[auth]` section
- Backward compatible — no `[auth]` = no auth (current behavior)
- 401 Unauthorized for missing/invalid token; 403 Forbidden for insufficient permissions
- Teardown error handling cleanup in `phases.rs`
- SSH client `build_ssh_args`/`build_scp_args` DRY refactor
- Error chain preservation (`anyhow!("{e}")` → `.context()`)

### Out of Scope / Non-Goals

- mTLS or certificate-based auth
- User accounts / identity management
- Token rotation / expiry
- Rate limiting
- HTTPS/TLS termination (expected to be handled by a reverse proxy)
- RBAC beyond read/write split

## Technical Constraints

- `ServerConfig` uses `deny_unknown_fields` — new `[auth]` section must be added to the struct
- Auth must be optional — `#[serde(default)]` on the auth config block
- Follow D014/D112 pattern — env var *names* in config, never raw tokens
- All existing tests must pass unchanged when no auth is configured
- `deny(missing_docs)` is enforced on smelt-cli (D127) — all new public items need docs

## Integration Points

- axum Router in `http_api.rs` — middleware layer for auth
- `ServerConfig` in `config.rs` — new `[auth]` section
- `examples/server.toml` — updated with `[auth]` documentation
- `README.md` — server mode section updated with auth config

## Open Questions

- None — design decisions settled during discussion.
