---
estimated_steps: 4
estimated_files: 1
---

# T01: Implement `ServerState::load_or_new` with restart-recovery tests

**Slice:** S03 — Load-on-startup + restart-recovery integration test
**Milestone:** M007

## Description

Add `pub fn ServerState::load_or_new(queue_dir: PathBuf, max_concurrent: usize) -> Self` to `queue.rs`. This function is the core of R028's restart-recovery: it reads the persisted state file, remaps any `Dispatching` or `Running` job to `Queued` (per D109), preserves `attempt` counts, and returns a fully-initialized `ServerState` with `queue_dir: Some(...)` so all subsequent mutations continue writing. Two unit tests verify the recovery semantics and the missing-file (cold-start) path.

## Steps

1. Add `pub fn load_or_new(queue_dir: PathBuf, max_concurrent: usize) -> Self` to the `ServerState` `impl` block in `queue.rs`. The implementation: call `read_queue_state(&queue_dir)` to get `Vec<QueuedJob>`; iterate mutably, setting `status = JobStatus::Queued` for any job whose status is `Dispatching` or `Running`; count how many were remapped; collect the vec into a `VecDeque`; call `Self::new_with_persistence(max_concurrent, queue_dir)` to build the base state; replace `state.jobs` with the reconstructed `VecDeque`; return `state`.

2. Add a `tracing::info!` after remapping: `"load_or_new: loaded {n} jobs from {path}, {remapped} remapped to Queued"` where `n` is total jobs and `remapped` is the count of status changes. This is the startup diagnostic that distinguishes a cold start (n=0) from a recovery (n>0) in the log.

3. Add `test_load_or_new_restart_recovery` in the `#[cfg(test)] mod tests` block:
   - Create `TempDir`
   - Build 3 `QueuedJob` values using `make_job` (or inline construction): job-A `status=Queued, attempt=0`; job-B `status=Running, attempt=2`; job-C `status=Queued, attempt=1`
   - Write them to a `VecDeque` and call `write_queue_state(dir.path(), &jobs)`
   - Call `ServerState::load_or_new(dir.path().to_path_buf(), 2)`
   - Assert `state.jobs.len() == 3`
   - Assert all three `status == JobStatus::Queued`
   - Assert attempts are 0, 2, 1 respectively (preserved as-is)
   - Assert `state.queue_dir == Some(dir.path().to_path_buf())`

4. Add `test_load_or_new_missing_file` in the same `mod tests` block:
   - Create fresh `TempDir` (no writes)
   - Call `ServerState::load_or_new(dir.path().to_path_buf(), 4)`
   - Assert `state.jobs.is_empty()`
   - Assert `state.queue_dir.is_some()`
   - Assert `state.max_concurrent == 4`

## Must-Haves

- [ ] `load_or_new` is `pub` and in the `ServerState` `impl` block in `queue.rs`
- [ ] `Dispatching` and `Running` jobs are remapped to `Queued`; `Queued`, `Retrying`, `Complete`, `Failed` jobs are left with their original status
- [ ] `attempt` count is preserved unchanged for all jobs
- [ ] `queue_dir` field on the returned state is `Some(queue_dir)` — not `None`
- [ ] `tracing::info!` emitted with job count and remapped count on every call
- [ ] `test_load_or_new_restart_recovery` passes: 3 jobs all `Queued`, attempts 0/2/1
- [ ] `test_load_or_new_missing_file` passes: empty queue, `queue_dir` is `Some`, `max_concurrent` matches
- [ ] `cargo test -p smelt-cli -- queue` — 13 tests pass (11 existing + 2 new)
- [ ] `cargo check -p smelt-cli` — zero warnings

## Verification

- `cargo test -p smelt-cli -- queue` — verify 13 tests pass, 0 failed
- `cargo check -p smelt-cli` — verify zero warnings
- Inspect test output: `test serve::queue::tests::test_load_or_new_restart_recovery ... ok` and `test serve::queue::tests::test_load_or_new_missing_file ... ok` both present

## Observability Impact

- Signals added/changed: `tracing::info!("load_or_new: loaded {n} jobs from {path}, {remapped} remapped to Queued")` — emitted on every daemon startup; visible in `.smelt/serve.log` (TUI mode) or stderr; `n=0, remapped=0` = normal first run; `n>0, remapped>0` = crash recovery happened
- How a future agent inspects this: search `.smelt/serve.log` for `"load_or_new: loaded"` to confirm startup mode; `cat queue_dir/.smelt-queue-state.toml` to see what was loaded
- Failure state exposed: if `read_queue_state` fails (corrupt file), the existing `warn!` from S02 is the signal; `load_or_new` returns an empty queue and the daemon starts cleanly — no second-level logging needed since the root cause is already logged

## Inputs

- `crates/smelt-cli/src/serve/queue.rs` — `read_queue_state`, `new_with_persistence`, `ServerState` struct, `JobStatus` variants (Queued/Dispatching/Running), existing test helpers (`make_job`, `TempDir` usage)
- S02 Forward Intelligence: `read_queue_state` returns `Vec<QueuedJob>` not `VecDeque` — must convert; `attempt` count is preserved in the TOML round-trip; jobs with `Dispatching`/`Running` status need remapping

## Expected Output

- `crates/smelt-cli/src/serve/queue.rs` — `load_or_new` method added to `ServerState` impl; 2 new unit tests in `mod tests`
