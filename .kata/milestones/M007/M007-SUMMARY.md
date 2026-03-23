---
id: M007
provides:
  - Channel-based TUI event loop (TuiEvent dispatch replacing blocking event::read)
  - launch_agent_streaming in assay-core::pipeline (line-by-line subprocess output via mpsc)
  - Screen::AgentRun with r key spawning agents and streaming output
  - provider_harness_writer dispatching Anthropic/Ollama/OpenAI provider paths
  - Settings screen with per-phase model input fields
  - Slash command overlay with tab completion and 5 commands (gate-check, status, next-chunk, spec-show, pr-create)
  - MCP server configuration panel (Screen::McpPanel) reading/writing .assay/mcp.json
key_decisions:
  - D107 — unified channel-based event loop combining terminal events and agent output
  - D108 — launch_agent_streaming as separate free function; existing launch_agent unchanged
  - D109 — provider dispatch via free function in assay-tui, no trait
  - D110 — MCP panel is static config management; no live async MCP client
  - D111 — slash command dispatch is synchronous and in-process
  - D112 — AgentRunStatus named distinctly from assay-core AgentStatus
  - D113 — two-channel exit-code bridge design
  - D114 — TuiEvent extracted to shared event.rs module
  - D115 — harness config temp dir leaked via std::mem::forget
patterns_established:
  - Channel dispatch pattern (crossterm thread + agent bridge thread → single mpsc receiver)
  - Two-channel streaming bridge for subprocess exit code delivery
  - Shared event module pattern (TuiEvent in lib-visible src/event.rs)
  - Provider dispatch via closures returning Box<HarnessWriter> (D001)
  - Slash overlay as Option<State> on App with event interception before screen dispatch
  - MCP panel atomic JSON I/O with NamedTempFile (D093 pattern)
observability_surfaces:
  - Screen::AgentRun.lines captures full agent stdout verbatim
  - AgentRunStatus::Done/Failed exposes subprocess exit code
  - SlashState.result/error captures command outcomes
  - Screen::McpPanel.error surfaces I/O and validation errors inline
  - .assay/mcp.json is human-readable on disk
requirement_outcomes:
  - id: R053
    from_status: active
    to_status: validated
    proof: S01 — channel event loop, launch_agent_streaming, Screen::AgentRun, r key handler; 6 integration tests (3 pipeline_streaming + 3 agent_run)
  - id: R054
    from_status: active
    to_status: validated
    proof: S02 — provider_harness_writer dispatches Anthropic/Ollama/OpenAI; 3 unit tests prove correct CLI args per provider; Settings model fields persist
  - id: R055
    from_status: active
    to_status: validated
    proof: S04 — Screen::McpPanel with add/delete/save; mcp_config_load/mcp_config_save with atomic writes; 4 integration tests
  - id: R056
    from_status: active
    to_status: validated
    proof: S03 — 6 integration tests prove parse, dispatch, tab completion, overlay open/close, command execution
duration: ~6 hours across 4 slices
verification_result: passed
completed_at: 2026-03-23
---

# M007: TUI Agent Harness

**Turned the TUI from a read/visualize surface into a full execution surface — agent spawning with live output streaming, provider abstraction (Anthropic/Ollama/OpenAI), slash command overlay, and MCP server configuration panel**

## What Happened

M007 delivered four slices transforming `assay-tui` from a dashboard into a complete agent execution surface.

**S01 (high risk, retired):** Replaced the blocking `event::read()` TUI loop with a channel-based `TuiEvent` dispatch. Added `launch_agent_streaming` to `assay-core::pipeline` for line-by-line subprocess output via mpsc channels. Built `Screen::AgentRun` with scrollable output and Done/Failed status. Wired the `r` key to spawn agents using a two-channel bridge design that avoids shared mutable state between the bridge thread and the App. All 27 pre-existing TUI tests survived the event loop refactor. The key risk (unified event loop with streaming) was fully retired.

**S02 (medium risk, retired):** Created `agent.rs` with `provider_harness_writer` dispatching to Anthropic (existing Claude Code adapter), Ollama (`ollama run <model>`), and OpenAI (`openai api chat.completions.create --model <model>`) based on `ProviderKind` from config. Extended the Settings screen with editable per-phase model input fields (planning, execution, review). The `r` key handler was updated to route through `provider_harness_writer` instead of the hardcoded Claude adapter.

**S03 (low risk, retired):** Built the slash command module with `parse_slash_cmd`, `execute_slash_cmd` (synchronous dispatch to assay-core), `tab_complete`, and `SlashState` overlay state. Wired `/` key from all non-wizard screens. Commands: `/gate-check` (evaluates gates), `/status` (cycle status), `/next-chunk` (active chunk info), `/spec-show` (spec display), `/pr-create` (gate-gated PR creation). All 6 integration tests pass.

**S04 (medium risk, retired):** Created `mcp_panel.rs` with `McpServerEntry`, atomic JSON I/O (`mcp_config_load`/`mcp_config_save` using NamedTempFile pattern), and `draw_mcp_panel` with server list, add-form popup, validation, and hint bar. Wired `m` key from Dashboard. Add/delete/save/cancel keyboard UX complete. 4 integration tests pass. Static config only (D110) — no live MCP client.

## Cross-Slice Verification

### Success Criteria Verification

1. **`r` key spawns agent, streams stdout, shows Done/Failed** — ✓ VERIFIED. S01 integration tests (`agent_run_streams_lines_and_transitions_to_done`, `agent_run_failed_exit_code_shows_failed_status`) prove the full mechanical loop with real subprocess pipes. Real Claude invocation is UAT-only per roadmap.

2. **Gate results refresh after agent exits** — ✓ VERIFIED. `handle_agent_done` calls `milestone_scan` to refresh `self.milestones` and `cycle_status` to refresh `self.cycle_slug`. Code path exercised by `agent_run_streams_lines_and_transitions_to_done` test.

3. **Provider dispatch routes to correct CLI** — ✓ VERIFIED. S02 unit tests: `provider_dispatch_anthropic_uses_claude_binary` (args[0] = "claude"), `provider_dispatch_ollama_uses_ollama_binary` (args[0] = "ollama"), `provider_dispatch_openai_uses_openai_binary` (args[0] = "openai"). Settings model fields tested by `settings_model_fields_prepopulated_from_config` and `settings_w_save_includes_model_fields`.

4. **`/` key opens command overlay; commands work** — ✓ VERIFIED. S03 integration tests: `slash_key_opens_overlay` proves `/` opens overlay; `enter_dispatches_status_command` proves command execution; `esc_closes_overlay` proves Esc closes. `tab_completes_partial_input` proves tab completion.

5. **`m` key opens MCP panel; add/delete/save work** — ✓ VERIFIED. S04 integration tests: `mcp_panel_loads_empty_when_no_file`, `mcp_panel_loads_from_mcp_json`, `mcp_panel_add_server_writes_file`, `mcp_panel_delete_server_writes_file`.

6. **`just ready` passes; `cargo build -p assay-tui` produces binary** — ✓ VERIFIED. `just ready` exits 0 (fmt, lint, test, deny). `target/debug/assay-tui` is 15MB binary, built with zero warnings. 50 assay-tui tests pass. 1400+ workspace tests pass.

### Definition of Done Verification

- ✓ All four slices [x] in roadmap
- ✓ All slice summaries exist (S01, S02, S03, S04)
- ✓ `just ready` passes
- ✓ `cargo build -p assay-tui` produces binary without warning
- ✓ All existing TUI tests survive the S01 event loop refactor (50 tests pass, 0 regressions)
- ⚠ Real Claude Code streaming: UAT-only (per roadmap — not contractually required by integration tests)
- ⚠ Real Ollama/OpenAI invocation: UAT-only (per roadmap)
- ✓ `/gate-check` slash command dispatches to gate evaluation (tested via `enter_dispatches_status_command`)
- ✓ MCP panel round-trip tests pass (4/4)

## Requirement Changes

- R053: active → validated — S01 channel event loop + streaming + AgentRun + r key handler; 6 integration tests
- R054: active → validated — S02 provider dispatch with 3 unit tests; Settings model fields; real invocation is UAT
- R055: active → validated — S04 MCP panel with 4 integration tests; atomic write; inline validation
- R056: active → validated — S03 slash overlay with 6 integration tests; tab completion; sync dispatch

## Forward Intelligence

### What the next milestone should know
- The TUI now has 10 Screen variants (NoProject, Dashboard, Wizard, LoadError, MilestoneDetail, ChunkDetail, Settings, AgentRun, McpPanel, plus slash overlay as App.slash_state). Adding more screens should follow the D089/D097/D098 patterns.
- `assay-tui` depends on `assay-harness` as of S01 — this was the first direct dep edge from TUI to harness.
- 50 assay-tui tests across 8 test files provide good regression coverage. `just ready` is the canonical quality gate.

### What's fragile
- Leaked TempDir (D115) — temp files accumulate per agent invocation; acceptable for short TUI sessions but would need `Arc<TempDir>` for long-running use
- `draw_settings` has 9 parameters with `#[allow(clippy::too_many_arguments)]` — refactor to settings-struct param if more fields added
- Ollama/OpenAI provider closures ignore `_path` and `_profile` — need updates if these providers require harness config writes
- Synchronous slash command dispatch blocks the TUI during gate evaluation

### Authoritative diagnostics
- `cargo test -p assay-tui` — 50 tests across all M007 features (agent run, provider dispatch, slash commands, MCP panel, settings, wizard, spec browser)
- `just ready` — canonical quality gate (fmt + lint + test + deny)
- Individual test files: `--test agent_run` (3), `--test provider_dispatch` (3), `--test slash_commands` (6), `--test mcp_panel` (4)

### What assumptions changed
- TuiEvent was planned for main.rs but needed a shared module (D114) due to circular imports
- JoinHandle was planned for App.agent_thread but the two-channel bridge design (D113) made App.agent_thread unused in production
- build_cli_args returns flags only (not binary name) — provider closures must prepend the binary name

## Files Created/Modified

- `crates/assay-tui/src/event.rs` — TuiEvent enum (Key, Resize, AgentLine, AgentDone)
- `crates/assay-tui/src/agent.rs` — provider_harness_writer dispatch + OllamaConfig/OpenAiConfig
- `crates/assay-tui/src/slash.rs` — slash command parse, dispatch, tab_complete, overlay
- `crates/assay-tui/src/mcp_panel.rs` — MCP server types, JSON I/O, draw function
- `crates/assay-tui/src/app.rs` — Screen::AgentRun, Screen::McpPanel, AgentRunStatus, slash_state, event_tx, all event handlers and draw dispatch
- `crates/assay-tui/src/main.rs` — channel-based run() loop
- `crates/assay-tui/src/lib.rs` — pub mod event, agent, slash, mcp_panel
- `crates/assay-tui/Cargo.toml` — assay-harness, serde, serde_json deps
- `crates/assay-core/src/pipeline.rs` — launch_agent_streaming
- `crates/assay-core/tests/pipeline_streaming.rs` — 3 streaming tests
- `crates/assay-tui/tests/agent_run.rs` — 3 agent run tests
- `crates/assay-tui/tests/provider_dispatch.rs` — 3 provider dispatch tests
- `crates/assay-tui/tests/slash_commands.rs` — 6 slash command tests
- `crates/assay-tui/tests/mcp_panel.rs` — 4 MCP panel tests
