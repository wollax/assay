# S05: TUI Analytics Screen — Research

**Date:** 2026-03-24

## Summary

S05 adds a `Screen::Analytics` variant to the TUI that renders gate failure frequency as a heatmap-style table and milestone velocity as a summary table. The data source is `compute_analytics()` from `assay-core::history::analytics` (delivered by S04). The `a` key from Dashboard transitions to the analytics screen; `Esc`/`q` returns to Dashboard.

This is a low-risk, self-contained UI slice. All data types and compute functions already exist and are tested. The work is purely TUI rendering + event handling — no new domain logic, no new types in assay-types, no schema changes. The established patterns for adding a new screen (Screen variant + draw function + handle_event match arm) are well-documented across S01–S04 of M006 and M007.

The main risk is the borrow-checker dance when adding a new Screen variant to the `draw()` match — D097/D098 document the proven patterns (pass individual fields, use `..` in match arms).

## Recommendation

Follow the exact pattern established by `Screen::Settings` and `Screen::McpPanel`:

1. Add `Screen::Analytics` variant (no fields — data stored on `App`)
2. Add `App.analytics_report: Option<AnalyticsReport>` field
3. `a` key in Dashboard calls `compute_analytics(&assay_dir)`, stores result, transitions screen
4. `draw_analytics(frame, area, report)` free function renders two tables
5. `Esc`/`q` returns to Dashboard
6. Integration tests drive synthetic key events and assert screen transitions

No separate module file needed — the draw function and event handling fit naturally in `app.rs` alongside the other screen renderers. If the draw function exceeds ~80 lines, extract to a `analytics.rs` module (like `mcp_panel.rs`).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Analytics data aggregation | `compute_analytics()` in `assay-core::history::analytics` | Already tested with 14 tests; returns `AnalyticsReport` with all needed fields |
| Table rendering | `ratatui::widgets::Table` | Used in draw_chunk_detail, draw_settings, draw_help_overlay — established pattern |
| Screen transition pattern | D089 Screen enum + D097 field-passing pattern | Every screen follows this; deviation would be inconsistent |
| Color-coding failure rates | S04's ANSI threshold logic (>50% red, >0% yellow, 0% green) | Port the same thresholds to ratatui `Style::fg(Color::Red/Yellow/Green)` |

## Existing Code and Patterns

- `crates/assay-core/src/history/analytics.rs` — `compute_analytics(&Path) -> Result<AnalyticsReport>` is the single entry point. Returns `FailureFrequency` (spec_name, criterion_name, fail_count, total_runs, enforcement) and `MilestoneVelocity` (milestone_slug, chunks_completed, days_elapsed, chunks_per_day). All types derive Serialize/Deserialize.
- `crates/assay-tui/src/app.rs:1566` — `draw_settings()` is the closest pattern for a full-screen data display: takes frame + area + individual fields, renders a bordered block with a table. Follow this exactly.
- `crates/assay-tui/src/app.rs:610` — Dashboard event handler shows the key-binding dispatch pattern. `a` key is currently unbound — add it here after the `s` (settings) handler.
- `crates/assay-tui/src/app.rs:465` — `draw()` method shows the Screen dispatch. Add `Screen::Analytics` arm calling `draw_analytics()`.
- `crates/assay-tui/tests/pr_status_panel.rs` — Demonstrates testing App state mutations without rendering. Good pattern for analytics screen tests.
- `crates/assay-tui/tests/spec_browser.rs` — Demonstrates testing screen navigation via synthetic `KeyEvent`s. Use same `key()` helper pattern.
- `crates/assay-cli/src/commands/history.rs` — CLI analytics formatter has the failure rate calculation (`fail_count as f64 / total_runs as f64 * 100.0`) and column formatting. Port the display logic but use ratatui Table/Row/Cell instead of println.

## Constraints

- **D097**: Screen-specific render functions take individual fields, not `&mut App` — borrow checker requirement with stateful widgets.
- **D098**: Use `..` pattern in draw() match arms to avoid binding variant fields.
- **D105**: All `draw_*` functions accept explicit `area: Rect` — `draw()` does the global layout split once.
- **D089**: `App` owns all state; `Screen` drives dispatch.
- **D099**: App-level fields for loaded data (not inside Screen variants) — preserves the existing `match &self.screen` pattern in `draw()`.
- **D091**: Data loading is synchronous, on navigation transitions only — call `compute_analytics` when `a` is pressed, not on every frame.
- **Zero-trait convention (D001)**: `draw_analytics` is a free function, not a Widget trait impl.
- `assay-tui` does not depend on `tracing` — use `eprintln` for warnings (D125).

## Common Pitfalls

- **Borrow-checker with Screen variant data** — Don't store analytics data in the Screen variant. Store `analytics_report: Option<AnalyticsReport>` on App (D099). The draw function reads `self.analytics_report.as_ref()` outside the match.
- **Empty analytics report** — `compute_analytics` can return an empty report (no history, no milestones). The draw function must handle this gracefully with a "No analytics data" message, not an empty screen.
- **Float formatting in velocity** — `chunks_per_day` is f64. Use `{:.1}` or `{:.2}` format to avoid ugly long decimals. S04 CLI already does this — match its formatting.
- **Table column widths** — Fixed `Constraint::Length` for short columns (spec name, counts), `Constraint::Fill` for the last column. Avoids horizontal overflow on narrow terminals.
- **Missing .assay directory** — `compute_analytics` requires an assay_dir. When `project_root` is `None`, `a` should be a no-op (same as `r` and `n` guards).
- **Help overlay update** — The help overlay (`draw_help_overlay`) should be updated to mention the `a` key for Analytics. If omitted, users won't discover the feature.

## Open Risks

- **Large history sets** — If `.assay/results/` has thousands of records, `compute_analytics` could take noticeable time on the `a` keypress. D091 says "revisit with std::thread if profiling shows latency" — for S05, sync is fine; add a TODO comment if concerned.
- **Screen::Analytics variant exhaustiveness** — Adding a new Screen variant requires updating ALL match arms in `handle_event` and `draw`. Missing one causes a compile error (good — the compiler catches it). But the help overlay text is not compiler-enforced — easy to forget.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | — | No skill needed — well-established in codebase with 7 existing screens |

## Sources

- S04-SUMMARY.md Forward Intelligence section — authoritative API contract for analytics types
- D097, D098, D099, D105 — screen rendering conventions
- D089, D091 — App architecture and data loading conventions
- M008-ROADMAP.md S05 definition — `a` key from Dashboard, gate failure heatmap, milestone velocity
