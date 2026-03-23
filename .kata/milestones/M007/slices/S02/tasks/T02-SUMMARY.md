---
id: T02
parent: S02
milestone: M007
provides:
  - "`ServerState` gains `queue_dir: Option<PathBuf>` field"
  - "`ServerState::new(n)` unchanged — sets `queue_dir: None`"
  - "`ServerState::new_with_persistence(max_concurrent, queue_dir)` constructor added"
  - "`enqueue`, `complete`, `cancel` each call `write_queue_state` when `queue_dir` is `Some`"
  - "`try_dispatch` does NOT write state (dispatching is transient — D116)"
  - "`test_server_state_writes_on_enqueue` verifies file exists after enqueue + read-back correctness"
key_files:
  - crates/smelt-cli/src/serve/queue.rs
key_decisions:
  - "cancel refactored to track result via a local bool before calling write_queue_state — avoids borrow conflict between jobs mutation and queue_dir borrow (same self)"
patterns_established:
  - "persistence opt-in via new_with_persistence; existing new() callers require zero changes"
  - "write_queue_state call pattern: `if let Some(ref dir) = self.queue_dir { write_queue_state(dir, &self.jobs); }` at end of mutation"
observability_surfaces:
  - "warn!() on I/O failure in write_queue_state — visible in .smelt/serve.log (TUI mode) or stderr"
  - "`cat queue_dir/.smelt-queue-state.toml` reflects live queue after every enqueue/complete/cancel"
  - "absence of state file after first enqueue indicates daemon started without persistence (new()) or write failed"
duration: 10min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T02: Add `queue_dir` to `ServerState` + wire writes in mutation methods

**`ServerState` gains opt-in persistence via `new_with_persistence()`; `enqueue`/`complete`/`cancel` atomically write `.smelt-queue-state.toml` to disk after every durable state change.**

## What Happened

Added `queue_dir: Option<PathBuf>` to `ServerState` at the end of the field list. Updated `new()` to set `queue_dir: None` — all 46 existing test callsites compile unchanged. Added `new_with_persistence(max_concurrent, queue_dir)` that sets `queue_dir: Some(...)`.

Wired `write_queue_state` into `enqueue`, `complete`, and `cancel` using the `if let Some(ref dir) = self.queue_dir` pattern at the end of each method. `try_dispatch` is intentionally unwired (D116 — dispatching is a transient status change, not a durable queue mutation).

The `cancel` method was lightly refactored to separate the mutation from the write: a local `cancelled: bool` is captured before the write call, avoiding a borrow conflict where both `self.jobs` (mutably borrowed in `remove`) and `self.queue_dir` (immutably borrowed) are needed on the same `self`.

Added `test_server_state_writes_on_enqueue` which creates a `TempDir`, calls `new_with_persistence`, calls `enqueue`, asserts the state file exists, then calls `read_queue_state` and asserts 1 job with `status == Queued`.

## Verification

- `cargo test -p smelt-cli` (unit tests only): **50 passed, 0 failed** (≥47 target met)
- `cargo test -p smelt-cli -- queue::tests`: **4 passed** — all queue tests including the new wiring test
- `cargo check -p smelt-cli`: exits 0 with **zero warnings**
- All pre-existing serve tests (`test_enqueue_and_dispatch`, `test_complete_failure_retries`, `test_queue_cancel_queued`, etc.) appear in output unchanged

## Diagnostics

- `cat queue_dir/.smelt-queue-state.toml` — TOML with `[[jobs]]` stanzas reflecting current queue after every enqueue/complete/cancel
- `.smelt-queue-state.toml.tmp` — only present during atomic write window; its presence without the final file indicates a failed rename
- Write failures logged via `tracing::warn!` — visible in `.smelt/serve.log` (TUI mode) or stderr
- `queue_dir: None` on a `ServerState` created via `new()` means no file is ever written — distinguishable by inspecting the struct or checking for the state file's absence

## Deviations

`cancel` was refactored slightly from the plan's original early-return form into a local-bool form to avoid a Rust borrow conflict. Behavior is identical.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/queue.rs` — `ServerState` gains `queue_dir` field; `new()` sets `None`; `new_with_persistence()` added; `enqueue`/`complete`/`cancel` wired; `test_server_state_writes_on_enqueue` added
