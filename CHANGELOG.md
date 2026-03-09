# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
- PR review findings for tech debt cleanup phase

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
