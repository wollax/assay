---
estimated_steps: 5
estimated_files: 1
---

# T02: Add `queue_dir` to `ServerState` + wire writes in mutation methods

**Slice:** S02 — Atomic state file — write on every transition
**Milestone:** M007

## Description

Store `queue_dir: Option<PathBuf>` on `ServerState`, add `new_with_persistence()` constructor, and wire `write_queue_state` calls into `enqueue`, `complete`, and `cancel`. The existing `new(n)` constructor is left untouched so all 46 current tests continue to pass without modification. A new wiring test (`test_server_state_writes_on_enqueue`) verifies the file is written to disk when `new_with_persistence` is used. This is the final deliverable for S02.

## Steps

1. Add `queue_dir: Option<PathBuf>` field to the `ServerState` struct (at the end of the field list to minimize diff).
2. Update `ServerState::new(max_concurrent)` to set `queue_dir: None` — preserves all existing callsites with no changes required.
3. Add `pub fn new_with_persistence(max_concurrent: usize, queue_dir: PathBuf) -> Self`:
   ```rust
   pub fn new_with_persistence(max_concurrent: usize, queue_dir: PathBuf) -> Self {
       ServerState {
           jobs: VecDeque::new(),
           running_count: 0,
           max_concurrent,
           queue_dir: Some(queue_dir),
       }
   }
   ```
4. Wire `write_queue_state` at the end of `enqueue`, `complete`, and `cancel`:
   - Pattern: `if let Some(ref dir) = self.queue_dir { write_queue_state(dir, &self.jobs); }`
   - Do NOT wire in `try_dispatch` — Dispatching is transient (D116)
5. Add unit test `test_server_state_writes_on_enqueue` in the `#[cfg(test)]` block:
   - Create a `TempDir`
   - Call `ServerState::new_with_persistence(2, tmp.path().to_path_buf())`
   - Call `state.enqueue(PathBuf::from("/tmp/test.toml"), JobSource::HttpApi)`
   - Assert the state file `tmp.path().join(".smelt-queue-state.toml")` exists
   - Call `read_queue_state(tmp.path())` and assert `len() == 1` and `jobs[0].status == JobStatus::Queued`
6. Run `cargo test -p smelt-cli` and confirm all tests pass (≥47).

## Must-Haves

- [ ] `ServerState::new(n)` is unchanged — all 46 existing tests pass without modification
- [ ] `new_with_persistence(max_concurrent, queue_dir)` returns a `ServerState` with `queue_dir: Some(...)`
- [ ] `enqueue` calls `write_queue_state` when `queue_dir` is `Some`
- [ ] `complete` calls `write_queue_state` when `queue_dir` is `Some`
- [ ] `cancel` calls `write_queue_state` when `queue_dir` is `Some`
- [ ] `try_dispatch` does NOT call `write_queue_state` (Dispatching is transient)
- [ ] `test_server_state_writes_on_enqueue` passes: state file exists after `enqueue`; `read_queue_state` returns 1 job with `status == Queued`
- [ ] `cargo test -p smelt-cli` — all tests pass (≥47 total), zero failures
- [ ] `cargo check -p smelt-cli` — zero warnings

## Verification

- `cargo test -p smelt-cli` passes showing ≥47 tests, 0 failed
- `cargo test -p smelt-cli -- queue::tests::test_server_state_writes_on_enqueue` passes individually
- `cargo check -p smelt-cli` exits 0 with zero warnings
- Confirm existing queue tests (`test_enqueue_and_dispatch`, `test_complete_failure_retries`, etc.) still appear in output — none deleted or renamed

## Observability Impact

- Signals added/changed: every call to `enqueue`, `complete`, `cancel` now triggers a `warn!()` on I/O failure — these appear in the tracing log (`.smelt/serve.log` in TUI mode)
- How a future agent inspects this: after any job transition, `cat queue_dir/.smelt-queue-state.toml` reflects the current queue state; if the file is absent or stale, it means the daemon started without persistence (`new()`) or a write failed
- Failure state exposed: `new_with_persistence` vs `new` is distinguishable by checking `queue_dir` is set; the absence of the state file after the first `enqueue` indicates the write path is broken

## Inputs

- `crates/smelt-cli/src/serve/queue.rs` with `write_queue_state` and `read_queue_state` from T01
- S01 `types.rs` providing `JobSource`, `JobStatus`, `QueuedJob` with serde
- All 46 existing smelt-cli tests must continue to pass unchanged

## Expected Output

- `crates/smelt-cli/src/serve/queue.rs` — `ServerState` gains `queue_dir: Option<PathBuf>` field; `new()` updated to set `None`; `new_with_persistence()` added; `enqueue`/`complete`/`cancel` call `write_queue_state`; new wiring test added
- `cargo test -p smelt-cli` output shows ≥47 tests, 0 failed, including `test_server_state_writes_on_enqueue`
