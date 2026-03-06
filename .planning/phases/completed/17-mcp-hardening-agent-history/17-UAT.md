# Phase 17 UAT: MCP Hardening & Agent History

## Tests

| # | Test | Status | Notes |
|---|------|--------|-------|
| 1 | gate_run timeout parameter accepted and defaults to 300s | PASS | Option<u64> with unwrap_or(300), tokio::time::timeout wrapping |
| 2 | gate_run rejects non-existent working directory with clear error | PASS | is_dir() check before spawn_blocking, returns CallToolResult::error |
| 3 | spec_list returns error envelope with scan errors | PASS | SpecListResponse envelope, errors omitted when empty via skip_serializing_if |
| 4 | GateRunResponse includes enforcement counts and blocked flag | PASS | 6 enforcement fields + blocked, computed from EnforcementSummary |
| 5 | gate_history tool returns run summaries in list mode | PASS | List mode (most-recent-first, default limit 10) and detail mode (full record) |
| 6 | All response structs have doc comments | PASS | 8 structs, 46 fields, 100% coverage |

## Result: 6/6 PASSED
