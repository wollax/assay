# M007: TUI Agent Harness — Context

**Gathered:** 2026-03-19
**Status:** Provisional — detail-planning deferred until M006 is complete

## Project Description

The TUI becomes a full agent harness: it can spawn AI agent sessions for the active chunk, display live output, show gate results as they arrive, and surface failures inline. M007 adds provider abstraction (Anthropic, OpenAI, Ollama), MCP server connection management, and slash commands. A developer with Ollama running locally can complete a full spec-driven development cycle entirely within the Assay TUI, with no external AI tool required.

## Why This Milestone

M006 makes the TUI a read/visualize surface. M007 makes it an execution surface. This is the milestone where the TUI becomes "the preferred primary interface" as described in the vision — a standalone development tool rather than a dashboard on top of other tools.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Press a key from the TUI dashboard to start the agent working on the active chunk — watch live output stream into a panel, gate results update in real time
- Configure Anthropic, OpenAI, or Ollama as the AI provider via the settings screen — the next agent run uses the configured provider
- Open the MCP servers panel, connect to additional MCP servers, and see their tools listed
- Type `/` in any TUI context to open the command input and run `/gate-check`, `/plan`, `/status`, `/pr-create` etc.

### Entry point / environment

- Entry point: `assay` TUI binary
- Environment: local terminal with at least one AI provider configured (Anthropic API key, OpenAI API key, or Ollama running locally)
- Live dependencies involved: `claude` / `openai` / `ollama` CLI or API, configured MCP servers

## Completion Class

- Contract complete means: provider abstraction dispatches correctly; agent output captured and displayed; MCP panel shows connected servers; slash commands execute
- Integration complete means: full loop from TUI — launch agent → gates run → results shown — works end-to-end with at least Anthropic provider
- Operational complete means: `just ready` passes; TUI binary launches and all M007 features work without panic

## Final Integrated Acceptance

- Configure Ollama provider in TUI settings — start agent on active chunk — watch output — see gate results update — chunk marked complete
- Connect an additional MCP server from the MCP panel — its tools appear in slash command completions
- `/pr-create` slash command in TUI opens a PR with the same behavior as `assay pr create` CLI

## Risks and Unknowns

- **Provider abstraction design** — Anthropic (Claude Code CLI), OpenAI (openai CLI?), and Ollama (ollama CLI) have different invocation patterns. The abstraction must handle these without becoming a framework. Favor a simple config-driven dispatch over a trait hierarchy.
- **Streaming agent output in Ratatui** — live output from a subprocess requires a background thread writing to a channel, with the TUI event loop consuming it. Deadlock risk if channels fill up. Mitigate with bounded channels.
- **MCP server management from TUI** — requires an async MCP client running alongside the TUI event loop. Complex interaction between sync Ratatui and async MCP. May require tokio runtime bridging.

## Existing Codebase / Prior Art

- `crates/assay-core/src/pipeline.rs` — `launch_agent()`, `run_session()`: base for TUI-invoked agent sessions
- `crates/assay-core/src/evaluator.rs` — async subprocess spawning pattern: relevant for streaming
- `crates/assay-harness/` — Claude Code / Codex / OpenCode adapters: the provider abstraction wraps these
- M006 TUI framework — all M007 panels are layered on top of M006's Ratatui app structure

## Relevant Requirements

- R053 — TUI agent spawning
- R054 — Provider abstraction
- R055 — TUI MCP server management
- R056 — TUI slash commands

## Scope

### In Scope

- Agent spawning from TUI with live output streaming
- Provider abstraction: Anthropic (Claude Code), OpenAI, Ollama
- MCP server connection panel
- Slash command input with completion

### Out of Scope (M007)

- PR status display in TUI (M008)
- PR labels, reviewers, templates (M008)
- History analytics panel (M008)
- OpenCode plugin (M008)
