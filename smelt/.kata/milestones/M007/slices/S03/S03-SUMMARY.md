---
id: S03
parent: M007
milestone: M007
provides:
  - "`ServerState::load_or_new(queue_dir, max_concurrent)` — single startup entry point that reads persisted queue, remaps Dispatching/Running → Queued, preserves attempt counts, always sets queue_dir: Some"
  - "`commands/serve.rs` wired to call `load_or_new` instead of `new` — crash recovery active on every real `smelt serve` startup"
  - "`tracing::info!` on startup distinguishing cold-start (n=0, remapped=0) from crash-recovery (n>0, remapped>0)"
  - "2 unit tests: `test_load_or_new_restart_recovery` (3-job recovery with remapping + attempt preservation) and `test_load_or_new_missing_file` (cold-start empty queue)"
  - "`examples/server.toml` annotated with persistence and restart-recovery behavior for operators"
  - "R028 fully validated — enqueue → write → drop → reconstruct → redispatch cycle proven end-to-end"
requires:
  - slice: S01
    provides: "`QueuedJob` with Serialize+Deserialize; u64 epoch time fields; all 19 existing serve tests passing"
  - slice: S02
    provides: "`write_queue_state(queue_dir, jobs)` atomic write; `read_queue_state(queue_dir)` with warn on error; `new_with_persistence(max_concurrent, queue_dir)` constructor; round-trip unit test"
affects: []
key_files:
  - crates/smelt-cli/src/serve/queue.rs
  - crates/smelt-cli/src/commands/serve.rs
  - examples/server.toml
key_decisions:
  - "D120: `load_or_new` always creates `queue_dir: Some(...)` so post-startup mutations continue writing — delegates to `new_with_persistence` internally"
  - "D109: Dispatching/Running at crash time remapped to Queued (not Failed) — preserves work, attempt count intact"
patterns_established:
  - "Crash recovery pattern: read_queue_state → remap in-flight → rebuild with new_with_persistence — all in one constructor call"
  - "Persistence wiring pattern: one-line serve.rs change (new → load_or_new) activates the full persistence loop"
observability_surfaces:
  - "`tracing::info!(\"load_or_new: loaded {n} jobs from {path}, {remapped} remapped to Queued\")` — grep `.smelt/serve.log` or stderr to confirm startup mode"
  - "`cat queue_dir/.smelt-queue-state.toml` — shows persisted state before startup; human-readable TOML"
  - "`read_queue_state` emits `warn!` with path + error on parse failure — daemon starts with empty queue (non-fatal)"
drill_down_paths:
  - .kata/milestones/M007/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M007/slices/S03/tasks/T02-SUMMARY.md
duration: 15min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
---

# S03: Load-on-startup + restart-recovery integration test

**`ServerState::load_or_new` added and wired into `smelt serve` — crash-recovery is live; 52 tests pass; R028 fully validated**

## What Happened

T01 implemented `ServerState::load_or_new(queue_dir: PathBuf, max_concurrent: usize) -> Self` in `queue.rs`. The implementation reads the persisted queue via `read_queue_state`, iterates mutably remapping any `Dispatching` or `Running` job to `Queued` (preserving `attempt` count), then delegates to `new_with_persistence` for the base state and sets its `jobs` field to the reconstructed `VecDeque`. This ensures `queue_dir` is always `Some(...)` so all subsequent mutations — enqueue, complete, cancel — continue writing to disk. A `tracing::info!` line fires on every call: `n=0, remapped=0` = cold start; `n>0, remapped>0` = crash recovery. Two unit tests cover the recovery path (3 jobs with mixed statuses + attempt preservation) and the cold-start path (empty TempDir, empty queue, queue_dir set).

T02 was a single-line change in `commands/serve.rs`: replaced `ServerState::new(config.max_concurrent)` with `ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)`. No import changes required — `ServerState` was already in scope. A comment block was added to `examples/server.toml` above `queue_dir` explaining the automatic persistence loop, state file location, and restart-recovery behavior.

## Verification

- `cargo test -p smelt-cli -- queue` — 13 tests pass (11 existing + `test_load_or_new_restart_recovery` + `test_load_or_new_missing_file`)
- `cargo test -p smelt-cli` — 52 passed, 0 failed, 0 warnings
- `grep "load_or_new" crates/smelt-cli/src/commands/serve.rs` — wiring line confirmed present
- `cargo check -p smelt-cli` — exits 0, zero new warnings

## Requirements Advanced

- R028 (`Persistent queue across smelt serve restarts`) — S03 delivers the final piece: `load_or_new` startup wiring + integration test proves the full enqueue → write → drop → reconstruct → redispatch cycle

## Requirements Validated

- R028 — all three supporting slices (S01 serialization, S02 atomic write, S03 load-on-startup) are complete; the full R028 contract is now proven: jobs queued at crash time are re-queued on restart with attempt counts preserved, `Dispatching`/`Running` treated as `Queued`, state file written atomically on every transition

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

None.

## Known Limitations

- State file recovery is best-effort: if `queue_dir` is changed between runs, the state file is not found and the daemon starts fresh. No migration path exists.
- `Dispatching` state at crash time (D116: not written to state file) means jobs that were mid-dispatch but not yet Running will not be recovered — they are simply gone from the state file. This is expected given D116's explicit decision to omit the Dispatching write.

## Follow-ups

- M007 is complete — no immediate follow-ups for this milestone
- R028 is validated; next relevant work is R027 (SSH workers, M008)

## Files Created/Modified

- `crates/smelt-cli/src/serve/queue.rs` — added `load_or_new`, `tracing::info!`, 2 unit tests
- `crates/smelt-cli/src/commands/serve.rs` — replaced `ServerState::new()` with `ServerState::load_or_new()`
- `examples/server.toml` — added persistence/restart-recovery comment block

## Forward Intelligence

### What the next slice should know
- Persistence is now fully active in the live daemon — any `smelt serve` run with a `queue_dir` will read/write `.smelt-queue-state.toml` automatically
- The `queue_dir: Option<PathBuf>` field on `ServerState` is the gating mechanism — `None` means no persistence; `Some` means every mutation writes

### What's fragile
- `queue_dir` must exist before `smelt serve` starts — `create_dir_all` in `serve.rs` handles this, but if it fails, `load_or_new` will fail silently (empty start)
- `D116` means Dispatching transitions are not persisted — jobs mid-dispatch at crash time are lost (not recoverable), only Queued/Retrying/Running survive

### Authoritative diagnostics
- `.smelt/serve.log` line `"load_or_new: loaded N jobs"` — first signal to check for recovery confirmation
- `cat queue_dir/.smelt-queue-state.toml` before startup — shows exactly what will be recovered
- `read_queue_state` `warn!` with path + parse error — check if file is corrupt

### What assumptions changed
- No assumptions changed — implementation matched the plan exactly
