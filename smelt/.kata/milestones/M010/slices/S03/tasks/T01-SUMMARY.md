---
id: T01
parent: S03
milestone: M010
provides:
  - Commented-out [auth] section in examples/server.toml with field docs
  - Authentication subsection in README.md Server Mode section
  - Updated examples table mentioning auth config
  - Full milestone verification pass (290 tests, clippy clean, doc clean)
key_files:
  - examples/server.toml
  - README.md
key_decisions: []
patterns_established: []
observability_surfaces:
  - none
duration: 5min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T01: Add [auth] docs to server.toml and README, run milestone verification

**Added commented-out `[auth]` section to `examples/server.toml` and "Authentication" subsection to README.md documenting opt-in bearer auth, env var config pattern, read/write permission model, and 401/403 error responses; full milestone verification passes clean (290 tests, 0 failures, clippy clean, doc clean)**

## What Happened

Added a 15-line commented-out `[auth]` section to `examples/server.toml` after the `[[workers]]` section, documenting `write_token_env` (required) and `read_token_env` (optional) with clear explanations that these are env var names not raw tokens, plus the read/write permission model.

Added a "### Authentication" subsection to README.md between "HTTP API Endpoints" and "Queue Persistence", covering: opt-in behavior (no `[auth]` = open access), env var config pattern with a TOML example, permission model (GET/HEAD = read, others = write), and 401 vs 403 error distinction. Updated the examples table to note auth config.

Ran full milestone verification: `cargo test --workspace` (290 tests, 0 failures), `cargo clippy --workspace` (clean), `cargo doc --workspace --no-deps` (zero warnings).

## Verification

| Check | Status | Evidence |
|-------|--------|----------|
| `write_token_env` in server.toml | ✓ PASS | Present in commented-out auth section |
| `read_token_env` in server.toml | ✓ PASS | Present in commented-out auth section |
| `### Authentication` in README | ✓ PASS | Subsection exists between HTTP API and Queue Persistence |
| 401/403 documented in README | ✓ PASS | Both error codes with descriptions |
| Examples table updated | ✓ PASS | server.toml row mentions auth |
| `cargo test --workspace` | ✓ PASS | 290 tests, 0 failures |
| `cargo clippy --workspace` | ✓ PASS | Clean |
| `cargo doc --workspace --no-deps` | ✓ PASS | Zero warnings |

## Diagnostics

None — documentation-only task.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `examples/server.toml` — Added commented-out `[auth]` section (~15 lines) after workers section
- `README.md` — Added "### Authentication" subsection (~20 lines) in Server Mode; updated examples table row
