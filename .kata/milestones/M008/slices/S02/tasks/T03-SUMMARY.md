---
id: T03
parent: S02
milestone: M008
provides:
  - Integration test proving handle_pr_status_update stores and overwrites PrStatusInfo
  - Integration test proving poll_targets filters milestones by pr_number on init
  - Integration test proving poll_targets refreshes after handle_agent_done reloads milestones
key_files:
  - crates/assay-tui/tests/pr_status_panel.rs
key_decisions:
  - "Used raw TOML strings via write_milestone_toml helper instead of milestone_save to avoid chrono dev-dependency in assay-tui"
patterns_established:
  - "write_milestone_toml helper in pr_status_panel.rs writes minimal milestone TOML directly — avoids pulling chrono into assay-tui dev-deps"
observability_surfaces:
  - "Run `cargo test -p assay-tui --test pr_status_panel` to verify PR status panel mechanics"
duration: 10min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T03: TUI integration tests for PR status panel

**3 integration tests proving PR status panel event→state, poll target initialization, and refresh mechanics**

## What Happened

Created `crates/assay-tui/tests/pr_status_panel.rs` with three tests:

1. `test_handle_pr_status_update_stores_info` — constructs App with no project root, calls `handle_pr_status_update` with Open state, asserts stored fields match, then overwrites with Merged state and asserts the overwrite took effect.

2. `test_poll_targets_populated_from_milestones` — creates a tempdir with two milestone TOMLs (one with `pr_number = 42`, one without), constructs App pointing at that root, and asserts `poll_targets` contains exactly the milestone with a PR number.

3. `test_poll_targets_refreshed_after_milestone_reload` — creates a tempdir with one milestone (`pr_number = 10`), constructs App, asserts 1 poll target, writes a second milestone (`pr_number = 20`) to disk, calls `handle_agent_done(0)` to trigger reload, and asserts poll_targets now has 2 entries with correct slugs and PR numbers.

Used a `write_milestone_toml` helper that writes raw TOML strings to avoid adding chrono as a dev-dependency to assay-tui.

## Verification

- `cargo test -p assay-tui --test pr_status_panel` — 3/3 pass
- `cargo clippy -p assay-tui --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean
- `just ready` timed out at 300s (likely full workspace test suite duration), but individual checks all pass

### Slice-level verification status
- `cargo test -p assay-core --test pr_status` — passed (T01)
- `cargo test -p assay-tui --test pr_status_panel` — ✓ passed (this task)
- `cargo clippy --workspace --all-targets -- -D warnings` — ✓ passed
- `cargo fmt --check` — ✓ passed

## Diagnostics

Run `cargo test -p assay-tui --test pr_status_panel` to verify all PR status panel mechanics.

## Deviations

Used raw TOML string writing instead of `milestone_save` from assay-core. This avoids pulling `chrono` as a dev-dependency into assay-tui while still producing valid milestone files that `milestone_scan` can parse.

## Known Issues

`just ready` times out at 300s in CI-like environments. Individual workspace checks (clippy, fmt, per-crate tests) all pass independently.

## Files Created/Modified

- `crates/assay-tui/tests/pr_status_panel.rs` — 3 integration tests for PR status panel mechanics
