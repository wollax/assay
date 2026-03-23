# Kata State

**Active Milestone:** M007 — TUI Agent Harness
**Active Slice:** S02 — Provider Dispatch and Harness Wiring (NEXT)
**Active Task:** S01 COMPLETE — begin S02
**Phase:** Executing
**Last Updated:** 2026-03-23
**Requirements Status:** 7 active (R053–R059) · 46 validated (R001–R052) · 2 deferred · 4 out of scope
**Test Count:** 1367+ (31 assay-tui; all workspace tests pass)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)
- [x] M006: TUI as Primary Surface (5/5 slices, 4 requirements validated [R049–R052], 1367 tests)

## M007 Roadmap

- [x] S01: Channel Event Loop and Agent Run Panel `risk:high` — refactor blocking run() to TuiEvent channel loop; add Screen::AgentRun with live streaming; launch_agent_streaming in assay-core::pipeline; r key from Dashboard. R053+R054 (Anthropic path). DONE (31 tests pass).
- [x] S02: Provider Dispatch and Harness Wiring `risk:medium` — provider_harness_writer dispatches per ProviderKind; Ollama + OpenAI adapters; Settings model input fields. R054 (all providers). DONE.
- [ ] S03: Slash Command Overlay `risk:low` — / key opens SlashState overlay; /gate-check, /status, /next-chunk, /pr-create commands; sync dispatch to assay-core. R056.
- [ ] S04: MCP Server Configuration Panel `risk:medium` — Screen::McpPanel reads/writes .assay/mcp.json; add/delete/save servers; no live connection. R055.

## S01 Completed (2026-03-23)

All three tasks complete:
- T01: `launch_agent_streaming` in assay-core::pipeline — real subprocess streaming, 2 tests green
- T02: TuiEvent/AgentStatus/Screen::AgentRun enums + App fields + stub implementations + 4 red-phase tests
- T03: Real handle_tui_event, r key handler, draw_agent_run, Esc→Dashboard, channel-based run() loop

31 assay-tui tests pass (27 existing + 4 new). Zero regressions.

Key deliverables from S01 for S02 consumption:
- `TuiEvent` enum: Key, Resize, AgentLine(String), AgentDone { exit_code: i32 }
- `App.event_tx: Option<Sender<TuiEvent>>` — set in run(), None in tests; r handler guards on is_some()
- `Screen::AgentRun { chunk_slug, lines, scroll_offset, status: AgentStatus }` wired and rendering
- `r` key hardcodes `["claude", "--print"]` — S02 replaces with provider_harness_writer dispatch
- Forwarder thread sends AgentDone { exit_code: 0 } sentinel — S02 wires real exit code

## Key Decisions

- D107: Unified TuiEvent channel loop (Key/Resize/AgentLine/AgentDone)
- D108: launch_agent_streaming — new free fn, existing launch_agent unchanged
- D109: provider_harness_writer — free fn dispatching to per-provider closures (D001)
- D110: MCP panel = static config management, no live async MCP client
- D111: Slash command dispatch synchronous in-process
- D112: App.event_tx: Option<Sender<TuiEvent>> — avoids changing handle_event signature

## Known Issues

None. `just ready` passes clean (fmt, lint, test, deny).

## Blockers

None.

## Next Action

Begin S02: Provider Dispatch and Harness Wiring — `provider_harness_writer(config)` dispatches per `ProviderKind`; Ollama + OpenAI adapters; Settings model input fields; accurate exit code delivery from forwarder thread. R054 (all providers).
