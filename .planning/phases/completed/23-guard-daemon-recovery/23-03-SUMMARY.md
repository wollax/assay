# 23-03 Summary: Daemon Event Loop & File System Watcher

## Tasks Completed: 2/2

### Task 1: SessionWatcher for file system events
- Created `watcher.rs` wrapping `notify::RecommendedWatcher`
- Watches both the file (for Modify via kqueue) and parent directory (for Create from atomic writes)
- Event filtering: only target file name passes through; temp files (`.tmp`, `~`) excluded
- Unbounded channel for `tokio::select!` integration
- 3 tests: constructor, modification detection, unrelated file filtering

### Task 2: GuardDaemon event loop and public API
- Created `daemon.rs` with `GuardDaemon` struct and async `run()` method
- `tokio::select!` multiplexes: poll timer, watcher events (1s debounce), SIGINT, SIGTERM
- Soft threshold: checkpoint + escalating prune via circuit breaker tier
- Hard threshold: checkpoint + prune (minimum Standard tier)
- Circuit breaker trip: final checkpoint + terminal `GuardCircuitBreakerTripped` error
- Re-evaluates thresholds after each prune to avoid stale state
- Graceful shutdown saves final checkpoint before exit
- Public API: `start_guard`, `stop_guard`, `guard_status` in `guard/mod.rs`
- `GuardStatus` enum with `Running { pid }` and `Stopped` variants

## Deviations

1. **Widened visibility of context::tokens constants** (auto-fix, blocking):
   - `DEFAULT_CONTEXT_WINDOW` and `SYSTEM_OVERHEAD_TOKENS` changed from `pub(super)` to `pub(crate)`
   - `tokens` module changed from `mod` to `pub(crate) mod`
   - `estimate_tokens_from_bytes` changed from `pub` + `#[allow(dead_code)]` to `pub(crate)`
   - Required for daemon to compute context percentage without duplicating magic numbers

2. **Added tokio and tracing to assay-core deps** (expected, noted in plan):
   - Both were in workspace `Cargo.toml` but not in assay-core's `Cargo.toml`

## Commits
- `360d70f`: feat(23-03): implement SessionWatcher for file system events
- `5699ac8`: feat(23-03): implement GuardDaemon event loop and public API

## Verification
- `cargo build -p assay-core` — clean
- `cargo test --lib -p assay-core` — 318 passed
- `cargo clippy -p assay-core` — no issues
- `just fmt-check` — clean
- `just test` — all workspace tests pass
