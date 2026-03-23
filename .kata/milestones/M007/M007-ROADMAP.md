# M007: TUI Agent Harness

**Vision:** Turn the TUI from a read/visualize surface into a full execution surface. When this milestone is complete, a developer can press `r` from the dashboard to watch a real AI agent work on the active chunk — output streaming live to the screen, gate results refreshing when it finishes — with no terminal window, no separate tool, no CLI commands required. Provider selection in the Settings screen controls which agent binary runs. A slash-command overlay gives power users quick access to gate-check, status, and PR operations. An MCP server panel lets users manage their tool extensions without editing JSON files.

## Success Criteria

- Pressing `r` from the TUI dashboard while an InProgress chunk is active spawns the configured AI agent, streams its stdout line-by-line into a `Screen::AgentRun` panel, and shows "Done (exit 0)" or "Failed (exit N)" when the subprocess exits — all without freezing the TUI or losing any output
- After the agent exits, the gate results for that chunk refresh automatically in the dashboard (pass/fail counts update without restarting the TUI)
- When provider is set to Anthropic in Settings, the agent run invokes the Claude Code CLI with the correct harness config and `--print` args; changing provider to Ollama invokes `ollama run <model>` instead; the configured model from `ProviderConfig` is passed to the invocation
- Pressing `/` from any TUI screen opens a command input overlay; typing `/gate-check` runs gate evaluation on the active chunk and shows pass/fail result inline; `/status` shows cycle status; `/pr-create` triggers PR creation — all without leaving the TUI
- Pressing `m` from the dashboard opens the MCP server configuration panel showing servers from `.assay/mcp.json`; the user can add or remove servers; changes persist atomically to disk; Esc returns to dashboard
- `just ready` passes; `cargo build -p assay-tui` produces `target/debug/assay-tui`; no deadlock or panic on normal agent output; TUI remains responsive during agent execution

## Key Risks / Unknowns

- **Unified TUI event loop with streaming** — The current `run()` function uses `event::read()` (blocking). Adding agent stdout streaming requires a channel-based event loop. The refactor touches the core loop that all other TUI functionality depends on — a mis-wired loop drops key events or deadlocks. Boundary: the new loop must deliver terminal events and agent lines with the same fidelity as the old loop, provable by tests.
- **Streaming vs completion** — `pipeline::launch_agent()` reads stdout after process completion (batch). The TUI needs line-by-line streaming. We need a new streaming variant that feeds into the channel without the existing blocking pattern conflicting with the TUI's event thread.
- **Provider CLI invocation differences** — Anthropic (Claude Code) uses `claude --print --spec ...`; OpenAI would use a different CLI or direct API; Ollama uses `ollama run <model>`. These three dispatch paths must be implemented without a trait hierarchy (D001).

## Proof Strategy

- **Channel-based event loop** → retired in S01: integration test drives a mock subprocess (echoes lines then exits), pumps the resulting `TuiEvent::AgentLine` / `TuiEvent::AgentDone` events through the real channel loop, and asserts `Screen::AgentRun` accumulates all lines and transitions to `Done` — no separate real-terminal test needed since the event loop and App state are isolated from the terminal
- **Streaming vs completion** → retired in S01: `launch_agent_streaming(line_tx)` spawns a background thread that reads stdout line-by-line and sends each line; the existing `launch_agent()` batch path is untouched; both are proven by their own tests
- **Provider dispatch differences** → retired in S02: unit tests verify correct CLI args per `ProviderKind`; `provider_dispatch_returns_correct_args(ProviderKind::Anthropic)`, `provider_dispatch_returns_correct_args(ProviderKind::Ollama)` etc. Real invocation is UAT-only

## Verification Classes

- Contract verification: channel loop unit tests (AgentLine delivery, AgentDone transition, no deadlock on bounded channel); provider dispatch arg generation tests; slash command parse + dispatch tests; MCP config round-trip tests; `cargo test --workspace`
- Integration verification: `launch_agent_streaming` integration test with a real subprocess (echo-based) proves line-by-line delivery with real pipes; settings → provider dispatch wiring proven by agent run test with mock harness writer
- Operational verification: `cargo build -p assay-tui` succeeds; TUI launches on real project; `just ready` passes (fmt, lint, test, deny)
- UAT / human verification: configure Ollama → press `r` → watch output stream → see gate results update; `/gate-check` slash command shows real gate results; MCP panel shows and saves real server config

## Milestone Definition of Done

This milestone is complete only when all are true:

- All four slices are complete with their tests passing
- `just ready` passes (fmt, lint, test, deny)
- `cargo build -p assay-tui` produces `target/debug/assay-tui` without warning
- All existing `cargo test -p assay-tui` tests still pass after the S01 event loop refactor (no regressions)
- `r` key from the TUI dashboard on a real InProgress project streams real Claude Code output to the screen
- Provider changes in Settings result in different CLI invocations for the next `r` run (verified manually with Anthropic and Ollama)
- `/gate-check` slash command produces gate pass/fail output for the active chunk without crashing the TUI
- MCP panel reads and writes `.assay/mcp.json` correctly; round-trip test passes

## Requirement Coverage

- Covers: R053 (TUI agent spawning — S01), R054 (provider abstraction — S02), R055 (TUI MCP server management — S04), R056 (TUI slash commands — S03)
- Partially covers: none
- Leaves for later: R057 (OpenCode plugin — M008), R058 (advanced PR workflow — M008), R059 (gate history analytics — M008)
- Orphan risks: none — all four M007 requirements are mapped to slices

## Slices

- [x] **S01: Channel Event Loop and Agent Run Panel** `risk:high` `depends:[]`
  > After this: pressing `r` from the TUI dashboard on a project with an InProgress chunk spawns the Claude Code agent, streams its stdout line-by-line into a `Screen::AgentRun` panel (proven by integration test with echo subprocess; real Claude invocation is UAT), and shows Done/Failed status when it exits — the event loop refactor from blocking `event::read()` to channel-based `TuiEvent` dispatch is complete and all 27 existing TUI tests still pass

- [x] **S02: Provider Dispatch and Harness Wiring** `risk:medium` `depends:[S01]`
  > After this: `ProviderKind` in `App.config` routes agent spawning to the correct harness adapter — Anthropic uses existing Claude Code adapter, Ollama uses `ollama run <model>`, OpenAI uses a new minimal adapter; Settings screen gains per-phase model input fields; unit tests prove correct CLI args per provider; pressing `r` with Ollama as provider invokes `ollama` instead of `claude`

- [x] **S03: Slash Command Overlay** `risk:low` `depends:[S01]`
  > After this: pressing `/` from any non-wizard TUI screen opens a command input overlay with tab completion; typing `/gate-check` runs gate evaluation on the active chunk and shows pass/fail results inline; `/status` shows cycle status; `/next-chunk` shows active chunk info; `/pr-create` triggers PR creation — all dispatching to existing assay-core functions, proven by integration tests driving synthetic key events through the overlay

- [ ] **S04: MCP Server Configuration Panel** `risk:medium` `depends:[]`
  > After this: pressing `m` from the dashboard opens `Screen::McpPanel` showing servers from `.assay/mcp.json` (or "none configured" when absent); pressing `a` opens an add-server form (name + command); pressing `d` deletes the selected server; `w` writes changes atomically to `.assay/mcp.json`; `Esc` returns to dashboard — proven by integration tests with tempdir fixtures; no live async MCP client required

## Boundary Map

### S01 → S02, S03

Produces:
- `TuiEvent` enum in `main.rs`: `Key(crossterm::event::KeyEvent)`, `Resize(u16, u16)`, `AgentLine(String)`, `AgentDone { exit_code: i32 }` — unified event type for the TUI main loop
- `run(terminal)` refactored: single `mpsc::Receiver<TuiEvent>` loop; one background thread feeds `Key`/`Resize` events; agent thread (when active) feeds `AgentLine`/`AgentDone` events; both push to same sender
- `launch_agent_streaming(cli_args, working_dir, line_tx: mpsc::Sender<String>) -> JoinHandle<i32>` free function in `assay-core::pipeline` — spawns child, reads stdout line-by-line, sends each line; returns thread handle; exit code delivered via the handle's join value
- `Screen::AgentRun { chunk_slug: String, lines: Vec<String>, scroll_offset: usize, status: AgentStatus }` variant in `Screen` enum
- `AgentStatus` enum: `Running`, `Done { exit_code: i32 }`, `Failed { exit_code: i32 }` — used to render terminal state and decide post-run gate refresh
- `App.agent_thread: Option<std::thread::JoinHandle<i32>>` — handle to the streaming background thread, polled on `AgentDone` event
- `r` key handler in Dashboard arm: gets active chunk slug from `cycle_status`, constructs `HarnessProfile`, calls `HarnessWriter`, spawns `launch_agent_streaming`, transitions to `Screen::AgentRun`
- `draw_agent_run(frame, area, chunk_slug, lines, scroll_offset, status)` free fn — scrollable output list, status line at bottom, `Esc` returns to Dashboard
- On `AgentDone`: refreshes `self.milestones`, `self.cycle_slug`, `self.detail_run` if applicable

Consumes:
- nothing (can be built on top of M006 App/Screen foundation)

### S02 → S03

Produces:
- `provider_harness_writer(config: &Config) -> Box<HarnessWriter>` free function in `assay-tui::agent` module — dispatches to correct adapter based on `ProviderKind`, wraps the three provider paths without a trait
- `OllamaConfig { model: String }` and `OpenAiConfig { model: String, api_key_env: String }` structs in `assay-tui::agent` — TUI-local invocation config, not persisted to assay-types
- Settings screen extended with model text-input fields: `planning_model`, `execution_model`, `review_model` per phase — editable via char input in the existing `Screen::Settings` flow
- All three provider paths tested: `provider_dispatch_anthropic_uses_claude_binary`, `provider_dispatch_ollama_uses_ollama_binary`, `provider_dispatch_openai_uses_openai_binary` unit tests pass

Consumes from S01:
- `r` key → `Screen::AgentRun` flow already wired (S01)
- `App.config: Option<assay_types::Config>` with `ProviderKind` available at agent spawn time

### S03 → (milestone complete)

Produces:
- `SlashCmd` enum: `GateCheck`, `SpecShow`, `Status`, `NextChunk`, `PrCreate` — the M007 command set
- `parse_slash_cmd(input: &str) -> Option<SlashCmd>` free function in `assay-tui::slash` module
- `execute_slash_cmd(cmd: SlashCmd, project_root: &Path) -> String` free function — calls assay-core synchronously, returns result as a display string
- `SlashState { input: String, suggestion: Option<String>, result: Option<String>, error: Option<String> }` struct
- `Screen::SlashCmd(SlashState)` variant
- `draw_slash_overlay(frame, area, state)` free fn — bottom-aligned 1-line input + optional result area
- `/` key handler: opens `Screen::SlashCmd(SlashState::default())`; Tab completes; Enter dispatches; Esc closes
- 6 integration tests in `tests/slash_commands.rs`: one per command plus parse-unknown-returns-none

Consumes from S01:
- `TuiEvent` loop (can receive `/` key from any screen)

### S04 → (milestone complete, depends: [])

Produces:
- `McpServerEntry { name: String, command: String, args: Vec<String> }` struct in `assay-tui::mcp_panel`
- `mcp_config_load(root: &Path) -> Vec<McpServerEntry>` free function — reads `.assay/mcp.json` or returns empty vec
- `mcp_config_save(root: &Path, servers: &[McpServerEntry]) -> Result<()>` free function — writes `.assay/mcp.json` atomically (NamedTempFile pattern per D093)
- `MCP_JSON_SCHEMA` — minimal JSON shape `{ "mcpServers": { "<name>": { "command": "...", "args": [...] } } }`
- `Screen::McpPanel { servers: Vec<McpServerEntry>, selected: usize, add_form: Option<AddServerForm>, error: Option<String> }` variant
- `draw_mcp_panel(frame, area, servers, selected, add_form, error)` free fn
- `m` key handler in Dashboard: loads from `.assay/mcp.json`, transitions to `Screen::McpPanel`
- `a` key adds server (opens inline form), `d` key deletes selected, `w` writes, `Esc` cancels/returns
- 4 integration tests: `mcp_panel_loads_empty_when_no_file`, `mcp_panel_loads_from_mcp_json`, `mcp_panel_add_server_writes_file`, `mcp_panel_delete_server_writes_file`

Consumes:
- M006 `App`/`Screen` foundation; nothing from S01/S02/S03 (fully independent)
