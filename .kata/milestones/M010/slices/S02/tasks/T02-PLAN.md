---
estimated_steps: 5
estimated_files: 2
---

# T02: Implement LocalFsBackend real method bodies

**Slice:** S02 — LocalFsBackend implementation and orchestrator wiring
**Milestone:** M010

## Description

Replaces all 7 stub method bodies in `LocalFsBackend` with real filesystem persistence. `push_session_event` uses the atomic tempfile-rename pattern (same as `persist_state`). `read_run_state` deserializes `state.json`. `save_checkpoint_summary` delegates to `checkpoint::persistence::save_checkpoint`. `send_message`/`poll_inbox` do file I/O. `annotate_run` writes the manifest path. After this task, all T01 integration tests should pass.

## Steps

1. Implement `push_session_event`: create `run_dir` if missing, serialize `OrchestratorStatus` with `serde_json::to_string_pretty`, write to `state.json` via `NamedTempFile::new_in(run_dir)` + `write_all` + `sync_all` + `persist`. Copy the pattern exactly from the existing `persist_state()` in `executor.rs`.
2. Implement `read_run_state`: read `run_dir/state.json`, deserialize with `serde_json::from_str`. Return `Ok(None)` if file doesn't exist (`io::ErrorKind::NotFound`). Return `Err` for other I/O errors.
3. Implement `save_checkpoint_summary`: delegate to `crate::checkpoint::persistence::save_checkpoint(assay_dir, checkpoint)`. Map the result to discard the `PathBuf` return (backend just needs `Result<()>`).
4. Implement `send_message`: create `inbox_path` dir if missing, write `contents` to `inbox_path/<name>` via atomic tempfile-rename. Implement `poll_inbox`: read all files in `inbox_path`, collect `(filename, contents)` pairs, delete each file after reading.
5. Implement `annotate_run`: write `manifest_path` string to `run_dir/gossip_manifest_path.txt` via atomic write.

## Must-Haves

- [ ] `push_session_event` uses atomic tempfile-rename (NamedTempFile + persist pattern)
- [ ] `read_run_state` returns `Ok(None)` when `state.json` doesn't exist, `Ok(Some(status))` when it does
- [ ] `save_checkpoint_summary` delegates to existing `checkpoint::persistence::save_checkpoint`
- [ ] `send_message` creates inbox dir if needed, writes message atomically
- [ ] `poll_inbox` reads and deletes messages, returns `(name, contents)` pairs
- [ ] `annotate_run` writes manifest path to a file under `run_dir`
- [ ] All stub `tracing::warn!` messages removed from implemented methods
- [ ] All T01 integration tests pass

## Verification

- `cargo test -p assay-core --features orchestrate --test state_backend` — all tests pass (green)
- `cargo clippy -p assay-core --features orchestrate` — no warnings

## Observability Impact

- Signals added/changed: Stub `tracing::warn!` removed; real I/O errors propagated via `AssayError` with path+operation context
- How a future agent inspects this: `backend.read_run_state(run_dir)` returns the last persisted status; file existence at `run_dir/state.json` confirms writes
- Failure state exposed: `AssayError::io` includes the path and operation label (e.g. "writing orchestrator state", "reading state.json")

## Inputs

- `crates/assay-core/src/state_backend.rs` — existing stub implementations from S01
- `crates/assay-core/src/orchestrate/executor.rs:111-133` — `persist_state()` pattern to replicate
- `crates/assay-core/src/checkpoint/persistence.rs:38-70` — `save_checkpoint()` to delegate to
- T01 integration tests defining the expected behavior

## Expected Output

- `crates/assay-core/src/state_backend.rs` — all 7 methods have real implementations
- T01 integration tests all pass (confirmed by verification command)
