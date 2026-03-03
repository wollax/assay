# Assay

Agentic development kit — spec-driven workflows with gated quality checks for AI coding agents.

## What It Does

Assay enforces quality gates between AI agent output and your main branch. Define specs with acceptance criteria, and Assay evaluates them automatically — both deterministic checks (shell commands, tests) and agent-evaluated assertions.

**v0.1.0** ships the foundation: TOML specs, command gate evaluation, an MCP server for agent integration, and a Claude Code plugin.

## Quick Start

```bash
# Install toolchain
mise install

# Initialize a project
cargo install --path crates/assay-cli
assay init

# Write a spec in .assay/specs/my-feature.toml, then run gates
assay gate run my-feature

# Or run all specs at once
assay gate run --all
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `assay init` | Initialize `.assay/` project structure |
| `assay spec show <name>` | Display a spec with its criteria |
| `assay spec list` | List all specs in the project |
| `assay gate run <name>` | Run gates for a spec |
| `assay gate run --all` | Run gates for all specs |
| `assay mcp serve` | Start the MCP server (stdio) |

## MCP Server

Assay exposes three tools via MCP (Model Context Protocol) for agent integration:

- **spec_list** — Enumerate available specs
- **spec_get** — Retrieve a spec by name
- **gate_run** — Execute all command criteria for a spec

## Claude Code Plugin

Install the plugin from `plugins/claude-code/` to get:

- MCP server auto-registration
- `/gate-check` and `/spec-show` skills
- PostToolUse hook for automatic gate reminders
- Stop hook preventing completion without passing gates

## Project Structure

```
crates/
  assay-types/   Shared serializable types (serde, schemars)
  assay-core/    Domain logic: specs, gates, config
  assay-cli/     CLI binary (clap)
  assay-tui/     TUI binary (ratatui) — scaffold
  assay-mcp/     MCP server library (rmcp)
plugins/
  claude-code/   Claude Code plugin
schemas/         JSON Schemas (generated from assay-types)
```

## Development

```bash
just build      # Build all crates
just test       # Run tests (119 tests)
just lint       # Clippy with -D warnings
just fmt        # Format code
just ready      # Full check suite: fmt + lint + test + deny
just schemas    # Regenerate JSON schemas
```

## License

MIT
