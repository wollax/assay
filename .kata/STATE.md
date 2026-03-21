# Kata State

**Active Milestone:** M007 — TUI Agent Harness
**Active Slice:** S02 — Provider Dispatch and Harness Wiring
**Active Task:** none (S01 complete; S02 not yet started)
**Phase:** Planning
**Last Updated:** 2026-03-21 (S01 complete: channel event loop + AgentRun panel; all 35 TUI tests pass; just ready green)
**Requirements Status:** 7 active (R053–R059) · 46 validated (R001–R052) · 2 deferred · 4 out of scope
**Test Count:** 1367 (35 assay-tui: 27 pre-existing + 8 agent_run; all workspace tests pass; just ready green)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)
- [x] M006: TUI as Primary Surface (5/5 slices, 4 requirements validated [R049–R052], 1367 tests)

## M007 Roadmap

- [x] S01: Channel Event Loop and Agent Run Panel — TuiEvent channel loop in main.rs; Screen::AgentRun with live streaming; launch_agent_streaming in assay-core::pipeline; r key from Dashboard; all 35 TUI tests pass; just ready green. R053+R054 (Anthropic path). DONE.
- [ ] S02: Provider Dispatch and Harness Wiring `risk:medium` — provider_harness_writer dispatches per ProviderKind; Ollama + OpenAI adapters; Settings model input fields. R054 (all providers).
- [ ] S03: Slash Command Overlay `risk:low` — / key opens SlashState overlay; /gate-check, /status, /next-chunk, /pr-create commands; sync dispatch to assay-core. R056.
- [ ] S04: MCP Server Configuration Panel `risk:medium` — Screen::McpPanel reads/writes .assay/mcp.json; add/delete/save servers; no live connection. R055.

## S01 Deliverables Summary

- `TuiEvent` enum (Key, Resize, AgentLine, AgentDone) in `assay_tui::app`
- `AgentRunStatus` enum (Running, Done, Failed) in `assay_tui::app`
- `Screen::AgentRun` variant with scrollable line list and status bar
- `launch_agent_streaming` in `assay_core::pipeline` (BufReader::lines, JoinHandle<i32>)
- Channel-based `run()` in `main.rs` with background crossterm thread
- `r` key handler: harness config → relay-wrapper thread → Screen::AgentRun
- 8 agent_run integration tests pass; 27 pre-existing TUI tests unchanged

## Key Decisions

- D107: Unified TuiEvent channel loop (Key/Resize/AgentLine/AgentDone)
- D108: launch_agent_streaming — new free fn, existing launch_agent unchanged
- D109: provider_harness_writer — free fn dispatching to per-provider closures (D001)
- D110: MCP panel = static config management, no live async MCP client
- D111: Slash command dispatch synchronous in-process
- D112: AgentRunStatus name avoids collision with assay-core::checkpoint::AgentStatus
- D113: Relay-wrapper thread guarantees no AgentLine lost before AgentDone
- D114: Harness config written to temp_dir for S01 MVP; S02 uses real worktree path

## Known Issues

None. `just ready` passes clean (fmt, lint, test, deny).

## Blockers

None.

## Next Action

S02: Provider Dispatch and Harness Wiring — implement `provider_harness_writer(config: &Config) -> Box<HarnessWriter>` in `assay-tui::agent` module; add Ollama adapter (`ollama run <model>`); add OpenAI adapter; extend Settings screen with per-phase model input fields. Depends on S01 (complete). Proves R054 fully.
