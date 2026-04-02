---
id: M011
provides:
  - manifest.rs (1924L) decomposed into 7-file directory module, all files under 500L
  - git/cli.rs (1365L) decomposed into 7-file directory module, all files under 500L
  - GET /health endpoint (unauthenticated, bypasses auth middleware) on smelt serve
  - README "Health Check" section documenting the endpoint
  - 297 workspace tests, 0 failures
key_decisions:
  - D141: ValidationErrors co-located in validation.rs
  - D142: minimal_toml() helper reduces test boilerplate in core.rs
  - D143: git mv for file-to-directory conversion; git/mod.rs needs zero changes
  - D144: resolve_credentials enhanced with NotUnicode handling during S01
  - D145: job.name path-traversal validation added to validate_manifest
  - D140: Health endpoint unauthenticated (Router::merge() pattern)
patterns_established:
  - Router::merge() for auth-bypass routes in build_router()
  - File-to-directory module conversion with re-exports (D128 applied to manifest.rs, git/cli.rs)
observability_surfaces:
  - GET /health — 200 when server is running; non-200 or connection refused = server down
  - cargo test -p smelt-cli health_endpoint_bypasses_auth — integration test for health route
requirement_outcomes:
  - id: R060
    from_status: active
    to_status: validated
    proof: "S01 decomposed manifest.rs (1924L→307L mod.rs) and git/cli.rs (1365L→332L mod.rs) into 14 files all under 500L; all 290 workspace tests passed unchanged; clippy and doc clean"
  - id: R063
    from_status: active
    to_status: validated
    proof: "S03 added GET /health via Router::merge() outside auth middleware; test_health_endpoint_bypasses_auth passes; README documents endpoint"
  - id: R061
    from_status: active
    to_status: active
    proof: "S02 was not implemented — test_cli_run_invalid_manifest still has a 10s timeout; flaky test fix was researched but not shipped"
  - id: R062
    from_status: active
    to_status: active
    proof: "S02 was not implemented — 52 eprintln! calls remain across 8 files; tracing migration was researched but not shipped"
duration: ~3 days (2026-03-24 → 2026-03-27)
verification_result: partial
completed_at: 2026-03-27T12:00:00Z
---

# M011: Code Quality III & Operational Hardening

**S01 and S03 delivered fully: manifest.rs and git/cli.rs decomposed below 500L, GET /health endpoint added and documented, 297 tests passing; S02 (eprintln→tracing migration, flaky test fix) was researched but not implemented — R061 and R062 remain active.**

## What Happened

**S01** (independent, merged first) converted the two largest files in the codebase into focused directory modules. `manifest.rs` (1924L) became a 7-file `manifest/` module with a 307L `mod.rs`, extracted `validation.rs` (237L), and 4 domain test submodules (core, forge, compose, kubernetes). `git/cli.rs` (1365L) became a 7-file `git/cli/` module with a 332L `mod.rs` and 5 domain test submodules (basic, worktree, branch, commit, merge). All 14 new files are under 500L (max 337L), all public API preserved via re-exports, all 290 workspace tests passed unchanged.

**S02** (independent, merged as PR #40) only contained planning artifacts — M011-ROADMAP.md updates, S02-RESEARCH.md, and PROJECT.md. The actual implementation (52 eprintln→tracing replacements, dual-mode formatter in main.rs, flaky test timeout fix) was never written. The roadmap checkbox `[x]` was set by the PR title but the code changes are absent.

**S03** added the unauthenticated `GET /health` endpoint to `http_api.rs` via `Router::merge()`, placing the route on a stateless `Router<()>` outside the auth middleware layer. An integration test (`test_health_endpoint_bypasses_auth`) proves the endpoint returns 200 with `{"status":"ok"}` even when `[auth]` is configured and no `Authorization` header is sent. T02 added the "Health Check" section to the README and ran the full milestone verification pass.

## Cross-Slice Verification

| # | Success Criterion | Status | Evidence |
|---|-------------------|--------|----------|
| 1 | No production source file in `crates/` exceeds 500L | ⚠️ PARTIAL | S01 targets (manifest.rs, git/cli.rs) proven. 6 pre-existing files still exceed 500L (compose.rs 892, k8s.rs 820, assay.rs 751, docker.rs 605, dispatch.rs 578, monitor.rs 542) — these were NOT in M011 scope |
| 2 | `cargo test --workspace` 0 failures | ✓ PASS | 297 passed, 0 failed, 9 ignored |
| 3 | Zero `eprintln!` calls in smelt-cli (except main.rs) | ✗ FAIL | 52 calls remain; S02 not implemented |
| 4 | `SMELT_LOG=info` produces tracing output | ✗ FAIL | Migration not done; eprintln! still in use |
| 5 | `GET /health` returns 200 without auth | ✓ PASS | `test_health_endpoint_bypasses_auth` passes |
| 6 | `GET /health` returns 200 with `[auth]` configured | ✓ PASS | Same test, auth-configured server path |
| 7 | `cargo clippy --workspace` clean | ✓ PASS | Zero warnings |
| 8 | `cargo doc --workspace --no-deps` zero warnings | ✓ PASS | Zero warnings |
| 9 | All 290+ tests pass unchanged | ✓ PASS | 297 total (net +7 vs baseline) |

### Definition of Done Checklist

- ✅ `manifest.rs` decomposed below 500L with all public API preserved via re-exports
- ✅ `git/cli.rs` decomposed below 500L with all public API preserved via re-exports
- ✗ `test_cli_run_invalid_manifest` passes reliably — still has 10s timeout (S02 not implemented)
- ✗ All `eprintln!` in smelt-cli replaced with tracing — 52 calls remain (S02 not implemented)
- ✗ Tracing output is clean and user-readable at default level — not implemented
- ✅ `GET /health` returns 200 without auth
- ✅ `cargo test --workspace` ≥290 tests, 0 failures (297)
- ✅ `cargo clippy --workspace` clean
- ✅ `cargo doc --workspace --no-deps` zero warnings

## Requirement Changes

- R060: active → validated — S01 proved both manifest.rs and git/cli.rs decomposed into focused directory modules under 500L each; 14 files max 337L; all 290 tests pass
- R063: active → validated — S03 proved GET /health returns 200 without auth even with [auth] configured; integration test passes; README documents endpoint
- R061: active → active — S02 was not implemented; test_cli_run_invalid_manifest still has 10s timeout
- R062: active → active — S02 was not implemented; 52 eprintln! calls remain across 8 smelt-cli source files

## Forward Intelligence

### What the next milestone should know

- **S02 work is fully researched but unimplemented.** S02-RESEARCH.md contains the exact implementation plan: dual-mode subscriber config in main.rs, per-level mapping for each eprintln! site, and the flaky test fix (increase timeout from 10s to 30s). Start there — do not re-research.
- **52 eprintln! calls concentrated in phases.rs (33) and watch.rs (10).** The remaining 9 are spread across init.rs (1), list.rs (1), main.rs (1), dry_run.rs (2), status.rs (3), tui.rs (1).
- **The default filter level is the critical risk.** Changing from `warn` to `info` may surface noisy events from tokio/hyper/bollard. Use `"smelt_cli=info,smelt_core=info,warn"` as the default filter, not bare `"info"`.
- **serve/tui.rs eprintln! is a deliberate exception** (alongside main.rs): it runs after ratatui::restore() when the terminal is back to normal but the tracing subscriber still points to the file appender. Keep it as eprintln!.
- **Integration tests assert exact stderr substrings** (`"Writing manifest..."`, `"Container removed"`, etc.). With `.without_time().with_target(false).with_level(false)`, the bare message text matches exactly. Tests do NOT set SMELT_LOG so they'll use the bare format.

### What's fragile

- **S02 roadmap checkbox** — S02 is marked `[x]` in M011-ROADMAP.md but the actual code was never written. The next milestone should complete S02 work and update R061/R062 to validated.
- **6 files >500L in smelt-core** — compose.rs (892L), k8s.rs (820L), assay.rs (751L), docker.rs (605L), monitor.rs (542L), dispatch.rs (578L). These predate M011's scope but will accumulate as technical debt.

### Authoritative diagnostics

- `rg 'eprintln!' crates/smelt-cli/src/ -c` — definitive count of remaining migration targets; should reach 2 (main.rs + tui.rs) after S02 implementation
- `cargo test -p smelt-cli health_endpoint_bypasses_auth` — single-test health endpoint verification
- `find crates/ -name '*.rs' | xargs wc -l | awk '$1 > 500' | sort -rn` — file size regression check

### What assumptions changed

- **S02 was assumed complete** — The PR title "Full tracing migration + flaky test fix" suggested implementation, but the commit only contained documentation. Always verify code changes, not just PR titles.
- **"No production file >500L" criterion is scoped to M011 targets** — The 6 remaining files >500L (compose.rs etc.) predate M011 and were not decomposition targets. The criterion as written in the roadmap is technically unmet, but the intent (decompose the identified files) was achieved.

## Files Created/Modified

- `crates/smelt-core/src/manifest/` (7 files) — decomposed from manifest.rs (1924L)
- `crates/smelt-core/src/git/cli/` (7 files) — decomposed from git/cli.rs (1365L)
- `crates/smelt-cli/src/serve/http_api.rs` — GET /health route added via Router::merge()
- `crates/smelt-cli/src/serve/tests/http.rs` — test_health_endpoint_bypasses_auth added
- `README.md` — "Health Check" section added under smelt serve documentation
- `.kata/milestones/M011/slices/S01/S01-SUMMARY.md` — slice summary
- `.kata/milestones/M011/slices/S03/S03-SUMMARY.md` — slice summary
- `.kata/milestones/M011/slices/S03/tasks/T01-SUMMARY.md` — task summary
- `.kata/milestones/M011/slices/S03/tasks/T02-SUMMARY.md` — task summary
