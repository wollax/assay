# S03: Documentation and final verification

**Goal:** Document the `[auth]` config in `examples/server.toml` and README.md, then verify all M010 milestone success criteria pass in one shot.
**Demo:** `examples/server.toml` has a commented-out `[auth]` section with clear field docs; README.md Server Mode section has an Authentication subsection; `cargo test --workspace`, `cargo doc --workspace --no-deps`, and `cargo clippy --workspace` all pass clean.

## Must-Haves

- `examples/server.toml` has an `[auth]` section with `write_token_env` and `read_token_env` fields, commented-out by default, with inline comments explaining env var semantics
- README.md Server Mode has an "Authentication" subsection between "HTTP API Endpoints" and "Queue Persistence"
- README.md auth section documents: opt-in behavior, env var names (not raw tokens), 401 vs 403 distinction, read/write permission model
- `examples/server.toml` description in README examples table mentions auth
- `cargo test --workspace` passes with ≥290 tests, 0 failures
- `cargo clippy --workspace` clean
- `cargo doc --workspace --no-deps` zero warnings

## Proof Level

- This slice proves: final-assembly (documentation completeness + full milestone verification)
- Real runtime required: no (docs only + cargo commands)
- Human/UAT required: yes (README readability is subjective)

## Verification

- `cargo test --workspace` — ≥290 tests, 0 failures
- `cargo clippy --workspace` — clean
- `cargo doc --workspace --no-deps` — 0 warnings
- `grep -c 'write_token_env' examples/server.toml` — ≥1
- `grep -c 'read_token_env' examples/server.toml` — ≥1
- `grep -c 'Authentication' README.md` — ≥1
- `grep -c '\[auth\]' examples/server.toml` — ≥1

## Observability / Diagnostics

- Runtime signals: none (documentation-only slice)
- Inspection surfaces: none
- Failure visibility: none
- Redaction constraints: examples must show env var names (`SMELT_WRITE_TOKEN`), never raw token values

## Integration Closure

- Upstream surfaces consumed: `AuthConfig` struct from S01 (`config.rs:74-86`), `resolve_auth()` behavior from S01 (`http_api.rs:69-97`), `auth_middleware()` permission model from S01 (`http_api.rs:111-170`)
- New wiring introduced in this slice: none (documentation only)
- What remains before the milestone is truly usable end-to-end: nothing — S01 and S02 delivered all code; this slice closes the docs gap and confirms the full verification pass

## Tasks

- [x] **T01: Add [auth] docs to server.toml and README, run milestone verification** `est:15m`
  - Why: Single deliverable — document auth config in both example and README, update examples table, then run the full milestone verification pass (test/clippy/doc) to confirm all M010 success criteria
  - Files: `examples/server.toml`, `README.md`
  - Do: Add commented-out `[auth]` section to server.toml following existing comment style; add Authentication subsection to README Server Mode between HTTP API Endpoints and Queue Persistence; update server.toml row in examples table to mention auth; run cargo test/clippy/doc
  - Verify: `cargo test --workspace` ≥290 tests 0 failures; `cargo clippy --workspace` clean; `cargo doc --workspace --no-deps` 0 warnings; grep confirms auth content in both files
  - Done when: Both files updated, all three cargo commands pass clean, all M010 success criteria verified

## Files Likely Touched

- `examples/server.toml`
- `README.md`
