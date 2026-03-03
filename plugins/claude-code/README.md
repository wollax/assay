# Assay — Claude Code Plugin

Agentic development kit with spec-driven workflows and gated quality checks for Claude Code.

## Prerequisites

The `assay` binary must be available in your PATH.

**Install from source:**

```bash
cargo install assay-cli
```

**Or build from the repository:**

```bash
git clone https://github.com/wollax/assay.git
cd assay
cargo install --path crates/assay-cli
```

## Installation

```bash
claude plugin add /path/to/assay/plugins/claude-code
```

Or for development, point directly at the plugin directory:

```bash
claude --plugin-dir /path/to/assay/plugins/claude-code
```

## Verification

After installing the plugin, verify it is working:

1. **MCP tools available:** The Assay MCP server should register three tools — `spec_list`, `spec_get`, and `gate_run`. These appear in Claude's tool list when the plugin is active.

2. **Skills available:** Two skills should be accessible:
   - `/assay:gate-check` — run quality gates for a spec
   - `/assay:spec-show` — display a spec's criteria and details

## What the Plugin Provides

| Component | Description |
| --- | --- |
| **MCP Server** | Registers `assay mcp serve` as a stdio MCP server providing `spec_list`, `spec_get`, and `gate_run` tools |
| **Skills** | `/assay:gate-check` and `/assay:spec-show` for structured spec and gate workflows |
| **Hooks** | PostToolUse reminder after file edits; Stop hook for gate enforcement |
| **CLAUDE.md** | Workflow snippet prescribing spec-first development with command reference |

## Configuration

### Stop Hook Enforcement

The Stop hook verifies quality gates before allowing the agent to complete work. Control its behavior with the `ASSAY_STOP_HOOK_MODE` environment variable:

| Value | Behavior |
| --- | --- |
| `enforce` (default) | Block completion when gates fail |
| `warn` | Warn about failing gates but allow completion |
| `off` | Disable the Stop hook entirely |

Set in your shell profile or per-session:

```bash
export ASSAY_STOP_HOOK_MODE=warn
```
