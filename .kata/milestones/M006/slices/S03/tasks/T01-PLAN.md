---
estimated_steps: 6
estimated_files: 2
---

# T01: Extend Screen/App types and write spec_browser contract tests

**Slice:** S03 — Chunk Detail View and Spec Browser
**Milestone:** M006

## Description

Add the two new Screen variants and five new App fields that S03 requires, wire them into the existing `draw()` and `handle_event()` as non-functional stubs, then write the full `tests/spec_browser.rs` test file whose assertions define the behavior T02 and T03 will implement.

The goal is a "red-but-compiling" state: the binary builds, existing tests still pass, and `cargo test -p assay-tui spec_browser` compiles and fails at assertion-level (not compile errors). This test file serves as the behavioral contract for MilestoneDetail and ChunkDetail for the rest of the slice.

## Steps

1. **Extend `Screen` enum** in `app.rs`: add `MilestoneDetail { slug: String }` and `ChunkDetail { milestone_slug: String, chunk_slug: String }` variants after `Wizard(WizardState)`.

2. **Extend `App` struct** in `app.rs`: add five new public fields after `project_root`:
   - `pub detail_list_state: ListState` — selection state for the chunk list in MilestoneDetail
   - `pub detail_milestone: Option<Milestone>` — loaded milestone data for MilestoneDetail and ChunkDetail
   - `pub detail_spec: Option<GatesSpec>` — loaded GatesSpec for ChunkDetail (None for Legacy or error)
   - `pub detail_spec_note: Option<String>` — diagnostic reason when detail_spec is None
   - `pub detail_run: Option<GateRunRecord>` — latest gate run record (None if no history)
   
   Add `use assay_types::{GatesSpec, GateRunRecord};` to the imports at top of app.rs. Initialize all five fields to their defaults in `with_project_root` (after the existing `list_state` initialization).

3. **Add stub match arms** in `draw()` for the new variants:
   ```rust
   Screen::MilestoneDetail { .. } => {
       let area = frame.area();
       frame.render_widget(
           Paragraph::new(Line::from("MilestoneDetail (T02)").dim()),
           area,
       );
   }
   Screen::ChunkDetail { .. } => {
       let area = frame.area();
       frame.render_widget(
           Paragraph::new(Line::from("ChunkDetail (T03)").dim()),
           area,
       );
   }
   ```

4. **Add stub match arms** in `handle_event()` for the new variants (before the closing brace):
   ```rust
   Screen::MilestoneDetail { .. } => {
       if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
           self.screen = Screen::Dashboard;
       }
       false
   }
   Screen::ChunkDetail { milestone_slug, .. } => {
       if key.code == KeyCode::Esc {
           let slug = milestone_slug.clone();
           self.screen = Screen::MilestoneDetail { slug };
       } else if key.code == KeyCode::Char('q') {
           return true;
       }
       false
   }
   ```
   Note: `milestone_slug` must be cloned before mutating `self.screen` to avoid borrow conflicts.

5. **Write `tests/spec_browser.rs`** with:
   - A `setup_project(tmp: &TempDir) -> PathBuf` helper that creates:
     - `.assay/milestones/alpha.toml` — minimal milestone with `slug = "alpha"`, `name = "Alpha"`, `status = "draft"`, `chunks = [{ slug = "c1", order = 1 }]`, `completed_chunks = []`
     - `.assay/specs/c1/gates.toml` — minimal GatesSpec with `name = "c1"` and two criteria (`[[criteria]]` entries with `name` and `description`)
   - Six test functions (use `use assay_tui::app::{App, Screen}; use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};`):
     - `enter_on_dashboard_navigates_to_milestone_detail`: construct App with project root, assert screen is Dashboard, press Enter, assert `matches!(app.screen, Screen::MilestoneDetail { ref slug } if slug == "alpha")`
     - `up_down_in_milestone_detail`: navigate to MilestoneDetail (via Enter on milestone), confirm screen is MilestoneDetail; add a second chunk ref to the fixture and verify Down then Up restores selection
     - `esc_from_milestone_detail`: navigate to MilestoneDetail, press Esc, assert `matches!(app.screen, Screen::Dashboard)`
     - `enter_on_chunk_navigates_to_chunk_detail`: navigate to MilestoneDetail, press Enter, assert `matches!(app.screen, Screen::ChunkDetail { ref milestone_slug, ref chunk_slug } if milestone_slug == "alpha" && chunk_slug == "c1")`
     - `esc_from_chunk_detail`: navigate to ChunkDetail (via MilestoneDetail Enter), press Esc, assert back to MilestoneDetail with slug "alpha"
     - `chunk_detail_no_history_all_pending`: navigate to ChunkDetail with the fixture project (which has no `.assay/results/c1/` directory), assert `app.detail_spec.is_some()` and `app.detail_run.is_none()`

6. Compile-check: `cargo build -p assay-tui` and `cargo test -p assay-tui spec_browser` (expect failures at assertions, not compile errors).

## Must-Haves

- [ ] `Screen` enum has `MilestoneDetail { slug: String }` and `ChunkDetail { milestone_slug: String, chunk_slug: String }` variants
- [ ] `App` struct has `detail_list_state`, `detail_milestone`, `detail_spec`, `detail_spec_note`, `detail_run` fields, all initialized in `with_project_root`
- [ ] `draw()` and `handle_event()` compile without exhaustive-match warnings (all Screen variants covered)
- [ ] `tests/spec_browser.rs` exists with 6 test functions and a `setup_project` helper
- [ ] `cargo build -p assay-tui` exits 0
- [ ] `cargo test -p assay-tui` existing tests (23) still pass; spec_browser tests compile and fail at assertion level

## Verification

- `cargo build -p assay-tui` → exits 0, no warnings
- `cargo test -p assay-tui` → 23 prior tests pass, spec_browser tests compile (failures expected)
- `cargo test -p assay-tui spec_browser` → 6 tests run (0 pass expected at this stage, failures at assert! not at unwrap/panic)

## Observability Impact

- Signals added/changed: `App.detail_milestone`, `App.detail_spec`, `App.detail_run` fields are now inspectable in tests and debugger to confirm navigation state
- How a future agent inspects this: `cargo test -p assay-tui spec_browser -- --nocapture` shows which assertions fail; `app.screen` variant + new detail fields fully describe navigation state
- Failure state exposed: `detail_spec_note` will carry reason strings when spec loading fails; none populated at stub stage

## Inputs

- `crates/assay-tui/src/app.rs` — current Screen/App definitions; existing draw() and handle_event() patterns to extend
- `crates/assay-tui/tests/app_state.rs` — reference pattern for test helper, key event construction, `with_project_root` usage
- `crates/assay-types/src/gates_spec.rs`, `crates/assay-types/src/gate_run.rs` — types for new App fields
- S03-RESEARCH.md — borrow-split constraints (D097), App field naming recommendations

## Expected Output

- `crates/assay-tui/src/app.rs` — Screen enum extended, App struct extended, draw()/handle_event() all arms present
- `crates/assay-tui/tests/spec_browser.rs` (new) — 6 test functions with shared fixture setup
