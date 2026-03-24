---
id: T01
parent: S03
milestone: M007
provides:
  - "`ServerState::load_or_new(queue_dir, max_concurrent)` — reads persisted state, remaps Dispatching/Running → Queued, returns ready ServerState"
  - "Preserves `attempt` counts across restart"
  - "`tracing::info!` on startup distinguishing cold-start (n=0) from crash-recovery (n>0)"
  - "`test_load_or_new_restart_recovery` — verifies 3-job recovery with status remapping and attempt preservation"
  - "`test_load_or_new_missing_file` — verifies cold-start returns empty queue with queue_dir set"
key_files:
  - crates/smelt-cli/src/serve/queue.rs
key_decisions:
  - "load_or_new calls new_with_persistence internally so queue_dir is always Some(...) — mutations after startup continue persisting automatically"
patterns_established:
  - "Crash recovery pattern: read persisted state → remap in-flight jobs → rebuild ServerState with same queue_dir"
observability_surfaces:
  - "tracing::info!(\"load_or_new: loaded {n} jobs from {path}, {remapped} remapped to Queued\") emitted on every startup; n=0/remapped=0 = cold start; n>0 = recovery"
duration: 10min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T01: Implement `ServerState::load_or_new` with restart-recovery tests

**`load_or_new` added to `ServerState` in `queue.rs` — reads persisted queue, remaps Dispatching/Running to Queued, preserves attempt counts; 2 new tests; all 13 queue tests pass**

## What Happened

Added `pub fn load_or_new(queue_dir: PathBuf, max_concurrent: usize) -> Self` to the `ServerState` impl block. The implementation calls `read_queue_state(&queue_dir)` to get the persisted jobs, iterates mutably remapping `Dispatching` or `Running` status to `Queued` while counting remaps, then calls `new_with_persistence` to build the base state and replaces its `jobs` field with the reconstructed `VecDeque`. This ensures the returned state has `queue_dir: Some(...)` so all subsequent mutations (enqueue, complete, cancel) continue writing to disk.

A `tracing::info!` line logs the total job count and remapped count on every call — `n=0, remapped=0` for a normal first run, `n>0, remapped>0` for crash recovery.

Two unit tests cover the two paths:
- `test_load_or_new_restart_recovery`: writes 3 jobs (Queued/0, Running/2, Queued/1), calls `load_or_new`, asserts all 3 are Queued with attempts 0/2/1 preserved and `queue_dir` set.
- `test_load_or_new_missing_file`: empty temp dir, cold-start path — asserts empty queue, `queue_dir` is Some, `max_concurrent` matches.

## Verification

```
cargo test -p smelt-cli -- queue
running 13 tests
test serve::queue::tests::test_load_or_new_restart_recovery ... ok
test serve::queue::tests::test_load_or_new_missing_file ... ok
... (11 existing tests) ...
test result: ok. 13 passed; 0 failed

cargo check -p smelt-cli
Finished `dev` profile — zero warnings
```

## Diagnostics

- Search `.smelt/serve.log` (or stderr) for `"load_or_new: loaded"` to confirm startup mode
- `cat queue_dir/.smelt-queue-state.toml` shows persisted jobs before startup
- `read_queue_state` already emits `warn!` on corrupt/unreadable files; `load_or_new` returns empty queue in that case (non-fatal)

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/queue.rs` — added `load_or_new`, added `tracing::info` import, added 2 unit tests
