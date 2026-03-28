---
estimated_steps: 5
estimated_files: 2
---

# T02: TrackerPoller struct and AnyTrackerSource enum

**Slice:** S05 — Dispatch Integration, State Backend Passthrough & Final Assembly
**Milestone:** M012

## Description

Create the `TrackerPoller` background task and the `AnyTrackerSource` enum dispatcher. The poller bridges tracker backends (GitHub, Linear, Mock) to the serve dispatch pipeline: it polls for ready issues, transitions labels (D157 — Ready→Queued before enqueue), generates manifests via `issue_to_manifest()`, writes them to temp files (D105 pattern), and enqueues into `ServerState`. The `AnyTrackerSource` enum solves the non-object-safe RPITIT trait problem (D084 pattern).

## Steps

1. Create `crates/smelt-cli/src/serve/tracker_poller.rs` with imports from `serve::tracker`, `serve::queue`, `serve::types`, `serve::config`, `serve::github`, `serve::linear`, `smelt_core::tracker`, `smelt_core::manifest`, `tokio_util::sync::CancellationToken`
2. Define `AnyTrackerSource` enum with variants `GitHub(GithubTrackerSource<SubprocessGhClient>)`, `Linear(LinearTrackerSource<ReqwestLinearClient>)`, `Mock(MockTrackerSource)` — implement `poll_ready_issues()`, `transition_state()`, and `ensure_labels()` via match delegation. `ensure_labels()` needs `&mut self` for the Linear variant. Gate `Mock` variant behind `#[cfg(test)]`.
3. Define `TrackerPoller` struct: `source: AnyTrackerSource`, `template: JobManifest`, `config: TrackerConfig`, `state: Arc<Mutex<ServerState>>`, `cancel: CancellationToken`, `interval: Duration`. Implement `pub async fn run(&mut self) -> anyhow::Result<()>`: call `self.source.ensure_labels().await?` once; then loop with `tokio::time::interval` (MissedTickBehavior::Skip) + `tokio::select!` on interval tick vs `cancel.cancelled()`; each tick: call `poll_ready_issues()` (warn+continue on error); for each issue: call `transition_state(Ready→Queued)` (warn+skip on error); call `issue_to_manifest()` (warn+skip on error); write manifest to `NamedTempFile` + `std::mem::forget(TempPath)` (D105); `state.lock().unwrap().enqueue(path, JobSource::Tracker)`; log info with issue_id and job_id.
4. Add `pub(crate) mod tracker_poller;` to `serve/mod.rs` and re-export `TrackerPoller` and `AnyTrackerSource`
5. Write unit tests using `MockTrackerSource`: (a) `test_poller_enqueues_issues` — mock returns 2 issues + 2 transition Ok → verify ServerState has 2 Queued jobs after one tick; (b) `test_poller_skips_on_transition_error` — mock returns 1 issue + transition Err → verify 0 jobs enqueued, poller continues; (c) `test_poller_continues_on_poll_error` — mock returns Err then Ok([issue]) → verify 1 job enqueued after second tick; (d) `test_poller_exits_on_cancellation` — cancel token before run → verify run returns Ok

## Must-Haves

- [ ] `AnyTrackerSource` enum with GitHub, Linear, Mock(test-only) variants delegating `poll_ready_issues`, `transition_state`, `ensure_labels`
- [ ] `TrackerPoller::run()` calls `ensure_labels()` once then polls on interval
- [ ] Double-dispatch prevention: `transition_state(Ready→Queued)` called before `enqueue()` (D157)
- [ ] Temp file written with D105 pattern (`std::mem::forget(TempPath)`)
- [ ] Poll errors and transition errors logged and skipped (poller continues)
- [ ] Cancellation token exits the loop cleanly
- [ ] ≥4 unit tests with MockTrackerSource

## Verification

- `cargo test -p smelt-cli --lib -- serve::tracker_poller` — all tests pass
- `cargo test --workspace` — all tests pass, 0 regressions
- `cargo clippy --workspace -- -D warnings` — zero warnings

## Observability Impact

- Signals added/changed: `tracing::info!` at poller startup (provider, interval_secs); `tracing::debug!` per poll cycle (issues_found count); `tracing::warn!` on poll error, transition error, manifest generation error; `tracing::info!` on successful enqueue (issue_id, job_id)
- How a future agent inspects this: `SMELT_LOG=debug` shows every poll cycle and issue; `SMELT_LOG=warn` shows only errors
- Failure state exposed: `ensure_labels()` failure at startup propagates as `Err` (poller doesn't start); poll/transition errors logged with issue context

## Inputs

- `crates/smelt-cli/src/serve/tracker.rs` — `TrackerSource` trait, `issue_to_manifest()`, `MockTrackerSource`
- `crates/smelt-cli/src/serve/github/source.rs` — `GithubTrackerSource`, `ensure_labels(&self)`
- `crates/smelt-cli/src/serve/linear/source.rs` — `LinearTrackerSource`, `ensure_labels(&mut self)`
- `crates/smelt-cli/src/serve/queue.rs` — `ServerState::enqueue()`
- `crates/smelt-cli/src/serve/types.rs` — `JobSource::Tracker`
- `crates/smelt-cli/src/serve/config.rs` — `TrackerConfig`

## Expected Output

- `crates/smelt-cli/src/serve/tracker_poller.rs` — new file: `AnyTrackerSource` enum, `TrackerPoller` struct with `run()`, 4+ unit tests
- `crates/smelt-cli/src/serve/mod.rs` — `pub(crate) mod tracker_poller` added
