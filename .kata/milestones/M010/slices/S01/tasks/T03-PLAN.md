---
estimated_steps: 4
estimated_files: 1
---

# T03: Auth integration tests covering all token×permission combinations

**Slice:** S01 — Bearer token auth middleware with read/write split
**Milestone:** M010

## Description

Write the definitive test suite proving the auth middleware works correctly across all token×permission combinations. Tests use `start_test_server()` with `ResolvedAuth` to create an auth-enabled server and exercise every combination: no header, bad token, read token on read/write endpoints, write token on all endpoints, and write-only mode (no read token configured). Also verify that 401/403 responses include JSON error bodies.

## Steps

1. Write `test_auth_missing_header_returns_401`: create server with auth (write token + read token), send GET/POST/DELETE without Authorization header, assert all return 401.
2. Write `test_auth_invalid_token_returns_401`: send requests with `Authorization: Bearer wrong-token`, assert 401 on GET and POST.
3. Write `test_auth_read_token_permission_split`: send GET with read token → 200; send POST with read token → 403; send DELETE with read token → 403. Also verify write token on GET → 200, POST → 200, DELETE → 200.
4. Write `test_auth_write_only_mode`: create server with auth where `read_token` is `None` (only write token configured). Verify write token works on GET (200), POST (200), DELETE (200). Verify any other token on GET → 401 (no read token to match against, and it's not the write token).

## Must-Haves

- [ ] Test: missing Authorization header → 401 on GET, POST, DELETE
- [ ] Test: invalid token → 401
- [ ] Test: read token on GET → 200
- [ ] Test: read token on POST → 403
- [ ] Test: read token on DELETE → 403
- [ ] Test: write token on GET → 200
- [ ] Test: write token on POST → 200
- [ ] Test: write token on DELETE → 200
- [ ] Test: write-only mode (no read token) works correctly
- [ ] 401/403 responses contain JSON `{"error": "..."}` body
- [ ] `cargo test --workspace` all pass
- [ ] `cargo clippy --workspace` clean
- [ ] `cargo doc --workspace --no-deps` zero warnings

## Verification

- `cargo test --workspace` passes with 0 failures (286+ existing + 4+ new auth tests)
- `cargo clippy --workspace` clean
- `cargo doc --workspace --no-deps` zero warnings
- All slice must-haves proven by the test results

## Observability Impact

- None — this task adds tests only, no runtime changes

## Inputs

- T01 output: `ResolvedAuth` struct, `auth_middleware` logic
- T02 output: `start_test_server()` with auth support
- `VALID_MANIFEST_TOML` constant in tests/mod.rs for POST body

## Expected Output

- `crates/smelt-cli/src/serve/tests/http.rs` — 4+ new test functions covering all auth combinations
- Full workspace test suite green with 290+ tests
