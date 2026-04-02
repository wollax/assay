# S03: Load-on-startup + restart-recovery integration test

**Goal:** Implement `ServerState::load_or_new(queue_dir, max_concurrent)` in `queue.rs`, wire it into `commands/serve.rs`, and prove end-to-end restart-recovery with an integration test — completing R028.
**Demo:** A unit test simulates a crash by writing a state file with 3 queued jobs (two `Queued`, one `Running`), dropping the original `ServerState`, reconstructing via `load_or_new()`, and asserting all 3 jobs are present as `Queued` with their original `attempt` counts intact. `commands/serve.rs` calls `load_or_new()` instead of `new()`. All 50 existing tests plus 2 new tests pass. `cargo test --workspace` is all green.

## Must-Haves

- `pub fn ServerState::load_or_new(queue_dir: PathBuf, max_concurrent: usize) -> Self` in `queue.rs`: reads state file via `read_queue_state`, remaps `Dispatching` and `Running` → `Queued` (D109), preserves `attempt`, pushes into a fresh `VecDeque`, delegates to `new_with_persistence(max_concurrent, queue_dir)` to set `queue_dir: Some`
- `load_or_new` on missing/empty file behaves identically to `new_with_persistence` (fresh empty queue, no panic, no warn)
- `load_or_new` on corrupt file logs `warn!` (via `read_queue_state`) and returns an empty queue — daemon starts cleanly
- `commands/serve.rs` calls `ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)` instead of `ServerState::new(config.max_concurrent)`
- Unit test `test_load_or_new_restart_recovery`: write state file with 3 jobs (statuses: Queued, Running, Queued; attempts: 0, 2, 1) via `write_queue_state`; call `load_or_new`; assert 3 jobs present, all status `Queued`, attempts unchanged (0, 2, 1)
- Unit test `test_load_or_new_missing_file`: `load_or_new` on fresh TempDir returns empty queue, `queue_dir` is `Some`
- `cargo test -p smelt-cli` — all 52 tests pass (50 existing + 2 new), zero new warnings
- `examples/server.toml` gains a `# queue_dir` comment noting that persistence is automatic — state is written after each enqueue/complete/cancel and loaded on restart

## Proof Level

- This slice proves: integration
- Real runtime required: no (unit tests with TempDir; `commands/serve.rs` change is verified by `cargo check`)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-cli -- queue` — `test_load_or_new_restart_recovery`, `test_load_or_new_missing_file` both pass; all existing queue tests still pass
- `cargo test -p smelt-cli` — 52 tests pass, 0 failed
- `cargo check -p smelt-cli` — exits 0 with zero warnings
- `grep "load_or_new" crates/smelt-cli/src/commands/serve.rs` — confirms wiring is present
- Manual spot-check: state file written before `load_or_new`, then read back — `cat queue_dir/.smelt-queue-state.toml` shows 3 jobs; reconstructed state shows all 3 as `Queued`

## Observability / Diagnostics

- Runtime signals: `read_queue_state` (called inside `load_or_new`) already emits `warn!` on parse failure with full path and error — this propagates through `load_or_new` automatically; new `tracing::info!` on successful load: `"load_or_new: loaded N jobs from {queue_dir}, M remapped to Queued"` to distinguish cold start from recovery
- Inspection surfaces: `cat queue_dir/.smelt-queue-state.toml` — shows the persisted jobs before startup; `.smelt/serve.log` (TUI mode) or stderr — shows the info/warn messages on startup
- Failure visibility: corrupt state file → `warn!` logged with path + parse error → daemon starts with empty queue (non-fatal); missing file → silent empty start (normal first-run path); `queue_dir: Some(...)` on the returned `ServerState` ensures all subsequent mutations continue writing
- Redaction constraints: `manifest_path` contains file paths (not secrets); no credentials stored

## Integration Closure

- Upstream surfaces consumed: `read_queue_state(queue_dir)` and `new_with_persistence(max_concurrent, queue_dir)` from S02 (`queue.rs`); `ServerConfig.queue_dir: PathBuf` from `serve/config.rs`; `JobStatus` variants from `types.rs`
- New wiring introduced in this slice: `commands/serve.rs` replaces `ServerState::new()` with `ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)` — this is the only caller change needed to activate persistence for the live daemon
- What remains before the milestone is truly usable end-to-end: nothing — this is the final slice; after S03, `smelt serve` has full persistence: writes on every mutation (S01+S02) and loads on startup (S03)

## Tasks

- [x] **T01: Implement `ServerState::load_or_new` with restart-recovery tests** `est:30m`
  - Why: Provides the missing half of R028 — reads the state file, remaps in-flight statuses to `Queued`, and re-hydrates the queue so previously-queued jobs survive restarts
  - Files: `crates/smelt-cli/src/serve/queue.rs`
  - Do: (1) Add `pub fn load_or_new(queue_dir: PathBuf, max_concurrent: usize) -> Self` to `ServerState` impl block — call `read_queue_state(&queue_dir)`, remap any job with status `Dispatching` or `Running` to `Queued` (mutate the returned `Vec` in place), collect into `VecDeque`, call `new_with_persistence(max_concurrent, queue_dir)` to get the base state, then set `jobs` field; (2) Add `tracing::info!("load_or_new: loaded {n} jobs from {}, {remapped} remapped to Queued", queue_dir.display())` after remapping; (3) Add `test_load_or_new_restart_recovery` — create TempDir, write state file with 3 `QueuedJob` values (statuses: Queued/attempt=0, Running/attempt=2, Queued/attempt=1) via `write_queue_state`, call `load_or_new`, assert `jobs.len()==3`, all `status==Queued`, attempts are 0/2/1 in order; (4) Add `test_load_or_new_missing_file` — fresh TempDir, no write, call `load_or_new`, assert `jobs.is_empty()`, `queue_dir.is_some()`
  - Verify: `cargo test -p smelt-cli -- queue` — 13 tests pass (11 existing + 2 new); `cargo check -p smelt-cli` zero warnings
  - Done when: both new tests pass; `load_or_new` is visible in `pub` API; `Dispatching`/`Running` jobs in the state file emerge as `Queued` after reconstruction

- [x] **T02: Wire `load_or_new` into `serve.rs` + update `examples/server.toml`** `est:15m`
  - Why: Activates persistence for the live daemon — without this wiring change the implementation exists but is never called at startup; also closes the boundary map (S03 final-assembly wiring)
  - Files: `crates/smelt-cli/src/commands/serve.rs`, `examples/server.toml`
  - Do: (1) In `commands/serve.rs::execute()`, replace `ServerState::new(config.max_concurrent)` with `ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)`; no other changes to `serve.rs` needed; (2) Add `use crate::serve::ServerState` if not already in scope (check existing imports — it may already be imported via `use crate::serve::{..., ServerState, ...}`); (3) In `examples/server.toml`, add a comment block above `queue_dir` explaining persistence behavior: queue state is written after every enqueue/complete/cancel and automatically loaded on restart — jobs queued at crash time are re-dispatched automatically
  - Verify: `cargo check -p smelt-cli` exits 0, zero warnings; `grep "load_or_new" crates/smelt-cli/src/commands/serve.rs` prints the wiring line; `cargo test -p smelt-cli` — all 52 tests pass (no regressions from the `serve.rs` change)
  - Done when: `serve.rs` no longer calls `ServerState::new()`; `cargo test -p smelt-cli` shows 52 passed, 0 failed; `examples/server.toml` documents persistence behavior

## Files Likely Touched

- `crates/smelt-cli/src/serve/queue.rs` — `load_or_new` implementation + 2 unit tests
- `crates/smelt-cli/src/commands/serve.rs` — replace `new()` with `load_or_new()`
- `examples/server.toml` — persistence documentation comment
