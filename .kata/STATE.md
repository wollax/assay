# Kata State

**Active Milestone:** M006 — TUI as Primary Surface
**Active Slice:** S05 — Help Overlay, Status Bar, and Integration Polish
**Active Task:** T02 (T01 done)
**Phase:** Executing
**Last Updated:** 2026-03-21
**Requirements Status:** 8 active (R052–R059) · 45 validated (R001–R051) · 2 deferred · 4 out of scope
**Test Count:** 1371 baseline (16 assay-tui + all prior; 2 new help_status tests failing by design — awaiting T02)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)

## M006 Roadmap

- [x] S01: App Scaffold, Dashboard, and Binary Fix `risk:high` — binary name fix (`assay-tui`), App+Screen enum, dashboard with real milestone data, no-project guard. R049. DONE.
- [x] S02: In-TUI Authoring Wizard `risk:high` `depends:[S01]` — WizardState multi-step form, draw_wizard popup, App wiring (n/Cancel/Submit); 23 assay-tui tests + 1356 workspace tests green. R050. DONE.
- [x] S03: Chunk Detail View and Spec Browser `risk:medium` `depends:[S01]` — MilestoneDetail + ChunkDetail screens with real data from assay-core; join_results criterion join; 6 spec_browser integration tests; R051 validated. DONE.
- [ ] S04: Provider Configuration Screen `risk:medium` `depends:[S01]` — ProviderConfig type in assay-types (D056 pattern), settings screen, config_save, backward-compat. R052. **PLANNED** (T01–T03)
- [ ] S05: Help Overlay, Status Bar, and Integration Polish `risk:low` `depends:[S01,S02,S03,S04]` — help overlay, status bar, just ready passes, full flow integration. **PLANNED** (T01–T02)

## Key Decisions Made During M006

- D088: `assay-tui` binary name is `assay-tui` (not `assay` — already claimed by assay-cli)
- D089: `App` struct with `Screen` enum, free render functions, no Widget trait impls (D001)
- D090: Wizard state as `WizardState` in `Screen::Wizard` variant; not in assay-core
- D091: TUI data loading is synchronous on navigation transitions (not inside terminal.draw())
- D092: `ProviderConfig` in assay-types follows D056 pattern exactly (serde default + skip + snapshot)
- D093: `config_save` free function in assay-core::config using NamedTempFile pattern
- D094: ChunkCount digit validation uses replace-semantics; only '1'–'7' accepted, others silently ignored
- D095: Combined bin+lib: app.rs in binary tree, wizard types accessed via `assay_tui::` lib path
- D096: draw() renders Dashboard unconditionally first, overlays Wizard popup if Screen::Wizard (refactored in S03 to full match)
- D098: `..` pattern in draw() match arms avoids Screen-variant borrow-split; clone-then-mutate in handle_event() for slug reads before screen transition
- D099: App-level detail_* fields for loaded data; preserves detail_list_state across Esc transitions
- D100: Criterion join by exact name match; unmatched → None (Pending); linear scan acceptable at ≤15 criteria
- D101: Settings screen uses `w` key to write/save (vim mnemonic); `Esc`/`q` cancel
- D102: Settings screen is full-screen bordered block (not a popup like wizard)
- D103: Save with no loaded config shows inline error; does not create a minimal config.toml

## Known Issues

- `just ready` deny check fails on 6 pre-existing `aws-lc-sys` CVEs (RUSTSEC-2026-0044 to -0049) via jsonschema dev-dep. Not introduced by S02. Address before M006 milestone sign-off (S05 or separate fix).

## Blockers

None.

## Key Decisions Made During M006 (continued)

- D104: Help overlay event guard — all keys are no-ops when show_help=true except `?`/Esc to dismiss
- D105: All draw_* accept explicit area: Rect; draw() splits frame.area() once into [content_area, status_area]
- D106: App.cycle_slug cached on App; refreshed only at lifecycle transitions (wizard submit, settings save)

## Next Action

S05 planned (T01–T02). S04 must complete before S05 can execute (S04 adds App.config, Screen::Settings, draw_settings — all consumed by S05). Begin S04 T01: ProviderKind/ProviderConfig types in assay-types + backward-compat tests + schema snapshot. Then S04 T02: config_save in assay-core. Then S04 T03: SettingsState, Screen::Settings, draw_settings, s/w/Esc wiring, settings_screen tests.
