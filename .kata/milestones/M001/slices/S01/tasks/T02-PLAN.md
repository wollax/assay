---
estimated_steps: 5
estimated_files: 2
---

# T02: Add GateEvalContext persistence functions and tests

**Slice:** S01 — Prerequisites — Persistence & Rename
**Milestone:** M001

## Description

Implement `save_context()`, `load_context()`, and `list_contexts()` in `assay-core/src/gate/session.rs`, following the proven `work_session.rs` pattern exactly. Sessions persist as pretty-printed JSON under `.assay/gate_sessions/<session_id>.json` using atomic tempfile-then-rename writes. Add a `GateEvalContextNotFound` error variant to `AssayError`. Include comprehensive tests covering round-trip, list ordering, error cases, and path traversal rejection.

## Steps

1. In `crates/assay-core/src/error.rs`: add `GateEvalContextNotFound { session_id: String }` variant to `AssayError` with display message `"gate eval context '{session_id}' not found"`. Mirror the `WorkSessionNotFound` pattern.
2. In `crates/assay-core/src/gate/session.rs`: add `use std::io::Write; use std::path::{Path, PathBuf}; use tempfile::NamedTempFile;` imports. Implement `pub fn save_context(assay_dir: &Path, context: &GateEvalContext) -> Result<PathBuf>` — create `.assay/gate_sessions/` dir, validate session_id with `history::validate_path_component`, serialize to pretty JSON, atomic tempfile-then-rename.
3. Implement `pub fn load_context(assay_dir: &Path, session_id: &str) -> Result<GateEvalContext>` — validate ID, construct path `gate_sessions/{session_id}.json`, read and deserialize, return `GateEvalContextNotFound` on missing file.
4. Implement `pub fn list_contexts(assay_dir: &Path) -> Result<Vec<String>>` — list `.json` files in `gate_sessions/`, extract stems, sort, return. Return empty vec if directory doesn't exist.
5. Add tests in the existing `mod tests` block: `save_and_load_round_trip`, `save_creates_directory`, `load_not_found`, `list_empty`, `list_returns_sorted`, `save_rejects_path_traversal`, `load_rejects_path_traversal`.

## Must-Haves

- [ ] `save_context()` uses atomic tempfile-then-rename writes
- [ ] `save_context()` creates `gate_sessions/` directory if missing
- [ ] `save_context()` validates session_id via `history::validate_path_component`
- [ ] `load_context()` returns `GateEvalContextNotFound` on missing file
- [ ] `load_context()` rejects path traversal IDs
- [ ] `list_contexts()` returns sorted IDs, empty vec for missing dir
- [ ] All new tests pass

## Verification

- `cargo test -p assay-core -- gate::session` — all new persistence tests pass
- `just ready` passes

## Observability Impact

- Signals added/changed: `GateEvalContextNotFound` error variant — structured error for missing sessions
- How a future agent inspects this: `ls .assay/gate_sessions/` to see persisted sessions; `load_context()` to read any session by ID; error messages include file path and session ID
- Failure state exposed: IO errors include path context; JSON deserialization errors include file path; not-found is a distinct error variant

## Inputs

- `crates/assay-core/src/gate/session.rs` — T01 output with GateEvalContext references
- `crates/assay-core/src/work_session.rs` — pattern to follow for save/load/list
- `crates/assay-core/src/error.rs` — existing error variants to extend

## Expected Output

- `crates/assay-core/src/error.rs` — new `GateEvalContextNotFound` variant
- `crates/assay-core/src/gate/session.rs` — `save_context`, `load_context`, `list_contexts` functions with tests
