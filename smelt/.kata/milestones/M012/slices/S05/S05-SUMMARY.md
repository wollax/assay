---
id: S05
parent: M012
milestone: M012
provides:
  - TrackerPoller background task wired into smelt serve tokio::select! loop as 6th arm
  - AnyTrackerSource enum dispatching GitHub/Linear/Mock variants (D171)
  - state_backend passthrough from JobManifest into SmeltRunManifest TOML via build_run_manifest_toml()
  - TUI Source column showing Tracker/HTTP/DirWatch for all jobs
  - examples/server.toml [tracker] section with documented GitHub and Linear examples
  - README.md tracker-driven dispatch and state_backend passthrough documentation
  - 398 workspace tests pass, 0 regressions
requires:
  - slice: S03
    provides: GithubTrackerSource with poll_ready_issues, transition_state, ensure_labels
  - slice: S04
    provides: LinearTrackerSource with GraphQL-backed poll_ready_issues, transition_state, ensure_labels
  - slice: S02
    provides: TrackerSource trait, TrackerConfig, TrackerIssue, TemplateManifest, MockTrackerSource, JobSource::Tracker
affects: []
key_files:
  - crates/smelt-core/src/assay.rs
  - crates/smelt-cli/src/serve/tracker_poller.rs
  - crates/smelt-cli/src/serve/mod.rs
  - crates/smelt-cli/src/commands/serve.rs
  - crates/smelt-cli/src/serve/tui.rs
  - examples/server.toml
  - README.md
key_decisions:
  - D171 — AnyTrackerSource enum for non-object-safe TrackerSource dispatch
  - D172 — TrackerPoller poll errors are non-fatal (log + continue)
  - D173 — TrackerPoller uses std::future::pending() placeholder when no tracker configured
patterns_established:
  - "Optional tokio::select! arm pattern: match Option<AnyTrackerSource>, pending() for None branch"
  - "Template TOML stored as raw string in TrackerPoller alongside parsed JobManifest for serialization roundtrip"
  - "D105 temp file pattern (NamedTempFile + std::mem::forget) used for manifest hand-off to enqueue()"
  - "Optional tagged-enum passthrough with serde(default, skip_serializing_if) for deny_unknown_fields compat (state_backend)"
observability_surfaces:
  - "tracing::info! on tracker poller configured (provider name, poll_interval_secs) at serve startup"
  - "tracing::debug! per poll cycle (issues_found count)"
  - "tracing::warn! on poll error, transition error, manifest generation error — poller continues"
  - "tracing::info! on successful enqueue (issue_id, job_id)"
  - "tracing::error! when poller exits with error — triggers full serve shutdown"
  - "TUI Source column: Tracker/HTTP/DirWatch visible at a glance"
  - "GET /api/v1/jobs JSON source field already includes tracker provenance"
drill_down_paths:
  - .kata/milestones/M012/slices/S05/tasks/T01-SUMMARY.md
  - .kata/milestones/M012/slices/S05/tasks/T02-SUMMARY.md
  - .kata/milestones/M012/slices/S05/tasks/T03-SUMMARY.md
duration: 37min (T01: 10m, T02: 15m, T03: 12m)
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
---

# S05: Dispatch Integration, State Backend Passthrough & Final Assembly

**Wired TrackerPoller into smelt serve's tokio::select! loop, forwarded state_backend through to Assay RunManifest TOML, added TUI Source column, and documented tracker-driven dispatch end-to-end**

## What Happened

Three tasks composed the final assembly milestone:

**T01 — state_backend passthrough:** Added `state_backend: Option<StateBackendConfig>` to `SmeltRunManifest` with `#[serde(default, skip_serializing_if = "Option::is_none")]` so manifests with or without the field both parse correctly under `deny_unknown_fields`. `build_run_manifest_toml()` now clones `manifest.state_backend` into the constructed `SmeltRunManifest`. Three unit tests verified None (no TOML section), Linear (produces `[state_backend.linear]` with team_id/project_id), and LocalFs (produces `state_backend = "local_fs"`) variants.

**T02 — TrackerPoller and AnyTrackerSource:** Created `tracker_poller.rs` with two components. `AnyTrackerSource` is an enum dispatching to `GithubTrackerSource`, `LinearTrackerSource`, or `MockTrackerSource` (test-only), solving the RPITIT non-object-safe trait problem (D171). `TrackerPoller` holds the source, template TOML (raw string), template JobManifest, config, shared `ServerState`, `CancellationToken`, and poll interval. Its `run()` method calls `ensure_labels()` once (fatal on error), then enters a `tokio::select!` loop alternating between `tokio::time::interval` ticks and cancellation. Each `poll_once()` cycle polls for ready issues, transitions each Ready→Queued before enqueue (D157 double-dispatch prevention, D172 skip-on-error), generates manifests via `issue_to_manifest()` + `toml::Value` manipulation (JobManifest lacks Serialize), writes to temp file via D105 pattern, and enqueues into `ServerState`. Six unit tests covered enqueue, transition error skip, poll error continue, cancellation exit, TOML roundtrip, and temp file creation.

**T03 — Final wiring, TUI, and docs:** In `serve.rs`, when `config.tracker` is `Some`, the code constructs `AnyTrackerSource` from the provider string (github/linear) with the appropriate concrete client types, builds `TrackerPoller`, and runs it as a 6th `tokio::select!` arm. When tracker is `None`, `std::future::pending::<()>()` fills the arm so the select compiles unconditionally (D173). In `tui.rs`, a `Source` column was added between Manifest and Status, mapping `JobSource::Tracker` → "Tracker", `JobSource::HttpApi` → "HTTP", `JobSource::DirectoryWatch` → "DirWatch". `examples/server.toml` received a fully documented `[tracker]` section (commented out) with GitHub and Linear examples. `README.md` gained a "Tracker-Driven Dispatch" subsection and "State Backend Passthrough" paragraph. All dead_code/unused_imports placeholders from T02 were removed.

## Verification

- `cargo test -p smelt-core --lib -- assay::tests` — 14 passed (11 existing + 3 new)
- `cargo test -p smelt-cli --lib -- serve::tracker_poller` — 6 passed
- `cargo test -p smelt-cli --lib -- serve::tui` — 3 passed (1 updated, 2 new)
- `cargo test --workspace` — 398 passed, 0 failed, 11 ignored
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Requirements Advanced

- R075 — state_backend passthrough proven: `build_run_manifest_toml()` serializes Linear/LocalFs/None variants correctly; tagged-enum serde produces correct TOML; backward-compat with manifests that lack the field
- R070 — GitHub tracker dispatch loop proven end-to-end with MockTrackerSource (label transitions, manifest generation, ServerState enqueue); real `gh` CLI UAT deferred
- R071 — Linear tracker dispatch loop proven end-to-end with MockTrackerSource; real GraphQL API UAT deferred

## Requirements Validated

- R075 — State backend passthrough in JobManifest: proven by T01 unit tests (`test_run_manifest_linear_state_backend`, `test_run_manifest_local_fs_state_backend`, `test_run_manifest_no_state_backend_when_none`) and T02/T03 wiring; all 398 workspace tests pass

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- **T01:** `StateBackendConfig::Linear` serializes as `[state_backend.linear]` (serde tagged-enum TOML convention), not `[state_backend]` as originally written in the must-have. The tagged-enum output is correct and fully functional.
- **T02:** Added `template_toml: String` field to TrackerPoller (not in plan) because `JobManifest` lacks `Serialize`; raw TOML must be preserved for roundtrip serialization via `toml::Value` manipulation.
- **T02:** `build_manifest_toml()` and `write_manifest_temp()` implemented as standalone functions rather than TrackerPoller methods — cleaner separation and independently testable.
- **T03:** Original task plan referenced "T05" in dead_code comments (prior numbering). All T05 references cleaned to T03.

## Known Limitations

- Real-world UAT with `gh` CLI (GitHub Issues) and Linear GraphQL API not yet performed — integration tests use `MockTrackerSource`. Live Docker execution with tracker dispatch remains to be verified.
- D157 double-dispatch prevention relies on label transition atomicity at the `gh` CLI level (single command with both `--add-label` and `--remove-label`). Distributed multi-server deployments with concurrent pollers could still race in edge cases.
- Linear label UUID cache (`ensure_labels()` result) is in-memory only; if labels are deleted externally while serve is running, transition calls will fail until restart.
- `state_backend` Custom variant stores `toml::Value` in Smelt's mirror type; if Assay's Custom variant evolves to use a different schema, the passthrough may need updating.

## Follow-ups

- Live UAT: file a GitHub Issue with `smelt:ready` label, observe `smelt serve` pick it up, transition labels, dispatch, and update to `smelt:pr-created`
- Live UAT: file a Linear issue, observe same lifecycle via GraphQL
- Validate `state_backend` passthrough with a real Assay container that reads `[state_backend.linear]` from RunManifest TOML
- Consider adding exponential backoff for repeated all-offline scenarios (D124 follow-up applies here too)

## Files Created/Modified

- `crates/smelt-core/src/assay.rs` — StateBackendConfig import, state_backend field on SmeltRunManifest, passthrough in build_run_manifest_toml(), 3 unit tests
- `crates/smelt-cli/src/serve/tracker_poller.rs` — New: AnyTrackerSource enum, TrackerPoller struct, run()/poll_once(), build_manifest_toml(), write_manifest_temp(), 6 unit tests; cleaned dead_code allows in T03
- `crates/smelt-cli/src/serve/mod.rs` — Added pub(crate) mod tracker_poller + re-exports of AnyTrackerSource, TrackerPoller; cleaned unused_imports allow
- `crates/smelt-cli/src/commands/serve.rs` — TrackerPoller construction from config, 6th tokio::select! arm, pending() fallback when no tracker
- `crates/smelt-cli/src/serve/tui.rs` — 7-column table with Source; 3 tests (1 updated, 2 new)
- `examples/server.toml` — Documented [tracker] section with GitHub and Linear examples (commented out)
- `README.md` — Tracker-Driven Dispatch subsection + State Backend Passthrough docs

## Forward Intelligence

### What the next slice should know
- All M012 tracker infrastructure is complete. The next phase is live end-to-end UAT with real GitHub/Linear credentials. The mock-based test suite is comprehensive but cannot substitute for the `gh` CLI auth flow and Linear GraphQL rate limits.
- `AnyTrackerSource` construction in `serve.rs` uses `String` matching on `tracker_config.provider` ("github" / "linear"). Adding a new provider requires: (1) new TrackerSource impl, (2) new AnyTrackerSource variant with delegation, (3) new match arm in serve.rs.

### What's fragile
- The `template_toml` + `toml::Value` manipulation approach for manifest serialization is a workaround for `JobManifest` lacking `Serialize`. If `JobManifest` gains `Serialize` in a future slice, `TrackerPoller` should be refactored to use it directly.
- Linear API key is resolved from env var at `smelt serve` startup. If the env var is unset, construction fails with an `anyhow::Error`. The error message is clear but the failure is at runtime (serve startup) not at config-parse time.

### Authoritative diagnostics
- `SMELT_LOG=debug` shows every TrackerPoller poll cycle with issue counts — the first signal to check if the poller seems idle
- TUI Source column is the fastest visual confirmation that tracker-sourced jobs are flowing through the dispatch pipeline
- `GET /api/v1/jobs` JSON `source` field provides programmatic proof of job origin

### What assumptions changed
- Original plan assumed `JobManifest` could be serialized directly; actual code uses `toml::Value` manipulation on the raw template string instead
- `[state_backend.linear]` TOML shape differs from `[state_backend]` with a `type` field — the tagged-enum serde convention produces nested table keys, not a type discriminator field
