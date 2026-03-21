# Kata State

**Active Milestone:** M006 — TUI as Primary Surface
**Active Slice:** S03 — Chunk Detail View and Spec Browser
**Active Task:** T03 — ChunkDetail screen — navigation, spec load, and render
**Phase:** Executing
**Last Updated:** 2026-03-20
**Requirements Status:** 9 active (R051–R059) · 44 validated (R001–R050) · 2 deferred · 4 out of scope
**Test Count:** 1356 (all passing)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)

## M006 Roadmap

- [x] S01: App Scaffold, Dashboard, and Binary Fix `risk:high` — binary name fix (`assay-tui`), App+Screen enum, dashboard with real milestone data, no-project guard. R049. DONE.
- [x] S02: In-TUI Authoring Wizard `risk:high` `depends:[S01]` — WizardState multi-step form, draw_wizard popup, App wiring (n/Cancel/Submit); 23 assay-tui tests + 1356 workspace tests green. R050. DONE.
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
- D094: ChunkCount digit validation uses replace-semantics; only '1'–'7' accepted, others silently ignored
- D095: Combined bin+lib: app.rs in binary tree, wizard types accessed via `assay_tui::` lib path
- D096: draw() renders Dashboard unconditionally first, overlays Wizard popup if Screen::Wizard (refactor needed in S03 for full-screen views)

## Known Issues

- `just ready` deny check fails on 6 pre-existing `aws-lc-sys` CVEs (RUSTSEC-2026-0044 to -0049) via jsonschema dev-dep. Not introduced by S02. Address before M006 milestone sign-off (S05 or separate fix).

## Blockers

None.

## Next Action

Execute S03/T03: Wire Enter in MilestoneDetail to load detail_spec (via load_spec_entry_with_diagnostics) and detail_run (via history::list + history::load), navigate to ChunkDetail; implement draw_chunk_detail with a Table of criteria and ✓/✗/? icons; Esc from ChunkDetail → MilestoneDetail. End state: all 6 spec_browser tests pass.
