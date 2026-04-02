# S02: Atomic state file — write on every transition

**Goal:** Add `write_queue_state` / `read_queue_state` free functions to `queue.rs`, store `queue_dir` in `ServerState`, and call `write_queue_state` after every durable mutation (`enqueue`, `complete`, `cancel`) so that `queue_dir/.smelt-queue-state.toml` reflects the live queue state after every change.
**Demo:** A round-trip unit test serializes a `ServerState` with two jobs of different statuses, reads back via `read_queue_state`, and asserts every field is equal — proving the file can reconstruct a full job set. All 46 existing smelt-cli tests still pass.

## Must-Haves

- `write_queue_state(queue_dir: &Path, jobs: &VecDeque<QueuedJob>)` writes to `queue_dir/.smelt-queue-state.toml.tmp` then atomically renames to `queue_dir/.smelt-queue-state.toml`; calls `fs::create_dir_all` before writing
- `read_queue_state(queue_dir: &Path) -> Vec<QueuedJob>` returns empty vec on missing file; logs `warn!()` and returns empty vec on parse error; never panics
- `QueueState { jobs: Vec<QueuedJob> }` wrapper struct with `Serialize + Deserialize` used as the TOML top-level table
- `ServerState` gains `queue_dir: Option<PathBuf>`; new constructor `new_with_persistence(max_concurrent, queue_dir)` added alongside unchanged `new(max_concurrent)` (existing tests unaffected)
- `enqueue`, `complete`, and `cancel` call `write_queue_state` when `queue_dir` is `Some`; `try_dispatch` does not (Dispatching is transient)
- Unit test `test_queue_state_round_trip`: creates 2 jobs with distinct statuses, writes via `write_queue_state`, reads via `read_queue_state`, asserts all fields equal
- Unit test `test_read_queue_state_missing_file`: `read_queue_state` on non-existent directory returns empty vec without error
- Unit test `test_read_queue_state_corrupt_file`: `read_queue_state` on garbage TOML content returns empty vec
- `cargo test -p smelt-cli` — all 46 tests pass, zero new warnings

## Proof Level

- This slice proves: integration
- Real runtime required: no (unit tests with TempDir)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-cli` passes with all 46 tests green (no regressions)
- `cargo test -p smelt-cli -- queue` — `test_queue_state_round_trip`, `test_read_queue_state_missing_file`, `test_read_queue_state_corrupt_file` all pass
- `cargo check -p smelt-cli` exits 0 with zero warnings
- Manual spot-check: state file TOML contains `[[jobs]]` entries with correct field names (`id`, `manifest_path`, `source`, `attempt`, `status`, `queued_at`, `started_at`)

## Observability / Diagnostics

- Runtime signals: `warn!()` emitted on parse error in `read_queue_state` with path and error message; `warn!()` emitted on write failure in `write_queue_state` (I/O error, rename failure)
- Inspection surfaces: `cat queue_dir/.smelt-queue-state.toml` — human-readable TOML with one `[[jobs]]` block per queued job; `.tmp` file absent under normal operation (only exists during atomic write window)
- Failure visibility: write failures are non-fatal (daemon continues with stale file); last write error visible in `tracing` log (redirected to `.smelt/serve.log` in TUI mode per D107); no silent data loss — in-memory state remains authoritative
- Redaction constraints: `manifest_path` contains file paths (not secrets); no credentials are stored in the state file

## Integration Closure

- Upstream surfaces consumed: `QueuedJob` with `Serialize + Deserialize` from S01 (`types.rs`); `now_epoch()` already used in `queue.rs`; `toml` already a production dep (D062); `ServerConfig.queue_dir: PathBuf` as the base path
- New wiring introduced in this slice: `write_queue_state` called from `enqueue`/`complete`/`cancel` in `queue.rs`; `queue_dir: Option<PathBuf>` stored on `ServerState`; `new_with_persistence` constructor added
- What remains before the milestone is truly usable end-to-end: S03 must implement `ServerState::load_or_new()` and wire it into `commands/serve.rs`; integration test for restart-recovery; `try_dispatch`/`dispatch_loop` must use the reconstructed state on startup

## Tasks

- [x] **T01: Add `QueueState` wrapper + `write_queue_state` / `read_queue_state` with unit tests** `est:45m`
  - Why: Establishes the serialization boundary (S02→S03 contract); proves the round-trip before wiring it into `ServerState` mutations
  - Files: `crates/smelt-cli/src/serve/queue.rs`
  - Do: (1) Add `QueueState { jobs: Vec<QueuedJob> }` with `#[derive(Serialize, Deserialize)]` at the top of `queue.rs`; (2) Implement `pub fn write_queue_state(queue_dir: &Path, jobs: &VecDeque<QueuedJob>)` — serialize to `QueueState { jobs: jobs.iter().cloned().collect() }`, call `toml::to_string_pretty()`, call `fs::create_dir_all(queue_dir)`, write to `.smelt-queue-state.toml.tmp`, then `fs::rename` to `.smelt-queue-state.toml`; on any error, `warn!("write_queue_state failed: {err}")` and return; (3) Implement `pub fn read_queue_state(queue_dir: &Path) -> Vec<QueuedJob>` — return empty vec if file does not exist (check with `path.exists()`); read file, parse via `toml::from_str::<QueueState>()`, on parse error `warn!()` + return empty vec; on success return `state.jobs`; (4) Add three unit tests: `test_queue_state_round_trip` (2 jobs, write+read+assert), `test_read_queue_state_missing_file` (non-existent dir), `test_read_queue_state_corrupt_file` (write garbage bytes then read)
  - Verify: `cargo test -p smelt-cli -- queue` — all three new tests pass; `cargo check -p smelt-cli` zero warnings
  - Done when: `test_queue_state_round_trip` passes asserting `id`, `manifest_path`, `source`, `attempt`, `status`, `queued_at`, `started_at` equal after round-trip; two error-tolerance tests pass; no compile warnings

- [x] **T02: Add `queue_dir` to `ServerState` + wire writes in mutation methods** `est:30m`
  - Why: Makes write calls automatic on every durable state transition; S03's `load_or_new()` will use `new_with_persistence` internally
  - Files: `crates/smelt-cli/src/serve/queue.rs`
  - Do: (1) Add `queue_dir: Option<PathBuf>` field to `ServerState` struct; (2) Leave `new(max_concurrent)` unchanged (sets `queue_dir: None`) to preserve all existing test callsites; (3) Add `pub fn new_with_persistence(max_concurrent: usize, queue_dir: PathBuf) -> Self` that sets `queue_dir: Some(queue_dir)`; (4) In `enqueue`: after push_back, if `queue_dir` is `Some(dir)`, call `write_queue_state(dir, &self.jobs)`; (5) In `complete`: after status mutation, call `write_queue_state` if `Some`; (6) In `cancel`: after `jobs.remove(pos)`, call `write_queue_state` if `Some`; `try_dispatch` is NOT wired (Dispatching is transient — D116); (7) Run `cargo test -p smelt-cli` and verify all 46 pass
  - Verify: `cargo test -p smelt-cli` — 46 tests pass; add one integration-style test `test_server_state_writes_on_enqueue` that creates `new_with_persistence` with a TempDir, calls `enqueue`, then reads the state file and asserts the job appears
  - Done when: `cargo test -p smelt-cli` shows ≥47 tests pass (original 46 + new wiring test); state file created by `enqueue` in a TempDir; `cargo check -p smelt-cli` zero warnings

## Files Likely Touched

- `crates/smelt-cli/src/serve/queue.rs` — all changes: `QueueState`, two free functions, `queue_dir` field, `new_with_persistence`, wired writes
- `crates/smelt-cli/Cargo.toml` — confirm `toml` is production dep (already confirmed; no change needed)
