---
id: T02
parent: S01
milestone: M001
provides:
  - save_context(), load_context(), list_contexts() persistence functions for GateEvalContext
  - GateEvalContextNotFound error variant in AssayError
key_files:
  - crates/assay-core/src/gate/session.rs
  - crates/assay-core/src/error.rs
key_decisions:
  - Followed work_session.rs pattern exactly for consistency — same atomic write, same validate_path_component, same list/sort approach
patterns_established:
  - GateEvalContext persistence under .assay/gate_sessions/<session_id>.json mirrors WorkSession under .assay/sessions/
observability_surfaces:
  - GateEvalContextNotFound error variant — structured error with session_id for missing contexts
  - IO/JSON errors include file path and operation context
  - .assay/gate_sessions/*.json files are human-readable pretty-printed JSON
duration: 10m
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T02: Add GateEvalContext persistence functions and tests

**Added save_context(), load_context(), and list_contexts() persistence functions for GateEvalContext with atomic writes and comprehensive tests.**

## What Happened

Added the `GateEvalContextNotFound` error variant to `AssayError` in `error.rs`, mirroring the existing `WorkSessionNotFound` pattern.

Implemented three persistence functions in `gate/session.rs` following the proven `work_session.rs` pattern exactly:

- `save_context()` — creates `.assay/gate_sessions/` dir, validates session_id via `history::validate_path_component`, serializes to pretty JSON, writes atomically via tempfile-then-rename
- `load_context()` — validates ID, reads and deserializes JSON, returns `GateEvalContextNotFound` on missing file
- `list_contexts()` — lists `.json` files in `gate_sessions/`, extracts stems, sorts, returns empty vec if directory doesn't exist

Added 7 tests: `save_and_load_round_trip`, `save_creates_directory`, `load_not_found`, `list_empty`, `list_returns_sorted`, `save_rejects_path_traversal`, `load_rejects_path_traversal`.

## Verification

- `cargo test -p assay-core -- gate::session` — all 19 tests pass (12 existing + 7 new)
- `just ready` — full check suite passes (fmt, lint, test, deny)

### Slice-level verification status (intermediate task):
- ✅ `just ready` passes
- ✅ `rg "AgentSession" --type rust crates/` returns zero matches (from T01)
- ✅ Persistence round-trip test: save a GateEvalContext, load it by ID, assert equality
- ✅ List test: save multiple contexts, list returns all IDs sorted
- ✅ Schema snapshot test passes with renamed snapshot file (from T01)
- ⬜ MCP server compiles with write-through changes (T03)

## Diagnostics

- `ls .assay/gate_sessions/` to see persisted sessions
- `load_context()` to read any session by ID
- IO errors include path context; JSON deserialization errors include file path; not-found is a distinct `GateEvalContextNotFound` error variant

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/error.rs` — added `GateEvalContextNotFound` error variant
- `crates/assay-core/src/gate/session.rs` — added `save_context`, `load_context`, `list_contexts` functions and 7 tests
