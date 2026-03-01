# Assay

Agentic development kit — spec-driven workflows with gated quality checks and reviews.

## Quick Start

```bash
mise install    # Install Rust, Just, and other tools
just build      # Build all crates
just test       # Run tests
just ready      # Run full check suite (fmt, lint, test, deny)
```

## Project Structure

```
crates/
  assay-types/   Shared serializable types
  assay-core/    Domain logic: specs, gates, reviews, workflows
  assay-cli/     CLI binary
  assay-tui/     TUI binary
plugins/
  claude-code/   Claude Code plugin
  codex/         OpenAI Codex plugin
  opencode/      OpenCode plugin
schemas/         JSON Schemas (generated from assay-types)
ide/             IDE (TBD)
```

## License

MIT
