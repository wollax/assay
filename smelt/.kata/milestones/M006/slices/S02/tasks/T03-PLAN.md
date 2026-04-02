---
estimated_steps: 5
estimated_files: 3
---

# T03: DirectoryWatcher with atomic file-move and integration tests

**Slice:** S02 — Directory Watch + HTTP API
**Milestone:** M006

## Description

Implements the directory-watch ingress path: a `DirectoryWatcher` that polls `queue_dir/` every 2 seconds, atomically moves discovered `.toml` files to `queue_dir/dispatched/<ts>-<name>.toml`, parses each as a `JobManifest`, and calls `ServerState::enqueue()`. The file-move-before-enqueue semantics (D100) prevent double-pickup on restart. Integration tests write real TOML files to a TempDir and verify both the file-move and the resulting queue state — no Docker required for these tests.

## Steps

1. Write `serve/queue_watcher.rs`. Define `pub(crate) struct DirectoryWatcher { queue_dir: PathBuf, state: Arc<Mutex<ServerState>> }` with `new(queue_dir, state)` constructor. Implement `pub(crate) async fn watch(&self)`: use `tokio::time::interval(Duration::from_secs(2))`; on each tick read `std::fs::read_dir(&self.queue_dir)?`; filter entries where `path.extension() == Some("toml")`; for each: call `create_dir_all(queue_dir/dispatched/)`, build dest path as `dispatched/<unix_ms>-<filename>`, call `std::fs::rename(src, dest)` (skip with warn if rename fails — file may already be moved), parse renamed file via `JobManifest::from_str()` + `validate()` (warn+skip on error), call `self.state.lock().unwrap().enqueue(dest, JobSource::DirectoryWatch)` and `tracing::info!(job_id=?, "manifest enqueued via directory watch")`.
2. Export `DirectoryWatcher` in `serve/mod.rs` via `pub(crate) use queue_watcher::DirectoryWatcher;`.
3. Add integration test `test_watcher_picks_up_manifest` in `serve/tests.rs`: create TempDir as queue_dir; write a valid manifest TOML (construct minimal valid content from known schema); create `ServerState` with max_concurrent=2; spawn `watcher.watch()` via `tokio::spawn`; sleep 3s; assert `state.lock().unwrap().jobs.len() == 1` and first job has status `Queued`.
4. Add integration test `test_watcher_moves_to_dispatched`: same setup; after 3s assert the original TOML file no longer exists in queue_dir root; assert `queue_dir/dispatched/` contains exactly 1 file matching `<ts>-<name>.toml`.
5. Run `cargo test -p smelt-cli serve::tests::test_watcher -- --nocapture`. Fix errors. Confirm both tests pass.

## Must-Haves

- [ ] `DirectoryWatcher::watch()` polls exactly on `tokio::time::interval` (not `sleep` loop)
- [ ] Manifest moved to `dispatched/<ts>-<name>.toml` via `std::fs::rename` BEFORE `enqueue()` is called
- [ ] `create_dir_all(dispatched/)` called before every rename attempt
- [ ] Parse errors → `tracing::warn!` + skip (do not crash or stop polling)
- [ ] `test_watcher_picks_up_manifest` passes (job appears in ServerState within 3s)
- [ ] `test_watcher_moves_to_dispatched` passes (file moved, no longer in queue_dir root)

## Verification

- `cargo test -p smelt-cli serve::tests::test_watcher -- --nocapture` → both tests pass
- `cargo build -p smelt-cli` → clean

## Observability Impact

- Signals added/changed: `tracing::info!` on successful enqueue via watch; `tracing::warn!` on parse errors or rename failures — allows diagnosing why a dropped manifest was silently skipped
- How a future agent inspects this: `SMELT_LOG=info smelt serve` shows each file pickup; `GET /api/v1/jobs` (T04) shows enqueued jobs; `dispatched/` subdirectory shows which files were processed
- Failure state exposed: manifests that fail parsing remain in `dispatched/` (after rename) with a tracing warning — a future agent can inspect the moved file for the error

## Inputs

- `crates/smelt-cli/src/serve/queue.rs` (T01) — `ServerState::enqueue()`, `JobSource`
- `smelt-core` — `JobManifest::from_str()` + `validate()` (already confirmed to exist)

## Expected Output

- `crates/smelt-cli/src/serve/queue_watcher.rs` — `DirectoryWatcher` with poll-and-move-and-enqueue logic
- `crates/smelt-cli/src/serve/tests.rs` — 2 watcher integration tests passing
- `crates/smelt-cli/src/serve/mod.rs` — `DirectoryWatcher` exported
