---
estimated_steps: 5
estimated_files: 2
---

# T01: AuthConfig struct, env var resolution, and auth middleware function

**Slice:** S01 — Bearer token auth middleware with read/write split
**Milestone:** M010

## Description

Implement the core building blocks for bearer token authentication: the `AuthConfig` serde struct for TOML parsing, the `ResolvedAuth` struct holding actual token values resolved from env vars, the resolution function that fails fast on empty/missing vars, and the `auth_middleware` async function that checks the `Authorization: Bearer <token>` header and enforces the read/write permission split. Update `build_router()` to accept optional auth state and conditionally apply the middleware layer.

## Steps

1. Add `AuthConfig` struct to `config.rs` with `write_token_env: String` (required when `[auth]` present) and `read_token_env: Option<String>` (optional). Add `#[serde(deny_unknown_fields)]` per existing pattern. Add `#[serde(default)] pub auth: Option<AuthConfig>` to `ServerConfig`.
2. Create `ResolvedAuth` struct (Clone, Debug) in `http_api.rs` with `write_token: String` and `read_token: Option<String>`. Add `pub(crate) fn resolve_auth(config: &AuthConfig) -> anyhow::Result<ResolvedAuth>` that reads env vars by name, rejects empty strings, and returns clear error messages naming the env var.
3. Write the `auth_middleware` async function in `http_api.rs`: extract `State(auth)` as `Option<ResolvedAuth>`; if `None`, call `next.run(request).await`; otherwise extract `Authorization` header, parse `Bearer <token>`, return 401 JSON on missing/malformed; determine permission level from method (GET/HEAD = read, else = write); for read operations accept read_token OR write_token; for write operations accept only write_token; return 403 JSON on permission denied.
4. Update `build_router()` signature to accept `Option<ResolvedAuth>` and apply `.layer(axum::middleware::from_fn_with_state(auth, auth_middleware))` on the router. The middleware receives `Option<ResolvedAuth>` as state so it's always applied but internally no-ops when `None`.
5. Verify: `cargo check --workspace`, `cargo doc --workspace --no-deps` zero warnings. All doc comments on new public items per D127.

## Must-Haves

- [ ] `AuthConfig` struct with `write_token_env: String` and `read_token_env: Option<String>`, `deny_unknown_fields`
- [ ] `ServerConfig.auth: Option<AuthConfig>` with `#[serde(default)]` — existing TOML files without `[auth]` still parse
- [ ] `ResolvedAuth` struct with resolved token values
- [ ] `resolve_auth()` fails on missing/empty env vars with descriptive error naming the env var
- [ ] `auth_middleware` checks Bearer token, returns 401 JSON for missing/invalid, 403 JSON for wrong permission
- [ ] `build_router()` accepts `Option<ResolvedAuth>` and applies middleware layer
- [ ] All new public items have doc comments (D127)

## Verification

- `cargo check --workspace` compiles without errors
- `cargo doc --workspace --no-deps` exits 0 with zero warnings
- Existing code that calls `build_router()` will need updating in T02, but T01's changes should compile in isolation (update the one callsite in http_api.rs tests if needed)

## Observability Impact

- Signals added: 401 and 403 JSON response bodies with `{"error": "..."}` reason strings
- How a future agent inspects this: HTTP response status + body on auth failure
- Failure state exposed: Startup abort with descriptive error when env var is missing or empty

## Inputs

- `crates/smelt-cli/src/serve/config.rs` — existing `ServerConfig` with `deny_unknown_fields`
- `crates/smelt-cli/src/serve/http_api.rs` — existing `build_router()` with `SharedState`
- Research finding: `axum::middleware::from_fn_with_state` is the correct API for injecting auth state
- D014/D112: env var names in config, never raw tokens
- D127: `deny(missing_docs)` enforced on smelt-cli
- D132/D133/D134: bearer tokens, read/write split, opt-in auth

## Expected Output

- `crates/smelt-cli/src/serve/config.rs` — `AuthConfig` struct added, `ServerConfig.auth` field added
- `crates/smelt-cli/src/serve/http_api.rs` — `ResolvedAuth`, `resolve_auth()`, `auth_middleware()` added; `build_router()` signature updated
