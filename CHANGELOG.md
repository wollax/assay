# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-03-15

### Added
- `gate_evaluate` MCP tool — single-call headless agent evaluation with diff computation, subprocess orchestration, structured per-criterion results, and automatic GateRunRecord persistence
- EvaluatorOutput JSON schema with lenient serde_json::Value intermediate parsing for subprocess output
- Diff token budgeting via context engine integration — model window minus spec/prompt overhead, head-first + tail fallback truncation
- DiffTruncation metadata (original/truncated size, strategy, affected files) in GateRunRecord
- `spec_validate` MCP tool with structured diagnostics — TOML parse errors, criterion uniqueness, prompt field validation, cross-spec dependency cycle detection
- `WorkSession` type with JSON persistence, phase transitions (created → agent_running → gate_evaluated → completed | abandoned), and timestamps
- Session MCP tools: `session_create`, `session_get`, `session_update`, `session_list` with spec_name/status filters
- Startup recovery scan for stale `agent_running` sessions — marks abandoned with recovery notes
- Context engine integration via external `cupel` crate for token-budgeted context windowing with passthrough optimization
- `warnings` field on all mutating MCP tool responses for surfacing non-fatal issues
- Outcome-filtered `gate_history` with limit parameter (default 10, max 50)
- `spec_get` resolve parameter showing effective timeouts with 3-tier precedence and working_dir validation
- Growth rate metrics in `estimate_tokens` — average tokens per turn, estimated turns remaining
- Base-branch-relative ahead/behind for worktree status (fixes false 0/0 for assay-managed branches)
- Diff context captured at gate_run time with 32 KiB head-biased truncation
- Enriched gate session error messages distinguishing timeout vs not-found with recovery hints

### Fixed
- 120+ tech debt issues resolved across types, core, CLI, and MCP crates
- SessionPhase marked `#[non_exhaustive]` for forward compatibility
- DiffTruncation byte fields use u64 for platform-independent serialization
- WorktreeInfo ahead/behind fields use u32 instead of usize
- Evaluator schema cached with LazyLock (generated once per process)
- EvaluateCriterionResult uses typed CriterionOutcome/Enforcement enums instead of freeform strings
- GateHistoryEntry includes required_passed/advisory_passed count fields
- Session path component validation prevents directory traversal

### Changed
- gate_evaluate uses subprocess model — parent process owns all parsing and persistence, evaluator never calls MCP tools
- Session management within gate_evaluate uses direct Rust function calls, not MCP round-trips
- RecoverySummary includes truncated flag for 100-session cap indication

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

[0.4.0]: https://github.com/wollax/assay/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/wollax/assay/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/wollax/assay/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/wollax/assay/releases/tag/v0.1.0
