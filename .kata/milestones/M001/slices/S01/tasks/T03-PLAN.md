---
estimated_steps: 5
estimated_files: 1
---

# T03: Wire MCP server write-through persistence

**Slice:** S01 — Prerequisites — Persistence & Rename
**Milestone:** M001

## Description

Wire the persistence functions from T02 into the MCP server so that gate sessions survive restarts. The existing in-memory `HashMap<String, GateEvalContext>` becomes a write-through cache: `gate_run` creates + saves, `gate_report` updates + saves, `gate_finalize` checks HashMap then falls back to disk load. Persistence failures log warnings but do not block MCP responses — the in-memory path remains the primary fast path.

## Steps

1. Identify how `assay_dir` is threaded through the MCP server (check the server struct/context for the `.assay` directory path). Ensure it's accessible in gate_run, gate_report, and gate_finalize handlers.
2. In the `gate_run` handler: after inserting the new GateEvalContext into the HashMap (inside the Mutex lock), call `gate::session::save_context(assay_dir, &context)`. If save fails, log a warning with `tracing::warn!` but continue — the in-memory session is still valid.
3. In the `gate_report` handler: after calling `report_evaluation()` and updating the context in the HashMap, call `save_context()` to persist the updated state. Same warning-on-failure pattern.
4. In the `gate_finalize` handler: before the existing HashMap lookup, add a fallback path — if the session is not in the HashMap, try `load_context()` from disk. After successful finalization and history save, delete the on-disk file with `std::fs::remove_file()` (best-effort, log warning on failure). The delete prevents stale files from accumulating.
5. Verify: `cargo build -p assay-mcp` compiles clean. `cargo test -p assay-mcp` passes. `just ready` passes.

## Must-Haves

- [ ] `gate_run` persists new context to disk after HashMap insert
- [ ] `gate_report` persists updated context to disk after evaluation
- [ ] `gate_finalize` falls back to disk load when session not in HashMap
- [ ] `gate_finalize` cleans up on-disk file after successful finalization
- [ ] Persistence failures log warnings but don't block MCP responses
- [ ] `just ready` passes

## Verification

- `cargo build -p assay-mcp` — compiles clean with no warnings
- `cargo test -p assay-mcp` — existing tests pass (they exercise gate_run/report/finalize paths)
- `just ready` passes

## Observability Impact

- Signals added/changed: `tracing::warn!` on persistence failures in gate_run/gate_report/gate_finalize — structured warning with session_id and error
- How a future agent inspects this: check `.assay/gate_sessions/` for persisted in-progress sessions; tracing logs show save/load/delete outcomes
- Failure state exposed: persistence warnings include session_id, file path, and error cause; in-memory operation continues on disk failure

## Inputs

- `crates/assay-mcp/src/server.rs` — T01 output with GateEvalContext type references
- `crates/assay-core/src/gate/session.rs` — T02 output with save_context/load_context functions
- MCP server's `assay_dir` path threading (to be discovered in step 1)

## Expected Output

- `crates/assay-mcp/src/server.rs` — write-through persistence wired into gate_run, gate_report, gate_finalize
