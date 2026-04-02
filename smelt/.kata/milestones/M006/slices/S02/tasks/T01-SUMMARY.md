---
id: T01
parent: S02
milestone: M006
provides:
  - JobId (String newtype with Display)
  - JobSource enum (DirectoryWatch | HttpApi, Serialize+Clone+Debug)
  - JobStatus enum (Queued/Dispatching/Running/Retrying/Complete/Failed, Serialize+Clone+Debug+PartialEq)
  - QueuedJob struct (id, manifest_path, source, attempt, status, queued_at, started_at)
  - ServerState struct with enqueue/try_dispatch/complete/cancel/retry_eligible methods
  - 4 passing queue unit tests + 10 ignored stubs for T02-T04
  - serve/ module wired into smelt-cli lib.rs
key_files:
  - crates/smelt-cli/src/serve/mod.rs
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/queue.rs
  - crates/smelt-cli/src/serve/tests.rs
  - crates/smelt-cli/src/serve/dispatch.rs
  - crates/smelt-cli/src/serve/queue_watcher.rs
  - crates/smelt-cli/src/serve/http_api.rs
  - crates/smelt-cli/src/lib.rs
key_decisions:
  - "Retrying stays in-place in VecDeque: complete() sets status=Retrying in-place (not re-enqueue); try_dispatch() picks up Queued OR Retrying jobs — cleaner retry semantics than re-enqueuing a new entry"
  - "JobId uses atomic u64 counter for test determinism (job-1, job-2...) rather than UUID in this foundational layer"
  - "retry_eligible takes max_attempts as parameter rather than storing it on ServerState — allows per-call-site policy"
patterns_established:
  - "serve/ module structure: mod.rs declares submodules, each concern in its own file"
  - "ServerState::complete() releases running_count slot and sets Retrying in-place; try_dispatch() dispatches both Queued and Retrying jobs"
observability_surfaces:
  - "JobStatus enum covers all runtime states — a future agent can inspect stuck/failed jobs by status field"
duration: 1.5h
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T01: Core types, JobQueue, ServerState, and unit tests

**`serve/` module foundation: JobId, JobStatus, JobSource, QueuedJob, ServerState with full queue state machine — all 4 unit tests pass, build clean.**

## What Happened

Created the entire `crates/smelt-cli/src/serve/` module from scratch (S01 was never implemented). The module structure was established with `mod.rs` declaring all submodules (`types`, `queue`, `dispatch`, `queue_watcher`, `http_api`, `tests`). Skeleton files were written for the T02-T04 components (`dispatch.rs`, `queue_watcher.rs`, `http_api.rs`) so the module compiles without errors.

The key design decision was in `complete()` vs `retry` semantics: the initial approach re-enqueued a fresh Queued entry for retries, which created two entries (one Retrying, one Queued). The final design keeps a single entry in the VecDeque and sets `status = Retrying` in-place. `try_dispatch()` dispatches both `Queued` and `Retrying` jobs. This avoids duplicate entries and makes `retry_eligible()` straightforward: find the single entry with `status == Retrying && attempt < max_attempts`.

`serde = { workspace = true }` was added directly to `crates/smelt-cli/Cargo.toml` since `smelt-cli` uses `#[derive(Serialize)]` on its own types.

## Verification

```
cargo test -p smelt-cli serve::queue -- --nocapture
  test serve::tests::test_queue_cancel_queued ... ok
  test serve::tests::test_queue_fifo_order ... ok
  test serve::tests::test_queue_max_concurrent ... ok
  test serve::tests::test_queue_retry_eligible ... ok

cargo build -p smelt-cli
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
```

All 4 must-have unit tests pass. Build is clean with 0 errors.

## Diagnostics

`JobStatus` enum is the primary inspection surface — a future agent reading `ServerState.jobs` can see every job's current state (Queued/Dispatching/Running/Retrying/Complete/Failed). T04 exposes this via `GET /api/v1/jobs`. The `attempt` field on `QueuedJob` tracks retry count for diagnosing retry exhaustion.

## Deviations

- Retry semantics: plan said "re-enqueue a fresh Queued copy" on failure. Instead, `complete()` sets `Retrying` in-place and `try_dispatch()` picks up both Queued and Retrying jobs. This avoids duplicate VecDeque entries and is simpler to reason about.
- Skeleton files for T02-T04 (`dispatch.rs`, `queue_watcher.rs`, `http_api.rs`) contain minimal placeholder structs/types to allow the module to compile. T02-T04 will fill in the real implementations.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/mod.rs` — module declarations for all serve submodules
- `crates/smelt-cli/src/serve/types.rs` — JobId, JobSource, JobStatus, QueuedJob types
- `crates/smelt-cli/src/serve/queue.rs` — ServerState with full queue state machine
- `crates/smelt-cli/src/serve/tests.rs` — 4 passing unit tests + 10 ignored T02-T04 stubs
- `crates/smelt-cli/src/serve/dispatch.rs` — skeleton (T02 fills in)
- `crates/smelt-cli/src/serve/queue_watcher.rs` — skeleton (T03 fills in)
- `crates/smelt-cli/src/serve/http_api.rs` — skeleton (T04 fills in)
- `crates/smelt-cli/src/lib.rs` — added `pub mod serve;`
- `crates/smelt-cli/Cargo.toml` — added `serde = { workspace = true }`
