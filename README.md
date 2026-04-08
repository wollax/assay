# Assay

Agentic development kit — spec-driven workflows with dual-track quality gates for AI coding agents.

## What It Does

Assay enforces quality gates between AI agent output and your main branch. Define specs with acceptance criteria, and Assay evaluates them automatically using two tracks:

- **Deterministic gates:** Shell commands, test suites, linter checks — binary, reproducible, cheap
- **Agent-evaluated gates:** Natural-language assertions verified by AI via MCP — nuanced, context-aware

Gates support **required/advisory enforcement levels**, persist results to **run history** for audit trails, and integrate with both CLI and MCP surfaces.

**v0.4.0** adds `gate_evaluate` — a single MCP call that computes diffs, spawns a headless evaluator subprocess, and persists structured per-criterion results. Also ships `WorkSession` persistence for tracking agent work across phases, `spec_validate` for static spec health checking, context engine integration for token-budgeted diff slicing, and comprehensive observability improvements.

## Quick Start

```bash
# Install toolchain
mise install

# Install the CLI
cargo install --path crates/assay-cli

# Initialize a project
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
| `assay history <name>` | View gate run history for a spec |
| `assay context diagnose` | Token usage, bloat breakdown, context % |
| `assay context list` | List sessions with sizes and token counts |
| `assay context prune` | Dry-run pruning strategies on a session |
| `assay context guard start` | Start the guard daemon for a session |
| `assay worktree create <spec>` | Create isolated git worktree for a spec |
| `assay worktree list` | List active worktrees |
| `assay worktree status <spec>` | Check worktree branch/dirty/ahead status |
| `assay worktree cleanup <spec>` | Remove a worktree |
| `assay checkpoint save` | Save team state checkpoint |
| `assay mcp serve` | Start the MCP server (stdio) |

## MCP Server

Assay exposes tools via MCP (Model Context Protocol) for agent integration:

- **spec_list** — Enumerate available specs
- **spec_get** — Retrieve a spec by name (with optional resolved config)
- **spec_validate** — Static spec health check with structured diagnostics
- **gate_run** — Execute command criteria for a spec (with timeout and enforcement)
- **gate_evaluate** — Single-call headless agent evaluation (diff + subprocess + persist)
- **gate_report** — Submit agent-evaluated gate results with reasoning
- **gate_finalize** — Complete an agent evaluation session
- **gate_history** — Query past gate run results (with outcome filter and limit)
- **session_create** — Start a work session linking worktree, spec, and agent
- **session_get** — Retrieve session details by ID
- **session_update** — Transition session phase and link gate runs
- **session_list** — Enumerate sessions with optional filters
- **context_diagnose** — Full session diagnostics with bloat analysis
- **estimate_tokens** — Token count, context %, and growth rate metrics
- **worktree_create** — Create isolated git worktree for agent work
- **worktree_list** — List active worktrees
- **worktree_status** — Check worktree branch and dirty state
- **worktree_cleanup** — Remove a worktree

## Claude Code Plugin

Install the plugin from `plugins/claude-code/` to get:

- MCP server auto-registration
- `/gate-check` and `/spec-show` skills
- PostToolUse hook for automatic gate reminders
- Stop hook preventing completion without passing gates
- Checkpoint hooks for team state protection

## Project Structure

```
crates/
  assay-types/   Shared serializable types (serde, schemars)
  assay-core/    Domain logic: specs, gates, config, context, pruning, guard
  assay-cli/     CLI binary (clap)
  assay-tui/     TUI binary (ratatui) — scaffold
  assay-mcp/     MCP server library (rmcp)
plugins/
  claude-code/   Claude Code plugin
schemas/         JSON Schemas (generated from assay-types)
```

## Examples

### [Close the Loop](examples/close-the-loop/) — Mid-Session Checkpoints + Auto-Promote

Demonstrates the M024 gate/spec loop: mid-session checkpoint evaluation with early abort on tool-budget violations, and automatic spec promotion (`in-progress` → `verified`) on clean runs. Exercises `EventCount` checkpoint criteria, `NoToolErrors` session-end gates, and `auto_promote` against a real `claude` CLI subprocess.

```bash
just demo-close-the-loop
```

## Development

```bash
just build      # Build all crates
just test       # Run tests (836 tests)
just lint       # Clippy with -D warnings
just fmt        # Format code
just ready      # Full check suite: fmt + lint + test + deny
just schemas    # Regenerate JSON schemas
```

## License

MIT
