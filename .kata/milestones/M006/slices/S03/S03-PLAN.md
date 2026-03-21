# S03: Chunk Detail View and Spec Browser

**Goal:** Extend the TUI with two new navigable screens — MilestoneDetail (chunk list) and ChunkDetail (criteria + gate results) — loaded from `assay-core` on navigation, with Esc returning to the parent screen.
**Demo:** Launch `assay-tui` on a project with milestones and chunks; press Enter on a milestone to see its chunk list; press Enter on a chunk to see a table of criteria with ✓/✗/? result icons from the latest gate run; Esc returns to the previous screen in both cases; `cargo test -p assay-tui` is green with 6 new spec_browser tests passing.

## Must-Haves

- `Screen::MilestoneDetail { slug: String }` and `Screen::ChunkDetail { milestone_slug: String, chunk_slug: String }` variants exist in the `Screen` enum
- Dashboard `Enter` key calls `milestone_load(assay_dir, slug)`, populates `App.detail_milestone`, and transitions to `MilestoneDetail`
- `App.detail_list_state` drives ↑↓ navigation (wrapping) within MilestoneDetail's chunk list
- MilestoneDetail `Enter` calls `load_spec_entry_with_diagnostics` + `history::list` + `history::load`, populates `detail_spec`/`detail_run`, and transitions to `ChunkDetail`
- ChunkDetail renders a `Table` with three columns — icon (✓/✗/?), criterion name, description — one row per criterion from `detail_spec`
- Criterion result joined by name: `Some(true)` → ✓, `Some(false)` → ✗, `None` / not found in run → ?
- Empty gate history (`history::list` returns `[]`) renders all criteria as `?` Pending — not an error
- `SpecEntry::Legacy` entry renders a one-line "Legacy flat spec — criteria not available" message instead of a table
- `Esc` from MilestoneDetail → Dashboard; `Esc` from ChunkDetail → MilestoneDetail (preserving `detail_list_state` selection and `detail_milestone`)
- 6 new tests in `tests/spec_browser.rs` all pass: enter_on_dashboard_navigates_to_milestone_detail, up_down_in_milestone_detail, esc_from_milestone_detail, enter_on_chunk_navigates_to_chunk_detail, esc_from_chunk_detail, chunk_detail_no_history_all_pending
- `cargo test -p assay-tui` → all 29+ tests pass; `just ready` → fmt/lint/test/deny green

## Proof Level

- This slice proves: integration
- Real runtime required: no (fixture-based integration tests)
- Human/UAT required: yes (keyboard flow in real terminal; breadcrumb navigation)

## Verification

- `cargo test -p assay-tui spec_browser` — all 6 new tests pass
- `cargo test -p assay-tui` — 29+ tests pass (23 prior + 6 new), 0 failed
- `cargo test --workspace` — full workspace green
- `just ready` — fmt ✓, lint ✓, test ✓, deny ✓

## Observability / Diagnostics

- Runtime signals: `App.screen` variant reveals current view; `App.detail_milestone.is_some()` confirms milestone data loaded; `App.detail_spec.is_some()` confirms spec data loaded; `App.detail_spec_note` carries reason when spec is None (Legacy or error)
- Inspection surfaces: `cargo test -p assay-tui spec_browser` — 6 tests serve as executable spec for all navigation transitions and data-loading behaviors; tests assert on `app.screen`, `app.detail_milestone`, `app.detail_spec`
- Failure visibility: milestone_load failure in Dashboard Enter stores error message in `Screen::LoadError`; spec/history load failure in MilestoneDetail Enter stores message in `detail_spec_note`
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `assay_core::milestone::milestone_load(assay_dir, slug)`, `assay_core::spec::load_spec_entry_with_diagnostics(slug, specs_dir)`, `assay_core::history::{list, load}`, `assay_types::{GatesSpec, GateRunRecord, Milestone}`
- New wiring introduced in this slice: Dashboard `Enter` → `milestone_load` → `MilestoneDetail`; MilestoneDetail `Enter` → spec+history load → `ChunkDetail`; Esc chains back; `draw_milestone_detail` and `draw_chunk_detail` free functions
- What remains before the milestone is truly usable end-to-end: S04 (Settings screen + config_save), S05 (help overlay, status bar, final `just ready` pass)

## Tasks

- [x] **T01: Extend Screen/App types and write spec_browser contract tests** `est:45m`
  - Why: Establishes the type contract that T02 and T03 implement against; tests drive TDD discipline and prevent regressions.
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/tests/spec_browser.rs`
  - Do: Add `Screen::MilestoneDetail { slug: String }` and `Screen::ChunkDetail { milestone_slug: String, chunk_slug: String }` variants to the `Screen` enum. Add `detail_list_state: ListState`, `detail_milestone: Option<Milestone>`, `detail_spec: Option<GatesSpec>`, `detail_spec_note: Option<String>`, `detail_run: Option<GateRunRecord>` fields to `App`; initialize all to defaults in `with_project_root`. Add stub match arms in `draw()` (render a placeholder Paragraph) and `handle_event()` (MilestoneDetail: Esc → Dashboard; ChunkDetail: Esc → MilestoneDetail stub). Import `assay_types::{GatesSpec, GateRunRecord}` in `app.rs`. Write `tests/spec_browser.rs` with a `setup_project` helper (tempdir + writes minimal milestone TOML with one chunk ref + writes gates.toml under specs/) and 6 test functions using `App::with_project_root`; test assertions will fail because stubs don't implement real behavior yet.
  - Verify: `cargo build -p assay-tui` exits 0; `cargo test -p assay-tui spec_browser` compiles and fails at assertions (not panics/compile errors)
  - Done when: `cargo build -p assay-tui` succeeds; `cargo test -p assay-tui spec_browser` shows 6 failing tests with assertion failures

- [ ] **T02: MilestoneDetail screen — navigation and render** `est:45m`
  - Why: Makes the milestone→chunk list navigation fully functional; advances the dashboard from a dead-end list to a navigable hierarchy.
  - Files: `crates/assay-tui/src/app.rs`
  - Do: In `handle_event` Dashboard arm, add `KeyCode::Enter`: guard that `self.milestones` is non-empty, get the selected index from `self.list_state.selected()`, extract `slug`, call `milestone_load(&assay_dir, &slug)` (assay_dir = project_root.join(".assay")); on Ok: set `self.detail_milestone`, reset `self.detail_list_state.select(Some(0))` if chunks non-empty, set `self.screen = Screen::MilestoneDetail { slug }`; on Err: set `self.screen = Screen::LoadError(msg)`. In `handle_event` for `Screen::MilestoneDetail { .. }`: Up/Down wrapping on `detail_list_state` bounded by `detail_milestone.as_ref().map(|m| m.chunks.len()).unwrap_or(0)`; `KeyCode::Esc` → `Screen::Dashboard`. Implement `draw_milestone_detail(frame, milestone: Option<&Milestone>, list_state: &mut ListState)`: render a bordered List of chunks sorted by `chunk.order`; each item shows chunk slug + status indicator (✓ if in `milestone.completed_chunks`, · otherwise); title "Milestone: {name}" in block border. Update `draw()` match arm for `Screen::MilestoneDetail { .. }` to call `draw_milestone_detail(frame, self.detail_milestone.as_ref(), &mut self.detail_list_state)`. Add hint "↑↓ navigate · Enter open chunk · Esc back" at bottom. Update Dashboard hint bar to include "· Enter open milestone".
  - Verify: `cargo test -p assay-tui spec_browser::enter_on_dashboard_navigates_to_milestone_detail` passes; `cargo test -p assay-tui spec_browser::up_down_in_milestone_detail` passes; `cargo test -p assay-tui spec_browser::esc_from_milestone_detail` passes; `cargo test -p assay-tui` shows at least 3 new passes
  - Done when: 3 of 6 spec_browser tests pass; `cargo build -p assay-tui` clean; 0 clippy warnings introduced

- [ ] **T03: ChunkDetail screen — spec+history load, criterion join, Table render** `est:60m`
  - Why: Delivers the core spec browser capability — the read-only pane showing criteria and gate run results that makes assay-tui a real project inspection tool.
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/tests/spec_browser.rs`
  - Do: Add `use assay_core::spec::{load_spec_entry_with_diagnostics, SpecEntry}; use assay_core::history; use ratatui::widgets::{Cell, Row, Table};` to imports. In `handle_event` for `Screen::MilestoneDetail { .. }`, add `KeyCode::Enter`: get selected chunk slug from `detail_milestone.chunks` sorted by order; call `load_spec_entry_with_diagnostics(chunk_slug, &specs_dir)` (specs_dir = assay_dir.join("specs")); match on SpecEntry::Directory { gates, .. } → set `detail_spec = Some(gates)`, `detail_spec_note = None`; match SpecEntry::Legacy → set `detail_spec = None`, `detail_spec_note = Some("Legacy flat spec — criteria not available in this view")`. Load history: call `history::list(&assay_dir, &chunk_slug)`, if non-empty take `.last()` and call `history::load(&assay_dir, &chunk_slug, run_id)`, store in `detail_run`; if empty set `detail_run = None`. Set `self.screen = Screen::ChunkDetail { milestone_slug: current_ms_slug, chunk_slug }`. In `handle_event` for `Screen::ChunkDetail { milestone_slug, .. }`: `KeyCode::Esc` → set `self.screen = Screen::MilestoneDetail { slug: milestone_slug.clone() }` (detail_milestone and detail_list_state are preserved). Implement `draw_chunk_detail(frame, spec: Option<&GatesSpec>, note: Option<&str>, run: Option<&GateRunRecord>)`: if spec is None, render a Paragraph with the note text or "No spec data"; if spec is Some, build Table rows by iterating `spec.criteria`, joining each criterion.name against `run.summary.results[*].criterion_name` to determine result icon (✓/✗/?), render with column widths 4/20/Fill. Add a `join_results` free function: `fn join_results(criteria: &[GateCriterion], run: Option<&GateRunRecord>) -> Vec<(&GateCriterion, Option<bool>)>`. Update `draw()` arm for ChunkDetail to call `draw_chunk_detail`. Fix any remaining test setup issues in `spec_browser.rs` so all 6 tests pass. Run `just ready` and fix any fmt/clippy issues.
  - Verify: `cargo test -p assay-tui spec_browser` → 6/6 pass; `cargo test -p assay-tui` → 29+ tests, 0 failed; `just ready` → all green
  - Done when: All 6 spec_browser tests pass, full `just ready` green, `cargo build -p assay-tui` produces assay-tui binary

## Files Likely Touched

- `crates/assay-tui/src/app.rs`
- `crates/assay-tui/tests/spec_browser.rs` (new)
