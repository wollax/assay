---
estimated_steps: 5
estimated_files: 2
---

# T01: Add [auth] docs to server.toml and README, run milestone verification

**Slice:** S03 — Documentation and final verification
**Milestone:** M010

## Description

Add a commented-out `[auth]` section to `examples/server.toml` documenting the bearer token auth config format. Add an "Authentication" subsection to the README.md Server Mode section explaining opt-in auth, env var semantics, permission model, and error responses. Update the examples table to mention auth. Then run the full milestone verification pass (`cargo test/clippy/doc`) to confirm all M010 success criteria are met.

## Steps

1. Read `examples/server.toml` and `crates/smelt-cli/src/serve/config.rs:74-86` to confirm exact `AuthConfig` field names
2. Add a commented-out `# [auth]` section to `examples/server.toml` after the `[[workers]]` section, following the existing comment style — document `write_token_env` (required) and `read_token_env` (optional) with clear comments about env var semantics (names, not values)
3. Add an "Authentication" subsection to README.md between "HTTP API Endpoints" (line ~263) and "Queue Persistence" (line ~265), covering: opt-in (no `[auth]` = no auth), env var config (names not raw tokens), permission model (GET/HEAD = read, others = write), error responses (401 missing header, 403 wrong token or insufficient permission)
4. Update the `server.toml` row in the README examples table (~line 291) to mention auth configuration
5. Run full milestone verification: `cargo test --workspace`, `cargo clippy --workspace`, `cargo doc --workspace --no-deps` — all must pass clean

## Must-Haves

- [ ] `examples/server.toml` contains `[auth]` section with `write_token_env` and `read_token_env` fields, commented-out by default
- [ ] `examples/server.toml` `[auth]` comments explain: fields are env var names (not raw tokens), `write_token_env` is required, `read_token_env` is optional
- [ ] README.md has "Authentication" subsection in Server Mode between HTTP API Endpoints and Queue Persistence
- [ ] README.md auth section documents: opt-in behavior, env var names pattern, read/write permission model (GET/HEAD = read), 401 vs 403 distinction
- [ ] README examples table server.toml description mentions auth
- [ ] `cargo test --workspace` ≥290 tests, 0 failures
- [ ] `cargo clippy --workspace` clean
- [ ] `cargo doc --workspace --no-deps` zero warnings

## Verification

- `grep 'write_token_env' examples/server.toml` — present in commented-out auth section
- `grep 'read_token_env' examples/server.toml` — present in commented-out auth section
- `grep '### Authentication' README.md` — subsection exists
- `grep '401' README.md` — error code documented
- `grep '403' README.md` — error code documented
- `cargo test --workspace` — ≥290 tests, 0 failures
- `cargo clippy --workspace` — no warnings
- `cargo doc --workspace --no-deps` — no warnings

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: Read `examples/server.toml` for auth config reference; read README.md Authentication section for user-facing docs
- Failure state exposed: None

## Inputs

- `examples/server.toml` — existing server config example (67 lines, comment-heavy style)
- `README.md` — existing Server Mode section (lines 222-275) with subsection pattern
- `crates/smelt-cli/src/serve/config.rs:74-86` — `AuthConfig` struct (authoritative field names)
- S01 summary — auth middleware behavior, error messages, permission model
- D014/D112 — env var passthrough pattern (names, not values)
- D134 — auth is opt-in
- D136 — `write_token_env` required, `read_token_env` optional
- D137 — GET/HEAD = read, all other methods = write

## Expected Output

- `examples/server.toml` — gains `# [auth]` commented section (~15 lines) after workers section
- `README.md` — gains "### Authentication" subsection (~20 lines) in Server Mode; examples table row updated
