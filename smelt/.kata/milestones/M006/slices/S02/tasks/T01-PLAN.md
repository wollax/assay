---
estimated_steps: 6
estimated_files: 5
---

# T01: Core types, JobQueue, ServerState, and unit tests

**Slice:** S02 — Directory Watch + HTTP API
**Milestone:** M006

## Description

S01 was never implemented — the `serve/` module does not exist in the codebase. This task creates all foundational types and the `JobQueue` struct that every subsequent task depends on. It also creates the test file (`serve/tests.rs`) with failing/skeleton tests that T02–T04 will fill in. The unit tests in this task cover the queue state machine itself (no Docker, no async I/O required).

## Steps

1. Create `crates/smelt-cli/src/serve/` directory. Add `mod.rs` that declares submodules: `pub mod types; pub mod queue; pub(crate) mod dispatch; pub(crate) mod queue_watcher; pub(crate) mod http_api; #[cfg(test)] mod tests;`
2. Write `types.rs`: `JobId(String)` newtype with `Display`; `JobSource { DirectoryWatch, HttpApi }` with Serialize+Clone+Debug; `JobStatus { Queued, Dispatching, Running, Retrying, Complete, Failed }` with Serialize+Clone+Debug+PartialEq; `QueuedJob { id: JobId, manifest_path: PathBuf, source: JobSource, attempt: u32, status: JobStatus, queued_at: std::time::Instant, started_at: Option<std::time::Instant> }` with Clone.
3. Write `queue.rs`: `ServerState { jobs: VecDeque<QueuedJob>, running_count: usize, max_concurrent: usize }` with `new(max_concurrent)`. Implement methods: `enqueue(manifest_path, source) -> JobId` (pushes Queued job, returns id); `try_dispatch() -> Option<QueuedJob>` (if running_count < max_concurrent AND Queued job exists: set status=Dispatching, running_count++, return Some); `complete(id, success: bool, attempt: u32, max_attempts: u32)` (if !success && attempt < max_attempts: set Retrying, re-enqueue; else: set Complete/Failed, running_count--); `cancel(id) -> bool` (if Queued: remove + true; else false); `retry_eligible(id) -> bool` (job.attempt < max_attempts AND status == Retrying).
4. Write `serve/tests.rs` skeleton with 4 queue unit tests using `ServerState` directly (no tokio, no Docker): `test_queue_fifo_order`, `test_queue_max_concurrent`, `test_queue_cancel_queued`, `test_queue_retry_eligible`. Add `#[ignore]` placeholder skeletons for T02–T04 integration tests (8 stubs) so the file exists.
5. Add `pub mod serve;` to `crates/smelt-cli/src/lib.rs`.
6. Run `cargo test -p smelt-cli serve::queue -- --nocapture` and fix any compilation errors until all 4 queue unit tests pass.

## Must-Haves

- [ ] `JobId`, `JobStatus`, `JobSource`, `QueuedJob`, `ServerState` types compile with `#[derive(Serialize, Clone, Debug)]` where noted
- [ ] `JobQueue::enqueue` appends a `Queued` job and returns its `JobId`
- [ ] `JobQueue::try_dispatch` respects `max_concurrent` cap — returns None when running_count == max_concurrent
- [ ] `JobQueue::cancel` returns true and removes a Queued job; returns false for a Running/Dispatching job
- [ ] `JobQueue::retry_eligible` returns true when attempt < max_attempts and status is Retrying
- [ ] `test_queue_fifo_order`, `test_queue_max_concurrent`, `test_queue_cancel_queued`, `test_queue_retry_eligible` all pass
- [ ] `cargo build -p smelt-cli` compiles clean (0 errors, 0 new warnings)

## Verification

- `cargo test -p smelt-cli serve::queue -- --nocapture` → 4 tests pass
- `cargo build -p smelt-cli` → no errors

## Observability Impact

- Signals added/changed: None yet — T02 adds tracing to dispatch transitions
- How a future agent inspects this: ServerState is the in-memory inspection surface; T04 exposes it via HTTP API
- Failure state exposed: JobStatus enum covers all failure states; a future agent reading the queue can see stuck/failed jobs by status

## Inputs

- `crates/smelt-cli/src/lib.rs` — needs `pub mod serve;` added
- `crates/smelt-cli/Cargo.toml` — already has `serde` via smelt-core; may need `serde = { workspace = true }` added directly for `#[derive(Serialize)]` on types in smelt-cli

## Expected Output

- `crates/smelt-cli/src/serve/mod.rs` — module declarations
- `crates/smelt-cli/src/serve/types.rs` — all core types
- `crates/smelt-cli/src/serve/queue.rs` — `ServerState` with all queue methods
- `crates/smelt-cli/src/serve/tests.rs` — 4 passing unit tests + 8 ignored stubs
- `crates/smelt-cli/src/lib.rs` — `pub mod serve;` added
