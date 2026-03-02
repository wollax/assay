---
phase: 08-mcp-server-tools
status: passed
verified: 2026-03-02
must_haves_checked: 16/16
---

# Phase 8: MCP Server Tools — Verification

## Summary

All 16 must-haves from Plan 01 and Plan 02 are satisfied by the actual source code. The spike is gone, AssayServer exposes three self-documenting tools, spawn_blocking is used correctly, domain errors surface as isError: true tool results, and the `assay mcp serve` subcommand is properly wired with tracing isolated to stderr.

## Must-Have Verification

### Plan 01 Must-Haves

### spike.rs is deleted and no references to SpikeServer remain in the codebase
**Evidence:** `Glob("**/spike.rs")` → no results. `Grep("SpikeServer|spike", crates/)` → no matches. `lib.rs:10` declares `mod server;` (not `mod spike;`). Summary 08-01 records the deletion commit (93cba26).

### AssayServer exposes three tools: spec_get, spec_list, gate_run
**Evidence:** `crates/assay-mcp/src/server.rs:96-208`. The `#[tool_router] impl AssayServer` block registers three `#[tool(...)]` methods: `spec_list` (line 119), `spec_get` (line 152), `gate_run` (line 176). The `ToolRouter<Self>` field is initialized via `Self::tool_router()` in `new()` (line 110-113).

### spec_get returns the full spec as JSON when given a valid spec name
**Evidence:** `server.rs:161-169`. Calls `load_spec()` to obtain the `Spec` value, then `serde_json::to_string(&spec)` and returns it inside `CallToolResult::success(vec![Content::text(json)])`. The `Spec` type derives `Serialize` (assay-types), so the full struct including all criteria fields is serialized.

### spec_list returns an array of {name, description, criteria_count} objects for all specs in the project
**Evidence:** `server.rs:131-145`. `assay_core::spec::scan(&specs_dir)` produces a `ScanResult`. Each `(slug, spec)` entry is mapped to `SpecListEntry { name: slug, description: spec.description, criteria_count: spec.criteria.len() }`. The `SpecListEntry` struct (lines 56-62) has exactly these three fields. `description` is skipped when empty via `#[serde(skip_serializing_if = "String::is_empty")]`.

### gate_run returns a summary with passed/failed/skipped counts and per-criterion status by default
**Evidence:** `server.rs:197-207`. `format_gate_response(&summary, include_evidence)` (line 203) maps to `GateRunResponse` (lines 65-73) containing `spec_name`, `passed`, `failed`, `skipped`, `total_duration_ms`, and `criteria: Vec<CriterionSummary>`. In summary mode (`include_evidence=false`), stdout/stderr are `None` and omitted from JSON output via `skip_serializing_if`. Unit test `test_format_gate_response_summary_mode` (line 416-479) verifies this.

### gate_run with include_evidence=true includes stdout/stderr per criterion in the response
**Evidence:** `server.rs:287-330`. When `include_evidence` is true, `stdout` and `stderr` fields of `CriterionSummary` are populated from `gate_result.stdout` and `gate_result.stderr`. Unit test `test_format_gate_response_evidence_mode` (line 481-528) verifies stdout/stderr are present for executable criteria and absent for skipped.

### gate_run uses tokio::task::spawn_blocking to avoid blocking the tokio runtime during gate evaluation
**Evidence:** `server.rs:197-201`. Exact code:
```rust
let summary = tokio::task::spawn_blocking(move || {
    assay_core::gate::evaluate_all(&spec_owned, &working_dir_owned, None, config_timeout)
})
.await
.map_err(|e| McpError::internal_error(format!("gate evaluation panicked: {e}"), None))?;
```
`evaluate_all` is synchronous (blocking I/O, process spawning). The `spawn_blocking` bridge is the correct pattern for async MCP handlers calling sync domain logic.

### All domain errors are returned as CallToolResult with isError: true, not as Err(McpError)
**Evidence:** `server.rs:264-267`. The `domain_error` helper:
```rust
fn domain_error(err: &assay_core::AssayError) -> CallToolResult {
    CallToolResult::error(vec![Content::text(err.to_string())])
}
```
All three tool handlers use the `match load_config(&cwd) { Ok(c) => c, Err(err_result) => return Ok(err_result) }` pattern (lines 121-124, 157-160, 181-184), ensuring domain errors become `Ok(CallToolResult { is_error: true })` not `Err(McpError)`. Unit test `test_domain_error_produces_error_result` (lines 531-563) verifies `result.is_error.unwrap_or(false)` is true.

### Tool descriptions are self-documenting: each describes what it does, what it returns, and when to use it
**Evidence:**
- `spec_list` (line 117): `"List all specs in the current Assay project. Returns an array of {name, description, criteria_count} objects. Use this to discover available specs before calling spec_get or gate_run."`
- `spec_get` (lines 150-151): `"Get a spec by name. Returns the full spec definition as JSON including name, description, and all criteria with their commands and timeouts. Use spec_list first to find available spec names."`
- `gate_run` (lines 174-175): `"Run quality gate checks for a spec. Evaluates all executable criteria (shell commands) and returns pass/fail status per criterion with aggregate counts. Set include_evidence=true for full stdout/stderr output per criterion."`

Each description answers: what the tool does, what it returns, and when/how to use it. An agent with no prior Assay knowledge can use these descriptions to discover and invoke tools correctly.

### The server instructions field provides orientation for agents unfamiliar with Assay
**Evidence:** `server.rs:218-224`. The `get_info()` method on `ServerHandler` returns:
```rust
instructions: Some(
    "Assay development kit. Manages specs (what to build) and gates \
     (quality checks). Use spec_list to discover specs, spec_get to \
     read one, gate_run to evaluate criteria."
        .to_string(),
),
```
This gives an agent the conceptual model (specs + gates) and the entry point (`spec_list`) in one sentence.

---

### Plan 02 Must-Haves

### `assay mcp serve` starts the MCP server and first byte on stdout is `{`
**Evidence:** `crates/assay-cli/src/main.rs:40-43` defines `McpCommand::Serve`. Lines 510-517 handle it:
```rust
Some(Command::Mcp { command }) => match command {
    McpCommand::Serve => {
        init_mcp_tracing();
        if let Err(e) = assay_mcp::serve().await {
            eprintln!("Error: {e:?}");
            std::process::exit(1);
        }
    }
},
```
`init_mcp_tracing()` (lines 470-483) configures tracing with `with_writer(std::io::stderr)` and `with_ansi(false)`, keeping stdout clean for JSON-RPC. E2E Scenario 1 (confirmed in 08-02-SUMMARY) verified the first byte is `{`.

### An agent calling spec_get with a valid spec name receives the full spec as structured JSON
**Evidence:** `server.rs:152-169`. Loads spec via `load_spec()`, serializes via `serde_json::to_string(&spec)`, returns as `CallToolResult::success`. Helper test `test_load_spec_valid` (lines 709-738) creates a real spec file in a tempdir and asserts the returned spec has the correct `name` and `criteria.len()`. E2E Scenario 4 confirmed.

### An agent calling gate_run receives a summary with evidence for each criterion, and the async handler does not block the tokio runtime
**Evidence:** `spawn_blocking` usage confirmed above (lines 197-201). E2E Scenarios 6 and 7 confirmed summary and evidence modes respectively. MCP-03 and MCP-07 both satisfied.

### An agent calling spec_list receives an array of available spec entries in the project
**Evidence:** `server.rs:119-146`. E2E Scenario 3 confirmed. Response serialization test `test_spec_list_entry_serialization` (lines 796-808) verifies JSON shape including `criteria_count`.

### An agent calling spec_get with a nonexistent spec receives isError: true
**Evidence:** `server.rs:161-164`. `load_spec()` returns `Err(domain_error(&e))` for any `AssayError`. Helper test `test_load_spec_not_found` (lines 677-706) creates a tempdir with config but no specs, calls `load_spec()` with `"nonexistent"`, asserts `err_result.is_error.unwrap_or(false)` and that the error text contains the spec name. E2E Scenario 5 confirmed.

### An agent calling any tool in a directory without .assay/ receives isError: true
**Evidence:** `server.rs:121-124`. If `load_config()` fails (config not found = no `.assay/` directory), the handler returns `Ok(err_result)` where `err_result.is_error = true`. Helper test `test_load_config_missing_project` (lines 647-674) creates an empty tempdir, calls `load_config()`, asserts `is_error` is true and error text mentions "config". E2E Scenario 8 confirmed.

---

## Artifact Size Check

| File | Actual Lines | Plan Minimum |
|------|-------------|--------------|
| `crates/assay-mcp/src/server.rs` | 947 | 250 |
| `crates/assay-mcp/src/lib.rs` | 18 | 8 |
| `crates/assay-cli/src/main.rs` | 537 | 480 |

All artifacts exceed minimum line counts.

## Dependency Check

`crates/assay-mcp/Cargo.toml` has all required dependencies:
- `assay-core.workspace = true` — for `spec::load`, `spec::scan`, `config::load`, `gate::evaluate_all`, `AssayError`
- `assay-types.workspace = true` — for `Config`, `Spec` type naming in helper signatures
- `rmcp.workspace = true` — MCP protocol types and macros
- `schemars.workspace = true` — for `#[derive(JsonSchema)]` on parameter structs
- `serde`, `serde_json`, `tokio`, `tracing` — all workspace dependencies
- Dev: `chrono.workspace = true` (for `GateResult` construction in unit tests), `tempfile.workspace = true` (for integration tests with real `.assay/` dirs)

## Conclusion

Status: **passed**

All 16 must-haves from Plan 01 and Plan 02 are verified against the actual source code with specific file:line evidence. The spike server is completely removed, AssayServer implements exactly the three required tools with correct behavior, spawn_blocking bridges sync gate evaluation into the async MCP handler, domain errors are consistently surfaced as isError: true tool results (not protocol errors), and the CLI wiring is correct with no stdout contamination risk. The E2E checkpoint (08-02 Task 2) confirmed all 9 scenarios passed against a real test project.
