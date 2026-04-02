---
id: T03
parent: S05
milestone: M012
provides:
  - TrackerPoller wired into smelt serve tokio::select! loop as 6th arm
  - AnyTrackerSource construction from ServerConfig tracker section (github/linear dispatch)
  - TUI Source column showing Tracker/HTTP/DirWatch for all jobs
  - 3 TUI tests (worker_host, tracker_source, dirwatch_source)
  - examples/server.toml [tracker] section with documented GitHub and Linear examples
  - README.md tracker-driven dispatch and state_backend documentation
key_files:
  - crates/smelt-cli/src/commands/serve.rs
  - crates/smelt-cli/src/serve/tui.rs
  - crates/smelt-cli/src/serve/tracker_poller.rs
  - examples/server.toml
  - README.md
key_decisions:
  - "TrackerPoller arm uses async block with match on Option — pending::<()>() when None so select! is unchanged without tracker config"
  - "Source column placed between Manifest and Status columns for natural reading order"
patterns_established:
  - "Optional tokio::select! arm pattern: match Option, pending() for None branch"
observability_surfaces:
  - "tracing::info! on tracker poller configured (provider, poll_interval_secs) at serve startup"
  - "tracing::error! when poller exits with error, triggers full serve shutdown"
  - "TUI Source column visually distinguishes job origins (Tracker/HTTP/DirWatch)"
duration: 12min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T03: Wire TrackerPoller into serve execute(), TUI Source column, and docs

**Wired TrackerPoller into smelt serve's tokio::select! loop, added 7-column TUI with Source column, documented [tracker] config and state_backend passthrough**

## What Happened

Completed the final assembly task for the tracker-driven dispatch milestone:

1. **serve.rs wiring**: Added TrackerPoller construction when `config.tracker` is `Some`. The code matches on `tracker_config.provider` to build `AnyTrackerSource::GitHub` or `AnyTrackerSource::Linear` with the correct concrete client types. For Linear, the API key is resolved from the environment variable at startup. The poller runs as a 6th arm in `tokio::select!`; when tracker is `None`, a `pending::<()>()` future ensures the arm never fires. Poller errors trigger full serve shutdown with `tracing::error!`.

2. **TUI Source column**: Added a `Source` column (7th column) between Manifest and Status. Maps `JobSource::Tracker` → "Tracker", `JobSource::HttpApi` → "HTTP", `JobSource::DirectoryWatch` → "DirWatch". Added `test_tui_render_tracker_source` and `test_tui_render_dirwatch_source` tests; updated existing `test_tui_render_worker_host` to also assert the Source header and HTTP value.

3. **examples/server.toml**: Added a fully documented `[tracker]` section (commented out) after the `[auth]` section, with both GitHub and Linear provider examples showing all configuration fields.

4. **README.md**: Added "Tracker-Driven Dispatch" subsection under Server Mode with GitHub and Linear config examples, lifecycle label explanation, and TUI source column mention. Added "State Backend Passthrough" paragraph documenting the `[state_backend]` manifest section.

5. **Cleanup**: Removed all `#[allow(dead_code)]` and `#[allow(unused_imports)]` annotations from `tracker_poller.rs` and `mod.rs` that were placeholders for T05 wiring (now T03).

## Verification

- `cargo test -p smelt-cli --lib -- serve::tui` — 3 tests pass (worker_host, tracker_source, dirwatch_source)
- `cargo test -p smelt-cli --lib -- serve::tracker_poller` — 6 tests pass
- `cargo test --workspace` — 398 passed, 0 failed, 11 ignored
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Diagnostics

- `tracing::info!` at serve startup when tracker is configured (provider name, poll interval)
- `tracing::error!` if tracker poller exits with error (e.g. ensure_labels failure)
- TUI Source column shows "Tracker" for tracker-sourced jobs, "HTTP" for API jobs, "DirWatch" for filesystem jobs
- `GET /api/v1/jobs` JSON already includes `source` field (from T02)

## Deviations

- Task plan referenced "T05" in dead_code comments — this was the original slice plan numbering. The actual task is T03; all T05 references were cleaned up.
- Fixed a clippy empty-line-after-doc-comments lint in tracker_poller.rs (pre-existing from T02).

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/serve.rs` — TrackerPoller construction and tokio::select! 6th arm
- `crates/smelt-cli/src/serve/tui.rs` — 7-column table with Source; 3 tests (1 updated, 2 new)
- `crates/smelt-cli/src/serve/tracker_poller.rs` — Removed dead_code/unused_imports allows; fixed clippy lint
- `crates/smelt-cli/src/serve/mod.rs` — Removed unused_imports allow on TrackerPoller re-export
- `examples/server.toml` — Documented [tracker] section with GitHub and Linear examples
- `README.md` — Tracker-driven dispatch subsection + state_backend passthrough docs
