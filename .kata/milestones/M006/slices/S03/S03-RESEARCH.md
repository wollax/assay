# S03: Chunk Detail View and Spec Browser — Research

**Researched:** 2026-03-20
**Domain:** Ratatui TUI, assay-core data APIs, Rust borrow checker patterns
**Confidence:** HIGH

## Summary

S03 adds two new screens — MilestoneDetail (chunk list) and ChunkDetail (criteria + gate results) — by extending the existing `Screen` enum and `App` struct in `app.rs`. The existing S01/S02 architecture is the right foundation: the same borrow-split pattern that makes the Dashboard arm work (`match &self.screen` + independent `&mut self.list_state` field) applies directly to the new screens.

The `assay-core` APIs are all synchronous and fully tested: `milestone_load` for fresh milestone data, `spec::load_spec_entry_with_diagnostics` for GatesSpec criteria, `history::list` + `history::load` for the latest gate run. Data is loaded in `handle_event` on navigation transitions — not inside `terminal.draw()` — per D091. No new dependencies are needed; `assay-core` and `assay-types` are already in `assay-tui/Cargo.toml`.

The riskiest part is the criteria-to-run-result join: `GatesSpec.criteria` (from the spec file) and `GateRunRecord.summary.results` (from the history file) must be matched by `criterion_name` / `name` field. A criterion with no matching result entry is **Pending** (`result: None`). A `CriterionResult` with `result: None` (the `skip_serializing_if` field) is also Pending. The gate run record has a `results` vec that may be shorter than the spec's criteria vec (e.g. if the run was interrupted or a subset was evaluated), so the join must tolerate missing entries in either direction.

## Recommendation

**Architecture: thin Screen variants + App-level detail fields.** Store only navigation keys (`slug`, `milestone_slug`, `chunk_slug`) inside the Screen variants; store loaded data in new App fields (`detail_milestone`, `detail_list_state`, `detail_spec`, `detail_run`). This preserves the existing `match &self.screen` pattern in `draw()` and the `match self.screen` pattern in `handle_event()`, consistent with D097 and the established S01 borrow-split idiom. Putting list_state inside the Screen variant would force `match &mut self.screen` in `draw()`, which is a larger refactor than necessary.

Use a `List` widget (same as Dashboard) for MilestoneDetail's chunk list. Use a `Table` widget for ChunkDetail's criteria pane: three columns — result icon (✓/✗/?), criterion name, description. This renders cleanly within terminal width without needing scroll state for typical spec sizes (≤10 criteria).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Loading milestone from disk | `assay_core::milestone::milestone_load(assay_dir, slug)` | Same atomic-read, validates TOML, sorts chunks; already used by S01 |
| Loading spec/gates criteria | `assay_core::spec::load_spec_entry_with_diagnostics(slug, specs_dir)` | Returns `SpecEntry` with `GatesSpec` including criteria; enriches not-found errors |
| Listing gate run history | `assay_core::history::list(assay_dir, spec_name)` | Returns sorted run IDs (oldest first); empty if no history — not an error |
| Loading a gate run record | `assay_core::history::load(assay_dir, spec_name, run_id)` | Returns full `GateRunRecord` with `CriterionResult` per criterion |
| Chunk list with selection | `ratatui::widgets::{List, ListItem, ListState}` + `render_stateful_widget` | Already imported; same stateful widget used for Dashboard milestone list |
| Criteria table | `ratatui::widgets::{Table, Row, Cell}` | Built into `ratatui` 0.30; `use ratatui::widgets::{Cell, Row, Table}` |
| Path safety | `assay_core::history::validate_path_component` (pub(crate)) | Not public, but `milestone_load` and `history::list` already call it; pass slug strings that came from parsed TOML — they're already validated |

## Existing Code and Patterns

- `crates/assay-tui/src/app.rs` — All new code goes here. Screen enum, App struct, draw(), handle_event() are the extension points. Screen variants are `pub enum Screen` (no derive Copy/Clone). handle_event() matches `self.screen` with destructuring patterns. draw() matches `&self.screen` with immutable borrows.
- `crates/assay-tui/src/wizard.rs` — Pattern reference for a self-contained screen module. MilestoneDetail/ChunkDetail don't need separate module files — they can live directly in `app.rs` as free functions.
- `crates/assay-tui/tests/app_wizard.rs` — Integration test pattern: construct `App::with_project_root(Some(tmp_root))`, drive via `handle_event(key(...))`, assert on `app.screen`. S03 tests follow the same pattern.
- `crates/assay-core/src/milestone/mod.rs` — `milestone_load(assay_dir, slug)`. Takes `.assay/` directory (NOT project root). Returns `Milestone` with `chunks: Vec<ChunkRef>` sorted by slug, `completed_chunks: Vec<String>`, `status: MilestoneStatus`.
- `crates/assay-core/src/spec/mod.rs` — `load_spec_entry_with_diagnostics(slug, specs_dir)` where `specs_dir = assay_dir.join("specs")`. Returns `Result<SpecEntry>`. To extract GatesSpec: `match entry { SpecEntry::Directory { gates, .. } => gates, SpecEntry::Legacy { spec, .. } => /* convert or show error */ }`. Wizard-created specs are always `Directory` variant.
- `crates/assay-core/src/history/mod.rs` — `list(assay_dir, spec_name)` returns `Vec<String>` sorted chronologically. `load(assay_dir, spec_name, run_id)` returns `GateRunRecord`. Latest run: `list(...).last()` then `load(...)`. If `list` returns empty, no history → all criteria are Pending.
- `crates/assay-types/src/gates_spec.rs` — `GatesSpec { name, criteria: Vec<GateCriterion> }` where `GateCriterion = Criterion { name, description, cmd, enforcement, .. }`.
- `crates/assay-types/src/gate_run.rs` — `GateRunRecord { summary: GateRunSummary }`, `GateRunSummary { results: Vec<CriterionResult> }`, `CriterionResult { criterion_name, result: Option<GateResult>, enforcement }`, `GateResult { passed: bool, .. }`. The `results` vec may be shorter than spec criteria count.
- `crates/assay-types/src/milestone.rs` — `Milestone { chunks: Vec<ChunkRef>, completed_chunks: Vec<String>, status }`. `ChunkRef { slug, order }`. Chunk is complete if `slug in milestone.completed_chunks`. Active chunk is `min-order ChunkRef` not in `completed_chunks`.

## Constraints

- **Borrow checker (D097):** Preserve `match &self.screen` in `draw()`. Extend App with `detail_list_state: ListState`, `detail_milestone: Option<Milestone>`, `detail_spec: Option<GatesSpec>`, `detail_run: Option<GateRunRecord>`. Pass these as individual arguments to render fns — do NOT pass `&mut self` to render fns.
- **Sync-only data loading (D091):** Load everything in `handle_event` on navigation, before the next `draw()` call. No background threads needed for typical spec sizes.
- **assay_dir vs project_root:** `milestone_load` and `history::list` take `assay_dir` (`project_root.join(".assay")`). `spec::load_spec_entry_with_diagnostics` takes `specs_dir` (`assay_dir.join("specs")`). Do not confuse these.
- **Zero traits (D001):** New render functions are free functions (`draw_milestone_detail`, `draw_chunk_detail`), not Widget trait impls.
- **SpecEntry variants:** Wizard-created specs are `Directory` (have `GatesSpec`). Legacy flat `.toml` specs are `Legacy` (have `Spec`, which has criteria as `Vec<Criterion>`). Both should be renderable — either extract `gates` from Directory or show a fallback "Legacy spec" message for Legacy entries.
- **No new dependencies:** `ratatui::widgets::{Table, Row, Cell}` are already in the workspace dep `ratatui = "0.30"`.

## Common Pitfalls

- **Criterion name join mismatch:** `GatesSpec.criteria[i].name` is the spec's criterion name. `CriterionResult.criterion_name` is what the run recorded. These must match exactly for the join. If the spec was updated (criterion renamed) after a run, the join produces "orphan" run results and "new" unrun criteria. The render function must handle both: criteria not in the run show as Pending; run results with no matching spec criterion can be safely ignored.
- **Empty history is not an error:** `history::list()` returns `Ok(vec![])` when `.assay/results/<slug>/` doesn't exist. This is correct — chunk has never been gate-checked. Render all criteria as `?` Pending. Do NOT treat this as a render error.
- **SpecEntry::Legacy cannot provide GatesSpec:** The `load_spec_entry_with_diagnostics` function may return a `Legacy` entry if the spec file is a flat `.toml`. In that case there is no `GatesSpec` — render a one-line "Legacy flat spec — criteria not available in this view" placeholder rather than panicking.
- **completed_chunks vs gate results:** `milestone.completed_chunks` tracks cycle-advance completion (the user ran `cycle_advance`), which is separate from gate run history. A chunk can appear in `completed_chunks` without having gate run records (if the user advanced manually). Show gate history from `history::list/load`, not from `completed_chunks`.
- **ListState stale selection on re-entry:** When transitioning back from ChunkDetail to MilestoneDetail via Esc, `detail_list_state` retains its previous selection — this is correct and expected behavior (user's position is preserved). When transitioning INTO MilestoneDetail fresh from Dashboard, reset `detail_list_state.select(Some(0))` if chunks are non-empty.
- **milestone_load vs App.milestones lookup:** `App.milestones` (from `milestone_scan`) already contains milestone data. For MilestoneDetail we can use `milestone_load` to get fresh data OR look up from `App.milestones`. Prefer `milestone_load` on navigation — it ensures we have the most current `completed_chunks` state. The scan result may be stale if the user ran `cycle_advance` in another terminal while the TUI is open.
- **handle_event Screen::MilestoneDetail | Screen::ChunkDetail without ref bindings:** The existing `handle_event` matches `self.screen` (by value). New arms must not move fields out of the screen variants. Use `ref slug` or `ref milestone_slug` bindings, or match via `matches!` guard then access via `if let`. Alternatively, restructure to `match &mut self.screen` in handle_event (this is a larger but cleaner change — evaluate in planning).

## Open Risks

- **Borrow checker in handle_event:** The current `match self.screen` pattern works for unit variants (`Dashboard`) and variants that contain state that is immediately re-assigned (`Wizard(ref mut state)`). Adding MilestoneDetail navigation requires: (a) reading the current list selection from `self.detail_list_state`, (b) reading chunk slug from `self.detail_milestone`, (c) setting `self.screen` to ChunkDetail. Step (c) requires mutating `self.screen` while we've already started a match on it. This is the same pattern as the Wizard Submit arm — which does `self.screen = Screen::Dashboard` inside the Wizard arm. That works. The new arms follow the same pattern: read needed data into local variables first, then mutate `self.screen` last.
- **Table widget StatefulWidget vs plain Widget:** `Table` can be rendered as a plain widget (no selection highlighting needed for ChunkDetail read-only view) or as a stateful widget (with `TableState` for row selection). For S03, plain `render_widget` is sufficient — criteria list is read-only. If S05 adds criterion selection for drill-down, `TableState` can be added then.
- **Large criteria lists:** Specs with many criteria (>30) will overflow a standard terminal without scroll. For S03, this is acceptable — typical specs have 5–15 criteria. If vertical overflow is observed during integration testing, add a basic scroll offset or truncate with "... and N more" indicator.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | (no dedicated skill available) | none found |

## Sources

- Codebase: `crates/assay-tui/src/app.rs` — Screen enum, App struct, draw() + handle_event() patterns; established borrow-split idiom (D097) (HIGH confidence)
- Codebase: `crates/assay-core/src/milestone/mod.rs` — milestone_load/scan public API, path contracts (HIGH confidence)
- Codebase: `crates/assay-core/src/spec/mod.rs` — load_spec_entry_with_diagnostics, SpecEntry variants, specs_dir path convention (HIGH confidence)
- Codebase: `crates/assay-core/src/history/mod.rs` — list/load API, empty-dir returns Ok(vec![]), path safety (HIGH confidence)
- Codebase: `crates/assay-types/src/gates_spec.rs` — GatesSpec.criteria field, GateCriterion = Criterion type alias (HIGH confidence)
- Codebase: `crates/assay-types/src/gate_run.rs` — GateRunRecord, CriterionResult, GateResult.passed (HIGH confidence)
- Codebase: `crates/assay-types/src/milestone.rs` — Milestone.chunks, Milestone.completed_chunks, ChunkRef.slug/order (HIGH confidence)
- Ratatui 0.30 source: `~/.cargo/registry/src/.../ratatui-0.30.0/src/widgets.rs` — Table/Row/Cell exported from ratatui::widgets (HIGH confidence)
- Ratatui 0.30 source: `~/.cargo/registry/src/.../ratatui-widgets-0.3.0/src/table/row.rs` — Row::new() API, Cell::from() construction (HIGH confidence)
