# S03: Chunk Detail View and Spec Browser — Research

**Researched:** 2026-03-20
**Updated:** 2026-03-21 (post-implementation validation)
**Domain:** Ratatui TUI, assay-core data APIs, Rust borrow checker patterns
**Confidence:** HIGH
**Status:** COMPLETE — all 6 spec_browser tests pass; `just ready` green

## Summary

**S03 is implemented and verified.** Two new full-screen TUI views — `MilestoneDetail` (chunk list) and `ChunkDetail` (criteria + gate results table) — were added to `app.rs` following the architecture established in S01/S02. All research predictions proved correct.

The `assay-core` data APIs (`milestone_load`, `load_spec_entry_with_diagnostics`, `history::list`+`load`) are synchronous and load correctly in `handle_event` on navigation transitions per D091. The borrow-split pattern from D097 (pass individual `App` fields to render fns rather than `&mut App`) was applied cleanly for both `draw_milestone_detail` and `draw_chunk_detail`. The `join_results` free function joins `GatesSpec.criteria` against `GateRunRecord.summary.results` by criterion name, returning `Option<bool>` per criterion — unmatched criteria render as `?` Pending.

The only deviation from the pre-implementation research: `GatesSpec` validation requires at least one executable criterion (`cmd`/`path`/`kind=AgentReport`). Test fixtures in `spec_browser.rs` needed a `cmd = "true"` field on one criterion. This was not documented as a constraint in the original research.

## Recommendation

S03 is complete. The implementation matches the research recommendations exactly. For S04 (Provider Configuration Screen), the following apply:

- `Config` struct lives in `assay_types::Config` — the D056 pattern (serde default + skip_serializing_if + schema snapshot update) applies for adding `provider: Option<ProviderConfig>`
- `config_save` does NOT yet exist in `assay-core::config` — it must be added as a new free function using the `NamedTempFile` atomic-write pattern (D093)
- `Screen::Settings` variant and its draw/event dispatch are stubs in `app.rs` — S04 replaces the stub with `draw_settings` (full-screen, not popup per D099)
- The `draw()` method now uses a proper `match &self.screen` with arms for all 7 variants (Dashboard, MilestoneDetail, ChunkDetail, Wizard, Settings, NoProject, LoadError) — S04 replaces the Settings stub arm

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Loading milestone from disk | `assay_core::milestone::milestone_load(assay_dir, slug)` | Same atomic-read, validates TOML; used in S03 Dashboard Enter |
| Loading spec/gates criteria | `assay_core::spec::load_spec_entry_with_diagnostics(slug, specs_dir)` | Returns `SpecEntry` with `GatesSpec.criteria`; handles Legacy gracefully |
| Listing gate run history | `assay_core::history::list(assay_dir, spec_name)` | Returns sorted run IDs; empty is Ok not error |
| Loading a gate run record | `assay_core::history::load(assay_dir, spec_name, run_id)` | Returns `GateRunRecord` with `CriterionResult` per criterion |
| Chunk list with selection | `ratatui::widgets::{List, ListItem, ListState}` + `render_stateful_widget` | Used in both Dashboard and MilestoneDetail; already imported |
| Criteria table | `ratatui::widgets::{Table, Row, Cell}` | Used in `draw_chunk_detail`; already imported in `app.rs` |
| Config load | `assay_core::config::load(root)` | Already called in `App::with_project_root`; returns `Config` from `assay_types` |
| Atomic config write (S04) | `NamedTempFile + sync_all + persist` pattern from `milestone_save` | Pattern from D093; `config_save` must be added to `assay-core::config` |

## Existing Code and Patterns (as-built)

- `crates/assay-tui/src/app.rs` — All S03 code lives here. Adds `Screen::MilestoneDetail { slug }`, `Screen::ChunkDetail { milestone_slug, chunk_slug }`, five `App` fields (`detail_list_state`, `detail_milestone`, `detail_spec`, `detail_spec_note`, `detail_run`), `draw_milestone_detail`, `draw_chunk_detail`, `join_results`, and full navigation wiring.
- `crates/assay-tui/src/lib.rs` — `pub mod app; pub mod wizard;` — two modules; S04 does not need a new module file.
- `crates/assay-tui/tests/spec_browser.rs` — `setup_project` test helper + 6 integration tests. Pattern: `App::with_project_root(Some(tmp))`, drive via `handle_event(key(...))`, assert on `app.screen` discriminant and `app.detail_*` fields.
- `crates/assay-tui/tests/app_wizard.rs` — 1 integration test for wizard slug-collision error; reference for App-level test pattern.
- `crates/assay-tui/tests/wizard_round_trip.rs` — 9 unit + integration tests for `WizardState` and file I/O round-trip.
- `crates/assay-core/src/config/mod.rs` — Has `load(root: &Path) -> Result<Config>`. Does NOT have `config_save` yet — S04 must add it.
- `crates/assay-types/src/lib.rs` — `Config` struct: `project_name`, `specs_dir`, `gates?`, `guard?`, `worktree?`, `sessions?`. No `provider` field yet — S04 adds it. Has `#[serde(deny_unknown_fields)]`.

## Constraints

- **Borrow checker (D097):** `draw()` uses `match &self.screen`; individual fields passed as args to render fns. New Settings render fn must follow this — `draw_settings(frame, config: Option<&Config>, list_state: &mut ListState)`.
- **Sync-only data loading (D091):** Load in `handle_event` on nav transitions. No background threads for S04 either.
- **assay_dir vs project_root:** `milestone_load` and `history::list` take `.assay/` dir; `spec::load_spec_entry_with_diagnostics` takes `specs_dir` = `assay_dir.join("specs")`; `config::load` takes project root.
- **Zero traits (D001):** All render functions are free functions, not Widget trait impls.
- **Config extension (D056/D092):** Adding `provider: Option<ProviderConfig>` to `Config` requires `#[serde(default, skip_serializing_if = "Option::is_none")]` on the field, a schema snapshot update, and a backward-compat test proving existing `config.toml` without the field still loads.
- **GatesSpec validation:** At least one criterion must have `cmd`, `path`, or `kind = "agent_report"` to pass validation. Test fixtures must include a valid `cmd = "true"` line.

## Common Pitfalls

- **Criterion name join mismatch:** `GatesSpec.criteria[i].name` vs `CriterionResult.criterion_name` must match exactly. Criteria added after a run appear as Pending; run results for removed criteria are silently ignored.
- **Empty history is not an error:** `history::list()` returns `Ok(vec![])` when no runs exist — render all criteria as `?` Pending.
- **SpecEntry::Legacy has no GatesSpec:** Show a one-line "Legacy flat spec — criteria not available in this view" fallback. Implemented via `detail_spec_note: Option<String>` in `App`.
- **GatesSpec validation requires executable criterion:** Test TOML fixtures must include at least one criterion with `cmd = "true"` (or `path`, or `kind = "agent_report"`). Missing this causes `load_spec_entry_with_diagnostics` to return an error → `ChunkDetail` shows error note, not criteria table.
- **config_save does not exist yet (S04):** `assay-core::config` exposes only `load`. S04 must add `pub fn config_save(root: &Path, config: &Config) -> Result<()>` using `NamedTempFile + sync_all + persist` — not write directly to `config.toml`.
- **deny_unknown_fields on Config (S04):** Adding `ProviderConfig` without `serde(default)` breaks existing `config.toml` files that lack a `[provider]` section. Must follow D056/D092 pattern exactly.
- **Settings `w` key conflict (D098):** `s` opens Settings from Dashboard; `w` saves and returns to Dashboard from Settings; `Esc`/`q` cancel without saving. `Enter` does NOT save (avoids conflict with future list-item activation semantics).

## Open Risks (S04 forward)

- **schema snapshot for Config:** Adding `ProviderConfig` to `Config` requires `cargo insta review` to update the locked schema snapshot. Missing this step causes `cargo test` to fail on schema divergence.
- **backward-compat test:** A `config_toml_roundtrip_without_provider` test must prove existing config files without a `[provider]` section still load without error. This is a D056 requirement, not optional.
- **Settings screen terminal size:** Full-screen (not popup per D099) means Settings renders at full terminal height. Provider selection list + model fields + error line must fit in ≥24 rows. No minimum terminal size check exists anywhere in the TUI.

## Post-Implementation Validation

The pre-implementation research was accurate. All predictions held:
- `App`-level detail fields (not Screen-variant-embedded) was the right call — preserved `detail_list_state` across Esc transitions without extra state management
- `match &self.screen` in `draw()` + clone-before-mutate in `handle_event()` resolved borrow-split cleanly (D097/D098 patterns)
- `Table` widget (not `List`) for criteria pane rendered correctly with 3 columns: icon/name/description
- `history::list()` returning `Ok(vec![])` for missing history dir was confirmed correct — all 6 spec_browser tests pass

One discovery not in the original research: `GatesSpec` validation requires at least one executable criterion. The test fixture's `cmd = "true"` field was added in T03 to satisfy this constraint. This is now documented as a pitfall above.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | (no dedicated skill available) | none found |

## Sources

- Codebase: `crates/assay-tui/src/app.rs` — Screen enum, App struct, draw() + handle_event() as-built; draw_milestone_detail, draw_chunk_detail, join_results implementations (HIGH confidence)
- Codebase: `crates/assay-tui/tests/spec_browser.rs` — 6 passing integration tests; setup_project fixture pattern (HIGH confidence)
- Codebase: `crates/assay-core/src/milestone/mod.rs` — milestone_load public API, path contracts (HIGH confidence)
- Codebase: `crates/assay-core/src/spec/mod.rs` — load_spec_entry_with_diagnostics, SpecEntry variants, GatesSpec validation requirement (HIGH confidence)
- Codebase: `crates/assay-core/src/history/mod.rs` — list/load API, empty-dir returns Ok(vec![]) (HIGH confidence)
- Codebase: `crates/assay-types/src/lib.rs` — Config struct, no provider field yet, deny_unknown_fields (HIGH confidence)
- Codebase: `crates/assay-core/src/config/mod.rs` — load() exists; config_save does NOT exist (HIGH confidence)
- S03-SUMMARY.md — as-built deviations, GatesSpec validation discovery, forward intelligence for S04 (HIGH confidence)
