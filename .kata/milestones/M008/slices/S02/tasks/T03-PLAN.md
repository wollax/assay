---
estimated_steps: 4
estimated_files: 1
---

# T03: TUI integration tests for PR status panel

**Slice:** S02 — TUI PR status panel with background polling
**Milestone:** M008

## Description

Write integration tests that prove the TUI PR status panel works mechanically: event delivery updates App state, poll targets track milestones correctly, and the system degrades gracefully on unexpected input. These tests drive `App` directly (no real terminal, no real `gh`) — consistent with the established integration test pattern in `crates/assay-tui/tests/`.

## Steps

1. Create `crates/assay-tui/tests/pr_status_panel.rs`. Import `App`, `Screen`, `PrStatusInfo`, `PrStatusState` from `assay_tui::app` and `assay_core::pr`.
2. Write test `test_handle_pr_status_update_stores_info`:
   - Construct `App::with_project_root(None)`, call `app.handle_pr_status_update("my-ms".into(), info)` with a `PrStatusInfo { state: Open, ci_pass: 2, ci_fail: 0, ci_pending: 1, review_decision: "APPROVED".into() }`.
   - Assert `app.pr_statuses.get("my-ms")` returns `Some` with matching fields.
   - Call again with a different state (Merged) for the same slug — assert it's overwritten.
3. Write test `test_poll_targets_populated_from_milestones`:
   - Create a tempdir with `.assay/milestones/` containing two milestone TOML files: one with `pr_number = 42` and one without `pr_number`.
   - Construct `App::with_project_root(Some(tempdir.path()))`.
   - Lock `app.poll_targets`, assert it contains exactly one entry `("ms-with-pr", 42)`.
4. Write test `test_poll_targets_refreshed_after_milestone_reload`:
   - Create a tempdir with one milestone with `pr_number = 10`.
   - Construct `App::with_project_root(Some(tempdir.path()))`.
   - Assert poll_targets has 1 entry.
   - Write a second milestone TOML with `pr_number = 20` to disk.
   - Call `app.handle_agent_done(0)` (which refreshes milestones and poll_targets).
   - Assert poll_targets now has 2 entries.
   Run `cargo test -p assay-tui --test pr_status_panel` and `just ready`. Fix any issues.

## Must-Haves

- [ ] `test_handle_pr_status_update_stores_info` passes — proves event→state storage
- [ ] `test_poll_targets_populated_from_milestones` passes — proves initialization filters correctly
- [ ] `test_poll_targets_refreshed_after_milestone_reload` passes — proves refresh keeps targets in sync
- [ ] All existing `assay-tui` tests still pass (no regressions)
- [ ] `just ready` green

## Verification

- `cargo test -p assay-tui --test pr_status_panel` — all 3+ tests pass
- `cargo test -p assay-tui` — all existing tests pass (no regressions)
- `just ready` — green

## Observability Impact

- Signals added/changed: None (tests only)
- How a future agent inspects this: run `cargo test -p assay-tui --test pr_status_panel` to verify PR status panel mechanics
- Failure state exposed: None (tests only)

## Inputs

- T02 output: `App.pr_statuses`, `App.poll_targets`, `App.handle_pr_status_update`, `App.refresh_poll_targets`
- T01 output: `PrStatusInfo`, `PrStatusState` types
- Existing test patterns in `crates/assay-tui/tests/agent_run.rs` — `App::with_project_root`, `key()` helper, direct state assertions
- Existing test patterns in `crates/assay-core/tests/pr.rs` — tempdir with milestones for realistic file state

## Expected Output

- `crates/assay-tui/tests/pr_status_panel.rs` — 3+ integration tests proving PR status panel mechanics
- `just ready` green (all workspace tests pass, clippy clean, fmt clean)
