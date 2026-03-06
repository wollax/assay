# Plan 20-04 Summary: MCP Context Tools

## Outcome

Both MCP tools (`context_diagnose` and `estimate_tokens`) implemented, registered in the tool router, and tested. All 60 assay-mcp tests pass.

## What Was Done

### Task 1: Parameter Structs and Tool Handlers
- Added `ContextDiagnoseParams` and `EstimateTokensParams` with optional `session_id` fields
- Implemented `context_diagnose` handler: resolves CWD, finds session dir, resolves session, calls `assay_core::context::diagnose()` via `spawn_blocking`, serializes `DiagnosticsReport` as JSON
- Implemented `estimate_tokens` handler: same pattern, calls `assay_core::context::estimate_tokens()`, serializes `TokenEstimate` as JSON
- Both tools registered automatically via `#[tool_router]` macro
- Updated server doc comment and ServerInfo instructions to mention new tools
- Updated `lib.rs` module docs and testing re-exports

### Task 2: Tests
- 4 parameter deserialization tests (with/without session_id for each tool)
- 2 async handler error-path tests (no session dir returns `isError: true`)

## Key Decisions
- Used `Parameters<T>` wrapper pattern (consistent with existing handlers) rather than `#[tool(params)]` attribute syntax
- Domain errors return `CallToolResult` with `isError: true` (not `McpError`)
- Session ID extracted from filename stem when not provided by caller

## Files Modified
- `crates/assay-mcp/src/server.rs` — tool handlers, param structs, tests
- `crates/assay-mcp/src/lib.rs` — doc comments, testing re-exports

## Verification
- `cargo check -p assay-mcp` passes
- `cargo test -p assay-mcp` — 60 tests pass (6 new)
