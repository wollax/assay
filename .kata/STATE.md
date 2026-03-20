# Kata State

**Active Milestone:** M006 — TUI as Primary Surface
**Active Slice:** S02 — In-TUI Authoring Wizard
**Active Task:** S02 planning
**Phase:** S01 complete; advancing to S02
**Last Updated:** 2026-03-20
**Requirements Status:** 9 active (R050–R059) · 44 validated (R001–R049) · 2 deferred · 4 out of scope
**Test Count:** 769+ (all passing; `cargo deny` blocked on pre-existing aws-lc-sys advisory unrelated to M006)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)

## M006 Roadmap

- [x] S01: App Scaffold, Dashboard, and Binary Fix `risk:high` — binary name fix (`assay-tui`), App+Screen enum, dashboard with real milestone data, no-project guard. R049 validated.
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

## S01 Completion Notes

- `assay-tui` binary (11.9 MB) and `assay` binary (40.8 MB) both present; no collision
- `App::new()` checks `.assay/` existence; `Screen::NoProject` if absent; silently degrades on bad config/milestones
- `compute_gate_data()` loads latest gate run per chunk, accumulates passed/failed per milestone
- Polling event loop (250ms); q/Q/Esc quit; Up/Down wrap; Enter currently no-op (wired in S02/S03)
- `list_state.select_previous()/select_next()` (ratatui built-in) handles wrap-around
- `app.screen` and `app.milestones.len() == app.gate_data.len()` are the key invariants for downstream slices

## Blockers

None.

## Next Action

Begin S02: In-TUI Authoring Wizard. The branch `kata/root/M006/S01` will be squash-merged to main by the Kata extension. Start S02 on a new branch `kata/root/M006/S02`. Key entry points already available: `App.project_root`, `Screen::Wizard` slot, `assay_core::wizard::create_from_inputs`. Plan: WizardState multi-step form (name → chunk slugs → chunk names → criteria per chunk), draw_wizard(), handle_wizard_event(), and an integration test that writes real files to a TempDir.
