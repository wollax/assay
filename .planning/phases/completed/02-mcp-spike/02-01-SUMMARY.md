---
phase: 02-mcp-spike
plan: 01
subsystem: assay-mcp
tags: [spike, mcp, rmcp, stdio, json-rpc, go-no-go]

requires: []
provides:
  - assay-mcp crate with SpikeServer and spike_echo tool
  - "assay mcp serve" CLI subcommand on stdio transport
  - GO decision for rmcp 0.17 architecture
affects:
  - Phase 8 (MCP Server Tools) — confirmed rmcp pattern for real tool implementation
  - Phases 3-10 — unblocked by GO decision

tech-stack:
  added:
    - rmcp 0.17 (MCP server framework, stdio transport)
    - tracing + tracing-subscriber (stderr-only logging with EnvFilter)
  patterns:
    - "#[tool_router] / #[tool_handler] macro pattern for tool registration"
    - "tracing-subscriber stderr writer to keep stdout clean for JSON-RPC"
    - "Implementation::from_build_env() for server metadata from Cargo.toml"

key-files:
  created:
    - crates/assay-mcp/src/lib.rs
    - crates/assay-mcp/src/spike.rs
    - crates/assay-mcp/src/logging.rs
    - crates/assay-mcp/Cargo.toml
  modified:
    - Cargo.toml
    - crates/assay-cli/Cargo.toml
    - crates/assay-cli/src/main.rs
    - .gitignore

decisions:
  - "GO: rmcp 0.17 + stdio + Claude Code integration path confirmed for v0.1 architecture"
  - "Spike code remains as working reference until Phase 8 replaces with real tools"
  - "tracing-subscriber to stderr with warn default — RUST_LOG overrides for debugging"
  - ".mcp.json added to .gitignore — local dev convenience, real plugin config in Phase 10"

metrics:
  duration: "1 session"
  completed: 2026-03-01
---

# Phase 02 Plan 01: MCP Spike Server Summary

Validated rmcp 0.17 + stdio transport + Claude Code MCP client exchange end-to-end, confirming GO for the v0.1 architecture.

## What Was Done

### Task 1: Implement MCP spike server and wire CLI subcommand

Created the `assay-mcp` crate with three modules:

- **`spike.rs`** — `SpikeServer` struct using rmcp's `#[tool_router]` and `#[tool_handler]` macros. Single `spike_echo` tool returns a hardcoded greeting. `ServerHandler` provides server info via `Implementation::from_build_env()`.
- **`logging.rs`** — `tracing-subscriber` init to stderr with `EnvFilter` (default `warn`, `RUST_LOG` override). Uses `try_init()` to avoid double-init panics.
- **`lib.rs`** — Public `serve()` entry point that initializes logging, creates `SpikeServer`, and serves on stdio transport.

Wired into CLI as `assay mcp serve` via nested clap subcommands (`Command::Mcp` -> `McpCommand::Serve`). Main function converted to `#[tokio::main] async fn main()`.

`just ready` passed clean (fmt-check, lint, test, deny).

### Task 2: Validate MCP protocol roundtrip and document GO decision

Three validation criteria tested and passed:

1. **Protocol roundtrip** — Raw JSON-RPC `initialize` request via stdin produced valid response with `result.serverInfo` containing protocol version, capabilities, and server implementation details.
2. **Stdout cleanliness** — All stdout output is valid JSON. No tracing, no panics, no stray bytes. Python JSON parser confirmed `CLEAN`.
3. **Claude Code integration** — Project-scoped `.mcp.json` configured with `cargo run --release -p assay-cli -- mcp serve`. Claude Code discovered the `assay` MCP server, listed `spike_echo` in available tools, and successfully called it receiving `"spike: hello from assay"`.

**Decision: GO** — Proceed with v0.1 architecture. rmcp 0.17 is the confirmed MCP server framework.

## Observations

- rmcp's macro system (`#[tool_router]`, `#[tool_handler]`, `#[tool]`) is ergonomic and produces clean tool registration code
- The `Implementation::from_build_env()` helper automatically populates server name/version from `Cargo.toml` — no manual version strings needed
- Keeping `tracing-subscriber` pointed at stderr is sufficient to prevent stdout contamination; no special buffering or flushing needed
- `ServiceExt::serve()` + `waiting()` is the idiomatic rmcp pattern for long-running stdio servers
- Spike code is intentionally minimal and disposable — Phase 8 will replace it entirely with real tools

## Commits

| Hash | Message |
|------|---------|
| `53e9c64` | feat(02-01): implement MCP spike server with spike_echo tool |
| `fa79a86` | chore(02-01): add .mcp.json to .gitignore |
