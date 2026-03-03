---
phase: 09-cli-surface-completion
plan: 01
subsystem: cli
tags: [clap, help-text, bare-invocation, no-color, binary-name]
dependency-graph:
  requires: [phase-05, phase-06, phase-07, phase-08]
  provides: [rich-help-text, bare-invocation-status, binary-name-fix, cli-issue-fixes]
  affects: [phase-10]
tech-stack:
  added: []
  patterns: [after_long_help-examples, CommandFactory-print_help, ANSI-constant]
key-files:
  created: []
  modified:
    - crates/assay-cli/Cargo.toml
    - crates/assay-cli/src/main.rs
decisions:
  - Rich help via clap after_long_help on all commands and subcommands (no external help library)
  - Bare invocation detects .assay/ directory presence to choose status vs hint+help
  - show_status() reuses config::load and spec::scan — no new core API needed
  - Hint message goes to stderr, help text goes to stdout via CommandFactory::print_help
  - ANSI_COLOR_OVERHEAD named constant replaces magic number 9
  - NO_COLOR uses var_os per no-color.org spec (handles non-UTF-8 env values)
metrics:
  duration: ~8 minutes
  completed: 2026-03-03
---

# Phase 9 Plan 01: CLI Polish and Help Enrichment Summary

Rich help text with usage examples on all commands, bare invocation status display, binary name fix, and 5 CLI issue resolutions.

## Tasks Completed

### Task 1: Enrich help text, fix binary name, and resolve CLI issues (`41ed97b`)

**Cargo.toml:**
- Added `[[bin]]` section with `name = "assay"` so the installed binary is named `assay` (not `assay-cli`)

**Help enrichment:**
- Added `after_long_help` with usage examples to `Cli` (top-level), all `Command` variants (`Init`, `Mcp`, `Spec`, `Gate`), and all leaf subcommands (`McpCommand::Serve`, `SpecCommand::Show`, `SpecCommand::List`, `GateCommand::Run`)
- Examples cover both human CLI usage and agent/JSON usage patterns
- `--help` shows examples; `-h` shows compact summary

**5 CLI issue fixes (from cli-spec-cleanup issue):**
1. ANSI constant: Extracted magic number `9` to `ANSI_COLOR_OVERHEAD` named constant
2. NO_COLOR: Changed from `std::env::var("NO_COLOR").is_err()` to `std::env::var_os("NO_COLOR").is_none()`
3. MCP error: Changed `{e:?}` (Debug) to `{e}` (Display) in MCP serve error handler
4. Init dedup: Replaced inline `current_dir()` in init handler with `project_root()` helper
5. Spec list alignment: Always pad slug to `name_width` regardless of description presence

### Task 2: Implement bare invocation behavior (`7e26e0f`)

- Added `show_status(root: &Path)` function that loads config and scans specs
- **In-project** (`.assay/` exists): Shows `assay {version} -- {project_name}`, then spec inventory with criteria counts (total and executable per spec)
- **Outside project** (no `.assay/`): Prints hint `"Not an Assay project. Run 'assay init' to get started."` to stderr, then prints full help text via `Cli::command().print_help()`
- Both paths exit 0 (config load failure exits 1)
- Added `use std::path::Path` and `use clap::CommandFactory` imports

## Deviations

None — plan executed exactly as written.

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| `after_long_help` for examples (not `about` or external help) | Clap's built-in mechanism, shows with `--help` but not `-h`, keeps compact output clean |
| `CommandFactory::print_help` for outside-project help | Reuses clap's own rendering, stays in sync with actual help automatically |
| `show_status` as standalone function | Follows existing pattern (`handle_spec_show`, `handle_spec_list`, etc.) |
| Hint message to stderr, help to stdout | Hint is diagnostic (stderr), help is normal output (stdout) — consistent with UNIX conventions |
| No gate execution in status display | Performance: bare invocation should be fast (filesystem reads only) |

## Verification

- `just ready` passes (fmt-check + lint + test + deny)
- `assay --help` shows all subcommands with examples
- `assay init --help`, `assay spec show --help`, `assay gate run --help`, `assay mcp serve --help` all show command-specific examples
- Bare `assay` inside project shows status with spec inventory
- Bare `assay` outside project shows hint + help text
- Binary name is `assay` via `[[bin]]` in Cargo.toml
- `main.rs` is 686 lines (exceeds 400 minimum)
