---
estimated_steps: 5
estimated_files: 1
---

# T02: MilestoneDetail screen — navigation and render

**Slice:** S03 — Chunk Detail View and Spec Browser
**Milestone:** M006

## Description

Replace the T01 stub for `Screen::MilestoneDetail` with a fully functional screen: Dashboard `Enter` loads real milestone data via `milestone_load`, transitions to MilestoneDetail, and the user can navigate the chunk list with ↑↓ (wrapping), confirm selection with Enter (passes to T03), and return to Dashboard with Esc.

This task makes the first three spec_browser tests pass: `enter_on_dashboard_navigates_to_milestone_detail`, `up_down_in_milestone_detail`, and `esc_from_milestone_detail`.

## Steps

1. **Wire Dashboard `Enter`** in `handle_event()` Dashboard arm: after existing key dispatches, add:
   ```rust
   KeyCode::Enter => {
       if let Some(idx) = self.list_state.selected() {
           if let Some(ms) = self.milestones.get(idx) {
               let slug = ms.slug.clone();
               let assay_dir = match &self.project_root {
                   Some(root) => root.join(".assay"),
                   None => return false,
               };
               match milestone_load(&assay_dir, &slug) {
                   Ok(loaded) => {
                       self.detail_list_state.select(
                           if loaded.chunks.is_empty() { None } else { Some(0) },
                       );
                       self.detail_milestone = Some(loaded);
                       self.screen = Screen::MilestoneDetail { slug };
                   }
                   Err(e) => {
                       self.screen = Screen::LoadError(format!(
                           "Failed to load milestone '{slug}': {e}"
                       ));
                   }
               }
           }
       }
   }
   ```
   Add `use assay_core::milestone::milestone_load;` to imports at top of app.rs.

2. **Implement MilestoneDetail `handle_event` arm**: replace the T01 stub with:
   - `KeyCode::Esc` → `self.screen = Screen::Dashboard`
   - `KeyCode::Down` → wrapping increment of `detail_list_state` bounded by chunk count from `detail_milestone`
   - `KeyCode::Up` → wrapping decrement
   - `KeyCode::Char('q')` → return `true` (quit)
   - Other keys → no-op; return `false`
   
   To get chunk count: `self.detail_milestone.as_ref().map(|m| m.chunks.len()).unwrap_or(0)`.

3. **Implement `draw_milestone_detail`** free function:
   ```rust
   fn draw_milestone_detail(
       frame: &mut ratatui::Frame,
       milestone: Option<&Milestone>,
       list_state: &mut ListState,
   )
   ```
   Layout: `[title(1), list(fill), hint(1)]` identical to draw_dashboard layout pattern.
   
   Title: `Paragraph::new(Line::from(format!(" Milestones › {} ", ms.name)).bold())` or "Loading…" if None.
   
   List: sort `ms.chunks` by `chunk.order`; for each chunk produce a `ListItem`:
   - `✓` if `chunk.slug` is in `ms.completed_chunks`, else `·`
   - Format: `"  {icon}  {slug}"`
   
   Use `List::new(items).block(Block::default().borders(Borders::ALL)).highlight_style(Style::default().bold().reversed())` and `render_stateful_widget`.
   
   Empty-chunks guard: if `chunks.is_empty()`, render a Paragraph "No chunks in this milestone." inside the bordered block instead.
   
   Hint: `Paragraph::new(Line::from("↑↓ navigate · Enter open chunk · Esc back").dim())`.

4. **Wire `draw_milestone_detail`** into `draw()`: replace the T01 stub arm:
   ```rust
   Screen::MilestoneDetail { .. } => {
       draw_milestone_detail(frame, self.detail_milestone.as_ref(), &mut self.detail_list_state);
   }
   ```
   (The `..` pattern sidesteps the borrow-split issue: we don't bind slug from the variant, so the match only holds a borrow on `self.screen`; `detail_milestone` and `detail_list_state` are separate fields and can be borrowed independently per NLL field-split rules.)

5. **Update Dashboard hint bar** in `draw_dashboard`: change the hint string from `"↑↓ navigate · n new milestone · q quit"` to `"↑↓ navigate · Enter open · n new · q quit"`.

## Must-Haves

- [ ] Dashboard `Enter` transitions to `Screen::MilestoneDetail { slug }` when a milestone is selected and `milestone_load` succeeds
- [ ] Dashboard `Enter` transitions to `Screen::LoadError(msg)` when `milestone_load` fails
- [ ] Dashboard `Enter` is a no-op when no milestone is selected (empty milestone list)
- [ ] `App.detail_milestone` is populated on successful transition to MilestoneDetail
- [ ] `App.detail_list_state` is reset to `Some(0)` (or `None` for empty chunk list) on transition
- [ ] MilestoneDetail `Esc` → `Screen::Dashboard`
- [ ] MilestoneDetail ↑↓ navigation wraps at boundaries using `detail_list_state`
- [ ] `draw_milestone_detail` renders a bordered List of chunks sorted by `chunk.order` with ✓/· status
- [ ] `cargo test -p assay-tui spec_browser::enter_on_dashboard_navigates_to_milestone_detail` passes
- [ ] `cargo test -p assay-tui spec_browser::up_down_in_milestone_detail` passes
- [ ] `cargo test -p assay-tui spec_browser::esc_from_milestone_detail` passes

## Verification

- `cargo test -p assay-tui spec_browser::enter_on_dashboard_navigates_to_milestone_detail` → PASS
- `cargo test -p assay-tui spec_browser::up_down_in_milestone_detail` → PASS
- `cargo test -p assay-tui spec_browser::esc_from_milestone_detail` → PASS
- `cargo test -p assay-tui` → 26+ tests pass (23 prior + 3 new), 0 failed
- `cargo clippy -p assay-tui --all-targets -- -D warnings` → clean

## Observability Impact

- Signals added/changed: `App.detail_milestone` is now populated on navigation — tests and debugger can inspect `.detail_milestone.as_ref().map(|m| &m.slug)` to confirm which milestone is loaded
- How a future agent inspects this: `cargo test -p assay-tui spec_browser::enter_on_dashboard_navigates_to_milestone_detail -- --nocapture` shows milestone slug in test output; `app.detail_milestone.is_some()` is a stable boolean check
- Failure state exposed: `Screen::LoadError(msg)` surfaces `milestone_load` errors inline — visible via `if let Screen::LoadError(msg) = &app.screen { assert!(msg.contains(...)) }` in tests

## Inputs

- `crates/assay-tui/src/app.rs` — T01's extended Screen/App types and stub arms to replace
- `crates/assay-tui/tests/spec_browser.rs` — T01's 3 MilestoneDetail tests whose assertions this task must satisfy
- `crates/assay-core/src/milestone/mod.rs` — `milestone_load(assay_dir, slug)` signature; returns `Milestone` with `chunks: Vec<ChunkRef>`, `completed_chunks: Vec<String>`
- `crates/assay-types/src/milestone.rs` — `ChunkRef { slug: String, order: u32 }`; sort by `.order`
- S01-SUMMARY.md Forward Intelligence: `assay_dir = project_root.join(".assay")` path contract; borrow-split pattern (D097)

## Expected Output

- `crates/assay-tui/src/app.rs` — Dashboard Enter wired; MilestoneDetail handle_event arm complete; `draw_milestone_detail` free function added; `draw()` MilestoneDetail arm wired
- 3 of 6 spec_browser tests pass
