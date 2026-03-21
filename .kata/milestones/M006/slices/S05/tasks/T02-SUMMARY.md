---
id: T02
parent: S05
milestone: M006
provides:
  - Global layout split in App::draw producing content_area + status_area
  - draw_status_bar free function (project name · cycle slug · dim hints)
  - draw_help_overlay free function (centered bordered keybinding table with Clear backing)
  - "? key handler: sets show_help=true from any non-wizard screen"
  - Help-overlay event guard (only ? or Esc dismiss; all other keys no-op)
  - Event::Resize handler in run() calls terminal.clear()
  - All draw_* signatures refactored to accept area: Rect parameter
  - All 6 help_status contract tests green
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/wizard.rs
  - crates/assay-tui/src/main.rs
key_decisions:
  - draw_help_overlay receives frame.area() (full screen) so it can compute its own centered popup geometry; this is correct — the overlay needs to know the full terminal size to center itself, independent of the content/status layout split
  - AppConfig struct added to app.rs as a minimal project-config surface for the status bar; kept thin (project_name only) since full config loading is deferred to a future slice
patterns_established:
  - Global layout split pattern: App::draw splits frame.area() into [content_area, status_area] first; all screen renderers receive content_area; status bar and overlays receive their own areas
  - Overlay-last rendering: draw_help_overlay called after all screen renderers so it renders on top of everything
  - Help-overlay event guard placed at top of handle_event before screen dispatch — ensures overlay intercepts all keys regardless of active screen
observability_surfaces:
  - app.show_help field — true means overlay is visible; directly testable; inspect when diagnosing unexpected key-press behavior
  - app.cycle_slug field — Some(slug) when InProgress milestone exists; None otherwise; status bar renders blank slug rather than panicking
  - draw_status_bar is a pure render function — no I/O side effects; status bar degradation is empty project name when app.config is None
duration: ~30min
verification_result: passed
completed_at: 2026-03-21
blocker_discovered: false
---

# T02: Global layout split, status bar, help overlay, `?` key, resize fix, final just ready

**Implemented full S05 deliverables: global layout split, status bar, centered help overlay, `?` key handler, help-overlay event guard, `Event::Resize` handling, and `draw_*` signature refactor — all 6 `help_status` tests green, `just ready` passes.**

## What Happened

All 8 steps executed in one pass:

1. **Signature refactor**: Updated all `draw_*` functions in `app.rs` to accept `area: Rect` as explicit parameter, removing all internal `frame.area()` calls from helper bodies. Updated `draw_wizard` in `wizard.rs` similarly (added `Rect` to imports, changed signature to `(frame, area: Rect, state)`).

2. **Global layout split**: `App::draw` now splits `frame.area()` into `[content_area, status_area]` via `Layout::vertical([Fill(1), Length(1)])`. Every screen renderer receives `content_area`; the status bar gets `status_area`.

3. **`draw_status_bar`**: Free function rendering project name, separator, cycle slug (dim), separator, `? help  q quit` (dim) as a single `Paragraph` of `Span`s. Gracefully renders empty project name and blank slug.

4. **`draw_help_overlay`**: Free function computing a centered `w=min(62,area.width)` × `h=22` popup rect, rendering `Clear` behind it, then a `Block::bordered()` title, then a two-column `Table` of keybinding rows grouped by section (Global, Dashboard, Detail views, Wizard, Settings).

5. **`?` key handler and event guard**: Added at top of `handle_event` — when `show_help` is true, only `?` or `Esc` dismiss it; all other keys return false immediately. The global `?` handler below sets `show_help = true` from any non-wizard screen.

6. **Settings cycle_slug refresh**: No `settings.rs` exists in this branch (S04 not yet landed). The step was a no-op; the wizard path already refreshes `cycle_slug` on submit.

7. **`Event::Resize` fix**: Replaced `if let Event::Key(key) = event::read()?` with a `match` arm handling both `Key` (drive `handle_event`) and `Resize` (call `terminal.clear()`).

8. **`just ready`**: Required one `cargo fmt` pass to reformat long import lines and some long `Row::new` expressions. All checks then green.

## Verification

- `cargo test -p assay-tui --test help_status` → **6/6 pass** (question_mark_opens_help, question_mark_again_closes_help, esc_closes_help_when_open, show_help_starts_false, cycle_slug_none_for_draft_milestone, cycle_slug_some_for_in_progress_milestone)
- `cargo test -p assay-tui` → **22 tests, 0 failures** (no regressions from signature refactor)
- `cargo test --workspace` → all crates pass (769 assay-core, 125 assay-mcp, 131 assay-types, etc.)
- `just ready` → fmt ✓, lint ✓, test ✓, deny ✓ — **"All checks passed"**
- `grep "frame.area()" crates/assay-tui/src/app.rs` — 2 matches, both in `App::draw` (the layout split and the overlay call); no `draw_*` helper body calls `frame.area()` internally

## Diagnostics

- Inspect `app.show_help` in tests or with a debugger to confirm overlay visibility state
- Inspect `app.cycle_slug` to confirm status bar has the correct milestone slug
- Resize events now call `terminal.clear()` — redraw artifacts after resize are gone; visible as immediate clean repaint on next frame

## Deviations

- **Step 6 (cycle_slug refresh in Settings save path)**: `settings.rs` does not exist in this branch (S04 not yet merged). Step was a no-op. The existing wizard submit path already refreshes `cycle_slug`. No functional gap.
- **`AppConfig` struct added**: The task plan referenced `self.config.as_ref().map(|c| c.project_name.as_str())` but no `AppConfig` type existed. Added a minimal `AppConfig { project_name: String }` struct to support the status bar render path; field defaults to `None`. Consistent with the plan's intent.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — global layout split; all `draw_*` signatures accept `area: Rect`; `draw_status_bar` and `draw_help_overlay` free functions; `?` handler + help-overlay guard in `handle_event`; `AppConfig` struct
- `crates/assay-tui/src/wizard.rs` — `draw_wizard` signature updated to `(frame, area: Rect, state)`; `Rect` added to layout imports
- `crates/assay-tui/src/main.rs` — `run()` uses `match event::read()?` handling both `Key` and `Resize` variants
