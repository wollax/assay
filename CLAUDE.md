# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Assay is an agentic development kit that combines spec-driven workflows with gated quality checks and reviews. It provides:

- A CLI tool (`crates/assay-cli`)
- A TUI tool (`crates/assay-tui`)
- An IDE (TBD, `ide/`)
- Installable plugins for agentic AI systems (`plugins/`)

## Build & Dev Commands

```bash
mise install          # Install all tools (Rust, Just, Node, cargo-deny)
just build            # Build all workspace crates
just test             # Run all tests
just lint             # Run clippy with -D warnings
just fmt              # Format code
just fmt-check        # Check formatting without modifying
just deny             # Run cargo-deny (licenses, advisories)
just ready            # Full check suite: fmt-check + lint + test + deny
just dev              # Watch for changes and rebuild
just cli -- <args>    # Run the CLI
just tui              # Run the TUI
```

## Workspace Layout

```
crates/
  assay-types  →  Shared serializable types (serde, schemars). No business logic.
  assay-core   →  Domain logic: specs, gates, reviews, workflows. Depends on assay-types.
  assay-cli    →  CLI binary (clap). Depends on assay-core.
  assay-tui    →  TUI binary (ratatui + crossterm). Depends on assay-core.
```

### Dependency Graph

```
assay-cli ──→ assay-core ──→ assay-types
assay-tui ──→ assay-core ──→ assay-types
```

## Conventions

- Lean towards functional and declarative patterns
- Use workspace dependencies from root `Cargo.toml` — never add deps to individual crates without adding to workspace first
- Types shared between crates belong in `assay-types`
- Business logic belongs in `assay-core`
- Binary crates (`assay-cli`, `assay-tui`) are thin wrappers that delegate to `assay-core`
- Run `just ready` before considering work complete
