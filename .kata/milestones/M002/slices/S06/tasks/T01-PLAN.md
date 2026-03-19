---
estimated_steps: 6
estimated_files: 2
---

# T01: Add orchestrate_run and orchestrate_status MCP tools

**Slice:** S06 — MCP Tools & End-to-End Integration
**Milestone:** M002

## Description

Add two new MCP tools to `AssayServer`: `orchestrate_run` for launching multi-session orchestration and `orchestrate_status` for reading persisted orchestrator state. Both follow the established pattern of the 20 existing tools: param struct with `Deserialize + JsonSchema`, `#[tool]` annotation, `spawn_blocking` wrapper, and `domain_error()` for structured failures. This directly delivers R021 (Orchestration MCP tools).

## Steps

1. **Add param and response types** in `server.rs`:
   - `OrchestrateRunParams { manifest_path: String, timeout_secs: Option<u64>, failure_policy: Option<String>, merge_strategy: Option<String> }`
   - `OrchestrateRunResponse` combining orchestration outcomes + merge report fields
   - `OrchestrateStatusParams { run_id: String }`
   - All with `Deserialize, JsonSchema` derives, matching existing patterns

2. **Implement `orchestrate_run` handler**:
   - Load manifest from path, validate it has multi-session content (sessions.len() > 1 or any depends_on)
   - Build `OrchestratorConfig` (parse failure_policy, max_concurrency from defaults)
   - Build `PipelineConfig` from CWD + loaded config
   - Inside `spawn_blocking`: construct session runner closure that builds harness config using plain function calls (`assay_harness::claude::generate_config/write_config/build_cli_args`) — NOT through `HarnessWriter` dyn Fn (D035)
   - Call `run_orchestrated()` with the session runner
   - After execution: checkout base branch with `git checkout`, call `extract_completed_sessions()`, call `merge_completed_sessions()` with `default_conflict_handler()`
   - Return combined JSON with per-session outcomes, merge report, and run_id

3. **Implement `orchestrate_status` handler**:
   - Accept `run_id`, construct path `.assay/orchestrator/<run_id>/state.json`
   - Read and deserialize `OrchestratorStatus`
   - Return as JSON `CallToolResult`
   - Domain error if file not found or invalid

4. **Register both tools** in the `#[tool_router]` by adding `#[tool]` annotations (automatic via macro)

5. **Re-export param types** in `lib.rs` under `#[cfg(any(test, feature = "testing"))]` for integration test access

6. **Add unit tests** in the `#[cfg(test)]` module at bottom of server.rs:
   - Param deserialization tests (full + minimal) for both tools
   - Schema generation test for both
   - Router registration test (both tool names in `list_all()`)
   - `orchestrate_status` with missing run_id returns domain error
   - `orchestrate_run` with missing manifest returns domain error

## Must-Haves

- [ ] `orchestrate_run` tool registered in router with correct description
- [ ] `orchestrate_status` tool registered in router with correct description
- [ ] Session runner closure uses plain function calls, not `HarnessWriter` dyn (D035)
- [ ] `spawn_blocking` wraps all sync orchestration calls (D007)
- [ ] Base branch checkout between execution and merge phases
- [ ] Param types re-exported for integration test access
- [ ] 6+ unit tests covering param deserialization, schema, router registration, and error paths

## Verification

- `cargo test -p assay-mcp --features orchestrate` — all existing 20 tool tests + new orchestrate tests pass
- `cargo clippy -p assay-mcp --features orchestrate -- -D warnings` — clean

## Observability Impact

- Signals added/changed: `orchestrate_run` returns structured JSON with run_id, per-session outcomes, and merge report; `orchestrate_status` returns full `OrchestratorStatus` snapshot
- How a future agent inspects this: call `orchestrate_status` with run_id to see session states; parse `orchestrate_run` response for merge failures
- Failure state exposed: `isError: true` with domain_error messages for missing manifests, invalid run_ids, orchestration failures

## Inputs

- `crates/assay-mcp/src/server.rs` — 20 existing tools as pattern templates; `run_manifest` tool (line 2539) is closest reference
- `crates/assay-core/src/orchestrate/executor.rs` — `run_orchestrated()` signature and `OrchestratorConfig`
- `crates/assay-core/src/orchestrate/merge_runner.rs` — `merge_completed_sessions()`, `extract_completed_sessions()`, `default_conflict_handler()`, `MergeRunnerConfig`
- `crates/assay-types/src/orchestrate.rs` — `OrchestratorStatus`, `FailurePolicy`, `MergeStrategy`

## Expected Output

- `crates/assay-mcp/src/server.rs` — two new `#[tool]` handlers with param/response types and unit tests
- `crates/assay-mcp/src/lib.rs` — re-exported param types for testing
