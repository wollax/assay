---
estimated_steps: 5
estimated_files: 2
---

# T01: Contract tests, App.show_help + App.cycle_slug fields, cycle_slug loading

**Slice:** S05 — Help Overlay, Status Bar, and Integration Polish
**Milestone:** M006

## Description

Creates the behavioral contract for all S05 behavior as integration tests, then adds the two new `App` struct fields (`show_help` and `cycle_slug`) and wires `cycle_slug` loading in `with_project_root`. Tests that require rendering changes (`?` key handler) compile but fail; tests that only require field presence or data loading pass immediately after T01. T02 completes them.

## Steps

1. Create `crates/assay-tui/tests/help_status.rs` with a `setup_project_with_status` helper (mirrors `spec_browser.rs`'s `setup_project` but accepts a `status` str so tests can control whether the milestone is `"draft"` or `"in_progress"`). Write 6 tests:
   - `show_help_starts_false`: create App via `App::with_project_root(Some(root))`, assert `app.show_help == false`
   - `question_mark_opens_help`: send `?` key, assert `app.show_help == true`
   - `question_mark_again_closes_help`: send `?` twice, assert `app.show_help == false`
   - `esc_closes_help_when_open`: manually set `app.show_help = true`, send `Esc`, assert `app.show_help == false` AND `handle_event` returned `false` (no quit)
   - `cycle_slug_none_for_draft_milestone`: setup project with `status = "draft"`, assert `app.cycle_slug == None`
   - `cycle_slug_some_for_in_progress_milestone`: setup project with `status = "in_progress"`, assert `app.cycle_slug == Some("alpha".to_string())`

2. Add `pub show_help: bool` and `pub cycle_slug: Option<String>` fields to the `App` struct in `crates/assay-tui/src/app.rs`. Initialize both in `with_project_root`: `show_help: false`, `cycle_slug` via `assay_core::milestone::cycle_status(&assay_dir).ok().flatten().map(|cs| cs.milestone_slug)` (call only when `project_root` is `Some`).

3. Add the `cycle_status` import to `app.rs`: extend the `use assay_core::milestone::{...}` import to include `cycle_status`. Also import `assay_core::milestone::CycleStatus` if the return type needs naming (it doesn't — `.map(|cs| cs.milestone_slug)` is sufficient).

4. Add `cycle_slug` refresh in the wizard Submit success path. In the `WizardAction::Submit` arm, after `self.milestones = loaded;`, add: `self.cycle_slug = cycle_status(&assay_dir).ok().flatten().map(|cs| cs.milestone_slug);` (the `assay_dir` binding is already in scope from the submit path).

5. Run `cargo test -p assay-tui --test help_status` to confirm the tests compile. Verify tests 1, 5, and 6 pass (field initialization and cycle_slug loading). Tests 2, 3, and 4 should fail with assertion errors (not compile errors) since the `?` key handler doesn't exist yet.

## Must-Haves

- [ ] `tests/help_status.rs` compiles without errors
- [ ] `show_help_starts_false` passes
- [ ] `cycle_slug_none_for_draft_milestone` passes
- [ ] `cycle_slug_some_for_in_progress_milestone` passes
- [ ] `question_mark_opens_help`, `question_mark_again_closes_help`, `esc_closes_help_when_open` compile and fail with assertion errors only (not compile errors)
- [ ] `App.show_help: bool` field present with default `false`
- [ ] `App.cycle_slug: Option<String>` field present; loaded from `cycle_status` in `with_project_root`
- [ ] `cycle_slug` refreshed in wizard Submit success path
- [ ] All prior assay-tui tests still pass (no regressions)

## Verification

- `cargo test -p assay-tui --test help_status` — 3 pass, 3 fail with assertion errors, 0 compile errors
- `cargo test -p assay-tui` — all prior tests pass (1 app_wizard + 6 spec_browser + 9 wizard_round_trip = 16 total still passing)
- `grep "show_help\|cycle_slug" crates/assay-tui/src/app.rs` — both fields visible in struct definition

## Observability Impact

- Signals added/changed: `App.show_help: bool` — directly observable field; test assertions use it; debugger-inspectable
- How a future agent inspects this: `app.show_help` field in test; `app.cycle_slug` confirms whether an active milestone slug was found from disk
- Failure state exposed: `cycle_slug = None` is the safe fallback when `cycle_status` fails (errors swallowed via `.ok()`) — no panic, no blank screen, just no slug in status bar

## Inputs

- `crates/assay-tui/src/app.rs` — `App` struct to add fields to; `with_project_root` to add loading logic; wizard Submit path to add refresh
- `crates/assay-tui/tests/spec_browser.rs` — reference `setup_project` pattern to follow for `setup_project_with_status`
- `assay_core::milestone::cycle_status` — sync function returning `Result<Option<CycleStatus>>`; call with `&assay_dir`

## Expected Output

- `crates/assay-tui/tests/help_status.rs` — new file with `setup_project_with_status` helper + 6 test functions; compiles clean; 3 pass immediately
- `crates/assay-tui/src/app.rs` — `App` struct extended with `show_help: bool` and `cycle_slug: Option<String>`; `with_project_root` calls `cycle_status`; wizard Submit path refreshes `cycle_slug`
