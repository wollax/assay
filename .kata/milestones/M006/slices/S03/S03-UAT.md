# S03: Chunk Detail View and Spec Browser — UAT

**Milestone:** M006
**Written:** 2026-03-21

## UAT Type

- UAT mode: mixed (artifact-driven integration tests + human-experience keyboard flow)
- Why this mode is sufficient: The 6 `spec_browser` integration tests cover all navigation state transitions and data-loading behaviors programmatically. Human-experience UAT adds the interactive keyboard feel and breadcrumb navigation that cannot be exercised in a headless test.

## Preconditions

1. `cargo build -p assay-tui` succeeds and produces `target/debug/assay-tui`
2. A project with `.assay/` exists and has at least one milestone with one or more chunks (the assay project's own `.assay/` fixtures work)
3. Terminal is at least 80×24

## Smoke Test

Run `cargo test -p assay-tui --test spec_browser` — all 6 tests should pass in under 1 second.

## Test Cases

### 1. Dashboard → MilestoneDetail navigation

1. Launch `./target/debug/assay-tui` in a project directory with `.assay/` milestones
2. Confirm the dashboard shows milestone list with names and status badges
3. Press `↓` to select a milestone (if more than one)
4. Press `Enter`
5. **Expected:** Screen transitions to a bordered chunk list titled "Milestone: \<name\>"; chunks are sorted by order; each chunk shows a slug and ✓ or · status icon

### 2. ↑↓ navigation within MilestoneDetail

1. Navigate to MilestoneDetail (see case 1)
2. Press `↓` and `↑` several times
3. **Expected:** Selection moves through the chunk list with wrapping (last chunk → first chunk on ↓, first → last on ↑)

### 3. Esc from MilestoneDetail → Dashboard

1. Navigate to MilestoneDetail
2. Press `Esc`
3. **Expected:** Screen returns to the Dashboard; milestone list is visible with the same selection position as before entering MilestoneDetail

### 4. MilestoneDetail → ChunkDetail navigation

1. Navigate to MilestoneDetail
2. Select a chunk with `↓`
3. Press `Enter`
4. **Expected:** Screen shows a table titled with the chunk slug; columns are icon / criterion name / description; each row shows ✓, ✗, or ? depending on the latest gate run result

### 5. ChunkDetail — no gate history shows all Pending

1. Navigate to ChunkDetail for a chunk that has never had a gate run (no history files in `.assay/history/<chunk-slug>/`)
2. **Expected:** All criteria rows show `?` (Pending) icon; no error message; table still renders

### 6. Esc from ChunkDetail → MilestoneDetail (breadcrumb preserved)

1. Navigate from Dashboard → MilestoneDetail → select chunk at position 2 → ChunkDetail
2. Press `Esc`
3. **Expected:** Screen returns to MilestoneDetail with the chunk list cursor on position 2 (same as when Enter was pressed); no reload needed

## Edge Cases

### Legacy flat spec

1. Navigate to ChunkDetail for a chunk whose `.assay/specs/<slug>/` directory contains a flat `spec.toml` (no `gates.toml`)
2. **Expected:** A single line "Legacy flat spec — criteria not available in this view" (or similar) is shown instead of a table; no panic

### Missing .assay directory

1. Launch `./target/debug/assay-tui` in a directory with no `.assay/`
2. **Expected:** "Not an Assay project" message is shown (NoProject screen); pressing `q` exits cleanly; no panic

## Failure Signals

- Screen stays on Dashboard after pressing `Enter` → milestone_load failed; check `Screen::LoadError(msg)` path
- ChunkDetail shows "No spec data" unexpectedly → `detail_spec_note` contains the reason; run `cargo test -p assay-tui --test spec_browser::chunk_detail_no_history_all_pending -- --nocapture` to reproduce
- Esc from ChunkDetail does not preserve chunk cursor → `detail_list_state` not retained; regression in ChunkDetail Esc arm
- Criteria all show `?` despite gate runs existing → `join_results` name-join mismatch (criterion name in spec differs from `criterion_name` in run record)
- Panic on launch → `App::with_project_root` failed to handle missing project directory; check NoProject guard

## Requirements Proved By This UAT

- R051 (TUI spec browser) — UAT proves: interactive keyboard navigation dashboard→milestone→chunk with breadcrumb Esc chains; criteria display with ✓/✗/? gate result icons from real gate history; empty-history renders as all Pending without error

## Not Proven By This UAT

- Gate result evidence drill-down (raw output per criterion) — not implemented; deferred to M007
- Live refresh of gate results while TUI is open — sync load-on-navigate only; deferred to M007
- S04 Settings screen — separate slice
- S05 help overlay and status bar — separate slice

## Notes for Tester

- The assay project's own `.assay/` directory (in the repo root) is a valid project for testing — it has real milestones and specs
- Chunks with gate history will show actual ✓/✗ icons; chunks without history show all `?` — both are correct
- The `?` icon means Pending (no result), not Unknown or Error
