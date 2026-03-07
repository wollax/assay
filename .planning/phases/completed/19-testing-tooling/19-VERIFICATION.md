# Phase 19 Verification

**Status:** passed
**Score:** 13/13 must-haves verified

## Must-Have Verification

### Plan 01 Must-Haves — Tighten cargo-deny Policies

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | `cargo deny check bans` passes with `multiple-versions = "deny"` | PASS | `deny.toml:26` — `multiple-versions = "deny"`; skip entries at lines 29-48 for known transitive duplicates (crossterm, getrandom, linux-raw-sys, rustix, bitflags); `just ready` confirms bans ok |
| 2 | `cargo deny check sources` passes with `unknown-registry = "deny"` and `unknown-git = "deny"` | PASS | `deny.toml:60-61` — `unknown-registry = "deny"`, `unknown-git = "deny"`; `just ready` confirms sources ok |
| 3 | `just deny` exits 0 | PASS | `just ready` output: "advisories ok, bans ok, licenses ok, sources ok" (2026-03-07) |

### Plan 02 Must-Haves — MCP Handler Tests & Open Issue Triage

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | Each MCP handler (spec_list, spec_get, gate_run, gate_report, gate_history) has at least one direct test that calls the handler method on AssayServer | PASS | `crates/assay-mcp/src/server.rs:2350` — `spec_list_valid_project_returns_specs`; `:2393` — `spec_get_valid_spec_returns_content`; `:2458` — `gate_run_command_spec_returns_results`; `:2592` — `gate_report_invalid_session_returns_error`; `:2624` — `gate_history_no_history_returns_empty` |
| 2 | Integration tests in crates/assay-mcp/tests/ exercise handler logic through the server with tempdir-based file system setup | PASS | `crates/assay-mcp/tests/mcp_handlers.rs:56` — `gate_lifecycle_run_report_finalize`; `:189` — `gate_run_with_timeout`; `:313` — `spec_list_with_parse_errors` |
| 3 | Open test-related issues are individually audited and either resolved with new tests or closed as stale with explanation | PASS | 19 issues triaged per 19-02-SUMMARY.md: 10 resolved with tests, 4 closed as acknowledged, 3 closed as stale, 2 closed as out of scope |
| 4 | All new tests pass via `cargo test -p assay-mcp` | PASS | `just ready` output: assay-mcp unit tests 53 passed, integration tests 8 passed (2026-03-07) |
| 5 | insta snapshots exist for MCP response payloads | PASS | `crates/assay-mcp/src/snapshots/assay_mcp__server__tests__spec_list_valid_project.snap`, `crates/assay-mcp/src/snapshots/assay_mcp__server__tests__gate_run_command_spec.snap`, `crates/assay-mcp/tests/snapshots/mcp_handlers__gate_lifecycle_finalize.snap` — 3 snapshot files |

### Plan 03 Must-Haves — Dogfooding Spec

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | `.assay/specs/self-check.toml` exists and is valid TOML parseable by assay | PASS | `.assay/specs/self-check.toml` — 33 lines, valid TOML with `name = "self-check"` |
| 2 | The self-check spec has at least 4 required deterministic criteria (fmt, clippy, test, deny) | PASS | `.assay/specs/self-check.toml:7-24` — formatting, linting, tests, deny criteria all with `cmd` and inheriting `enforcement = "required"` from `[gate]` section at line 5 |
| 3 | The self-check spec has at least 1 advisory AgentReport criterion | PASS | `.assay/specs/self-check.toml:27-32` — `code-quality-review` with `kind = "AgentReport"`, `enforcement = "advisory"` |
| 4 | `just ready` passes | PASS | `just ready` output: "All checks passed." (2026-03-07); 513 tests passed, 3 ignored |
| 5 | `cargo run -p assay-cli -- gate run self-check` exits 0 on a clean build | PASS | UAT test #7 confirmed PASS; 4 pass, 0 fail, 0 warned, 1 skipped (agent criterion skipped without evaluator) |

## Quality Gate

- **`just ready`:** PASS (2026-03-07) — fmt-check ok, clippy ok, 513 tests passed (3 ignored), cargo-deny ok
- **Merge commit:** `68c6a4d` — PR #59 merged to main; CI passed at merge time

## Test Coverage Summary

Phase 19 test contributions:
- `crates/assay-mcp/src/server.rs` tests module — 9 handler unit tests (spec_list, spec_get, gate_run, gate_report, gate_history + advisory format)
- `crates/assay-mcp/tests/mcp_handlers.rs` — 3 integration tests (lifecycle, timeout, parse errors) + 5 error path tests
- `crates/assay-types/src/gate.rs` — GateKind unknown variant, GateResult roundtrip
- `crates/assay-types/src/enforcement.rs` — invalid deser, TOML roundtrip, JSON roundtrip
- `crates/assay-types/src/criterion.rs` — enforcement roundtrip, missing fields deser
- `crates/assay-types/src/gates_spec.rs` — gate section TOML roundtrip
- `crates/assay-core/src/gate/mod.rs` — evaluate_all_gates spawn failure
- `crates/assay-core/src/spec/mod.rs` — scan empty dir, SpecError Display
- `crates/assay-core/src/history/mod.rs` — load not found, invalid JSON, roundtrip
- 3 insta snapshot files for MCP response payloads

Total tests added by Phase 19: 19 (workspace total grew from 279 to 298 at merge time)

## Gaps

None.
