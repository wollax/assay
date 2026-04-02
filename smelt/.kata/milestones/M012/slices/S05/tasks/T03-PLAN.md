---
estimated_steps: 5
estimated_files: 5
---

# T03: Wire TrackerPoller into serve execute(), TUI Source column, and docs

**Slice:** S05 — Dispatch Integration, State Backend Passthrough & Final Assembly
**Milestone:** M012

## Description

Final assembly: wire `TrackerPoller` into `smelt serve`'s `tokio::select!` loop, add a `Source` column to the TUI, update `examples/server.toml` with `[tracker]` configuration, and update `README.md` with tracker-driven dispatch documentation. This closes the milestone.

## Steps

1. In `commands/serve.rs`: when `config.tracker` is `Some(ref tracker_config)`, construct `AnyTrackerSource` by matching `tracker_config.provider.as_str()` — `"github"` → `AnyTrackerSource::GitHub(GithubTrackerSource::new(...))`, `"linear"` → `AnyTrackerSource::Linear(LinearTrackerSource::new(...))`, unknown → return error; load template manifest from `tracker_config.manifest_template` (already validated at startup by `ServerConfig::load()`); build `TrackerPoller::new(source, template, config, state, cancel_token.child_token(), interval)`; add `poller.run()` as a sixth arm in `tokio::select!`; when `config.tracker` is `None`, the select runs as before (no poller arm — use an `async { std::future::pending::<()>().await }` placeholder or conditional arm)
2. In `tui.rs`: update the `render()` function to add a `Source` column. Map `job.source` to display string: `JobSource::Tracker` → `"Tracker"`, `JobSource::HttpApi` → `"HTTP"`, `JobSource::DirectoryWatch` → `"DirWatch"`. Add column width `Constraint::Length(10)` and header `"Source"`. Update `test_tui_render_worker_host` to assert `Source` column header appears. Add `test_tui_render_tracker_source` that creates a job with `source: JobSource::Tracker` and asserts `"Tracker"` appears in TUI output.
3. Update `examples/server.toml`: add a commented `[tracker]` section after the `[auth]` section showing both GitHub and Linear provider examples with all fields documented (provider, manifest_template, poll_interval_secs, label_prefix, default_harness, default_timeout, repo for GitHub, api_key_env + team_id for Linear)
4. Update `README.md`: add a "Tracker-Driven Dispatch" subsection under Server Mode documenting `[tracker]` config with a GitHub and Linear example; add a `[state_backend]` paragraph in the manifest documentation explaining the passthrough to Assay
5. Run `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo doc --workspace --no-deps` — all must pass clean

## Must-Haves

- [ ] `TrackerPoller` runs inside `tokio::select!` when `[tracker]` is configured
- [ ] `smelt serve` without `[tracker]` still works exactly as before (zero regression)
- [ ] TUI shows `Source` column with values `Tracker`, `HTTP`, `DirWatch`
- [ ] TUI test asserts `Source` header and `Tracker` value render correctly
- [ ] `examples/server.toml` has documented `[tracker]` section (commented out)
- [ ] `README.md` documents tracker-driven dispatch and `state_backend`
- [ ] `cargo test --workspace` all pass; `cargo clippy` clean; `cargo doc` clean

## Verification

- `cargo test -p smelt-cli --lib -- serve::tui` — Source column tests pass
- `cargo test --workspace` — all 387+ tests pass, 0 regressions
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Observability Impact

- Signals added/changed: `tracing::info!` when tracker poller is started in serve (provider name, poll interval); no new signals beyond what T02 already adds
- How a future agent inspects this: TUI `Source` column visually distinguishes tracker-sourced jobs; `GET /api/v1/jobs` JSON already includes `source` field
- Failure state exposed: Poller startup failure (bad provider, missing env var) surfaces as `anyhow::Error` and `smelt serve` exits with error message

## Inputs

- `crates/smelt-cli/src/serve/tracker_poller.rs` — `TrackerPoller`, `AnyTrackerSource` (from T02)
- `crates/smelt-cli/src/serve/github/source.rs` — `GithubTrackerSource` constructor
- `crates/smelt-cli/src/serve/linear/source.rs` — `LinearTrackerSource` constructor
- `crates/smelt-cli/src/serve/tui.rs` — current 6-column table
- `examples/server.toml` — current server config example
- `README.md` — current documentation

## Expected Output

- `crates/smelt-cli/src/commands/serve.rs` — TrackerPoller wired into tokio::select!
- `crates/smelt-cli/src/serve/tui.rs` — 7-column table with Source; 2 tests updated/added
- `examples/server.toml` — `[tracker]` section added (commented)
- `README.md` — tracker-driven dispatch + state_backend docs added
