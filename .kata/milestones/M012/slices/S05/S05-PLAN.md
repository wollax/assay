# S05: Dispatch Integration, State Backend Passthrough & Final Assembly

**Goal:** Wire TrackerPoller into `smelt serve`, pass `state_backend` through to Assay RunManifest, show tracker-sourced jobs in TUI, update docs, and prove end-to-end via integration test.
**Demo:** `smelt serve` with `[tracker]` config picks up issues from a mock tracker, transitions labels, enqueues manifests, and displays tracker-sourced jobs in TUI. `state_backend` in a job manifest appears in the generated Assay RunManifest TOML.

## Must-Haves

- `TrackerPoller` struct runs as a background task inside `smelt serve`'s `tokio::select!` loop
- `TrackerPoller` calls `ensure_labels()` once at startup, then polls on configurable interval
- `TrackerPoller` transitions `Ready → Queued` (D157) before enqueuing each issue
- Non-object-safe `TrackerSource` dispatched via `AnyTrackerSource` enum (D084 pattern)
- `SmeltRunManifest` gains `state_backend: Option<StateBackendConfig>` with `#[serde(default, skip_serializing_if)]`
- `build_run_manifest_toml()` serializes `state_backend` from `JobManifest` into RunManifest TOML
- TUI shows a `Source` column distinguishing `Tracker` / `HttpApi` / `DirectoryWatch`
- `examples/server.toml` documents `[tracker]` section
- `README.md` documents tracker-driven dispatch and `state_backend`
- Integration test proves full poller→enqueue→dispatch flow using `MockTrackerSource`

## Proof Level

- This slice proves: integration (mock tracker end-to-end through dispatch pipeline)
- Real runtime required: no (mock tracker, no Docker/gh/Linear)
- Human/UAT required: yes (real GitHub/Linear end-to-end deferred to UAT)

## Verification

- `cargo test -p smelt-core --lib -- assay::tests` — state_backend serialization tests pass
- `cargo test -p smelt-cli --lib -- serve::tracker_poller` — TrackerPoller unit tests pass
- `cargo test -p smelt-cli --lib -- serve::tui` — TUI render tests pass with Source column
- `cargo test --workspace` — all 387+ tests pass, 0 regressions
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Observability / Diagnostics

- Runtime signals: `tracing::info!` on poller startup (provider, poll_interval); `tracing::debug!` on each poll cycle (issue count); `tracing::warn!` on poll/transition errors; `tracing::info!` on successful enqueue (issue_id, job_id)
- Inspection surfaces: TUI `Source` column shows `Tracker` for tracker-sourced jobs; HTTP API `GET /api/v1/jobs` already returns `source` field
- Failure visibility: Poll errors logged with `tracing::warn!` and skipped (poller continues); transition errors logged and issue skipped; startup `ensure_labels()` failure is fatal (logged at `error!` level, poller returns error)
- Redaction constraints: API keys never logged; `api_key_env` field names may appear in debug logs but never resolved values

## Integration Closure

- Upstream surfaces consumed: `GithubTrackerSource` (S03), `LinearTrackerSource` (S04), `TrackerSource` trait + `issue_to_manifest()` + `MockTrackerSource` (S02), `ServerState::enqueue()`, `dispatch_loop`, `ServerConfig`, `SmeltRunManifest`, `StateBackendConfig`
- New wiring introduced in this slice: `TrackerPoller` background task in `tokio::select!`; `state_backend` passthrough in `build_run_manifest_toml()`; TUI Source column; `[tracker]` documentation
- What remains before the milestone is truly usable end-to-end: Real-world UAT with GitHub Issues (`gh` CLI) and Linear API; live Docker execution with tracker dispatch

## Tasks

- [x] **T01: State backend passthrough in AssayInvoker** `est:20m`
  - Why: R075 — `state_backend` field must flow from JobManifest through to Assay RunManifest TOML; this is independent of the poller and can be proven with unit tests immediately
  - Files: `crates/smelt-core/src/assay.rs`
  - Do: Add `state_backend: Option<StateBackendConfig>` to `SmeltRunManifest` with `#[serde(default, skip_serializing_if = "Option::is_none")]`; update `build_run_manifest_toml()` to copy `manifest.state_backend` into `SmeltRunManifest`; add unit tests for None (omitted) and each `StateBackendConfig` variant (linear, github, etc.)
  - Verify: `cargo test -p smelt-core --lib -- assay::tests` passes; existing tests still pass (None case is backward-compat)
  - Done when: `build_run_manifest_toml()` with a manifest containing `state_backend = Linear { team_id, project_id }` produces TOML with a `[state_backend]` section; manifests without it produce no section

- [x] **T02: TrackerPoller struct and AnyTrackerSource enum** `est:30m`
  - Why: Core polling loop and non-object-safe trait dispatch — the central integration piece that bridges tracker backends to the serve dispatch pipeline
  - Files: `crates/smelt-cli/src/serve/tracker_poller.rs`, `crates/smelt-cli/src/serve/mod.rs`
  - Do: Create `tracker_poller.rs` with `AnyTrackerSource` enum (GitHub/Linear/Mock variants) delegating `poll_ready_issues` and `transition_state` manually; create `TrackerPoller` struct holding `AnyTrackerSource`, `JobManifest` (template), `TrackerConfig`, `Arc<Mutex<ServerState>>`, `CancellationToken`, `Duration`; implement `run()` method: call `ensure_labels()` once, then loop with `tokio::time::interval` + `cancellation_token.cancelled()` in select; each cycle: poll → for each issue: transition Ready→Queued → issue_to_manifest → write temp file (D105 pattern) → state.enqueue(); add module to `serve/mod.rs`; add unit tests with MockTrackerSource proving: successful poll+enqueue, transition failure skips issue, poll error continues loop, cancellation exits
  - Verify: `cargo test -p smelt-cli --lib -- serve::tracker_poller` — all tests pass
  - Done when: TrackerPoller with MockTrackerSource polls issues, transitions labels, writes temp manifest, and enqueues into ServerState; cancellation exits cleanly

- [x] **T03: Wire TrackerPoller into serve execute(), TUI Source column, and docs** `est:30m`
  - Why: Final assembly — poller must run in the serve loop; TUI must show source; docs must be updated; this completes the milestone
  - Files: `crates/smelt-cli/src/commands/serve.rs`, `crates/smelt-cli/src/serve/tui.rs`, `examples/server.toml`, `README.md`
  - Do: In `execute()`: when `config.tracker` is `Some`, construct `AnyTrackerSource` from config (match on provider string), build `TrackerPoller`, add `poller.run()` as a new `tokio::select!` arm; in `tui.rs`: add `Source` column showing `job.source` as string (Tracker/HttpApi/DirWatch), update widths and header; update existing TUI test and add one for `JobSource::Tracker` rendering; in `examples/server.toml`: add commented `[tracker]` section with GitHub and Linear examples; in `README.md`: add Tracker-Driven Dispatch section documenting `[tracker]` config and `[state_backend]` manifest field; run full test suite
  - Verify: `cargo test --workspace` — all tests pass; `cargo clippy --workspace -- -D warnings` — clean; `cargo doc --workspace --no-deps` — clean
  - Done when: `smelt serve` with `[tracker]` config constructs and runs TrackerPoller; TUI shows Source column; docs updated

## Files Likely Touched

- `crates/smelt-core/src/assay.rs`
- `crates/smelt-cli/src/serve/tracker_poller.rs` (new)
- `crates/smelt-cli/src/serve/mod.rs`
- `crates/smelt-cli/src/commands/serve.rs`
- `crates/smelt-cli/src/serve/tui.rs`
- `examples/server.toml`
- `README.md`
