---
id: T02
parent: S01
milestone: M006
provides:
  - "draw_dashboard(frame, milestones, list_state) — renders bordered List with name/badge/progress per milestone"
  - "draw_no_project(frame) — renders bold red centered message when .assay/ is absent"
  - "main() data loading: milestone_scan(&assay_dir) + config::load(&project_root) before App construction"
  - "App.screen = Screen::NoProject set at startup when .assay/ missing; Screen::Dashboard otherwise"
  - "render_stateful_widget for List so ListState selection (↑↓ keys) works"
key_files:
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/src/main.rs
key_decisions:
  - "draw_dashboard takes (frame, &[Milestone], &mut ListState) separately — avoids borrow conflict from matching on &app.screen while also needing &mut app.list_state"
  - "draw() uses early-return matches! guards for Dashboard and NoProject arms instead of a single match &app.screen — same borrow-checker motivation"
  - "milestone_scan errors use unwrap_or_default() for graceful degradation (empty vec, no panic)"
  - "config::load errors use .ok() — None stored in App.config, not propagated as fatal error"
patterns_established:
  - "discriminant-first borrow pattern in draw(): matches!(app.screen, Variant) + early return allows borrowing app fields freely after the check"
observability_surfaces:
  - "App.milestones.is_empty() — distinguishes 'no project' from 'project with no milestones'"
  - "App.screen — Screen::NoProject when .assay/ absent; Screen::Dashboard otherwise"
  - "App.config.is_some() — true if config::load succeeded"
duration: 20min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T02: Dashboard rendering with real milestone data

**draw_dashboard renders a bordered List of milestones (name, status badge, chunk progress) loaded from milestone_scan; draw_no_project shows a clean exit message when .assay/ is absent.**

## What Happened

Updated `main.rs` to call `milestone_scan(&assay_dir)` and `config::load(&project_root)` before constructing `App`, using `.assay/` existence as the branch condition. The path contract is respected: `milestone_scan` receives the `.assay/` subdirectory; `config::load` receives the project root.

Implemented `draw_dashboard` as a free function taking `&[Milestone]` and `&mut ListState` separately (not `&mut App`) to avoid Rust's borrow checker rejecting simultaneous field access when matching on `&app.screen`. `draw_no_project` renders a bold red centered paragraph with a quit hint.

The `draw` function uses `matches!(app.screen, Screen::Dashboard)` early-return guards rather than a single `match &app.screen`, which is the standard Rust pattern for "read one field, mutably borrow another" situations.

Also added `↑`/`↓` keyboard navigation to `handle_event` (needed for the dashboard list to be usable).

## Verification

- `cargo build -p assay-tui 2>&1 | grep -c '^error'` → 0 (exit code 1 from grep = no lines matched = 0 errors ✓)
- `cargo build -p assay-tui 2>&1 | grep -c 'Finished'` → 1 ✓
- `target/debug/assay-tui` exists ✓
- `cargo build -p assay-cli 2>&1 | grep -c 'Finished'` → 1; both `target/debug/assay` and `target/debug/assay-tui` exist ✓
- `grep -c 'milestone_scan' crates/assay-tui/src/main.rs` → 2 (import + call site) ✓
- Code review: `milestone_scan` called with `assay_dir`, `config::load` called with `project_root` ✓
- Code review: `render_stateful_widget` used (not `render_widget`) ✓
- Code review: status badge uses explicit `match` arms ✓

Slice-level checks (T02 is not the final task — partial expected):
- `cargo build -p assay-tui … Finished` → ✓ PASS
- `cargo build -p assay-cli … Finished` → ✓ PASS (no binary collision)
- `cargo test -p assay-tui` → 0 tests (T03 will add integration tests) → PENDING
- `just ready` → not run (T03 scope)

## Diagnostics

- `App.screen` — read variant to see if `.assay/` was found at startup
- `App.milestones.len()` — how many milestones were loaded; 0 either means no `.assay/` or no milestone TOML files
- `App.config.is_some()` — whether config.toml loaded successfully
- `milestone_scan` failure degrades to `vec![]` — no panic path for corrupt files

## Deviations

Added `↑`/`↓` key navigation to `handle_event` — not explicitly in T02 plan but needed for the list to be functional. Cost: ~10 lines. No plan impact.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/lib.rs` — added `draw_dashboard`, `draw_no_project`; rewrote `draw` dispatch; added ↑↓ navigation
- `crates/assay-tui/src/main.rs` — real data loading with path-contract-correct calls before App construction
