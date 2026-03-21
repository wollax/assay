# Kata State

**Active Milestone:** M007 — TUI Agent Harness (planned)
**Active Slice:** None — M006 complete
**Active Task:** None
**Phase:** Planning
**Last Updated:** 2026-03-21
**Requirements Status:** 7 active (R053–R059) · 46 validated (R001–R052) · 2 deferred · 4 out of scope
**Test Count:** 1371+ (22 assay-tui; all workspace tests pass; just ready green)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)
- [x] M006: TUI as Primary Surface (5/5 slices, 4 requirements validated [R049–R052], 1371+ tests)

## M006 Roadmap (complete)

- [x] S01: App Scaffold, Dashboard, and Binary Fix — binary name fix (`assay-tui`), App+Screen enum, dashboard with real milestone data, no-project guard. R049. DONE.
- [x] S02: In-TUI Authoring Wizard — WizardState multi-step form, draw_wizard popup, App wiring (n/Cancel/Submit); 23 assay-tui tests + 1356 workspace tests green. R050. DONE.
- [x] S03: Chunk Detail View and Spec Browser — MilestoneDetail + ChunkDetail screens; join_results criterion join; 6 spec_browser integration tests; R051 validated. DONE.
- [x] S04: Provider Configuration Screen — ProviderConfig + ProviderKind in assay-types (D056 pattern), config_save atomic write, Screen::Settings full-screen view with provider list + model fields, backward-compat round-trip tests, schema snapshot locked. R052. DONE.
- [x] S05: Help Overlay, Status Bar, and Integration Polish — `?` help overlay, persistent status bar, global layout split with `area: Rect` refactor, Event::Resize fix, cycle_slug loading, just ready green. DONE.

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
- D097: Screen-specific render fns take individual fields (not &mut App) to satisfy borrow checker with stateful widgets
- D098: `..` pattern in draw() match arms avoids Screen-variant borrow-split; clone-then-mutate in handle_event()
- D099: App-level detail_* fields for loaded data; preserves detail_list_state across Esc transitions
- D100: Criterion join by exact name match; unmatched → None (Pending); linear scan acceptable at ≤15 criteria
- D101: Settings screen uses `w` key to write/save (vim mnemonic); `Esc`/`q` cancel
- D102: Settings screen is full-screen bordered block (not a popup like wizard)
- D103: Save with no loaded config shows inline error; does not create a minimal config.toml
- D104: Help overlay event guard — all keys are no-ops when show_help=true except `?`/Esc to dismiss
- D105: All draw_* accept explicit area: Rect; draw() splits frame.area() once into [content_area, status_area]
- D106: App.cycle_slug cached on App; refreshed only at lifecycle transitions (wizard submit, settings save)

## Known Issues

None. `just ready` passes clean. Previous aws-lc-sys CVEs (RUSTSEC-2026-0044 to -0049) are listed as ignore entries in deny.toml.

## Blockers

None.

## Next Action

M006 complete. Begin M007 planning: TUI agent spawning (R053), provider abstraction (R054), MCP server management panel (R055), slash commands (R056).
