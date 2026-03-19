---
estimated_steps: 5
estimated_files: 1
---

# T03: Add `cycle_status`, `cycle_advance`, `chunk_status` MCP tools

**Slice:** S02 — Development Cycle State Machine
**Milestone:** M005

## Description

Adds three new `#[tool]`-annotated methods to `AssayServer` in `crates/assay-mcp/src/server.rs`: `cycle_status` (queries current cycle state), `cycle_advance` (runs gates and advances the cycle via `spawn_blocking`), and `chunk_status` (reads the last gate run from history without running new gates). Completes R044 by wiring the cycle state machine to the MCP transport layer. Three presence tests verify the tools appear in the router.

## Steps

1. Add parameter structs near the other milestone param structs (around line 531):
   ```rust
   /// Parameters for the `cycle_status` tool.
   #[derive(Debug, Deserialize, JsonSchema)]
   pub struct CycleStatusParams {}

   /// Parameters for the `cycle_advance` tool.
   #[derive(Debug, Deserialize, JsonSchema)]
   pub struct CycleAdvanceParams {
       /// Optional milestone slug to advance. If omitted, targets the first in_progress milestone.
       #[serde(default)]
       pub milestone_slug: Option<String>,
   }

   /// Parameters for the `chunk_status` tool.
   #[derive(Debug, Deserialize, JsonSchema)]
   pub struct ChunkStatusParams {
       /// Slug of the chunk (spec) to report gate status for.
       pub chunk_slug: String,
   }
   ```

2. Add a local response struct for `chunk_status` near other response types:
   ```rust
   #[derive(Debug, Serialize)]
   struct ChunkStatusResponse {
       chunk_slug: String,
       has_history: bool,
       latest_run_id: Option<String>,
       passed: Option<usize>,
       failed: Option<usize>,
       required_failed: Option<usize>,
   }
   ```

3. Implement `cycle_status` tool method on `AssayServer`:
   ```rust
   #[tool(description = "Return the active development cycle status: the first in_progress milestone, its active chunk, and progress counts. Returns null if no milestone is in_progress.")]
   pub async fn cycle_status(&self, _params: Parameters<CycleStatusParams>) -> Result<CallToolResult, McpError> {
       let cwd = resolve_cwd()?;
       let assay_dir = cwd.join(".assay");
       match assay_core::milestone::cycle_status(&assay_dir) {
           Ok(Some(status)) => {
               let json = serde_json::to_string(&status).map_err(|e| McpError::internal_error(...))?;
               Ok(CallToolResult::success(vec![Content::text(json)]))
           }
           Ok(None) => Ok(CallToolResult::success(vec![Content::text("null")])),
           Err(e) => Ok(domain_error(&e)),
       }
   }
   ```

4. Implement `cycle_advance` tool method — wraps `cycle_advance` in `spawn_blocking`, consistent with `gate_run` pattern:
   ```rust
   #[tool(description = "Evaluate gates for the active chunk of the in_progress milestone and advance the development cycle. Targets the first in_progress milestone unless milestone_slug is specified. Returns updated CycleStatus on success, or error if required gates fail or preconditions are not met.")]
   pub async fn cycle_advance(&self, params: Parameters<CycleAdvanceParams>) -> Result<CallToolResult, McpError> {
       let cwd = resolve_cwd()?;
       let config = match load_config(&cwd) { Ok(c) => c, Err(e) => return Ok(e) };
       let assay_dir = cwd.join(".assay");
       let specs_dir = cwd.join(".assay").join(&config.specs_dir.unwrap_or_else(|| "specs".to_string()));
       let working_dir = resolve_working_dir(&cwd, &config);
       let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);
       let milestone_slug = params.0.milestone_slug.clone();
       let result = tokio::task::spawn_blocking(move || {
           assay_core::milestone::cycle_advance(&assay_dir, &specs_dir, &working_dir, milestone_slug.as_deref(), config_timeout)
       }).await.map_err(|e| McpError::internal_error(format!("cycle_advance panicked: {e}"), None))?;
       match result {
           Ok(status) => {
               let json = serde_json::to_string(&status).map_err(|e| McpError::internal_error(...))?;
               Ok(CallToolResult::success(vec![Content::text(json)]))
           }
           Err(e) => Ok(domain_error(&e)),
       }
   }
   ```

5. Implement `chunk_status` tool method — reads history without running gates:
   - Validate slug via `validate_path_component`
   - Call `assay_core::history::list(&assay_dir, &params.0.chunk_slug)` → get all run IDs
   - If empty: return `ChunkStatusResponse { chunk_slug, has_history: false, rest: None }`
   - Get latest: `all_ids.last()` (list returns oldest-first, so last = most recent)
   - Call `assay_core::history::load(&assay_dir, &params.0.chunk_slug, latest_run_id)` → get record
   - Build `ChunkStatusResponse { chunk_slug, has_history: true, latest_run_id: Some(...), passed: Some(...), failed: Some(...), required_failed: Some(...) }`
   - Serialize and return

6. Add 3 presence tests at the bottom of the `#[cfg(test)]` block near the other milestone router tests:
   ```rust
   #[tokio::test] #[serial]
   async fn cycle_status_tool_in_router() { /* assert tool_names.contains(&"cycle_status") */ }
   #[tokio::test] #[serial]
   async fn cycle_advance_tool_in_router() { /* assert tool_names.contains(&"cycle_advance") */ }
   #[tokio::test] #[serial]
   async fn chunk_status_tool_in_router() { /* assert tool_names.contains(&"chunk_status") */ }
   ```

## Must-Haves

- [ ] `CycleStatusParams`, `CycleAdvanceParams`, `ChunkStatusParams` structs defined with correct derives
- [ ] `cycle_status` tool returns JSON `CycleStatus` or `"null"` (not an error) when no active milestone
- [ ] `cycle_advance` tool wraps core function in `tokio::task::spawn_blocking` (consistent with `gate_run`)
- [ ] `cycle_advance` returns `domain_error` (not a panic) when gates fail or preconditions unmet
- [ ] `chunk_status` returns `{ has_history: false }` gracefully when no history exists for the chunk
- [ ] All 3 presence tests pass; existing 4 milestone tests still pass
- [ ] `cargo test --workspace` green

## Verification

```bash
# MCP presence tests
cargo test -p assay-mcp -- cycle

# All MCP milestone tests (existing + new)
cargo test -p assay-mcp -- milestone

# Full workspace
cargo test --workspace
```

## Observability Impact

- Signals added/changed: `cycle_advance` MCP tool surfaces the full `CycleStatus` JSON on success (including `completed_count`, `total_count`, `active_chunk_slug`, `phase`); on gate failure the error message includes `required_failed` count and chunk slug; `chunk_status` exposes `required_failed` from the last run without running new gates
- How a future agent inspects this: call `cycle_status` with no params to get current position; call `chunk_status` with a chunk slug to check if the last run passed without incurring gate evaluation cost
- Failure state exposed: `domain_error(&e)` in `cycle_advance` maps `AssayError::Io` to `isError: true` MCP response with the operation/path message; agents can differentiate "no active milestone" from "gates failed" by the error message text

## Inputs

- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status`, `cycle_advance`, `CycleStatus` (created in T02)
- `crates/assay-mcp/src/server.rs` — existing patterns: `spawn_blocking` in `gate_run` (~line 1283), `domain_error` helper, `resolve_cwd`, `load_config`, `resolve_working_dir`, `validate_path_component`, `history::list`/`history::load` in `gate_history` (~line 2100)
- `crates/assay-mcp/src/server.rs:531` — existing milestone param struct location for placement

## Expected Output

- `crates/assay-mcp/src/server.rs` — `CycleStatusParams`, `CycleAdvanceParams`, `ChunkStatusParams` added; `ChunkStatusResponse` struct added; 3 new `#[tool]` methods on `AssayServer`; 3 new presence tests
