---
estimated_steps: 4
estimated_files: 1
---

# T01: Add `QueueState` wrapper + `write_queue_state` / `read_queue_state` with unit tests

**Slice:** S02 — Atomic state file — write on every transition
**Milestone:** M007

## Description

Establish the S02→S03 serialization contract by adding `QueueState` (the TOML wrapper struct), two free functions (`write_queue_state`, `read_queue_state`), and three unit tests that prove the round-trip works and the error-tolerance cases are handled. No mutation wiring yet — T02 handles that. This task delivers the contract artifact the boundary map specifies.

## Steps

1. In `crates/smelt-cli/src/serve/queue.rs`, add `use std::path::Path;` and `use tracing::warn;` to imports (check if already present; add only what's missing).
2. Add `QueueState` struct just above the `new_job_id` function:
   ```rust
   #[derive(serde::Serialize, serde::Deserialize)]
   struct QueueState {
       jobs: Vec<QueuedJob>,
   }
   ```
3. Implement `pub fn write_queue_state(queue_dir: &Path, jobs: &VecDeque<QueuedJob>)`:
   - Build `QueueState { jobs: jobs.iter().cloned().collect() }`
   - Serialize with `toml::to_string_pretty(&state)` — on error, `warn!` and return
   - Call `std::fs::create_dir_all(queue_dir)` — on error, `warn!` and return
   - Write to `queue_dir/.smelt-queue-state.toml.tmp` via `std::fs::write` — on error, `warn!` and return
   - Rename `.tmp` to `queue_dir/.smelt-queue-state.toml` via `std::fs::rename` — on error, `warn!` and return
4. Implement `pub fn read_queue_state(queue_dir: &Path) -> Vec<QueuedJob>`:
   - Construct target path `queue_dir/.smelt-queue-state.toml`
   - If `!path.exists()`, return `vec![]`
   - Read file contents; on error, `warn!` and return `vec![]`
   - Parse via `toml::from_str::<QueueState>(&content)`; on error, `warn!` and return `vec![]`
   - Return `state.jobs`
5. Add three unit tests inside the existing `#[cfg(test)]` block (or create one if absent):
   - `test_queue_state_round_trip`: use `tempfile::TempDir`; build two `QueuedJob` values with distinct `id`, `manifest_path`, `status`, `attempt`, `queued_at`, `started_at`; call `write_queue_state`; call `read_queue_state`; assert `len() == 2` and compare fields pairwise
   - `test_read_queue_state_missing_file`: call `read_queue_state` on a `TempDir` path (no file written); assert result is empty
   - `test_read_queue_state_corrupt_file`: write `b"not toml at all!!!"` to `queue_dir/.smelt-queue-state.toml`; call `read_queue_state`; assert result is empty

## Must-Haves

- [ ] `write_queue_state` writes TOML wrapped in `[[jobs]]` blocks (top-level `QueueState` table)
- [ ] `write_queue_state` uses `create_dir_all` before writing — first-run with empty `queue_dir` does not panic or error
- [ ] `write_queue_state` writes to `.tmp` then renames (atomic) — never writes directly to the target path
- [ ] `write_queue_state` uses `warn!()` on every failure path and returns without propagating the error
- [ ] `read_queue_state` returns empty vec on missing file (no `unwrap`, no panic)
- [ ] `read_queue_state` returns empty vec on parse error (no `unwrap`, no panic)
- [ ] `test_queue_state_round_trip` asserts all 7 fields (`id`, `manifest_path`, `source`, `attempt`, `status`, `queued_at`, `started_at`) for both jobs
- [ ] `test_read_queue_state_missing_file` passes
- [ ] `test_read_queue_state_corrupt_file` passes
- [ ] `cargo test -p smelt-cli -- queue` — all three new tests pass
- [ ] `cargo check -p smelt-cli` — zero warnings

## Verification

- `cargo test -p smelt-cli -- queue` passes with the three new tests visible by name
- `cargo check -p smelt-cli` exits 0 with zero warnings
- Spot-check: in `test_queue_state_round_trip`, assert `read_back[0].id == jobs[0].id` and `read_back[1].status == jobs[1].status`

## Observability Impact

- Signals added/changed: `warn!()` logs for write failures and parse errors in `queue.rs` — visible in `.smelt/serve.log` (TUI mode) or stderr (non-TUI mode)
- How a future agent inspects this: `cat queue_dir/.smelt-queue-state.toml` shows human-readable TOML with one `[[jobs]]` stanza per job; absence of `.tmp` file means write completed atomically
- Failure state exposed: any partial write leaves `.tmp` file behind (not `.smelt-queue-state.toml`), so a failed write is distinguishable from a successful one by checking which file exists

## Inputs

- `crates/smelt-cli/src/serve/types.rs` — `QueuedJob`, `JobId`, `JobSource`, `JobStatus` with `Serialize + Deserialize` (from S01)
- `crates/smelt-cli/src/serve/queue.rs` — existing `ServerState` and `VecDeque<QueuedJob>`
- `crates/smelt-cli/Cargo.toml` — `toml` is already a production dep (confirmed by S02 research)

## Expected Output

- `crates/smelt-cli/src/serve/queue.rs` — gains `QueueState` struct, `write_queue_state`, `read_queue_state`, and three unit tests
- `cargo test -p smelt-cli -- queue` shows at minimum: `test_queue_state_round_trip ... ok`, `test_read_queue_state_missing_file ... ok`, `test_read_queue_state_corrupt_file ... ok`
