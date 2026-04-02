---
estimated_steps: 3
estimated_files: 2
---

# T02: Wire `load_or_new` into `serve.rs` + update `examples/server.toml`

**Slice:** S03 — Load-on-startup + restart-recovery integration test
**Milestone:** M007

## Description

Replace the `ServerState::new(config.max_concurrent)` call in `commands/serve.rs` with `ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)`. This one-line change activates the full persistence loop: on every startup the daemon now loads any previously-persisted queue state and re-queues non-terminal jobs before accepting new work. Update `examples/server.toml` with a comment explaining the automatic persistence behavior so operators know what to expect.

## Steps

1. In `crates/smelt-cli/src/commands/serve.rs`, in the `execute()` function, replace:
   ```rust
   let state = Arc::new(Mutex::new(ServerState::new(config.max_concurrent)));
   ```
   with:
   ```rust
   let state = Arc::new(Mutex::new(ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)));
   ```
   Check the existing `use crate::serve::{...}` import at the top of the file — `ServerState` is already imported there (it's used in the type of `state`). No new import needed.

2. Verify the change compiles cleanly: `cargo check -p smelt-cli`. No other changes to `serve.rs` are needed — `config.queue_dir` is already available (used in `std::fs::create_dir_all(&config.queue_dir)` two lines earlier).

3. In `examples/server.toml`, add a comment block above the `queue_dir` line explaining persistence:
   ```toml
   # Queue state is automatically persisted to queue_dir/.smelt-queue-state.toml
   # after every enqueue, complete, and cancel. On restart, smelt serve loads this
   # file and re-queues any jobs that were Queued, Retrying, Dispatching, or Running
   # at shutdown time — no operator intervention required.
   queue_dir = "/tmp/smelt-queue"
   ```

## Must-Haves

- [ ] `commands/serve.rs` calls `ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)` — `new()` no longer called
- [ ] `cargo check -p smelt-cli` exits 0 with zero warnings
- [ ] `cargo test -p smelt-cli` — all 52 tests pass (50 from S01+S02 + 2 new from T01), 0 failed
- [ ] `examples/server.toml` has a comment above `queue_dir` explaining automatic persistence and restart-recovery behavior

## Verification

- `grep "load_or_new" crates/smelt-cli/src/commands/serve.rs` — prints the wiring line
- `grep -v "load_or_new\|new_with_persistence" crates/smelt-cli/src/commands/serve.rs | grep "ServerState::new"` — prints nothing (confirms `new()` is no longer called)
- `cargo check -p smelt-cli` — exits 0, zero warnings
- `cargo test -p smelt-cli` — 52 passed, 0 failed

## Observability Impact

- Signals added/changed: the `tracing::info!("load_or_new: loaded …")` log from T01 now fires on every real `smelt serve` startup — visible in `.smelt/serve.log` or stderr immediately after binding the TCP listener; operators can grep this line to confirm whether a restart recovered jobs or started fresh
- How a future agent inspects this: `grep "load_or_new" .smelt/serve.log` after restart to confirm recovery ran; if the log line shows `n=0` on a restart where jobs were expected, investigate whether `queue_dir` is the same path as the previous run
- Failure state exposed: `ServerState::new()` no longer called in production — any test or integration that constructs `ServerState` directly for a serve-like scenario should use `new_with_persistence` or `load_or_new`

## Inputs

- `crates/smelt-cli/src/serve/queue.rs` — `ServerState::load_or_new` from T01 (must be complete before this task)
- `crates/smelt-cli/src/commands/serve.rs` — existing `execute()` function; `config.queue_dir` and `config.max_concurrent` are already in scope
- `examples/server.toml` — existing file to annotate

## Expected Output

- `crates/smelt-cli/src/commands/serve.rs` — `load_or_new` call replaces `new()`
- `examples/server.toml` — persistence comment added above `queue_dir`
