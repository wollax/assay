---
id: T03
parent: S02
milestone: M005
provides:
  - "`CycleStatusParams`, `CycleAdvanceParams`, `ChunkStatusParams` param structs with correct derives (Debug, Deserialize, JsonSchema)"
  - "`ChunkStatusResponse` response struct with `has_history`, `latest_run_id`, `passed`, `failed`, `required_failed` fields (Option-typed, skip_serializing_if)"
  - "`cycle_status` tool: returns JSON `CycleStatus` or literal `\"null\"` when no milestone is in_progress"
  - "`cycle_advance` tool: wraps `assay_core::milestone::cycle_advance` in `spawn_blocking`; returns updated `CycleStatus` JSON or `domain_error` on failure"
  - "`chunk_status` tool: reads history without running gates; returns `{ has_history: false }` gracefully when no runs exist"
  - "3 presence tests: `cycle_status_tool_in_router`, `cycle_advance_tool_in_router`, `chunk_status_tool_in_router` — all pass"
key_files:
  - crates/assay-mcp/src/server.rs
key_decisions:
  - "validate_path_component is pub(crate) in assay-core — not accessible from assay-mcp; chunk_status relies on history::list to reject invalid slugs naturally (same pattern as gate_history)"
  - "config_timeout suppressed with `let _ = config_timeout` comment: cycle_advance core takes 4 params (no timeout); MCP still loads config for specs_dir and working_dir resolution"
  - "Config.specs_dir is a plain String (not Option<String>), so used directly as `&config.specs_dir` in path join"
patterns_established:
  - "cycle_advance MCP tool follows gate_run spawn_blocking pattern: load config/paths in async context, move owned values into spawn_blocking closure"
  - "chunk_status no-history path: return early with { has_history: false } response before attempting history::load"
observability_surfaces:
  - "`cycle_status` MCP tool: zero-side-effect snapshot of active milestone/chunk/phase/progress as JSON"
  - "`cycle_advance` MCP tool: full `CycleStatus` JSON on success; `domain_error` on failure (distinguishes no-active-milestone from gates-failed by message text)"
  - "`chunk_status` MCP tool: last run `passed`/`failed`/`required_failed` without incurring gate evaluation cost"
duration: 20min
verification_result: passed
completed_at: 2026-03-19T00:00:00Z
blocker_discovered: false
---

# T03: Add `cycle_status`, `cycle_advance`, `chunk_status` MCP tools

**Three MCP tools wire the cycle state machine to the transport layer: `cycle_status` snapshots current position, `cycle_advance` runs gates and advances the milestone, and `chunk_status` reads the last gate run without new evaluation.**

## What Happened

Added param structs (`CycleStatusParams`, `CycleAdvanceParams`, `ChunkStatusParams`) after the existing `MilestoneGetParams` block and a `ChunkStatusResponse` struct near the other response types. Implemented three `#[tool]`-annotated async methods on `AssayServer`:

- `cycle_status`: calls `assay_core::milestone::cycle_status` synchronously (no blocking needed — pure read), returns JSON or `"null"`.
- `cycle_advance`: follows the `gate_run` `spawn_blocking` pattern — loads config/dirs in the async context, moves owned values into the closure, maps panics to `McpError::internal_error` and gate/precondition failures to `domain_error`.
- `chunk_status`: uses `history::list` to enumerate runs (oldest-first), takes the last entry as the most recent, loads the record, and fills `ChunkStatusResponse`. Returns early with `{ has_history: false }` when no runs exist.

Added 3 presence tests at the end of the `#[cfg(test)]` block.

## Verification

```
# Presence tests (3 new)
cargo test -p assay-mcp -- cycle      → 3 passed (cycle_status, cycle_advance in_router + session_update_full_lifecycle)
cargo test -p assay-mcp -- chunk      → 1 passed (chunk_status_tool_in_router)

# Existing milestone tests (4)
cargo test -p assay-mcp -- milestone  → 4 passed

# Core cycle integration tests (10, from T01/T02)
cargo test -p assay-core --features assay-types/orchestrate --test cycle → 10 passed

# Full workspace
cargo test --workspace → all green
```

## Diagnostics

- `cycle_status` MCP call: returns `{ milestone_slug, milestone_name, phase, active_chunk_slug, completed_count, total_count }` or `"null"`
- `cycle_advance` failure: `domain_error` maps `AssayError::Io { operation: "cycle_advance", path, source }` → `isError: true` MCP response; message text distinguishes "no active (in_progress) milestone found", "milestone is not in_progress", "gates failed: N required criteria did not pass"
- `chunk_status` call: `{ has_history: false }` when no runs; otherwise `{ has_history: true, latest_run_id, passed, failed, required_failed }`

## Deviations

- `validate_path_component` removed from `chunk_status` implementation: the function is `pub(crate)` in assay-core and inaccessible from assay-mcp. `history::list` returns an error for invalid slugs naturally. This matches how `gate_history` handles the same concern.
- `config_timeout` (from the plan's code sketch) not passed to `cycle_advance` core: the core function signature takes 4 params without a timeout. Config is still loaded for `specs_dir` and `working_dir` resolution.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-mcp/src/server.rs` — param structs (`CycleStatusParams`, `CycleAdvanceParams`, `ChunkStatusParams`), `ChunkStatusResponse`, 3 `#[tool]` methods (`cycle_status`, `cycle_advance`, `chunk_status`), 3 presence tests
