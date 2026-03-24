# S02: TUI PR status panel with background polling

**Goal:** Add a PR status badge to the TUI dashboard for milestones with open PRs, polled via a background thread using `gh pr view --json`.
**Demo:** TUI dashboard shows a PR status badge (e.g., `🟢 OPEN ✓2/2`) and review status next to milestones that have `pr_number` set, refreshed every 60s via background polling.

## Must-Haves

- `PrStatusInfo` struct with state (Open/Merged/Closed), CI check summary (pass/fail/pending counts), and review decision
- `pr_status_poll()` free function in `assay-core::pr` that calls `gh pr view <n> --json state,statusCheckRollup,reviewDecision` and returns `PrStatusInfo`
- `TuiEvent::PrStatusUpdate` variant for channel-based delivery from polling thread to main loop
- Background polling thread spawned on app startup, polling milestones with `pr_number` every 60s
- `HashMap<String, PrStatusInfo>` on `App` for dashboard rendering
- Dashboard `ListItem` line includes PR status badge when status is available
- Polling failures degrade gracefully (no crash, no error display — badge just absent)
- `gh` availability checked once at startup — polling thread not spawned if `gh` is missing
- Shared `Arc<Mutex<Vec<(String, u64)>>>` between App and polling thread for milestone slug→pr_number mapping, updated on milestone refresh
- Integration tests proving: polling function parses `gh` JSON correctly, dashboard renders badge, polling thread delivers updates via channel

## Proof Level

- This slice proves: integration (mock `gh` subprocess + real TUI channel dispatch + dashboard rendering)
- Real runtime required: no (mock `gh` binary for all tests)
- Human/UAT required: yes (real `gh pr view` against a live PR for visual confirmation)

## Verification

- `cargo test -p assay-core --test pr_status` — new test file: `pr_status_poll` parses mock `gh` JSON for OPEN/MERGED/CLOSED states, empty statusCheckRollup, and in-progress checks
- `cargo test -p assay-tui --test pr_status_panel` — new test file: App receives `PrStatusUpdate` event, stores it in `pr_statuses` map, dashboard renders badge text
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean

## Observability / Diagnostics

- Runtime signals: `tracing::debug` on each poll cycle (milestone slug, result or error); `tracing::warn` when `gh` is not found at startup
- Inspection surfaces: `App.pr_statuses: HashMap<String, PrStatusInfo>` is public — integration tests read it directly; a future slash command could dump it
- Failure visibility: polling errors are swallowed (degraded to absent badge); `gh` missing at startup logs once and skips thread spawn
- Redaction constraints: none (no secrets in `gh pr view` output)

## Integration Closure

- Upstream surfaces consumed: `Milestone.pr_number` (from S01/pre-existing), `TuiEvent` enum (D107/D114), `App.milestones` (D089), `App.event_tx` (D113), `write_fake_gh`/`with_mock_gh_path` test helpers from `crates/assay-core/tests/pr.rs`
- New wiring introduced in this slice: polling thread spawned in `main.rs` run() loop alongside crossterm thread; `TuiEvent::PrStatusUpdate` dispatched in main loop; `App.pr_statuses` rendered in `draw_dashboard`; `Arc<Mutex>` shared state for poll targets updated in `handle_agent_done` and wizard submit
- What remains before the milestone is truly usable end-to-end: S03 (OpenCode plugin), S04 (analytics engine), S05 (TUI analytics screen)

## Tasks

- [x] **T01: PrStatusInfo type + pr_status_poll function + integration tests** `est:45m`
  - Why: Core polling function is the foundation — must parse `gh pr view --json` output correctly before wiring into TUI
  - Files: `crates/assay-core/src/pr.rs`, `crates/assay-core/tests/pr_status.rs`
  - Do: Add `PrStatusInfo` struct (state enum, ci_pass/ci_fail/ci_pending counts, review_decision string) to `assay-core::pr`. Implement `pr_status_poll(pr_number) -> Result<PrStatusInfo>` using `Command::new("gh")`. Parse JSON with `serde_json`. Write integration tests using `write_fake_gh`/`with_mock_gh_path` pattern from existing `pr.rs` tests — cover OPEN with passing checks, MERGED with no checks, CLOSED, empty statusCheckRollup, in-progress checks, and `gh` not found error.
  - Verify: `cargo test -p assay-core --test pr_status` — all tests pass
  - Done when: `pr_status_poll` returns correct `PrStatusInfo` for all test scenarios; `gh` not found returns `Err` gracefully

- [x] **T02: TuiEvent variant + polling thread + App state + dashboard badge rendering** `est:60m`
  - Why: Wires the core function into the TUI event loop with background polling and renders the badge on the dashboard
  - Files: `crates/assay-tui/src/event.rs`, `crates/assay-tui/src/app.rs`, `crates/assay-tui/src/main.rs`, `crates/assay-tui/src/lib.rs`
  - Do: (1) Add `TuiEvent::PrStatusUpdate { slug: String, info: PrStatusInfo }` variant to `event.rs`. (2) Add `pr_statuses: HashMap<String, PrStatusInfo>` and `poll_targets: Arc<Mutex<Vec<(String, u64)>>>` to `App`. Initialize `poll_targets` from `self.milestones` filtering on `pr_number.is_some()`. (3) Add `handle_pr_status_update(&mut self, slug, info)` method. (4) Update `poll_targets` in `handle_agent_done` and wizard submit paths where `self.milestones` is refreshed. (5) In `main.rs run()`, check `gh` availability once; if found, spawn polling thread that clones `tx` and `poll_targets`, loops with 60s sleep, reads targets from mutex, calls `pr_status_poll` for each, sends `PrStatusUpdate` on success. (6) Add `TuiEvent::PrStatusUpdate` arm to main loop dispatch. (7) In `draw_dashboard`, append PR badge to `ListItem` line when `pr_statuses` has an entry for the milestone slug. Badge format: `🟢 OPEN ✓2/3` or `🟣 MERGED` or `🔴 CLOSED` with CI counts.
  - Verify: `cargo build -p assay-tui` compiles clean; `cargo clippy -p assay-tui -- -D warnings` clean
  - Done when: Polling thread spawns, badge renders in dashboard list items, graceful degradation when `gh` is missing

- [x] **T03: TUI integration tests for PR status panel** `est:30m`
  - Why: Proves the full loop mechanically — event delivery, state update, and badge rendering — without a real `gh` binary or live PR
  - Files: `crates/assay-tui/tests/pr_status_panel.rs`
  - Do: (1) Test that `App.handle_pr_status_update` stores info in `pr_statuses` map and the entry is retrievable. (2) Test that `poll_targets` is populated from milestones with `pr_number`. (3) Test that poll_targets is refreshed after milestone reload (simulate `handle_agent_done` with a project root containing milestones). (4) Test graceful degradation: `handle_pr_status_update` for unknown slug is a no-op (no panic).
  - Verify: `cargo test -p assay-tui --test pr_status_panel` — all tests pass
  - Done when: All 4+ tests pass, `just ready` green

## Files Likely Touched

- `crates/assay-core/src/pr.rs`
- `crates/assay-core/tests/pr_status.rs`
- `crates/assay-tui/src/event.rs`
- `crates/assay-tui/src/app.rs`
- `crates/assay-tui/src/main.rs`
- `crates/assay-tui/src/lib.rs`
- `crates/assay-tui/tests/pr_status_panel.rs`
