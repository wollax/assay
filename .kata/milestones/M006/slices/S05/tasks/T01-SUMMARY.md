---
id: T01
parent: S05
milestone: M006
provides:
  - help_status.rs contract tests (6 tests; 4 pass, 2 fail with assertion errors)
  - App.show_help field (bool, default false)
  - App.cycle_slug field (Option<String>, loaded from cycle_status in with_project_root)
  - cycle_slug refresh in wizard Submit success path
key_files:
  - crates/assay-tui/tests/help_status.rs
  - crates/assay-tui/src/app.rs
key_decisions:
  - none
patterns_established:
  - setup_project_with_status helper mirrors spec_browser.rs setup_project but accepts a status str to control milestone state in tests
observability_surfaces:
  - app.show_help: bool — directly inspectable field; true = overlay visible
  - app.cycle_slug: Option<String> — Some(slug) when InProgress milestone exists; None otherwise; cycle_status I/O errors degrade gracefully to None
duration: short
verification_result: passed
completed_at: 2026-03-21
blocker_discovered: false
---

# T01: Contract tests, App.show_help + App.cycle_slug fields, cycle_slug loading

**Added `show_help` and `cycle_slug` fields to `App`, wired `cycle_slug` loading from `cycle_status`, and created the `help_status.rs` contract test suite with 6 tests (4 passing immediately, 2 awaiting T02's `?` key handler).**

## What Happened

1. Created `crates/assay-tui/tests/help_status.rs` with a `setup_project_with_status(tmp, status)` helper (mirrors `spec_browser.rs`'s `setup_project` but accepts a `status` str to control milestone state). Wrote 6 tests covering `show_help` initialization, `?` toggle behavior, `Esc` closing the overlay, and `cycle_slug` presence for draft vs in_progress milestones.

2. Added `pub show_help: bool` and `pub cycle_slug: Option<String>` to `App` in `app.rs`. Initialized both in `with_project_root`: `show_help: false`; `cycle_slug` via `cycle_status(&assay_dir).ok().flatten().map(|cs| cs.milestone_slug)`.

3. Extended the `use assay_core::milestone::{...}` import to include `cycle_status`.

4. Added `cycle_slug` refresh in the wizard `Submit` success path after `self.milestones = loaded;`.

## Verification

- `cargo test -p assay-tui --test help_status` — 4 pass (`show_help_starts_false`, `cycle_slug_none_for_draft_milestone`, `cycle_slug_some_for_in_progress_milestone`, `question_mark_again_closes_help`), 2 fail with assertion errors only (`question_mark_opens_help`, `esc_closes_help_when_open`), 0 compile errors
- `cargo test -p assay-tui --test spec_browser` — 6 passed, 0 failed (no regressions)
- `cargo test -p assay-tui --test wizard_round_trip` — 9 passed, 0 failed (no regressions)
- `cargo test -p assay-tui --test app_wizard` — 1 passed, 0 failed (no regressions)
- `grep "show_help\|cycle_slug" crates/assay-tui/src/app.rs` — both fields visible in struct definition and initialization

## Diagnostics

- `app.show_help` — false on construction, not yet toggled by `?` key (T02 will wire that)
- `app.cycle_slug` — `None` for draft milestones, `Some("alpha")` for in_progress milestones; errors from `cycle_status` degrade to `None` via `.ok()` (no panic, no blank screen)

## Deviations

`question_mark_again_closes_help` passes vacuously (T01): the `?` key handler doesn't exist yet, so pressing `?` twice leaves `show_help = false`, satisfying the `!app.show_help` assertion. This is expected — the test will become meaningfully passing only after T02 wires the toggle (where it would be false→true→false). No action needed.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/tests/help_status.rs` — new file; 6 contract tests for help overlay and cycle_slug loading
- `crates/assay-tui/src/app.rs` — added `show_help: bool` and `cycle_slug: Option<String>` fields; wired `cycle_status` in `with_project_root`; refreshed `cycle_slug` in wizard Submit success path
