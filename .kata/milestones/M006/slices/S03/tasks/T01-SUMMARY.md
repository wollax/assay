---
id: T01
parent: S03
milestone: M006
provides:
  - Screen::MilestoneDetail { slug } and Screen::ChunkDetail { milestone_slug, chunk_slug } variants in the Screen enum
  - App.detail_list_state, detail_milestone, detail_spec, detail_spec_note, detail_run fields (all initialized to defaults)
  - Stub draw() arms for both new variants (renders placeholder text)
  - Stub handle_event() arms — Esc from MilestoneDetail → Dashboard, Esc from ChunkDetail → MilestoneDetail, q from ChunkDetail exits
  - tests/spec_browser.rs with setup_project() fixture and 6 contract tests (red-but-compiling)
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/spec_browser.rs
key_decisions:
  - "handle_event ChunkDetail arm uses `ref milestone_slug` to borrow before cloning for self.screen reassignment, avoiding borrow-split conflicts (consistent with D097)"
  - "Milestone TOML fixture must include created_at/updated_at; Milestone has non-optional DateTime fields not visible from field docs alone"
patterns_established:
  - "spec_browser.rs test pattern: setup_project() creates minimal .assay fixture; each test navigates via handle_event() from Dashboard; assertion-level failures signal unimplemented T02/T03 navigation"
observability_surfaces:
  - "App.detail_spec_note: carries human-readable reason when detail_spec is None (error or legacy spec); inspectable via `app.detail_spec_note` in tests or debugger"
  - "App.screen discriminant is printed on failure by all 6 spec_browser tests"
  - "`cargo test -p assay-tui --test spec_browser -- --nocapture` shows which assertion fails and what discriminant the screen holds"
duration: 25min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T01: Extend Screen/App types and write spec_browser contract tests

**Added MilestoneDetail/ChunkDetail to Screen enum, five detail fields to App, stub draw/event arms, and 6 red-but-compiling spec_browser contract tests.**

## What Happened

Extended `Screen` with `MilestoneDetail { slug }` and `ChunkDetail { milestone_slug, chunk_slug }` variants. Added five `App` fields (`detail_list_state`, `detail_milestone`, `detail_spec`, `detail_spec_note`, `detail_run`) importing `GatesSpec` and `GateRunRecord` from `assay-types`. Initialized all five fields to defaults in `with_project_root`.

Added stub `draw()` arms rendering placeholder `Paragraph` widgets, and stub `handle_event()` arms: Esc/q from `MilestoneDetail` → `Dashboard`; Esc from `ChunkDetail` → `MilestoneDetail` (cloning slug before self.screen reassignment to avoid borrow conflict); q from `ChunkDetail` exits.

Wrote `tests/spec_browser.rs` with a `setup_project()` helper that creates a minimal `.assay/milestones/alpha.toml` (with required `created_at`/`updated_at` fields) and `.assay/specs/c1/gates.toml` with two criteria. Six contract tests drive `handle_event()` via `KeyCode::Enter` — all fail at assertion level because Enter-key navigation to MilestoneDetail is not yet wired (T02).

## Verification

- `cargo build -p assay-tui` → exits 0, no warnings
- `cargo test -p assay-tui --no-fail-fast` → 10 prior tests pass (1 app_wizard + 9 wizard_round_trip), 6 spec_browser tests fail at assertion level (Discriminant(1) = Dashboard, not MilestoneDetail)
- `cargo test -p assay-tui --test spec_browser` → 6 tests run, 0 pass, failures at `assert!(matches!(app.screen, Screen::MilestoneDetail { .. }))` — correct red state

## Diagnostics

- `cargo test -p assay-tui --test spec_browser -- --nocapture` shows `got Discriminant(1)` on each failure, identifying which screen variant was received
- `app.screen` discriminant + `app.detail_milestone.is_some()` / `app.detail_spec.is_some()` fully describe navigation state at any point in tests

## Deviations

Prior test count was 10 (not 23 as estimated in the task plan). Both app_wizard and wizard_round_trip integration tests pass. The count discrepancy is a planning estimation error; actual pass state is correct.

`Milestone` struct has non-optional `created_at` and `updated_at` fields. Initial test fixture omitted them, causing `LoadError` on startup. Fixed by adding RFC 3339 timestamp strings to the TOML fixture.

## Known Issues

None. All 6 spec_browser tests fail at the first navigation assertion — expected and correct for this stage.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — Screen enum extended with 2 variants; App struct extended with 5 fields; draw() and handle_event() stub arms added
- `crates/assay-tui/tests/spec_browser.rs` — New file: setup_project() helper + 6 contract tests
