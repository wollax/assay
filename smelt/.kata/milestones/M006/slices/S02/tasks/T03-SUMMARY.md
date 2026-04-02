---
id: T03
parent: S02
milestone: M006
provides:
  - DirectoryWatcher struct with poll-and-move-and-enqueue logic
  - VALID_MANIFEST_TOML constant for reuse in T04 HTTP tests
key_files:
  - crates/smelt-cli/src/serve/queue_watcher.rs
  - crates/smelt-cli/src/serve/tests.rs
  - crates/smelt-cli/src/serve/mod.rs
key_decisions:
  - "VALID_MANIFEST_TOML requires all mandatory sections (job with base_ref, environment, credentials, session, merge with strategy+target) — JobManifest uses deny_unknown_fields and has no defaults on these"
patterns_established:
  - "DirectoryWatcher::watch() uses tokio::time::interval with MissedTickBehavior::Skip — consistent with dispatch_loop polling pattern from T02"
  - "File-move-before-enqueue (D100): std::fs::rename to dispatched/ BEFORE calling ServerState::enqueue() prevents double-pickup on restart"
  - "Integration tests use TempDir + tokio::spawn + sleep(3s) to exercise watcher polling — handle.abort() cleans up the infinite loop"
observability_surfaces:
  - "tracing::info! on successful enqueue via directory watch (includes job_id and manifest path)"
  - "tracing::warn! on rename failure, parse error, validation error — allows diagnosing skipped manifests"
  - "dispatched/ subdirectory as inspection surface — shows which files were processed"
duration: 12min
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T03: DirectoryWatcher with atomic file-move and integration tests

**Implemented `DirectoryWatcher` that polls `queue_dir/` every 2s, atomically moves `.toml` files to `dispatched/<ts>-<name>.toml`, parses as `JobManifest`, and enqueues via `ServerState::enqueue()` — both integration tests pass.**

## What Happened

Implemented `DirectoryWatcher` in `queue_watcher.rs` with:
- `new(queue_dir, state)` constructor
- `watch()` async method using `tokio::time::interval(2s)` with `MissedTickBehavior::Skip`
- On each tick: reads `queue_dir/`, filters `.toml` files, creates `dispatched/` dir, renames file with unix_ms prefix, parses with `JobManifest::from_str()` + `validate()`, enqueues with `DirectoryWatch` source
- All errors (rename, read, parse, validate) produce `tracing::warn!` and skip — watcher never crashes

Added `pub(crate) use queue_watcher::DirectoryWatcher;` re-export in `mod.rs`.

Replaced stub tests with real integration tests. Required discovering the correct minimal valid TOML schema: `JobManifest` uses `#[serde(deny_unknown_fields)]` and requires `job.base_ref`, `[credentials]`, `[[session]]` with all fields, and `[merge]` with `strategy` + `target`. Extracted `VALID_MANIFEST_TOML` constant for reuse by T04 HTTP tests.

## Verification

- `cargo test -p smelt-cli serve::tests::test_watcher -- --nocapture` → 2 passed
- `cargo test -p smelt-cli serve::tests -- --nocapture` → 8 passed, 6 ignored (T04 stubs)
- `cargo build -p smelt-cli` → clean (no errors)

### Slice-level verification (partial — T04 pending):
- `cargo test -p smelt-cli serve::tests` → 8 pass, 6 ignored ✅
- `cargo build -p smelt-cli 2>&1 | grep -E "^error"` → none ✅
- T04 HTTP API tests still ignored (expected)

## Diagnostics

- `SMELT_LOG=info smelt serve` will show `manifest enqueued via directory watch` with job_id for each pickup
- `tracing::warn!` events surface: rename failures (concurrent watcher), parse errors (bad TOML), validation errors (missing fields)
- `queue_dir/dispatched/` contains all processed files named `<unix_ms>-<original>.toml` — inspect to see what was picked up and when
- Failed-to-parse manifests still appear in `dispatched/` after rename — a future agent can read the file content to diagnose the parse error

## Deviations

- Test manifest TOML required `base_ref`, `[credentials]`, `[[session]]` (not `[[step]]`), and `[merge]` sections — the plan's "construct minimal valid content from known schema" step required discovering these mandatory fields at runtime since the plan referenced `[[step]]` which doesn't exist in the schema

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/queue_watcher.rs` — `DirectoryWatcher` with poll-and-move-and-enqueue logic (replaced placeholder)
- `crates/smelt-cli/src/serve/tests.rs` — Added `VALID_MANIFEST_TOML` constant, replaced T03 test stubs with real integration tests
- `crates/smelt-cli/src/serve/mod.rs` — Added `pub(crate) use queue_watcher::DirectoryWatcher;` re-export
