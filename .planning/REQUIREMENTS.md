# Requirements — v0.1.0 Proof of Concept

## Foundation

- [x] **FND-01**: Upgrade schemars from 0.8 to 1.x across the workspace (required by rmcp)
- [x] **FND-02**: Unified `AssayError` enum with thiserror and `#[non_exhaustive]`, starting with `Io` variant
- [x] **FND-03**: Result type alias `pub type Result<T> = std::result::Result<T, AssayError>`
- [x] **FND-04**: `GateKind` enum with `#[serde(tag = "kind")]` — `Command { cmd }` and `AlwaysPass` variants
- [x] **FND-05**: `GateResult` struct with `passed`, `stdout`, `stderr`, `exit_code`, `duration_ms`, `timestamp` fields
- [x] **FND-06**: `Criterion` struct with `name`, `description`, optional `cmd` field (forward-compatible with `prompt`)
- [x] **FND-07**: Schema generation binary (`assay-types/examples/generate-schemas.rs`) + `just schemas` recipe
- [x] **FND-08**: New `assay-mcp` crate added to workspace (library crate for MCP server)

## Config & Initialization

- [x] **CFG-01**: `assay init` creates `.assay/` directory with `config.toml` and `specs/` subdirectory
- [x] **CFG-02**: Template-based `config.toml` generation with project name (inferred from directory) and sensible defaults
- [x] **CFG-03**: Example spec file created in `.assay/specs/` during init
- [x] **CFG-04**: Idempotent init — refuse to overwrite existing `.assay/` directory
- [x] **CFG-05**: Config loading via `assay_core::config::load()` and `from_str()` free functions
- [x] **CFG-06**: Config validation via `assay_core::config::validate()` with structured error reporting

## Spec Files

- [x] **SPEC-01**: TOML spec file parsing via `assay_core::spec::load()` and `from_str()` free functions
- [x] **SPEC-02**: Spec struct with `name`, `description`, `criteria: Vec<Criterion>`
- [x] **SPEC-03**: Criteria with optional `cmd` field — present = executable, absent = descriptive
- [x] **SPEC-04**: Spec validation — name required, non-empty after trim, unique criteria names
- [x] **SPEC-05**: Spec directory scanning — find all `.toml` files in `.assay/specs/`
- [x] **SPEC-06**: `assay spec show <name>` CLI command displaying parsed spec

## Gate Evaluation

- [ ] **GATE-01**: Command gate execution via `std::process::Command` with exit code evaluation
- [ ] **GATE-02**: Structured `GateResult` with stdout/stderr evidence capture
- [ ] **GATE-03**: Timeout enforcement on gate commands with configurable default (300s)
- [ ] **GATE-04**: Explicit `working_dir` parameter on `gate::evaluate()` — never inherit
- [ ] **GATE-05**: `assay gate run <spec>` CLI command running all executable criteria
- [ ] **GATE-06**: `GateKind::FileExists { path }` variant for file existence checks
- [ ] **GATE-07**: Aggregate gate results — summary showing "N/M criteria passed" per spec
- [ ] **GATE-08**: Gate evaluation is sync with documented async guidance (`spawn_blocking`)

## MCP Server

- [ ] **MCP-01**: MCP server with stdio transport via rmcp in `assay-mcp` crate
- [ ] **MCP-02**: `spec_get` tool — retrieve parsed spec by name, return as structured JSON
- [ ] **MCP-03**: `gate_run` tool — execute all command criteria for a spec, return `Vec<GateResult>`
- [ ] **MCP-04**: `spec_list` tool — enumerate available specs in the project
- [ ] **MCP-05**: `assay mcp serve` CLI subcommand starting the MCP server
- [ ] **MCP-06**: tracing-subscriber initialized to stderr (stdout reserved for JSON-RPC protocol)
- [ ] **MCP-07**: `spawn_blocking` bridge for sync gate evaluation in async tool handlers
- [ ] **MCP-08**: Tool descriptions clear enough for agent discovery without additional prompting

## Claude Code Plugin

- [ ] **PLG-01**: `plugin.json` manifest with name, version, description, author
- [ ] **PLG-02**: `.mcp.json` pointing to `assay mcp serve` (stdio transport)
- [ ] **PLG-03**: `/gate-check` skill — run gates for current spec and report results
- [ ] **PLG-04**: `/spec-show` skill — display current spec via MCP
- [ ] **PLG-05**: CLAUDE.md workflow snippet for project-level agent instructions
- [ ] **PLG-06**: PostToolUse hook — auto-trigger gate evaluation after Write/Edit tool use
- [ ] **PLG-07**: Stop hook — prevent agent completion without passing gates

---

## Future Requirements (v0.2+)

- Agent-evaluated criteria (`prompt` field on Criterion, subprocess invocation)
- Markdown spec bodies (TOML frontmatter + Markdown content)
- Spec lifecycle/status tracking (Draft/Active/Implementing/Review/Done)
- Spec dependencies and versioning
- Composite gates (AND/OR logic)
- Threshold gates (e.g., coverage >= 80%)
- Parallel gate execution
- Workflow state machine
- Structured review system
- Global config (`~/.config/assay/`)
- Additional MCP tools beyond the initial 3
- Codex/OpenCode plugins
- TUI dashboard features

## Out of Scope

- Interactive init wizard — agent-first tool, non-interactive by default
- Markdown-only spec format — Assay is structured TOML-first
- Gate caching/memoization — gates must re-evaluate fresh
- SSE/HTTP MCP transport — v0.1 is local-only
- Plugin marketplace publishing — need working product first
- Custom spec DSL — agents + shell commands replace the need
- Agent marketplace or capability routing — YAGNI

## Traceability

| REQ-ID | Phase | Status |
|--------|-------|--------|
| FND-01 | 1 — Workspace Prerequisites | Complete |
| FND-02 | 3 — Error Types and Domain Model | Complete |
| FND-03 | 3 — Error Types and Domain Model | Complete |
| FND-04 | 3 — Error Types and Domain Model | Complete |
| FND-05 | 3 — Error Types and Domain Model | Complete |
| FND-06 | 3 — Error Types and Domain Model | Complete |
| FND-07 | 4 — Schema Generation | Complete |
| FND-08 | 1 — Workspace Prerequisites | Complete |
| CFG-01 | 5 — Config and Initialization | Complete |
| CFG-02 | 5 — Config and Initialization | Complete |
| CFG-03 | 5 — Config and Initialization | Complete |
| CFG-04 | 5 — Config and Initialization | Complete |
| CFG-05 | 5 — Config and Initialization | Complete |
| CFG-06 | 5 — Config and Initialization | Complete |
| SPEC-01 | 6 — Spec Files | Complete |
| SPEC-02 | 6 — Spec Files | Complete |
| SPEC-03 | 6 — Spec Files | Complete |
| SPEC-04 | 6 — Spec Files | Complete |
| SPEC-05 | 6 — Spec Files | Complete |
| SPEC-06 | 6 — Spec Files | Complete |
| GATE-01 | 7 — Gate Evaluation | Not Started |
| GATE-02 | 7 — Gate Evaluation | Not Started |
| GATE-03 | 7 — Gate Evaluation | Not Started |
| GATE-04 | 7 — Gate Evaluation | Not Started |
| GATE-05 | 7 — Gate Evaluation | Not Started |
| GATE-06 | 7 — Gate Evaluation | Not Started |
| GATE-07 | 7 — Gate Evaluation | Not Started |
| GATE-08 | 7 — Gate Evaluation | Not Started |
| MCP-01 | 2 — MCP Spike | Not Started |
| MCP-02 | 8 — MCP Server Tools | Not Started |
| MCP-03 | 8 — MCP Server Tools | Not Started |
| MCP-04 | 8 — MCP Server Tools | Not Started |
| MCP-05 | 8 — MCP Server Tools | Not Started |
| MCP-06 | 2 — MCP Spike | Not Started |
| MCP-07 | 8 — MCP Server Tools | Not Started |
| MCP-08 | 8 — MCP Server Tools | Not Started |
| PLG-01 | 9 — CLI Surface Completion | Not Started |
| PLG-02 | 10 — Claude Code Plugin | Not Started |
| PLG-03 | 10 — Claude Code Plugin | Not Started |
| PLG-04 | 10 — Claude Code Plugin | Not Started |
| PLG-05 | 10 — Claude Code Plugin | Not Started |
| PLG-06 | 10 — Claude Code Plugin | Not Started |
| PLG-07 | 10 — Claude Code Plugin | Not Started |
