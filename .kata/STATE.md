# Kata State

**Active Milestone:** M006 — TUI as Primary Surface
**Active Slice:** S02 — In-TUI Authoring Wizard
**Active Task:** — (all tasks complete; slice ready for summary + merge)
**Phase:** Summarizing
**Last Updated:** 2026-03-20
**Requirements Status:** 10 active (R049–R059) · 43 validated (R001–R048) · 2 deferred · 4 out of scope
**Test Count:** 1333 (all passing)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)

## M006 Roadmap

- [ ] S01: App Scaffold, Dashboard, and Binary Fix `risk:high` — binary name fix (`assay-tui`), App+Screen enum, dashboard with real milestone data, no-project guard. R049.
- [x] S02: In-TUI Authoring Wizard `risk:high` `depends:[S01]` — WizardState multi-step form, draw_wizard popup, App wiring (n/Cancel/Submit); 23 assay-tui tests green. R050. (pending slice summary + PR)
- [ ] S03: Chunk Detail View and Spec Browser `risk:medium` `depends:[S01]` — milestone → chunk list → chunk detail with criteria and gate results. R051.
- [ ] S04: Provider Configuration Screen `risk:medium` `depends:[S01]` — ProviderConfig type in assay-types (D056 pattern), settings screen, config_save, backward-compat. R052.
- [ ] S05: Help Overlay, Status Bar, and Integration Polish `risk:low` `depends:[S01,S02,S03,S04]` — help overlay, status bar, just ready passes, full flow integration.

## Key Decisions Made During M006 Planning

- D088: `assay-tui` binary name is `assay-tui` (not `assay` — already claimed by assay-cli)
- D089: `App` struct with `Screen` enum, free render functions, no Widget trait impls (D001)
- D090: Wizard state as `WizardState` in `Screen::Wizard` variant; not in assay-core
- D091: TUI data loading is synchronous on navigation transitions (not inside terminal.draw())
- D092: `ProviderConfig` in assay-types follows D056 pattern exactly (serde default + skip + snapshot)
- D093: `config_save` free function in assay-core::config using NamedTempFile pattern

## Blockers

None.

## Next Action

Execute M006/S02/T03: Implement `draw_wizard` rendering function (ratatui popup via Clear + Flex::Center, slug preview, inline error, cursor positioning).
