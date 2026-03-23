# Kata State

**Active Milestone:** M007 — TUI Agent Harness
**Active Slice:** S01 — Channel Event Loop and Agent Run Panel
**Active Task:** T02 (T01 done)
**Phase:** Executing
**Last Updated:** 2026-03-23
**Requirements Status:** 7 active (R053–R059) · 46 validated (R001–R052) · 2 deferred · 4 out of scope
**Test Count:** 1367 (27 assay-tui; all workspace tests pass; just ready green)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)
- [x] M006: TUI as Primary Surface (5/5 slices, 4 requirements validated [R049–R052], 1367 tests)

## M006 Roadmap (complete)

- [x] S01: App Scaffold, Dashboard, and Binary Fix — binary name fix (`assay-tui`), App+Screen enum, dashboard with real milestone data, no-project guard. R049. DONE.
- [x] S02: In-TUI Authoring Wizard — WizardState multi-step form, draw_wizard popup, App wiring (n/Cancel/Submit); wizard_round_trip integration test. R050. DONE.
- [x] S03: Chunk Detail View and Spec Browser — MilestoneDetail + ChunkDetail screens; join_results criterion join; 6 spec_browser integration tests. R051 validated. DONE.
- [x] S04: Provider Configuration Screen — ProviderKind+ProviderConfig in assay-types (D092), config_save atomic write (D093), Screen::Settings full-screen view (↑↓ selection, w saves, Esc cancels), 5 settings integration tests including restart-persistence, schema snapshots locked. R052. DONE.
- [x] S05: Help Overlay, Status Bar, and Integration Polish — `?` help overlay, persistent status bar, global layout split with `area: Rect` refactor, Event::Resize fix, cycle_slug loading, just ready green. DONE.

## Key Decisions Made During M006

- D088–D106: full list in DECISIONS.md (search for "M006" in the Scope column)

Key patterns:
- Screen-specific render fns take individual fields (not &mut App) — borrow checker + stateful widgets (D097)
- App-level detail_* fields for loaded data; not embedded in Screen variants (D099)
- All draw_* accept explicit area: Rect; global layout split once in App::draw (D105)
- ProviderConfig follows D056 pattern exactly for backward-compat Config extension (D092)
- config_save uses NamedTempFile+sync_all+persist consistent with milestone_save (D093)

## M007 Roadmap

- [ ] S01: Channel Event Loop and Agent Run Panel `risk:high` — refactor blocking run() to TuiEvent channel loop; add Screen::AgentRun with live streaming; launch_agent_streaming in assay-core::pipeline; r key from Dashboard. R053+R054 (Anthropic path).
- [ ] S02: Provider Dispatch and Harness Wiring `risk:medium` — provider_harness_writer dispatches per ProviderKind; Ollama + OpenAI adapters; Settings model input fields. R054 (all providers).
- [ ] S03: Slash Command Overlay `risk:low` — / key opens SlashState overlay; /gate-check, /status, /next-chunk, /pr-create commands; sync dispatch to assay-core. R056.
- [ ] S04: MCP Server Configuration Panel `risk:medium` — Screen::McpPanel reads/writes .assay/mcp.json; add/delete/save servers; no live connection. R055.

## Key Decisions

- D107: Unified TuiEvent channel loop (Key/Resize/AgentLine/AgentDone)
- D108: launch_agent_streaming — new free fn, existing launch_agent unchanged
- D109: provider_harness_writer — free fn dispatching to per-provider closures (D001)
- D110: MCP panel = static config management, no live async MCP client
- D111: Slash command dispatch synchronous in-process
- D112: AgentRunStatus (not AgentStatus) — TUI-local enum, avoids name collision with assay-core::checkpoint::AgentStatus
- D113: Two-channel exit-code bridge design — bridge thread owns JoinHandle via inner join thread + exit_rx; App.agent_thread unused for join

## Known Issues

None. `just ready` passes clean (fmt, lint, test, deny). RUSTSEC-2026-0044 to -0049 are listed as ignore entries in deny.toml (pre-existing, not introduced by M006).

## Blockers

None.

## Next Action

S01 planned. Begin T01: write failing integration tests for pipeline_streaming and agent_run.
