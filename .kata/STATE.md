# Kata State

**Active Milestone:** M007 — TUI Agent Harness
**Active Slice:** S02 — Provider Dispatch and Harness Wiring
**Active Task:** (none — S02 not yet started)
**Phase:** Planning
**Last Updated:** 2026-03-23
**Requirements Status:** 6 active (R054–R059) · 47 validated (R001–R053) · 2 deferred · 4 out of scope
**Test Count:** 1400+ (30 assay-tui; all workspace tests pass; just ready green)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)
- [x] M006: TUI as Primary Surface (5/5 slices, 4 requirements validated [R049–R052], 1367 tests)

## M007 Roadmap

- [x] S01: Channel Event Loop and Agent Run Panel — TuiEvent channel loop, Screen::AgentRun, launch_agent_streaming, r key wired, two-channel bridge design, assay-harness dep, 6 new integration tests, just ready green. R053 validated. DONE.
- [ ] S02: Provider Dispatch and Harness Wiring `risk:medium` — provider_harness_writer dispatches per ProviderKind; Ollama + OpenAI adapters; Settings model input fields. R054 (all providers).
- [ ] S03: Slash Command Overlay `risk:low` — / key opens SlashState overlay; /gate-check, /status, /next-chunk, /pr-create commands; sync dispatch to assay-core. R056.
- [ ] S04: MCP Server Configuration Panel `risk:medium` — Screen::McpPanel reads/writes .assay/mcp.json; add/delete/save servers; no live connection. R055.

## S01 Key Deliverables

- `TuiEvent` enum in `assay_tui::event` (Key, Resize, AgentLine, AgentDone)
- `run()` refactored to `mpsc::Receiver<TuiEvent>` loop with crossterm background thread
- `launch_agent_streaming(cli_args, working_dir, line_tx) -> JoinHandle<i32>` in assay-core::pipeline
- `Screen::AgentRun { chunk_slug, lines, scroll_offset, status }` + `AgentRunStatus` enum
- `App.event_tx: Option<mpsc::Sender<TuiEvent>>` wired from run()
- `handle_r_key()` — two-channel bridge: (line_tx/line_rx) for stdout, (exit_tx/exit_rx) for exit code
- Integration tests: `pipeline_streaming.rs` (3 tests), `agent_run.rs` (3 tests)

## S01 Key Decisions

- D107: Unified TuiEvent channel loop (Key/Resize/AgentLine/AgentDone) — no tokio runtime
- D108: launch_agent_streaming — new free fn; existing launch_agent unchanged
- D112: AgentRunStatus (not AgentStatus) — TUI-local enum, avoids collision with assay-core::checkpoint::AgentStatus
- D113: Two-channel exit-code bridge — bridge thread owns JoinHandle via inner join thread + exit_rx; App.agent_thread always None in production
- D114: TuiEvent extracted to src/event.rs — avoids circular imports between main.rs and app.rs
- D115: TempDir leaked via std::mem::forget — keeps harness config files alive during subprocess execution

## S02 Starting Context

- The `r` key currently hardcodes the Claude Code adapter (calls `assay_harness::claude::*` directly in `handle_r_key`)
- S02 must implement `provider_harness_writer(config: &Config) -> Box<HarnessWriter>` and replace the hardcoded path
- `App.event_tx` is already wired — S02 can add new TuiEvent variants (e.g. for provider switch notifications) without touching the channel infrastructure
- Settings screen (Screen::Settings) already shows ProviderKind selection — S02 adds model-per-phase text input fields

## Known Issues

None. `just ready` passes clean (fmt, lint, test, deny).

## Blockers

None.

## Next Action

Begin S02: Provider Dispatch and Harness Wiring. Implement `provider_harness_writer`, Ollama adapter, OpenAI minimal adapter, and Settings model input fields.
