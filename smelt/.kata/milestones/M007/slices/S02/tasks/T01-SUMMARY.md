---
id: T01
parent: S02
milestone: M007
provides:
  - "`QueueState { jobs: Vec<QueuedJob> }` struct with Serialize + Deserialize in queue.rs"
  - "`pub fn write_queue_state(queue_dir: &Path, jobs: &VecDeque<QueuedJob>)` — atomic TOML write via .tmp rename"
  - "`pub fn read_queue_state(queue_dir: &Path) -> Vec<QueuedJob>` — tolerant TOML reader (empty on missing/corrupt)"
  - "Three unit tests: round-trip, missing file, corrupt file — all pass"
  - "S02→S03 serialization contract established and verified"
key_files:
  - crates/smelt-cli/src/serve/queue.rs
key_decisions:
  - "JobSource lacks PartialEq; source comparison in round-trip test uses format!(\"{:?}\") rather than PartialEq — no derive change needed since Debug is already derived"
patterns_established:
  - "write_queue_state: warn!+return on every failure path; never propagates errors; atomic via .tmp rename"
  - "read_queue_state: exists-check first, then warn!+vec![] on any I/O or parse failure"
observability_surfaces:
  - "warn!() logs on write failure (serialize, create_dir_all, write .tmp, rename) — visible in .smelt/serve.log (TUI) or stderr"
  - "warn!() logs on read failure (I/O or TOML parse) with full path"
  - "Absence of .smelt-queue-state.toml.tmp after write indicates atomic completion"
duration: 20m
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T01: Add `QueueState` wrapper + `write_queue_state` / `read_queue_state` with unit tests

**TOML round-trip serialization layer for queue state added to `queue.rs`: atomic writes via `.tmp` rename, tolerant reads, three passing unit tests.**

## What Happened

Added three items to `crates/smelt-cli/src/serve/queue.rs`:

1. **`QueueState` struct** — minimal wrapper `{ jobs: Vec<QueuedJob> }` with `serde::Serialize + Deserialize`. Placed above `new_job_id`. Produces `[[jobs]]` TOML blocks when serialized with `toml::to_string_pretty`.

2. **`write_queue_state(queue_dir: &Path, jobs: &VecDeque<QueuedJob>)`** — serializes to `QueueState`, calls `create_dir_all`, writes to `.smelt-queue-state.toml.tmp`, then renames to `.smelt-queue-state.toml`. Every failure path emits a `warn!()` and returns without propagating the error.

3. **`read_queue_state(queue_dir: &Path) -> Vec<QueuedJob>`** — checks file existence first (returns `vec![]` if absent), reads and parses TOML; on any I/O or parse error emits `warn!()` and returns `vec![]`.

Added `use std::path::Path;` and `use tracing::warn;` imports (neither was previously present).

Added a `#[cfg(test)] mod tests` block (none existed in queue.rs) with three tests:
- `test_queue_state_round_trip` — builds two `QueuedJob` values with distinct statuses/fields, write+read+assert all 7 fields for both jobs
- `test_read_queue_state_missing_file` — fresh TempDir, no write, asserts empty result
- `test_read_queue_state_corrupt_file` — writes `b"not toml at all!!!"` directly, asserts empty result

## Verification

```
cargo test -p smelt-cli -- queue
```
Output: 10 tests pass (3 new + 7 existing queue tests), 0 failed.

```
cargo check -p smelt-cli
```
Output: Finished with 0 warnings from our code (pre-existing deprecation warnings in test files for `assert_cmd::cargo_bin` are unrelated to this task).

## Diagnostics

- `cat queue_dir/.smelt-queue-state.toml` shows TOML with `[[jobs]]` stanzas — each entry has `id`, `manifest_path`, `source`, `attempt`, `status`, `queued_at`, `started_at`
- Failed atomic write leaves `.smelt-queue-state.toml.tmp` but not the target file — distinguishable by inspection
- All warnings routed through `tracing::warn!` — visible in `.smelt/serve.log` (TUI mode) or stderr

## Deviations

`JobSource` does not derive `PartialEq`, so source comparison in `test_queue_state_round_trip` uses `format!("{:?}", ...)` equality instead of direct `assert_eq!` on the enum. This is a minor test technique adaptation; no plan change and no production code impact.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/queue.rs` — added `use std::path::Path`, `use tracing::warn`, `QueueState` struct, `write_queue_state`, `read_queue_state`, and `#[cfg(test)] mod tests` with three unit tests
