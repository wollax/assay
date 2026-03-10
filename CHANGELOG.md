# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-03-10

### Added
- Git worktree lifecycle management: create, list, status, cleanup (CLI + MCP tools)
- Gate output head+tail truncation with byte budget, UTF-8 safety, and independent stdout/stderr budgets
- Truncation metadata (`truncated`, `original_bytes`) in MCP gate criterion responses
- Error message improvements: command-not-found detection (exit 127/126), spec-not-found diagnostics with fuzzy matching, TOML parse error formatting with line/column/caret
- Spec-not-found errors include list of available spec names across CLI and MCP
- MCP stdout fallback for failure reason extraction when stderr is empty
- Display impls for all public enums in assay-types
- Eq derives on all types without float fields
- Json error variant and ergonomic constructors on AssayError

### Fixed
- TTY detection added to `colors_enabled()` for correct NO_COLOR handling
- Truncation serialization aligned with source type for failed gate arm
- GuardDaemon stores project_dir for checkpoint saves instead of using current_dir()
- PID file writes followed by fsync for durability

### Changed
- CLI commands extracted to `commands/` module structure (main.rs slimmed)
- GateCriterion/Criterion structural duplication merged into single Criterion type
- Enforcement validation and gate evaluation shared logic extracted (evaluate_criteria, validate_criteria)
- StreamCounters gains tally() and gate_blocked() methods with doc comments
- StreamConfig fields documented; construction deduplicated via new()
- Help text duplication removed between top-level and subcommand groups
- Color branch duplication collapsed in spec show table and history table
- Magic strings extracted to constants (DIRECTORY_SPEC_INDICATOR, COLUMN_GAP)
- Unnecessary clones removed from MCP gate_run handler
- History save API consolidated; generate_run_id visibility tightened

## [0.2.0] - 2026-03-08

### Added
- Type system foundation: relocate result types to assay-types, enforce serde hygiene
- FileExists gate wiring into evaluation dispatch
- Enforcement levels: required/advisory on criteria with separate pass/fail tracking
- Run history: JSON persistence with atomic writes and configurable retention
- CLI history command with table/detail views
- Agent gate recording: gate_report/gate_finalize MCP tools with session lifecycle
- MCP hardening: timeout parameter, working_dir validation, error envelopes
- gate_history MCP tool for querying past run results
- CLI hardening: error propagation via anyhow, enforcement-aware streaming output
- Dogfooding spec (self-check.toml) for Assay self-validation
- Session JSONL parser with token extraction and bloat categorization
- CLI context diagnose/list commands and MCP context_diagnose/estimate_tokens tools
- Team state checkpointing: CLI commands and plugin hooks
- Composable pruning engine with 6 strategies, dry-run default, team protection
- Guard daemon with threshold-based pruning, circuit breaker, and reactive overflow recovery

### Fixed
- Documentation inconsistencies in verification files, triage summary accuracy, and closed issue resolution sections (tech debt cleanup phase)

### Changed
- CLI migrated to run() -> anyhow::Result<i32> pattern
- cargo-deny policies tightened (multiple-versions and sources set to deny)
- Feature-gated test-only public API exports

## [0.1.0] - 2026-03-02

### Added
- Domain model: GateKind, GateResult, Criterion types with serde/schemars
- Schema generation pipeline with inventory-based auto-discovery
- Config loading/validation and `assay init` project scaffolding
- Spec file parsing, validation, scanning, and CLI commands
- Gate evaluation engine with timeout enforcement and streaming display
- MCP server with spec_get, spec_list, and gate_run tools
- Claude Code plugin with MCP config, skills, and hooks
- CLI: init, spec show, spec list, gate run, gate run --all, mcp serve
- PostToolUse hook for automatic gate reminders
- Stop hook for gate enforcement before agent completion

### Fixed
- PR review findings across phases 6-10

[0.3.0]: https://github.com/wollax/assay/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/wollax/assay/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/wollax/assay/releases/tag/v0.1.0
