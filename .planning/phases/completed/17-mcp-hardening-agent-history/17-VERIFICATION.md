# Phase 17 Verification: MCP Hardening & Agent History

## Status: PASSED

## Must-Haves Verification

### Plan 01: MCP Tool Hardening

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | gate_run accepts optional timeout parameter and returns timeout error if exceeded | PASS | `GateRunParams.timeout: Option<u64>` with default 300s; `tokio::time::timeout` wraps `spawn_blocking`; timeout returns `CallToolResult::error` |
| 2 | gate_run validates working_dir exists before evaluation starts | PASS | `working_dir.is_dir()` check before `spawn_blocking`; returns `CallToolResult::error` on failure |
| 3 | spec_list returns scan errors alongside successful entries | PASS | `SpecListResponse { specs, errors }` envelope; errors use `skip_serializing_if = "Vec::is_empty"` |
| 4 | GateRunResponse includes required_passed, advisory_passed, and blocked fields | PASS | Fields computed from `EnforcementSummary`; `blocked = required_failed > 0` |

### Plan 02: Gate History & Documentation

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | Agents can query gate history via gate_history MCP tool | PASS | `gate_history` tool registered with `#[tool]` attribute |
| 2 | gate_history returns list of run summaries by default, full record when run_id provided | PASS | List mode: `GateHistoryListResponse`; detail mode: raw `GateRunRecord` JSON |
| 3 | All MCP response structs have doc comments | PASS | 8 response structs documented: SpecListEntry, SpecListResponse, SpecListError, GateRunResponse, CriterionSummary, GateReportResponse, GateHistoryListResponse, GateHistoryEntry |
| 4 | Tool descriptions are accurate and complete | PASS | spec_list mentions envelope format; gate_run mentions timeout and enforcement counts; gate_history describes both modes |

## Test Suite

- `cargo test --workspace`: 265 passed, 3 ignored
- `cargo clippy --workspace -- -D warnings`: clean
- `cargo fmt --all -- --check`: clean
- `cargo deny check`: clean
- `just ready`: all checks passed

## Score: 8/8 must-haves verified
