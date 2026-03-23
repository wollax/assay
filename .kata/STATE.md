# Kata State

**Active Milestone:** M007 — TUI Agent Harness
**Active Slice:** S03 — Slash Command Overlay
**Active Task:** (none — S03 not yet started)
**Phase:** Planning
**Last Updated:** 2026-03-23
**Requirements Status:** 5 active (R055–R059) · 48 validated (R001–R054) · 2 deferred · 4 out of scope
**Test Count:** 1400+ (40 assay-tui; all workspace tests pass; just ready green)

## Completed Milestones

- [x] M001: Single-Agent Harness End-to-End (7/7 slices, 19 requirements validated, ~991 tests)
- [x] M002: Multi-Agent Orchestration & Harness Platform (6/6 slices, 5 new requirements validated, ~1183 tests)
- [x] M003: Conflict Resolution & Polish (2/2 slices, 3 new requirements validated [R026, R028, R029], 1222 tests)
- [x] M004: Coordination Modes — Mesh & Gossip (4/4 slices, 6 new requirements validated [R034–R038], 1271 tests)
- [x] M005: Spec-Driven Development Core (6/6 slices, 10 requirements validated [R039–R048], 1333 tests)
- [x] M006: TUI as Primary Surface (5/5 slices, 4 requirements validated [R049–R052], 1367 tests)

## M007 Roadmap

- [x] S01: Channel Event Loop and Agent Run Panel — TuiEvent channel loop, Screen::AgentRun, launch_agent_streaming, r key wired, two-channel bridge design, assay-harness dep, 6 new integration tests, just ready green. R053 validated. DONE.
- [x] S02: Provider Dispatch and Harness Wiring — provider_harness_writer dispatches per ProviderKind (Anthropic/Ollama/OpenAI); Settings model text-input fields; 40 tests pass. R054 validated. DONE.
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

## S02 Key Deliverables

- `crates/assay-tui/src/agent.rs` — `provider_harness_writer(Option<&Config>) -> Box<HarnessWriter>` dispatching Anthropic/Ollama/OpenAI
- `OllamaConfig { model }` and `OpenAiConfig { model, api_key_env }` TUI-local structs in `agent.rs`
- `pub mod agent` in `lib.rs`; `r` key handler routes through `provider_harness_writer`
- `Screen::Settings` extended: `planning_model`, `execution_model`, `review_model: String`, `model_focus: Option<usize>`
- Tab/Char/Backspace/Esc model-focus state machine; `w` saves buffers to `ProviderConfig`
- 40 assay-tui tests pass (3 provider_dispatch + 2 model-field + 35 pre-S02)
- D115: Anthropic closure prepends `"claude"` before `build_cli_args` flags
- D116: `w` save falls through to save arm even when model_focus is Some
- D117: Tab cycle linear (0→1→2→None), not wrap-around

## Known Issues

None. `just ready` passes clean (fmt, lint, test, deny).

## Blockers

None.

## Next Action

Begin S03: Slash Command Overlay. Implement `SlashCmd` enum, `parse_slash_cmd`, `execute_slash_cmd`, `SlashState`, `Screen::SlashCmd`, `draw_slash_overlay`, and `/` key handler wiring. Write integration tests in `tests/slash_commands.rs` first.
