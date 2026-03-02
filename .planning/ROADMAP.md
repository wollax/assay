# Roadmap — v0.1.0 Proof of Concept

## Overview

| Milestone | Status | Phases | Requirements |
|-----------|--------|--------|--------------|
| v0.1.0 Proof of Concept | In Progress | 10 | 43 |

### Milestones

- v0.1.0 Proof of Concept 🔄

---

## v0.1.0 — Proof of Concept

**Goal:** Prove Assay's dual-track gate differentiator through a thin vertical slice — foundation types, spec-driven gates, MCP server, and Claude Code plugin.

**Phases:** 10
**Requirements:** 43 (FND: 8, CFG: 6, SPEC: 6, GATE: 8, MCP: 8, PLG: 7)

---

#### Phase 1: Workspace Prerequisites

**Goal:** Eliminate the mandatory schemars blocker and add all new workspace dependencies so every downstream phase can build cleanly.

**Dependencies:** None (first phase)

**Requirements:**
- **FND-01**: Upgrade schemars from 0.8 to 1.x across the workspace (required by rmcp)
- **FND-08**: New `assay-mcp` crate added to workspace (library crate for MCP server)

**Success Criteria:**
1. `just ready` passes with schemars 1.x and the new `assay-mcp` crate in the workspace
2. All existing derives (`Serialize`, `Deserialize`, `JsonSchema`) compile without modification beyond the version bump
3. `cargo check -p assay-mcp` succeeds (empty lib crate with rmcp dependency)

---

#### Phase 2: MCP Spike

**Goal:** Validate that rmcp 0.17 + stdio transport + Claude Code's MCP client exchange protocol successfully. This is a GO/NO-GO gate for the entire v0.1 architecture.

**Dependencies:** Phase 1

**Requirements:**
- **MCP-01**: MCP server with stdio transport via rmcp in `assay-mcp` crate
- **MCP-06**: tracing-subscriber initialized to stderr (stdout reserved for JSON-RPC protocol)

**Success Criteria:**
1. A hardcoded single-tool MCP server starts via `assay mcp serve` and responds to JSON-RPC initialize/tool calls on stdin/stdout
2. No non-JSON-RPC bytes appear on stdout during server operation (tracing goes to stderr)
3. Claude Code can discover and call the spike tool when the plugin is installed locally
4. Spike result documented as GO (proceed) or NO-GO (pivot architecture)

**Plans:**
| Plan | Wave | Objective | Tasks | Autonomous |
|------|------|-----------|-------|------------|
| 02-01 | 1 | Implement spike server, wire CLI, validate protocol roundtrip | 2 | Task 1: yes, Task 2: human-verify |

---

#### Phase 3: Error Types and Domain Model

**Goal:** Establish the shared type system and error handling that every crate depends on. Types are pure data (DTOs); no business logic in assay-types.

**Dependencies:** Phase 1

**Requirements:**
- **FND-02**: Unified `AssayError` enum with thiserror and `#[non_exhaustive]`, starting with `Io` variant
- **FND-03**: Result type alias `pub type Result<T> = std::result::Result<T, AssayError>`
- **FND-04**: `GateKind` enum with `#[serde(tag = "kind")]` — `Command { cmd }` and `AlwaysPass` variants
- **FND-05**: `GateResult` struct with `passed`, `stdout`, `stderr`, `exit_code`, `duration_ms`, `timestamp` fields
- **FND-06**: `Criterion` struct with `name`, `description`, optional `cmd` field (forward-compatible with `prompt`)

**Success Criteria:**
1. `GateKind` serializes to TOML with internal tagging (`kind = "Command"`) and roundtrips correctly
2. `GateResult` captures all evidence fields and serializes to JSON for MCP consumption
3. `Criterion` with `cmd = None` is valid (descriptive-only criterion) and `cmd = Some(...)` is valid (executable criterion)
4. `AssayError` variants include contextual information (file paths, operation descriptions), not bare passthrough wrappers
5. `just ready` passes with all new types derived for `Serialize`, `Deserialize`, `JsonSchema`

**Plans:** 2 plans
Plans:
- [ ] 03-01-PLAN.md — Domain types (GateKind, GateResult, Criterion) in assay-types with serde roundtrip tests
- [ ] 03-02-PLAN.md — AssayError enum and Result alias in assay-core with context-rich error handling

---

#### Phase 4: Schema Generation

**Goal:** Produce JSON Schema files from domain types so external tools and agents can validate Assay config and spec formats.

**Dependencies:** Phase 3

**Requirements:**
- **FND-07**: Schema generation binary (`assay-types/examples/generate-schemas.rs`) + `just schemas` recipe

**Success Criteria:**
1. `just schemas` produces JSON Schema files in `schemas/` for all public domain types
2. Generated schemas reflect schemars 1.x output and validate against sample TOML-converted-to-JSON input
3. Schema files are deterministic (re-running produces identical output)

**Plans:** 1 plan
Plans:
- [x] 04-01-PLAN.md — Schema registry, generator binary, just recipes, roundtrip + snapshot tests

---

#### Phase 5: Config and Initialization

**Goal:** Users can initialize an Assay project and the system can load/validate its configuration.

**Dependencies:** Phase 3

**Requirements:**
- **CFG-01**: `assay init` creates `.assay/` directory with `config.toml` and `specs/` subdirectory
- **CFG-02**: Template-based `config.toml` generation with project name (inferred from directory) and sensible defaults
- **CFG-03**: Example spec file created in `.assay/specs/` during init
- **CFG-04**: Idempotent init — refuse to overwrite existing `.assay/` directory
- **CFG-05**: Config loading via `assay_core::config::load()` and `from_str()` free functions
- **CFG-06**: Config validation via `assay_core::config::validate()` with structured error reporting

**Success Criteria:**
1. Running `assay init` in a fresh directory creates `.assay/config.toml` and `.assay/specs/` with an example spec
2. Running `assay init` a second time in the same directory fails with a clear error (does not overwrite)
3. `config::load()` parses a valid TOML config and returns structured data; invalid TOML produces error messages that include the file path and field name
4. `config::validate()` rejects configs with missing or empty required fields

**Plans:** 3 plans
Plans:
- [x] 05-01-PLAN.md — Redesign Config type, add GatesConfig, new error variants, update snapshots/schemas
- [x] 05-02-PLAN.md — Config loading and validation (from_str, load, validate) via TDD
- [x] 05-03-PLAN.md — Init logic in assay-core and CLI init subcommand

---

#### Phase 6: Spec Files

**Goal:** Users can write TOML spec files with criteria, and the system can parse, validate, and enumerate them.

**Dependencies:** Phase 3, Phase 5 (specs live in `.assay/specs/` created by init)

**Requirements:**
- **SPEC-01**: TOML spec file parsing via `assay_core::spec::load()` and `from_str()` free functions
- **SPEC-02**: Spec struct with `name`, `description`, `criteria: Vec<Criterion>`
- **SPEC-03**: Criteria with optional `cmd` field — present = executable, absent = descriptive
- **SPEC-04**: Spec validation — name required, non-empty after trim, unique criteria names
- **SPEC-05**: Spec directory scanning — find all `.toml` files in `.assay/specs/`
- **SPEC-06**: `assay spec show <name>` CLI command displaying parsed spec

**Success Criteria:**
1. A TOML spec file with multiple criteria (some with `cmd`, some without) parses successfully
2. Validation rejects specs with empty names, whitespace-only names, or duplicate criteria names — with clear error messages
3. `spec::scan()` finds all `.toml` files in `.assay/specs/` and returns their parsed representations
4. `assay spec show <name>` displays a human-readable representation of a named spec

**Plans:** 2 plans
Plans:
- [x] 06-01-PLAN.md — Spec type updates, error variants, and spec module (from_str, validate, load, scan) via TDD
- [x] 06-02-PLAN.md — Wire CLI spec show and spec list subcommands with table/JSON output

---

#### Phase 7: Gate Evaluation

**Goal:** Command gates execute, capture evidence, enforce timeouts, and produce structured results that both CLI and MCP can consume.

**Dependencies:** Phase 3, Phase 6 (gates evaluate criteria from specs)

**Requirements:**
- **GATE-01**: Command gate execution via `std::process::Command` with exit code evaluation
- **GATE-02**: Structured `GateResult` with stdout/stderr evidence capture
- **GATE-03**: Timeout enforcement on gate commands with configurable default (300s)
- **GATE-04**: Explicit `working_dir` parameter on `gate::evaluate()` — never inherit
- **GATE-05**: `assay gate run <spec>` CLI command running all executable criteria
- **GATE-06**: `GateKind::FileExists { path }` variant for file existence checks
- **GATE-07**: Aggregate gate results — summary showing "N/M criteria passed" per spec
- **GATE-08**: Gate evaluation is sync with documented async guidance (`spawn_blocking`)

**Success Criteria:**
1. A command gate runs `echo hello` and `GateResult` contains `passed: true`, stdout: `"hello\n"`, exit_code: `0`, and a non-zero `duration_ms`
2. A command gate with a failing command produces `passed: false` with stderr evidence and the correct non-zero exit code
3. A command exceeding the timeout is killed, and `GateResult` reflects the timeout with appropriate error information
4. `assay gate run <spec>` prints a summary showing "N/M criteria passed" and individual criterion results
5. `gate::evaluate()` requires an explicit `working_dir` — no default, no inheritance from the process

---

#### Phase 8: MCP Server Tools

**Goal:** The MCP server exposes spec and gate operations as tools that AI agents can discover and call programmatically.

**Dependencies:** Phase 2 (spike validates MCP works), Phase 6 (spec tools), Phase 7 (gate tools)

**Requirements:**
- **MCP-02**: `spec_get` tool — retrieve parsed spec by name, return as structured JSON
- **MCP-03**: `gate_run` tool — execute all command criteria for a spec, return `Vec<GateResult>`
- **MCP-04**: `spec_list` tool — enumerate available specs in the project
- **MCP-05**: `assay mcp serve` CLI subcommand starting the MCP server
- **MCP-07**: `spawn_blocking` bridge for sync gate evaluation in async tool handlers
- **MCP-08**: Tool descriptions clear enough for agent discovery without additional prompting

**Success Criteria:**
1. An agent calling `spec_get` with a valid spec name receives the full spec as structured JSON
2. An agent calling `gate_run` receives a `Vec<GateResult>` with evidence for each criterion, and the async handler does not block the tokio runtime
3. An agent calling `spec_list` receives an array of available spec names in the project
4. `assay mcp serve` starts the server and the first byte on stdout is `{` (clean JSON-RPC, no clap output leakage)
5. Tool descriptions are self-documenting: an agent unfamiliar with Assay can understand what each tool does from the description alone

**Plans:** 2 plans
Plans:
- [x] 08-01-PLAN.md — AssayServer with three MCP tools (spec_get, spec_list, gate_run), parameter schemas, response formatting, error handling
- [x] 08-02-PLAN.md — CLI wiring verification, integration tests, and end-to-end MCP server validation

---

#### Phase 9: CLI Surface Completion

**Goal:** All CLI subcommands are wired and functional, delegating to core and MCP libraries.

**Dependencies:** Phase 5 (init), Phase 6 (spec show), Phase 7 (gate run), Phase 8 (mcp serve)

**Requirements:**
- **PLG-01**: `plugin.json` manifest with name, version, description, author

**Success Criteria:**
1. `assay init`, `assay spec show`, `assay gate run`, and `assay mcp serve` all work end-to-end as documented
2. `assay --help` lists all subcommands with clear descriptions
3. `plugin.json` contains accurate metadata matching the workspace version

**Note:** This phase is deliberately small. The CLI subcommands are implemented incrementally in Phases 5-8 as each domain capability lands. Phase 9 ensures the CLI surface is polished, help text is complete, and `plugin.json` metadata is accurate.

---

#### Phase 10: Claude Code Plugin

**Goal:** A working Claude Code plugin that installs Assay as an MCP server and provides skills and hooks for spec-driven development workflows.

**Dependencies:** Phase 8 (MCP server must be functional), Phase 9 (CLI must be complete)

**Requirements:**
- **PLG-02**: `.mcp.json` pointing to `assay mcp serve` (stdio transport)
- **PLG-03**: `/gate-check` skill — run gates for current spec and report results
- **PLG-04**: `/spec-show` skill — display current spec via MCP
- **PLG-05**: CLAUDE.md workflow snippet for project-level agent instructions
- **PLG-06**: PostToolUse hook — auto-trigger gate evaluation after Write/Edit tool use
- **PLG-07**: Stop hook — prevent agent completion without passing gates

**Success Criteria:**
1. Installing the plugin in Claude Code registers the Assay MCP server and all skills appear in the skill list
2. An agent can call `/gate-check` and receive structured pass/fail results for the current spec
3. An agent can call `/spec-show` and see the full spec with criteria
4. After a Write or Edit tool use, the PostToolUse hook triggers gate evaluation automatically
5. An agent cannot mark work as complete while gates are failing (Stop hook enforces gate pass)

---

## Requirement Coverage

| Phase | Requirements | Count |
|-------|-------------|-------|
| 1 | FND-01, FND-08 | 2 |
| 2 | MCP-01, MCP-06 | 2 |
| 3 | FND-02, FND-03, FND-04, FND-05, FND-06 | 5 |
| 4 | FND-07 | 1 |
| 5 | CFG-01, CFG-02, CFG-03, CFG-04, CFG-05, CFG-06 | 6 |
| 6 | SPEC-01, SPEC-02, SPEC-03, SPEC-04, SPEC-05, SPEC-06 | 6 |
| 7 | GATE-01, GATE-02, GATE-03, GATE-04, GATE-05, GATE-06, GATE-07, GATE-08 | 8 |
| 8 | MCP-02, MCP-03, MCP-04, MCP-05, MCP-07, MCP-08 | 6 |
| 9 | PLG-01 | 1 |
| 10 | PLG-02, PLG-03, PLG-04, PLG-05, PLG-06, PLG-07 | 6 |
| **Total** | | **43** |

**Validation:** 43 requirements, each mapped to exactly one phase. Coverage is 100%.

---

## Progress Summary

| Phase | Name | Status | Requirements | Completed |
|-------|------|--------|--------------|-----------|
| 1 | Workspace Prerequisites | Complete | 2 | 2 |
| 2 | MCP Spike | Complete | 2 | 2 |
| 3 | Error Types and Domain Model | Complete | 5 | 5 |
| 4 | Schema Generation | Complete | 1 | 1 |
| 5 | Config and Initialization | Complete | 6 | 6 |
| 6 | Spec Files | Complete | 6 | 6 |
| 7 | Gate Evaluation | Complete | 8 | 8 |
| 8 | MCP Server Tools | Complete | 6 | 6 |
| 9 | CLI Surface Completion | Not Started | 1 | 0 |
| 10 | Claude Code Plugin | Not Started | 6 | 0 |

**Overall:** 42/43 requirements complete (98%)

---

## Dependency Graph

```
Phase 1 ──> Phase 2 (MCP spike needs workspace deps)
Phase 1 ──> Phase 3 (domain types need schemars 1.x)
Phase 3 ──> Phase 4 (schemas generated from types)
Phase 3 ──> Phase 5 (config uses domain types)
Phase 3 + Phase 5 ──> Phase 6 (specs use types, live in .assay/)
Phase 3 + Phase 6 ──> Phase 7 (gates evaluate spec criteria)
Phase 2 + Phase 6 + Phase 7 ──> Phase 8 (MCP tools wrap spec/gate)
Phase 5 + Phase 6 + Phase 7 + Phase 8 ──> Phase 9 (CLI wires everything)
Phase 8 + Phase 9 ──> Phase 10 (plugin needs working MCP + CLI)
```

**Critical path:** 1 -> 3 -> 6 -> 7 -> 8 -> 10

**Parallelizable:**
- Phase 2 (MCP spike) and Phase 3 (domain model) can run in parallel after Phase 1
- Phase 4 (schemas) can run in parallel with Phase 5 (config) after Phase 3
- Phase 5 (config) and Phase 6 (specs) have a soft dependency (specs use .assay/ dir) but core parsing is independent
