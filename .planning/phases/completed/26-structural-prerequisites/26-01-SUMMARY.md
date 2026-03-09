---
phase: 26-structural-prerequisites
plan: 01
subsystem: cli
tags: [refactor, module-extraction, cli-structure]
dependency-graph:
  requires: []
  provides: [cli-commands-module-hierarchy, modular-cli-handlers]
  affects: [phase-32-cli-polish, phase-28-worktree-manager]
tech-stack:
  added: []
  patterns: [flat-module-per-subcommand, shared-helpers-in-mod-rs, pub-crate-visibility]
key-files:
  created:
    - crates/assay-cli/src/commands/mod.rs
    - crates/assay-cli/src/commands/gate.rs
    - crates/assay-cli/src/commands/context.rs
    - crates/assay-cli/src/commands/spec.rs
    - crates/assay-cli/src/commands/checkpoint.rs
    - crates/assay-cli/src/commands/mcp.rs
    - crates/assay-cli/src/commands/init.rs
  modified:
    - crates/assay-cli/src/main.rs
decisions:
  - Subcommand enums moved into their respective modules (not kept in main.rs)
  - Each module exposes a pub(crate) handle() function for dispatch
  - Shared helpers live in commands/mod.rs with pub(crate) visibility
  - init.rs includes show_status() (bare invocation handler) alongside handle_init()
  - main.rs retains Command enum with after_long_help attributes (182 lines, not 80)
metrics:
  duration: 10m
  completed: 2026-03-09
---

# Phase 26 Plan 01: CLI Monolith Extraction Summary

Extracted the 2563-line CLI monolith (main.rs) into a commands/ module hierarchy with one flat file per subcommand group, unblocking all v0.3.0 CLI feature work.

## What Was Done

### Task 1: Create commands/ module structure and extract shared helpers
- Created `commands/mod.rs` with 6 pub mod declarations and ~20 shared helper functions (ANSI constants, color formatting, number formatting, project root resolution)
- Extracted `init.rs` with `handle_init()` and `show_status()` (bare invocation handler)
- Extracted `mcp.rs` with `McpCommand` enum, `handle()` dispatcher, and `init_mcp_tracing()`

### Task 2: Extract remaining command modules and slim main.rs
- Extracted `gate.rs` (853 lines): GateCommand, StreamConfig, StreamCounters, stream_criterion, all gate run/history handlers, save_run_record
- Extracted `context.rs` (735 lines): ContextCommand, GuardCommand, diagnose/list/prune handlers, all guard daemon handlers with #[cfg(unix)]/#[cfg(not(unix))] pairs preserved
- Extracted `spec.rs` (344 lines): SpecCommand, show/list/new handlers, print_criteria_table, print_spec_table
- Extracted `checkpoint.rs` (169 lines): CheckpointCommand, save/show/list handlers
- Slimmed main.rs to 182 lines (Cli struct, Command enum with help text, dispatch match, main)

## Decisions Made

1. **main.rs is 182 lines, not ~80** — the Command enum's `after_long_help` attributes are tightly coupled to the enum definition and cannot move to submodules without breaking clap derive. The structural code itself is minimal.
2. **Each module has a `handle()` dispatcher** — main.rs calls `commands::gate::handle(command)` rather than matching on GateCommand variants directly, keeping dispatch one level deep.
3. **init.rs hosts show_status()** — the bare `assay` invocation (no subcommand) shows project status, which is closely related to init behavior. This keeps main.rs purely structural.

## Deviations

None — plan executed exactly as written.

## Verification

- `cargo check -p assay-cli` — zero errors, zero warnings
- `cargo test -p assay-cli` — zero failures
- `just fmt-check` — passes
- `just lint` — passes (clippy with -D warnings)
- `just test` — all 329+ workspace tests pass
- All 7 module files exist with correct content

---

*Phase: 26-structural-prerequisites, Plan: 01*
*Completed: 2026-03-09*
