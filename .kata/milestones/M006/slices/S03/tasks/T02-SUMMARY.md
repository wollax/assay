---
id: T02
parent: S03
milestone: M006
provides:
  - Dashboard Enter key loads milestone via milestone_load and transitions to Screen::MilestoneDetail
  - Dashboard Enter transitions to Screen::LoadError on milestone_load failure
  - MilestoneDetail handle_event: Esc → Dashboard, q → quit, ↑↓ wrapping navigation, Enter → ChunkDetail
  - draw_milestone_detail free function: bordered chunk list sorted by order, ✓/· status icons, empty guard
  - detail_list_state reset to Some(0) (or None for empty chunk list) on MilestoneDetail entry
  - Dashboard hint bar updated to include "Enter open" instruction
key_files:
  - crates/assay-tui/src/app.rs
key_decisions:
  - "Enter in MilestoneDetail derives sorted chunk index by cloning and sorting chunks (same order as draw), avoiding the need to store a pre-sorted copy in App state"
  - "MilestoneDetail Enter → ChunkDetail wired in T02 (not T03) since the test enter_on_chunk_navigates_to_chunk_detail was green; T03 still owns ChunkDetail render and detail_spec/detail_run loading"
patterns_established:
  - "draw_milestone_detail signature: (frame, Option<&Milestone>, &mut ListState) — same pattern as draw_dashboard, ready for reuse in draw()"
  - "Sorted-chunk index derivation: clone chunks, sort_by_key(|c| c.order), index into sorted vec — used in both draw and handle_event"
observability_surfaces:
  - "app.detail_milestone.as_ref().map(|m| &m.slug) confirms which milestone is loaded after navigation"
  - "Screen::LoadError(msg) surfaces milestone_load failures inline; pattern-match on app.screen to extract msg in tests"
  - "cargo test -p assay-tui --test spec_browser -- --nocapture shows which tests pass/fail with milestone slug visible"
duration: 20min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T02: MilestoneDetail screen — navigation and render

**Dashboard Enter now loads real milestone data and navigates to MilestoneDetail with ↑↓ wrapping navigation, Esc-back, and a bordered chunk list with ✓/· status icons.**

## What Happened

Replaced the T01 stubs for Dashboard Enter and MilestoneDetail with full implementations:

1. Added `use assay_core::milestone::milestone_load;` import.
2. Wired `KeyCode::Enter` in the Dashboard arm: selects the highlighted milestone, calls `milestone_load`, sets `detail_list_state` to `Some(0)` (or `None` for empty), stores the result in `detail_milestone`, and transitions to `Screen::MilestoneDetail { slug }`. On `milestone_load` failure, transitions to `Screen::LoadError(msg)`.
3. Replaced the MilestoneDetail `handle_event` stub with: Esc → Dashboard, q → quit (returns true), Down → wrapping increment mod chunk_count, Up → wrapping decrement, Enter → ChunkDetail navigation (sorted by order, same as draw).
4. Added `draw_milestone_detail(frame, Option<&Milestone>, &mut ListState)` free function with `[title(1), list(fill), hint(1)]` layout, bordered chunk list sorted by `chunk.order`, ✓/· icons derived from `completed_chunks`, empty-chunks guard, and hint bar.
5. Wired `draw_milestone_detail` into `draw()` replacing the T01 stub arm using `..` pattern to avoid borrow-split issues.
6. Updated Dashboard hint bar from `"↑↓ navigate · n new milestone · q quit"` to `"↑↓ navigate · Enter open · n new · q quit"`.

As a bonus, `enter_on_chunk_navigates_to_chunk_detail` and `esc_from_chunk_detail` also pass because MilestoneDetail Enter → ChunkDetail was implemented here. Only `chunk_detail_no_history_all_pending` remains failing (T03's responsibility: loading `detail_spec`/`detail_run`).

## Verification

```
cargo test -p assay-tui --test spec_browser
```
- enter_on_dashboard_navigates_to_milestone_detail ✓ PASS
- up_down_in_milestone_detail ✓ PASS
- esc_from_milestone_detail ✓ PASS
- enter_on_chunk_navigates_to_chunk_detail ✓ PASS (bonus)
- esc_from_chunk_detail ✓ PASS (bonus)
- chunk_detail_no_history_all_pending ✗ FAIL (expected — T03 scope)

```
cargo test -p assay-tui
```
All 15 tests pass (1 app_wizard + 9 wizard_round_trip + 5 spec_browser), 1 expected failure (chunk_detail_no_history_all_pending).

`cargo build -p assay-tui` — clean compile.

Pre-existing clippy error in `assay-types/src/manifest.rs` (derivable_impls on RunManifest) blocks `-D warnings` for the whole crate graph; confirmed pre-existing before this task.

## Diagnostics

- `app.detail_milestone.as_ref().map(|m| &m.slug)` — confirms which milestone loaded after Dashboard Enter
- `std::mem::discriminant(&app.screen)` — identifies screen variant (used in test assertions)
- `Screen::LoadError(msg)` — surfaces milestone_load errors; test: `if let Screen::LoadError(msg) = &app.screen { ... }`
- `cargo test -p assay-tui --test spec_browser -- --nocapture` — shows all 6 test results

## Deviations

MilestoneDetail Enter → ChunkDetail was implemented in T02 (plan said "passes to T03"). The test `enter_on_chunk_navigates_to_chunk_detail` was already green after implementing the MilestoneDetail arm, so it made sense to keep it. T03 still owns loading `detail_spec`, `detail_run`, and rendering `ChunkDetail`.

## Known Issues

`cargo clippy -p assay-tui --all-targets -- -D warnings` fails due to a pre-existing `clippy::derivable_impls` warning in `crates/assay-types/src/manifest.rs` (RunManifest). This is not introduced by T02. `cargo build -p assay-tui` is clean.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — Dashboard Enter wired; MilestoneDetail handle_event arm complete; draw_milestone_detail free function added; draw() MilestoneDetail arm wired; Dashboard hint updated
