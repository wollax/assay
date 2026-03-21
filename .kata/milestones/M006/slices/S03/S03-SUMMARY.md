---
id: S03
parent: M006
milestone: M006
provides:
  - Screen::MilestoneDetail { slug } and Screen::ChunkDetail { milestone_slug, chunk_slug } variants in the Screen enum
  - App.detail_list_state, detail_milestone, detail_spec, detail_spec_note, detail_run fields (all initialized to defaults in with_project_root)
  - Dashboard Enter key: calls milestone_load, populates detail_milestone, transitions to MilestoneDetail (or LoadError on failure)
  - MilestoneDetail navigation: ↑↓ wrapping on detail_list_state, Esc → Dashboard, Enter → ChunkDetail
  - draw_milestone_detail free fn: bordered chunk list sorted by order, ✓/· status icons, empty guard
  - MilestoneDetail Enter: loads spec via load_spec_entry_with_diagnostics, run via history::list+load, transitions to ChunkDetail
  - join_results free fn: joins GatesSpec.criteria with GateRunRecord results by criterion name → Vec<(&Criterion, Option<bool>)>
  - draw_chunk_detail free fn: Ratatui Table with icon/name/description columns; Paragraph fallback for Legacy/None spec
  - ChunkDetail Esc: preserves detail_milestone and detail_list_state, returns to MilestoneDetail
  - tests/spec_browser.rs: setup_project helper + 6 integration tests all passing
requires:
  - slice: S01
    provides: Screen enum, App.project_root, handle_event dispatch, Navigation pattern (D097, D098, D099)
affects:
  - S05
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/spec_browser.rs
key_decisions:
  - "D098 — `..` pattern in draw() match arms sidesteps Screen-variant borrow-split; clone-then-mutate in handle_event() for slug reads before screen transition"
  - "D099 — App-level detail_* fields for loaded data rather than embedding in Screen variants; preserves detail_list_state across Esc transitions"
  - "D100 — Criterion join by exact name match; unmatched criteria or skipped results → None (Pending); linear scan acceptable at ≤15 criteria typical size"
patterns_established:
  - "draw_milestone_detail / draw_chunk_detail signature pattern: (frame, data_refs, &mut ListState) — pass individual fields, not &mut App (D097)"
  - "Sorted-chunk index derivation: clone chunks, sort_by_key(order), index — used in both draw and handle_event to keep selection consistent"
  - "setup_project test helper pattern in spec_browser.rs: tempdir + minimal .assay fixture, navigate via handle_event, assert on app.screen discriminant"
  - "GatesSpec validation requires at least one criterion with cmd/path/kind=AgentReport; test fixtures must include cmd field"
observability_surfaces:
  - "app.screen discriminant — identifies current view; printed by all 6 spec_browser tests on failure"
  - "app.detail_milestone.is_some() — confirms milestone data loaded on MilestoneDetail entry"
  - "app.detail_spec.is_some() / app.detail_spec_note.as_deref() — confirm spec variant; note carries reason for None (Legacy or error)"
  - "app.detail_run.is_some() — distinguishes has-history from no-history (no-history → all Pending, not error)"
  - "Screen::LoadError(msg) — surfaces milestone_load failures inline, visible in tests and at runtime"
  - "cargo test -p assay-tui --test spec_browser — 6 tests serve as executable spec for all navigation transitions"
drill_down_paths:
  - .kata/milestones/M006/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M006/slices/S03/tasks/T02-SUMMARY.md
  - .kata/milestones/M006/slices/S03/tasks/T03-SUMMARY.md
duration: 75min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# S03: Chunk Detail View and Spec Browser

**Two new navigable TUI screens — MilestoneDetail (chunk list) and ChunkDetail (criteria + gate results) — loaded from assay-core on navigation, with Esc chains, wrapping list nav, and 6 integration tests all green.**

## What Happened

S03 extended the App state machine and navigation graph established in S01 with two new full-screen views.

**T01** added the type contracts: `Screen::MilestoneDetail { slug }` and `Screen::ChunkDetail { milestone_slug, chunk_slug }` variants, five `App` fields (`detail_list_state`, `detail_milestone`, `detail_spec`, `detail_spec_note`, `detail_run`), stub draw/event arms, and `tests/spec_browser.rs` with a `setup_project` fixture and 6 red-but-compiling contract tests. A key discovery: `Milestone` has non-optional `created_at`/`updated_at` fields requiring RFC 3339 strings in TOML fixtures.

**T02** implemented MilestoneDetail fully: Dashboard `Enter` calls `milestone_load`, stores the result in `detail_milestone`, resets `detail_list_state`, and transitions to `Screen::MilestoneDetail`. The `handle_event` arm handles ↑↓ wrapping navigation over sorted chunks, Esc back to Dashboard, and Enter forward to ChunkDetail (bonus — 5 of 6 tests green after T02). `draw_milestone_detail` renders a bordered chunk list sorted by `chunk.order` with ✓/· status icons derived from `completed_chunks`. A `..` borrow pattern in `draw()` plus clone-then-mutate in `handle_event()` resolves Screen-variant borrow conflicts (D098).

**T03** completed ChunkDetail: MilestoneDetail `Enter` calls `load_spec_entry_with_diagnostics` (setting `detail_spec`/`detail_spec_note` per `SpecEntry` variant) and `history::list`+`load` (setting `detail_run = None` when history is empty — not an error). `join_results` joins `GatesSpec.criteria` against `GateRunRecord.summary.results` by criterion name, returning `Option<bool>` per criterion (D100). `draw_chunk_detail` renders a Ratatui `Table` with icon/name/description columns when spec is `Some`, or a dim `Paragraph` with the note when spec is `None`. One fixture fix: `GatesSpec` validation requires at least one criterion with a `cmd`/`path`/`kind=AgentReport` field — added `cmd = "true"` to one test criterion. One clippy fix: nested `if let` in Dashboard Enter collapsed for `-D warnings` compliance.

## Verification

- `cargo test -p assay-tui --test spec_browser` → 6/6 pass (enter_on_dashboard_navigates_to_milestone_detail, up_down_in_milestone_detail, esc_from_milestone_detail, enter_on_chunk_navigates_to_chunk_detail, esc_from_chunk_detail, chunk_detail_no_history_all_pending)
- `cargo test -p assay-tui` → 15 tests pass (9 unit/wizard + 6 spec_browser), 0 failed
- `just ready` → fmt ✓, lint ✓, test ✓, deny ✓ — All checks passed

## Requirements Advanced

- R051 (TUI spec browser) — Now fully validated: navigation dashboard→milestone→chunk, criteria display, gate result join, Esc chains, empty history handled as Pending, all proven by 6 integration tests

## Requirements Validated

- R051 — Validated: `assay-tui` spec browser navigation and criterion-result display proven by integration tests (6 spec_browser tests pass), data loaded from real assay-core APIs (`milestone_load`, `load_spec_entry_with_diagnostics`, `history::list`+`load`)

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- MilestoneDetail Enter → ChunkDetail navigation was implemented in T02 (not T03 as planned); T03 still owned spec/run loading and rendering, so this was a forward task not a scope creep
- Prior test count was 10 (not 23 as estimated in the plan); this was a planning estimation error, not an execution deviation
- `GatesSpec` validation requires at least one executable criterion (`cmd`/`path`/`kind=AgentReport`); test fixture required `cmd = "true"` not mentioned in T03 plan

## Known Limitations

- ChunkDetail renders spec criteria but has no drill-down into individual gate result evidence
- No live-refresh — spec/run data is loaded once on navigation; changes to gate history while TUI is open are not reflected
- `SpecEntry::Legacy` shows a one-line message instead of criteria (by design — legacy specs have no structured criteria)

## Follow-ups

- S04 (Settings screen) and S05 (help overlay, status bar, final `just ready` pass) remain
- Evidence drill-down (criterion → raw gate output) deferred to M007

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — Screen enum extended; App struct extended with 5 fields; draw_milestone_detail and draw_chunk_detail free fns; join_results free fn; full navigation wiring; clippy collapsible_if fix
- `crates/assay-tui/tests/spec_browser.rs` — New file: setup_project helper + 6 integration tests

## Forward Intelligence

### What the next slice should know
- `draw()` now has a proper `match &self.screen` with arms for all 6 variants (Dashboard, MilestoneDetail, ChunkDetail, Wizard, Settings, NoProject, LoadError); S04 Settings arm is currently a stub Paragraph — replace it with `draw_settings` call
- The D097/D098 patterns are load-bearing: pass individual fields to draw_* fns, not `&mut self`; use `..` in draw() match arms; clone slug data before mutating `self.screen` in handle_event()
- `App.detail_list_state` is deliberately not reset on ChunkDetail→MilestoneDetail Esc — chunk cursor position is preserved (correct UX)

### What's fragile
- `join_results` uses linear scan over `run.summary.results`; no indexing — fine for ≤15 criteria but would need attention at larger sizes
- `setup_project` fixture in `spec_browser.rs` requires at least one criterion with `cmd = "true"` to pass GatesSpec validation; future test changes that remove the cmd field will silently cause LoadError during navigation

### Authoritative diagnostics
- `cargo test -p assay-tui --test spec_browser -- --nocapture` — shows all 6 test results with screen discriminant on failure
- `app.detail_spec_note.as_deref()` — the canonical reason string when `detail_spec` is `None`; check this first when ChunkDetail shows "No spec data"

### What assumptions changed
- Assumed prior test count was 23 (from S02 summary); actual was 10 (9 unit + 1 wizard integration); the slice plan's "29+" target was recalibrated to "15+" at T01
