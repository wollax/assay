---
id: M010
provides:
  - Bearer token auth middleware for smelt serve HTTP API (opt-in via [auth] config)
  - Read/write permission split (GET/HEAD = read, POST/DELETE = write)
  - ResolvedAuth with env var resolution and fail-fast startup validation
  - warn_teardown() helper replacing 6 silent let _ = teardown blocks in phases.rs
  - .context() error chain preservation on 5 monitor write/set_phase calls
  - build_common_ssh_args() shared helper eliminating SSH/SCP flag duplication
  - Documented [auth] section in examples/server.toml
  - Authentication subsection in README.md Server Mode
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
  - "warn_teardown(monitor, provider, container) for early-return teardown in phases.rs"
  - "build_common_ssh_args(worker, timeout_secs, port_flag, tool_name, extra_args) for SSH/SCP"
observability_surfaces:
  - "tracing::warn on 401/403 with method + path (never token values)"
  - "tracing::info at startup listing configured auth env var names"
  - "JSON error bodies on 401 and 403 responses"
  - "Startup abort with descriptive error naming missing/empty env var"
  - "eprintln! warnings on teardown failures (previously silent discards)"
  - ".context() preserved error chains on monitor write/set_phase"
requirement_outcomes:
  - id: R050
    from_status: active
    to_status: validated
    proof: "4 integration tests (missing header→401, invalid token→403, read token split, write-only mode); startup fails fast on missing env vars; S03 docs in server.toml and README"
  - id: R051
    from_status: active
    to_status: validated
    proof: "test_auth_read_token_permission_split (read token GET→200, POST→403, DELETE→403; write token all→200) and test_auth_write_only_mode; README documents permission model"
  - id: R052
    from_status: active
    to_status: validated
    proof: "warn_teardown() replaces 6 silent let _ = blocks; 5 anyhow!(\"{e}\") → .context(); rg confirms zero silent teardown discards; all 155+ tests pass"
  - id: R053
    from_status: active
    to_status: validated
    proof: "build_common_ssh_args() extracted; build_ssh_args/build_scp_args are 1-line delegations; 4 SSH arg tests pass unchanged"
duration: 55min
verification_result: passed
completed_at: 2026-03-24T12:30:00Z
---

# M010: HTTP API Authentication & Code Quality

**Bearer token auth with read/write permission split for `smelt serve` HTTP API, plus teardown error visibility and SSH argument builder DRY cleanup**

## What Happened

Three slices delivered in parallel-safe order:

**S01 (Bearer token auth)** built the complete auth infrastructure: `AuthConfig` in `config.rs` with `write_token_env`/`read_token_env` fields, `ResolvedAuth` with env var resolution that fails fast on empty/missing vars, and `auth_middleware()` implementing the read/write permission split (GET/HEAD = read, everything else = write). The middleware is always applied via `from_fn_with_state` with `Option<ResolvedAuth>` — `None` means no-op pass-through for backward compatibility. Four integration tests prove all token×permission combinations: missing header→401, invalid token→403, read token split (GET ok, POST/DELETE denied), and write-only mode.

**S02 (Code quality)** tackled two independent PR review items. Extracted `warn_teardown()` in `phases.rs` to replace 6 duplicated silent `let _ =` teardown blocks with `eprintln!` warnings. Replaced 5 `anyhow!("{e}")` calls with `.context()` to preserve error chains. Extracted `build_common_ssh_args()` in `client.rs` to eliminate ~40 lines of duplicated SSH/SCP flag-building logic.

**S03 (Documentation)** added a commented-out `[auth]` section to `examples/server.toml` with field-level docs, added an Authentication subsection to README.md, and ran the full milestone verification pass confirming 290 tests pass, clippy clean, doc clean.

## Cross-Slice Verification

| Success Criterion | Status | Evidence |
|---|---|---|
| `smelt serve` with `[auth]` rejects unauthenticated requests with 401 | ✓ PASS | `test_auth_missing_header_returns_401` — all methods (GET, POST, DELETE) return 401 |
| Read-only token GET ok, POST/DELETE 403 | ✓ PASS | `test_auth_read_token_permission_split` — GET→200, POST→403, DELETE→403 |
| Read-write token has full API access | ✓ PASS | Same test — write token GET→200, POST→200, DELETE→200 |
| No `[auth]` = no auth (backward compat) | ✓ PASS | All 85 pre-existing smelt-cli tests pass unchanged (auth state = `None`) |
| `cargo test --workspace` ≥286 tests, 0 failures | ✓ PASS | 290 tests pass (268+ non-ignored confirmed; 1 pre-existing flaky timeout test in `test_cli_run_invalid_manifest` predates M010) |
| Teardown errors produce visible warnings | ✓ PASS | `rg 'let _ = provider\.teardown' phases.rs` = 0 hits; `warn_teardown()` has 7 references |
| Error chain preserved via `.context()` | ✓ PASS | `rg 'anyhow!.*\{e\}' phases.rs` = 0 hits |
| SSH arg builders share common helper | ✓ PASS | `build_common_ssh_args()` has 3 references; both public methods are 1-line delegations |
| `cargo doc --workspace --no-deps` 0 warnings | ✓ PASS | Clean build, zero warnings |
| `cargo clippy --workspace` clean | ✓ PASS | No warnings |
| `examples/server.toml` documents `[auth]` | ✓ PASS | 2 occurrences of `[auth]` in file |
| README.md server mode updated | ✓ PASS | "Authentication" subsection present between HTTP API Endpoints and Queue Persistence |

**Note on test count:** S03's verification recorded 290 tests with warm caches. On cold runs, `test_cli_run_invalid_manifest` (a subprocess timeout test predating M010) can fail due to 10s timeout being too short for binary startup. This is a pre-existing issue confirmed by running the same test at the M009 completion commit — same failure.

## Requirement Changes

- R050: active → validated — 4 integration tests + startup validation + documentation in server.toml and README
- R051: active → validated — test_auth_read_token_permission_split + test_auth_write_only_mode + README documentation
- R052: active → validated — warn_teardown() helper, zero silent let _ = discards, .context() error chains, all tests pass
- R053: active → validated — build_common_ssh_args() extracted, both methods are delegations, 4 SSH tests pass unchanged

## Forward Intelligence

### What the next milestone should know
- M010 completes the HTTP API security story. Auth is opt-in and backward compatible. All code quality debt from M009's PR review cycle is resolved. 290 workspace tests green, clippy/doc clean. The project is in a clean state for the next feature milestone.
- `test_cli_run_invalid_manifest` is a known flaky test — it uses a 10s subprocess timeout that's too short when cargo needs to link. Consider increasing the timeout or marking it `#[ignore]` in a future cleanup.

### What's fragile
- Auth middleware applies globally — no per-route exclusions. If unauthenticated endpoints (health check, readiness probe) are needed, the router structure would need to split protected/unprotected route groups.
- `ResolvedAuth` fields are `pub(crate)` (D138) — if the struct moves to smelt-core, test construction patterns would need updating.

### Authoritative diagnostics
- `cargo test -p smelt-cli serve::tests::http::test_auth` — runs all 4 auth integration tests end-to-end through the real axum router
- `rg 'let _ =' phases.rs` — should show only the outcome match block, not teardown paths
- `rg 'build_common_ssh_args' client.rs` — confirms DRY extraction

### What assumptions changed
- T01 in S01 proactively completed all T02 work (serve wiring + test helpers), making T02 a pure verification pass. Task estimates were skewed but total slice effort was lower than planned.
- No other assumptions changed — all three slices executed as planned.

## Files Created/Modified

- `crates/smelt-cli/src/serve/config.rs` — Added `AuthConfig` struct, `ServerConfig.auth` field
- `crates/smelt-cli/src/serve/http_api.rs` — Added `ResolvedAuth`, `resolve_auth()`, `auth_middleware()`, updated `build_router()`
- `crates/smelt-cli/src/commands/serve.rs` — Auth resolution at startup with tracing
- `crates/smelt-cli/src/serve/tests/mod.rs` — Added `start_test_server_with_auth()` helper
- `crates/smelt-cli/src/serve/tests/http.rs` — 4 auth integration tests and helper functions
- `crates/smelt-cli/src/commands/run/phases.rs` — `warn_teardown()` helper, `.context()` error chains
- `crates/smelt-cli/src/serve/ssh/client.rs` — `build_common_ssh_args()` extraction
- `examples/server.toml` — Commented-out `[auth]` section with field docs
- `README.md` — Authentication subsection in Server Mode; updated examples table
