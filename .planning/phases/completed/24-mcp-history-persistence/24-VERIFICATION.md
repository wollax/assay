# Phase 24 Verification

**Status:** passed
**Score:** 6/6 must-haves verified

## Must-Have Checks

### Truth 1: MCP gate_run persists history for command-only specs (no agent criteria)
**Status:** PASS
**Evidence:** In `crates/assay-mcp/src/server.rs` lines 562-581, the `else` branch after the `agent_info.is_some()` check constructs a `GateRunRecord` (with `run_id`, `assay_version`, `timestamp`, `working_dir`, and `summary`) and calls `assay_core::history::save()`. This executes for every command-only spec (i.e., when `agent_info` is `None`).

### Truth 2: History file appears in .assay/results/<spec_name>/ after a command-only gate_run
**Status:** PASS
**Evidence:** The integration test `gate_run_command_only_persists_history` in `crates/assay-mcp/tests/mcp_handlers.rs` (line 228) explicitly asserts that (a) the results directory `.assay/results/cmd-only-spec/` exists, and (b) it contains exactly one `.json` history file. The test passes.

### Truth 3: Save failures are non-fatal (logged via tracing::warn, not returned as errors)
**Status:** PASS
**Evidence:** In `server.rs` line 575-580, the save call is wrapped in `if let Err(e) = ...` with `tracing::warn!` logging the error message. The function continues to return `Ok(CallToolResult::success(...))` regardless of save outcome.

### Artifact 1: crates/assay-mcp/src/server.rs — History save logic for command-only gate_run path
**Status:** PASS
**Evidence:** Lines 562-581 contain the complete implementation: `GateRunRecord` construction and `history::save()` call in the `else` branch of the `agent_info` check.

### Artifact 2: crates/assay-mcp/tests/mcp_handlers.rs — Integration test proving command-only MCP gate_run persists history
**Status:** PASS
**Evidence:** Test `gate_run_command_only_persists_history` (lines 228-309) creates a command-only spec, runs `gate_run`, and verifies: no session_id returned, results directory exists, exactly one JSON history file present, record contains correct `spec_name`, `passed` count, non-empty `run_id`, and `working_dir`.

### Key Link 1: gate_run handler (agent_info.is_none() branch) -> assay_core::history::save()
**Status:** PASS
**Evidence:** The call chain is verified: `agent_info` is `None` -> `else` branch -> `generate_run_id()` -> construct `GateRunRecord` -> `assay_core::history::save(&assay_dir, &record, max_history)`.

## Test Results

### Targeted test
```
cargo test -p assay-mcp --test mcp_handlers gate_run_command_only_persists_history -- --test-threads=1
-> 1 passed, 7 filtered out
```

### Full suite (`just ready`)
```
cargo fmt --all -- --check        -> OK
cargo clippy --workspace          -> OK
cargo test --workspace            -> all passed (326 unit + integration tests across workspace)
cargo deny check                  -> advisories ok, bans ok, licenses ok, sources ok
Plugin versions match workspace   -> OK
All checks passed.
```
