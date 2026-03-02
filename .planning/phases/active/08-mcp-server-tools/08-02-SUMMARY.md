---
phase: 08-mcp-server-tools
plan: 02
status: complete
completed: 2026-03-02
---

# 08-02 Summary: CLI Wiring Verification and Integration Tests

## Objective

Wire the CLI subcommand to the new AssayServer and add integration tests that verify the full tool flow end-to-end. Prove that all MCP tool behaviors (spec_get, spec_list, gate_run) work correctly across the full stack from CLI entry point through MCP transport to tool execution and response formatting.

## Tasks Completed

### Task 1 — Verify CLI wiring and add integration tests (auto)

**Commit:** 93f1fdb

**Files modified:**
- `crates/assay-mcp/Cargo.toml` — added `tempfile` as dev-dependency
- `crates/assay-mcp/src/server.rs` — added integration tests

**What was done:**

CLI wiring was verified to be correct as-is. The existing `McpCommand::Serve` arm in `main.rs` already calls `assay_mcp::serve()`, which now routes to the real AssayServer (not the spike). No changes to `main.rs` were required.

Integration tests added to `server.rs` cover:

- `test_load_config_valid_project` — tempdir with valid `.assay/config.toml`, `load_config()` returns Ok
- `test_load_config_missing_project` — empty tempdir, `load_config()` returns Err with helpful message
- `test_load_spec_valid` — tempdir with config and spec file, `load_spec()` returns Ok with correct data
- `test_load_spec_not_found` — tempdir with config but no specs, `load_spec()` returns Err mentioning the spec name
- `test_resolve_working_dir_default` — no gates section, returns cwd
- `test_resolve_working_dir_relative` — relative working_dir config, returns cwd.join(subdir)
- `test_resolve_working_dir_absolute` — absolute working_dir config, returns the configured path
- `test_spec_list_entry_serialization` — verifies SpecListEntry wire format, description omitted when empty
- `test_gate_run_response_serialization` — verifies GateRunResponse wire format, skip_serializing_if works (no null fields)

Tests use explicit path arguments (no CWD manipulation) — the helper functions accept `cwd: &Path` directly, making them directly testable without fragile `set_current_dir` calls.

### Task 2 — End-to-end MCP server verification (checkpoint:human-verify)

**Commit:** N/A (verification only)

All 9 verification scenarios passed:

| Scenario | Requirement | Result |
|----------|-------------|--------|
| 1. Clean JSON-RPC start — first byte on stdout is `{` | MCP-05 | PASS |
| 2. tools/list returns three tools (gate_run, spec_get, spec_list) with descriptions | MCP-08 | PASS |
| 3. spec_list returns array of spec entries | MCP-04 | PASS |
| 4. spec_get returns full spec JSON | MCP-02 | PASS |
| 5. spec_get with bad name returns isError: true | — | PASS |
| 6. gate_run returns bounded summary (passed:2, failed:0, skipped:1) | MCP-03, MCP-07 | PASS |
| 7. gate_run with include_evidence=true includes stdout/stderr | — | PASS |
| 8. Missing .assay/ returns isError: true with helpful message | — | PASS |
| 9. `just ready` — all checks passed | — | PASS |

## Requirements Satisfied

| Requirement | Description | Verified by |
|-------------|-------------|-------------|
| MCP-02 | spec_get returns full spec JSON for valid name | Scenario 4, Task 1 tests |
| MCP-03 | gate_run returns bounded pass/fail summary per criterion | Scenario 6 |
| MCP-04 | spec_list returns array of spec entries with name, description, criteria_count | Scenario 3 |
| MCP-05 | First byte on stdout is `{` — clean JSON-RPC, no clap leakage | Scenario 1 |
| MCP-07 | gate_run uses spawn_blocking, does not block tokio runtime | Scenario 6 (non-blocking behavior confirmed) |
| MCP-08 | Tool descriptions are self-documenting | Scenario 2 |

## Deviations

None. All tasks completed as specified in the plan.

## Decisions Recorded

- `assay-types` added as direct dependency of `assay-mcp` (transitive access through `assay-core` does not allow type naming in function signatures) — recorded in STATE.md during 08-01
- `chrono` added as dev-dependency of `assay-mcp` for `GateResult` construction in tests — recorded in STATE.md during 08-01
- Domain errors returned as `CallToolResult::error` (isError: true), protocol errors as `Err(McpError)` — recorded in STATE.md during 08-01
- Per-call config/spec resolution (no startup validation, no stale state) — recorded in STATE.md during 08-01
- No tool name prefix (`spec_get` not `assay_spec_get`) — MCP servers already namespace tools — recorded in STATE.md during 08-01
- `first_nonempty_line` extracts failure reason from stderr for summary mode; empty stderr gets "unknown" — recorded in STATE.md during 08-01
