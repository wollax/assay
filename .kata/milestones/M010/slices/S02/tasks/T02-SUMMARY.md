---
id: T02
parent: S02
milestone: M010
provides:
  - LocalFsBackend with real filesystem persistence for all 7 StateBackend methods
key_files:
  - crates/assay-core/src/state_backend.rs
  - crates/assay-core/tests/state_backend.rs
key_decisions:
  - Fixed T01 contract test to expect checkpoints/latest.md instead of checkpoint.json, matching save_checkpoint's actual output format
patterns_established:
  - Atomic tempfile-rename pattern (NamedTempFile + persist) used consistently for all write operations in LocalFsBackend
  - poll_inbox returns Ok(vec![]) for non-existent inbox directory (graceful degradation)
observability_surfaces:
  - AssayError::io with path+operation context on every I/O failure
  - AssayError::json with path+operation context on serialization/deserialization failure
  - state.json written to run_dir confirms push_session_event writes; backend.read_run_state() reads it back
duration: 5 minutes
verification_result: passed
completed_at: 2026-03-26
blocker_discovered: false
---

# T02: Implement LocalFsBackend real method bodies

**Replaced all 7 stub method bodies in LocalFsBackend with real filesystem persistence using atomic tempfile-rename writes.**

## What Happened

Implemented all 7 `StateBackend` trait methods in `LocalFsBackend`:

1. **push_session_event**: Creates run_dir if missing, serializes `OrchestratorStatus` to pretty JSON, writes atomically via `NamedTempFile::new_in` + `write_all` + `sync_all` + `persist` to `state.json`. Pattern copied from `executor.rs::persist_state()`.
2. **read_run_state**: Reads `run_dir/state.json`, deserializes. Returns `Ok(None)` for `NotFound`, propagates other errors.
3. **save_checkpoint_summary**: Delegates to `crate::checkpoint::persistence::save_checkpoint(assay_dir, checkpoint)`, discarding the `PathBuf` return.
4. **send_message**: Creates inbox dir if missing, writes message contents atomically to `inbox_path/<name>`.
5. **poll_inbox**: Reads all files in inbox directory, collects `(filename, contents)` pairs, deletes each file after reading. Returns empty vec if inbox dir doesn't exist.
6. **annotate_run**: Creates run_dir if missing, writes manifest_path string atomically to `gossip_manifest_path.txt`.

All stub `tracing::warn!` messages removed.

Fixed T01 contract test `test_local_fs_backend_save_checkpoint_summary` — it expected `checkpoint.json` but `save_checkpoint` writes to `checkpoints/latest.md`.

## Verification

- `cargo test -p assay-core --features orchestrate --test state_backend` — **16/16 passed** (all T01 contract tests green)
- `cargo clippy -p assay-core --features orchestrate` — **no warnings**
- Slice-level checks all pass:
  - `cargo test -p assay-types --test schema_snapshots run_manifest_schema_snapshot` — 1 passed
  - `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — 5 passed
  - `cargo test -p assay-core --features orchestrate --test mesh_integration` — 2 passed
  - `cargo test -p assay-core --features orchestrate --test gossip_integration` — 2 passed
  - `cargo test -p assay-core --features orchestrate --test orchestrate_spans` — 5 passed
  - `cargo test -p assay-core --features orchestrate --test integration_modes` — 3 passed

## Diagnostics

- `backend.read_run_state(run_dir)` returns the last persisted `OrchestratorStatus`
- File existence at `run_dir/state.json` confirms writes occurred
- `AssayError::io` includes path and operation label (e.g. "writing orchestrator state", "reading state.json")
- `AssayError::json` includes path and operation label for serialization failures

## Deviations

- Fixed T01 contract test expectation: `checkpoint.json` → `checkpoints/latest.md` to match what `save_checkpoint()` actually writes. This was a test authoring error in T01, not a plan deviation.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/state_backend.rs` — All 7 stub methods replaced with real implementations; added `use std::io::Write` and `use tempfile::NamedTempFile` imports
- `crates/assay-core/tests/state_backend.rs` — Fixed checkpoint test to expect `checkpoints/latest.md` instead of `checkpoint.json`
