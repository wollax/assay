# Kata State

**Active Milestone:** M006 — TUI as Primary Surface
**Active Slice:** S02 — In-TUI Authoring Wizard
**Active Task:** (planning — S02 tasks not yet decomposed)
**Phase:** Planning (S01 complete; S02 is next)
**Last Updated:** 2026-03-20
**Requirements Status:** 10 active (R049–R059) · 43 validated (R001–R048) · 2 deferred · 4 out of scope
**Test Count:** 1340 (all passing; 7 new assay-tui tests)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)

## M006 Roadmap

- [x] S01: App Scaffold, Dashboard, and Binary Fix `risk:high` — COMPLETE. [[bin]] fix, App/Screen/WizardState, dashboard with real milestone_scan data, empty-list guard, no-project guard, 7 tests, just ready green.
- [ ] S02: In-TUI Authoring Wizard `risk:high` `depends:[S01]` — WizardState multi-step form, create_from_inputs round-trip, new milestone appears in dashboard. R050.
- [ ] S03: Chunk Detail View and Spec Browser `risk:medium` `depends:[S01]` — milestone → chunk list → chunk detail with criteria and gate results. R051.
- [ ] S04: Provider Configuration Screen `risk:medium` `depends:[S01]` — ProviderConfig type in assay-types (D056 pattern), settings screen, config_save, backward-compat. R052.
- [ ] S05: Help Overlay, Status Bar, and Integration Polish `risk:low` `depends:[S01,S02,S03,S04]` — help overlay, status bar, just ready passes, full flow integration.

## Key Decisions Made During M006

- D088: `assay-tui` binary name is `assay-tui` (not `assay` — already claimed by assay-cli)
- D089: `App` struct with `Screen` enum, free render functions, no Widget trait impls (D001)
- D090: Wizard state as `WizardState` in `Screen::Wizard` variant; not in assay-core
- D091: TUI data loading is synchronous on navigation transitions (not inside terminal.draw())
- D092: `ProviderConfig` in assay-types follows D056 pattern exactly (serde default + skip + snapshot)
- D093: `config_save` free function in assay-core::config using NamedTempFile pattern
- D094: lib.rs + thin main.rs split so tests/ can import assay_tui::
- D095: Screen-specific render fns take separate fields (not &mut App) to satisfy borrow checker with stateful widgets

## Key Forward Intelligence for S02

- `WizardState` is a stub (`Default`-derived empty struct); S02 replaces its fields in place (do not add a new type)
- `App.project_root` is `Option<PathBuf>` pointing at project root (parent of `.assay/`); pass to `create_from_inputs`
- `Screen::Wizard(WizardState)` variant slot exists; wire `n` key in `handle_event` to transition there
- The borrow-checker pattern (D095) is established: all render fns take individual fields, not `&mut App`
- `Config` is in `assay_types::Config`; `assay_core::config` module has no `Config` struct

## Blockers

None.

## Next Action

Begin S02 (In-TUI Authoring Wizard): read S02 plan from M006-ROADMAP.md boundary map, implement WizardState fields and multi-step form logic, wire `n` key dispatch, add wizard round-trip integration test, just ready green.
