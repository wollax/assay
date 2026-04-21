# Assay — Project Documentation Index

## Project Overview

- **Type:** Monorepo with 3 parts
- **Primary Language:** Rust (stable, Edition 2024)
- **Version:** 0.5.0
- **Architecture:** Layered crate DAG with sync-first core

### Quick Reference

#### Assay (crates/)
- **Type:** Backend (library + CLI + MCP server + TUI)
- **Crates:** assay-types, assay-core, assay-backends, assay-harness, assay-mcp, assay-cli, assay-tui
- **Root:** `crates/`

#### Smelt (smelt/crates/)
- **Type:** Backend (container execution daemon)
- **Crates:** smelt-core, smelt-cli
- **Root:** `smelt/crates/`

#### Plugins (plugins/)
- **Type:** Extension (AI agent plugins)
- **Packages:** claude-code, codex, opencode, smelt-agent
- **Root:** `plugins/`

## Generated Documentation

### Overview & Structure
- [Project Overview](./project-overview.md)
- [Source Tree Analysis](./source-tree-analysis.md)

### Architecture (per part)
- [Architecture — Assay](./architecture-assay.md)
- [Architecture — Smelt](./architecture-smelt.md)
- [Architecture — Plugins](./architecture-plugins.md)
- [Integration Architecture](./integration-architecture.md)

### Guides
- [Development Guide](./development-guide.md)
- [Deployment Guide](./deployment-guide.md)
- [Contribution Guide](./contribution-guide.md)

### Exhaustive Scans (raw analysis)
- [Scan — Assay](./scan-assay.md) — Complete API surface, domain model, data flows
- [Scan — Smelt](./scan-smelt.md) — Complete API surface, domain model, data flows
- [Scan — Plugins](./scan-plugins.md) — All plugins, skills, hooks, configuration

## Existing Documentation

### Root Level
- [README.md](../README.md) — Project overview and quick start
- [CONTRIBUTING.md](../CONTRIBUTING.md) — Dev setup and workflow
- [CHANGELOG.md](../CHANGELOG.md) — Release history (v0.1.0–v0.4.0)
- [AGENTS.md](../AGENTS.md) — AI agent instructions
- [CLAUDE.md](../CLAUDE.md) — Claude Code project instructions
- [WORKFLOW-assay.md](../WORKFLOW-assay.md) — Smelt workflow configuration

### Part-Specific
- [smelt/README.md](../smelt/README.md) — Smelt installation, CLI, server mode, examples
- [plugins/claude-code/README.md](../plugins/claude-code/README.md) — Claude Code plugin
- [plugins/codex/README.md](../plugins/codex/README.md) — Codex plugin
- [plugins/opencode/README.md](../plugins/opencode/README.md) — OpenCode plugin
- [schemas/README.md](../schemas/README.md) — JSON Schema generation
- [ide/README.md](../ide/README.md) — IDE placeholder (TBD)
- [examples/close-the-loop/README.md](../examples/close-the-loop/README.md) — M024 checkpoint demo

### AI Context
- [project-context.md](../_bmad-output/project-context.md) — 95 implementation rules for AI agents

## Getting Started

```bash
# Prerequisites
mise install                    # Install Rust toolchain

# Build
just build                      # Build all crates

# Test
just test                       # Run all tests (~836 tests)

# Full CI check
just ready                      # fmt + lint + test + deny + plugin-version

# Run CLI
cargo run -p assay-cli -- init  # Initialize Assay project
cargo run -p assay-tui          # Launch TUI

# Run Smelt
cargo run -p smelt-cli -- init  # Generate job manifest skeleton
```

---

*Generated: 2026-04-21 | Scan level: exhaustive | Mode: initial_scan*
