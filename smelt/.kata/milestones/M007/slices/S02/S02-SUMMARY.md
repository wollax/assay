---
id: S02
parent: M007
milestone: M007
provides:
  - "`QueueState { jobs: Vec<QueuedJob> }` — TOML-compatible top-level wrapper struct with Serialize + Deserialize"
  - "`pub fn write_queue_state(queue_dir: &Path, jobs: &VecDeque<QueuedJob>)` — atomic write via .tmp rename, warn-on-failure"
  - "`pub fn read_queue_state(queue_dir: &Path) -> Vec<QueuedJob>` — tolerant reader: empty vec on missing/corrupt file"
  - "`ServerState::new_with_persistence(max_concurrent, queue_dir)` — opt-in persistence constructor"
  - "`ServerState.queue_dir: Option<PathBuf>` — controls whether mutations trigger writes"
  - "`enqueue`, `complete`, `cancel` each atomically write state file when `queue_dir` is `Some`"
  - "4 new unit tests covering round-trip, missing file, corrupt file, and wiring on enqueue"
requires:
  - slice: S01
    provides: "`QueuedJob` with `Serialize + Deserialize`; `u64` Unix epoch fields for serialization"
affects:
  - S03
key_files:
  - crates/smelt-cli/src/serve/queue.rs
key_decisions:
  - "D114: QueueState wrapper in queue.rs (not a new file) — TOML needs a top-level table; maps cleanly to [[jobs]] array-of-tables"
  - "D115: new() unchanged with queue_dir: None; new_with_persistence() added alongside — preserves all 46 existing test callsites"
  - "D116: try_dispatch does NOT write state — Dispatching is transient; only durable mutations (enqueue/complete/cancel) write"
  - "T01: JobSource lacks PartialEq; round-trip test uses format!(\"{:?}\") comparison — no production impact"
  - "T02: cancel refactored to local-bool form to avoid borrow conflict between jobs mutation and queue_dir borrow"
patterns_established:
  - "Persistence opt-in via new_with_persistence; zero changes required to existing callers"
  - "write_queue_state: warn!+return on every failure path; never propagates errors; atomic via .tmp rename"
  - "read_queue_state: exists-check first, then warn!+vec![] on any I/O or parse failure"
  - "`if let Some(ref dir) = self.queue_dir { write_queue_state(dir, &self.jobs); }` at end of each mutation"
observability_surfaces:
  - "warn!() on write failure (serialize/create_dir_all/write .tmp/rename) — visible in .smelt/serve.log (TUI) or stderr"
  - "warn!() on read failure (I/O or TOML parse) with full path"
  - "`cat queue_dir/.smelt-queue-state.toml` — human-readable TOML with [[jobs]] stanzas after every enqueue/complete/cancel"
  - "Absence of .smelt-queue-state.toml.tmp after write indicates atomic completion; its presence indicates failed rename"
drill_down_paths:
  - .kata/milestones/M007/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M007/slices/S02/tasks/T02-SUMMARY.md
duration: 30m
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
---

# S02: Atomic state file — write on every transition

**Atomic TOML queue persistence added to `queue.rs`: `write_queue_state`/`read_queue_state` free functions + opt-in `new_with_persistence` constructor; every durable mutation writes `.smelt-queue-state.toml` atomically; 50 tests, 0 failures.**

## What Happened

**T01** established the serialization boundary. Added `QueueState { jobs: Vec<QueuedJob> }` as the TOML top-level wrapper (needed because `Vec<QueuedJob>` alone can't be a TOML root). Implemented `write_queue_state` using the `.tmp` + rename pattern for atomicity, with `warn!+return` on every failure path. Implemented `read_queue_state` with existence check first, then tolerant parse (warn + empty vec on any error). Added three unit tests: round-trip across all 7 fields, missing directory, and corrupt file. All pass.

**T02** wired the serialization layer into `ServerState`. Added `queue_dir: Option<PathBuf>` field and `new_with_persistence(max_concurrent, queue_dir)` constructor alongside the unchanged `new()`. Wired `write_queue_state` into `enqueue`, `complete`, and `cancel` using the `if let Some(ref dir) = self.queue_dir` pattern at the end of each method. `try_dispatch` is intentionally unwired (D116). Added `test_server_state_writes_on_enqueue` to verify the file appears after enqueue and round-trips correctly. `cancel` was lightly refactored to a local-bool form to avoid a Rust borrow conflict; behavior is identical.

## Verification

- `cargo test -p smelt-cli` — **50 passed, 0 failed** (up from 46 before S02)
- `cargo test -p smelt-cli -- queue` — **11 passed** (4 new + 7 existing queue tests)
- `cargo check -p smelt-cli` — exits 0, **zero warnings**
- All 19 original serve tests pass unchanged
- State file TOML manually verified to contain `[[jobs]]` entries with correct field names

## Requirements Advanced

- R028 — S02 delivers the atomic write half of persistent queue: `write_queue_state`/`read_queue_state` are the serialization contract that S03 will consume to implement `load_or_new()` and close R028.

## Requirements Validated

- None validated by this slice alone — R028 remains Active until S03 proves end-to-end restart-recovery.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

- `cancel` refactored to local-bool form (vs early-return form in plan) to satisfy Rust's borrow checker — behavior identical, no plan impact.
- `JobSource` lacks `PartialEq`; round-trip test uses `format!("{:?}")` comparison instead of direct `assert_eq!` on the enum — minor test technique adaptation, no production impact.

## Known Limitations

- `new_with_persistence` is implemented but not yet called from `commands/serve.rs` — the daemon still starts without persistence until S03 wires `load_or_new()`.
- `read_queue_state` is implemented and tested but not yet called on startup — R028 is only half-satisfied until S03.

## Follow-ups

- S03: implement `ServerState::load_or_new(queue_dir, max_concurrent)` using `read_queue_state` + `new_with_persistence`; call it from `commands/serve.rs`; add integration test for enqueue → serialize → drop → reconstruct → dispatch cycle.

## Files Created/Modified

- `crates/smelt-cli/src/serve/queue.rs` — `QueueState` struct, `write_queue_state`, `read_queue_state`, `queue_dir` field on `ServerState`, `new_with_persistence`, wired writes in `enqueue`/`complete`/`cancel`, 4 unit tests

## Forward Intelligence

### What the next slice should know
- `new_with_persistence(max_concurrent, queue_dir)` is the constructor S03's `load_or_new` should delegate to after reading the state file.
- `read_queue_state` returns `Vec<QueuedJob>` not `VecDeque` — S03 will need to convert or push into a fresh `VecDeque`.
- Jobs in `Dispatching` or `Running` status at crash time will appear in the state file with those statuses — S03 must remap them to `Queued` during `load_or_new`.
- `attempt` count is preserved in the TOML round-trip — S03 can use the loaded value directly.

### What's fragile
- `read_queue_state` silently swallows corrupt TOML (warn + empty) — if the state file is partially written (power failure during rename), the backup `.tmp` file won't be read, so jobs could be lost. This is acceptable for M007's scope but worth noting for operational documentation.
- The `write_queue_state` call at the end of `cancel` uses a local bool to avoid a borrow conflict — if `cancel`'s mutation logic is ever refactored, the borrow pattern should be re-verified.

### Authoritative diagnostics
- `cat queue_dir/.smelt-queue-state.toml` — ground truth for what was last successfully persisted
- `tracing` warn logs — first signal of write/read failures; visible in `.smelt/serve.log` (TUI mode)
- Presence of `.smelt-queue-state.toml.tmp` without `.smelt-queue-state.toml` — indicates a failed rename during atomic write

### What assumptions changed
- Plan assumed 46 baseline tests; actual count at start of S02 was already 46, and S02 raised it to 50 — confirms plan was accurate.
