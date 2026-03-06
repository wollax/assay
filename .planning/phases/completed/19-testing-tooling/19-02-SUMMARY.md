# Phase 19 Plan 02: MCP Handler Tests & Open Issue Triage Summary

Added 19 new tests across 4 crates covering MCP handler methods, integration lifecycle flows, and test coverage gaps identified in prior PR reviews. Triaged and closed 19 open test-related issues.

## Task Results

### Task 1: Direct Handler Unit Tests

**Commit:** `1ece2d5`

Added 8 async handler tests in `crates/assay-mcp/src/server.rs`:
- `spec_list_valid_project_returns_specs` (with insta snapshot)
- `spec_get_valid_spec_returns_content`
- `spec_get_missing_spec_returns_error`
- `gate_run_command_spec_returns_results` (with insta snapshot)
- `gate_run_nonexistent_spec_returns_error`
- `gate_run_nonexistent_working_dir_returns_error`
- `gate_report_invalid_session_returns_error`
- `gate_history_no_history_returns_empty`

Also added `test_format_gate_response_advisory_failed_not_blocked` to close the advisory-not-blocked test gap.

**Snapshots created:**
- `crates/assay-mcp/src/snapshots/assay_mcp__server__tests__spec_list_valid_project.snap`
- `crates/assay-mcp/src/snapshots/assay_mcp__server__tests__gate_run_command_spec.snap`

### Task 2: Integration Tests & Issue Triage

**Commit:** `55163ac`

**Part A — Integration tests** (`crates/assay-mcp/tests/mcp_handlers.rs`):
- `gate_lifecycle_run_report_finalize` — full session: gate_run → gate_report → gate_finalize, verifies history persistence
- `gate_run_with_timeout` — timeout parameter verification
- `spec_list_with_parse_errors` — malformed TOML produces error envelope alongside valid specs

**Part B — Test gap resolution** (11 new tests across 4 files):
- `assay-types/src/gate.rs`: GateKind unknown variant (TOML + JSON), GateResult JSON roundtrip with skip fields
- `assay-types/src/enforcement.rs`: invalid enforcement deser, TOML roundtrip via GateSection, JSON roundtrip
- `assay-types/src/criterion.rs`: enforcement field TOML roundtrip, missing required fields deser failure
- `assay-types/src/gates_spec.rs`: gate section TOML roundtrip
- `assay-core/src/gate/mod.rs`: evaluate_all_gates spawn failure
- `assay-core/src/spec/mod.rs`: scan empty directory, SpecError Display format
- `assay-core/src/history/mod.rs`: load file not found, load invalid JSON, load roundtrip with non-empty results

**Part B — Issue triage** (19 issues closed):
- 10 resolved with new tests
- 4 closed as acknowledged (naming/style suggestions)
- 3 closed as stale (redundant or harmless)
- 2 closed as out of scope (CLI integration tests)

## Deviations

1. **Handler methods made `pub`** — Required for integration tests to call handler methods directly. Parameter types also made public with public fields, and re-exported through `lib.rs`.
2. **Insta redactions not used** — Workspace `insta` dependency lacks the `redactions` feature. Used manual normalization of dynamic fields instead.
3. **Integration tests require `--test-threads=1`** — CWD-dependent tests race in parallel. This is inherent to the `set_current_dir` approach; integration tests document this requirement.

## Metrics

- Tests before: 279 (across workspace)
- Tests after: 298 (across workspace)
- New tests: 19
- Insta snapshots: 3 new
- Issues triaged: 19 (all test-related)
- Duration: ~20 minutes
