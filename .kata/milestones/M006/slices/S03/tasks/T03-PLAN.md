---
estimated_steps: 7
estimated_files: 2
---

# T03: ChunkDetail screen — spec+history load, criterion join, Table render

**Slice:** S03 — Chunk Detail View and Spec Browser
**Milestone:** M006

## Description

Implement the ChunkDetail screen: MilestoneDetail `Enter` loads the chunk's spec (`load_spec_entry_with_diagnostics`) and latest gate run (`history::list` + `history::load`), populates `App.detail_spec` and `App.detail_run`, and transitions to `Screen::ChunkDetail`. `draw_chunk_detail` renders a Ratatui `Table` with one row per criterion showing a pass/fail/pending icon, the criterion name, and its description. Esc from ChunkDetail returns to `Screen::MilestoneDetail` preserving navigation context. All 6 spec_browser tests must pass and `just ready` must be green.

## Steps

1. **Add imports** to app.rs:
   ```rust
   use assay_core::spec::{load_spec_entry_with_diagnostics, SpecEntry};
   use assay_core::history;
   use ratatui::widgets::{Cell, Row, Table};
   ```
   (`Table`, `Cell`, `Row` are re-exported from `ratatui::widgets` in ratatui 0.30.)

2. **Implement `join_results` free function** in app.rs (private):
   ```rust
   fn join_results<'a>(
       criteria: &'a [assay_types::Criterion],
       run: Option<&'a GateRunRecord>,
   ) -> Vec<(&'a assay_types::Criterion, Option<bool>)>
   ```
   For each criterion in `criteria`: look up by `criterion.name` in `run.summary.results` where `r.criterion_name == criterion.name`. If not found → `(criterion, None)` (Pending). If found with `r.result = Some(gate_result)` → `(criterion, Some(gate_result.passed))`. If found with `r.result = None` (skip_serializing_if field) → `(criterion, None)` (Pending/skipped). Return the joined vec.

3. **Implement `draw_chunk_detail`** free function:
   ```rust
   fn draw_chunk_detail(
       frame: &mut ratatui::Frame,
       chunk_slug: &str,
       spec: Option<&GatesSpec>,
       spec_note: Option<&str>,
       run: Option<&GateRunRecord>,
   )
   ```
   Layout: `[title(1), table_or_message(fill), hint(1)]`.
   
   Title: `"  {chunk_slug}  — Criteria"` or use milestone breadcrumb if available (keep simple: just chunk slug).
   
   If `spec` is `None`: render `Paragraph::new(Line::from(spec_note.unwrap_or("No spec data")).dim())` inside bordered block.
   
   If `spec` is `Some(gs)`: call `join_results(&gs.criteria, run)`. Build `Table` rows:
   - For each `(criterion, result_opt)`:
     - Icon cell: `"✓"` if `Some(true)`, `"✗"` if `Some(false)`, `"?"` if `None`
     - Name cell: `&criterion.name`
     - Description cell: `&criterion.description`
   - Style: icon cell color — green for ✓, red for ✗, dim for ?
   - Column widths: `[Constraint::Length(3), Constraint::Length(24), Constraint::Fill(1)]`
   - Block: `Block::default().borders(Borders::ALL).title(format!(" {} criteria ", gs.name))`
   - `render_widget(table, table_area)` (plain widget, no TableState — read-only view)
   
   Hint: `"Esc back"` (no navigation in ChunkDetail).

4. **Wire `draw_chunk_detail`** into `draw()`: replace the T01 stub ChunkDetail arm:
   ```rust
   Screen::ChunkDetail { chunk_slug, .. } => {
       let slug = chunk_slug.clone(); // drop borrow of self.screen before further borrows
       draw_chunk_detail(
           frame,
           &slug,
           self.detail_spec.as_ref(),
           self.detail_spec_note.as_deref(),
           self.detail_run.as_ref(),
       );
   }
   ```
   Clone `chunk_slug` into a local first to avoid holding the borrow of `self.screen` while borrowing other fields.

5. **Implement MilestoneDetail `Enter`** in `handle_event()` MilestoneDetail arm (after T02's ↑↓/Esc implementation):
   Add `KeyCode::Enter`:
   ```rust
   KeyCode::Enter => {
       if let Some(ms) = &self.detail_milestone {
           if let Some(idx) = self.detail_list_state.selected() {
               // Sort chunks by order to match display order.
               let mut sorted_chunks = ms.chunks.clone();
               sorted_chunks.sort_by_key(|c| c.order);
               if let Some(chunk) = sorted_chunks.get(idx) {
                   let chunk_slug = chunk.slug.clone();
                   let milestone_slug = ms.slug.clone();
                   let assay_dir = match &self.project_root {
                       Some(root) => root.join(".assay"),
                       None => return false,
                   };
                   let specs_dir = assay_dir.join("specs");
                   // Load spec entry.
                   match load_spec_entry_with_diagnostics(&chunk_slug, &specs_dir) {
                       Ok(SpecEntry::Directory { gates, .. }) => {
                           self.detail_spec = Some(gates);
                           self.detail_spec_note = None;
                       }
                       Ok(SpecEntry::Legacy { .. }) => {
                           self.detail_spec = None;
                           self.detail_spec_note = Some(
                               "Legacy flat spec — criteria not available in this view"
                                   .to_string(),
                           );
                       }
                       Err(e) => {
                           self.detail_spec = None;
                           self.detail_spec_note = Some(format!("Failed to load spec: {e}"));
                       }
                   }
                   // Load latest gate run (empty history is not an error).
                   self.detail_run = match history::list(&assay_dir, &chunk_slug) {
                       Ok(ids) if !ids.is_empty() => {
                           let run_id = ids.last().unwrap().clone();
                           history::load(&assay_dir, &chunk_slug, &run_id).ok()
                       }
                       _ => None,
                   };
                   self.screen = Screen::ChunkDetail {
                       milestone_slug,
                       chunk_slug,
                   };
               }
           }
       }
   }
   ```

6. **Implement ChunkDetail `Esc`** in `handle_event()` ChunkDetail arm (replace T01 stub):
   ```rust
   Screen::ChunkDetail { milestone_slug, .. } => {
       match key.code {
           KeyCode::Esc => {
               let slug = milestone_slug.clone();
               self.screen = Screen::MilestoneDetail { slug };
               // detail_milestone and detail_list_state are preserved intentionally.
           }
           KeyCode::Char('q') => return true,
           _ => {}
       }
       false
   }
   ```

7. **Make all spec_browser tests pass**: review each failing test and fix any test setup issues (e.g., ensure the fixture writes a syntactically valid `[[criteria]]` TOML block; ensure history directory is absent for the `no_history_all_pending` test; fix any type import issues in spec_browser.rs). Run `just ready` and fix any fmt/clippy issues (particularly: ensure `join_results` and all new fns have correct lifetimes; no `clippy::too_many_arguments` if needed).

## Must-Haves

- [ ] `join_results` free function correctly maps: criterion in spec with no match in run → `None` (Pending); criterion with matching run result `passed: true` → `Some(true)`; criterion with matching run result `passed: false` → `Some(false)`; criterion with run result `result: None` (skipped) → `None` (Pending)
- [ ] MilestoneDetail `Enter` loads spec via `load_spec_entry_with_diagnostics` and latest run via `history::list` + `history::load`; empty history sets `detail_run = None` without error
- [ ] MilestoneDetail `Enter` with `SpecEntry::Directory` sets `detail_spec = Some(gates)` and `detail_spec_note = None`
- [ ] MilestoneDetail `Enter` with `SpecEntry::Legacy` sets `detail_spec = None` and `detail_spec_note = Some("Legacy flat spec...")`
- [ ] ChunkDetail `Esc` transitions to `Screen::MilestoneDetail { slug: milestone_slug }` preserving `detail_milestone` and `detail_list_state`
- [ ] `draw_chunk_detail` renders a `Table` with three columns (icon/name/description) when `spec` is `Some`
- [ ] `draw_chunk_detail` renders a Paragraph with the `spec_note` message when `spec` is `None`
- [ ] All 6 `cargo test -p assay-tui spec_browser` tests pass
- [ ] `cargo test -p assay-tui` → 29+ tests, 0 failed
- [ ] `just ready` → fmt ✓, lint ✓, test ✓, deny ✓

## Verification

- `cargo test -p assay-tui spec_browser` → 6/6 PASS
- `cargo test -p assay-tui` → 0 failed (29+ total)
- `cargo test --workspace` → workspace green
- `just ready` → "All checks passed"

## Observability Impact

- Signals added/changed: `App.detail_spec.is_some()` — stable boolean check confirming spec was loaded as Directory entry; `App.detail_spec_note.as_deref()` — carries reason string when spec is None (Legacy or I/O error); `App.detail_run.is_some()` — confirms whether gate history exists for the chunk
- How a future agent inspects this: `cargo test -p assay-tui spec_browser::chunk_detail_no_history_all_pending -- --nocapture`; `app.detail_spec.is_some()` and `app.detail_run.is_none()` are directly testable without a terminal; `draw_chunk_detail` renders "?" for all criteria in no-history case — visually inspectable in UAT
- Failure state exposed: `detail_spec_note` carries load-failure message for debugging; `Screen::ChunkDetail` being active while `detail_spec.is_none()` surfaces as the note-message screen, not a panic

## Inputs

- `crates/assay-tui/src/app.rs` — T01 stub + T02 MilestoneDetail impl to extend
- `crates/assay-tui/tests/spec_browser.rs` — T01's 6 tests; the last 3 (enter_on_chunk, esc_from_chunk, no_history_all_pending) need this task's implementation to pass
- `crates/assay-core/src/spec/mod.rs` — `load_spec_entry_with_diagnostics(slug, specs_dir)` returns `Result<SpecEntry>`; `SpecEntry::Directory { slug, gates, .. }` extracts `GatesSpec`; `specs_dir = assay_dir.join("specs")`
- `crates/assay-core/src/history/mod.rs` — `list(assay_dir, spec_name) -> Result<Vec<String>>` (sorted oldest-first; empty if no history dir — not an error); `load(assay_dir, spec_name, run_id) -> Result<GateRunRecord>`
- `crates/assay-types/src/gates_spec.rs` — `GatesSpec.criteria: Vec<GateCriterion>` where `GateCriterion = Criterion`; `Criterion { name, description, .. }`
- `crates/assay-types/src/gate_run.rs` — `GateRunRecord.summary.results: Vec<CriterionResult>`; `CriterionResult { criterion_name: String, result: Option<GateResult>, .. }`; `GateResult { passed: bool, .. }`
- S03-RESEARCH.md — pitfall: "criterion name join mismatch"; pitfall: "empty history is not an error"; pitfall: "SpecEntry::Legacy cannot provide GatesSpec"

## Expected Output

- `crates/assay-tui/src/app.rs` — `join_results` free function; `draw_chunk_detail` free function; MilestoneDetail `Enter` wired; ChunkDetail `Esc` wired; `draw()` ChunkDetail arm wired
- `crates/assay-tui/tests/spec_browser.rs` — all 6 tests passing
- All 29+ `cargo test -p assay-tui` tests green; `just ready` passing
