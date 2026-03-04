# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
