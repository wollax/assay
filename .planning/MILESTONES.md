# Project Milestones: Assay

## v0.1.0 Proof of Concept (Shipped: 2026-03-02)

**Delivered:** Thin vertical slice proving spec-driven gates with MCP server and Claude Code plugin integration.

**Phases completed:** 1-10 (18 plans total)

**Key accomplishments:**

- MCP server with three tools (spec_list, spec_get, gate_run) enabling agents to discover and evaluate specs
- Gate evaluation engine with command execution, timeout enforcement, streaming display, and structured evidence
- Project initialization (`assay init`) scaffolding .assay/ structure with template config and example spec
- CLI surface: init, spec show/list, gate run with --all flag, mcp serve
- Claude Code plugin with MCP config, skills, PostToolUse reminders, and Stop hook gate enforcement
- Serializable domain types (GateKind, GateResult, Criterion) with serde/schemars and comprehensive roundtrip tests

**Stats:**

- 227 files, 5,028 lines of Rust, 108 lines of shell
- 10 phases, 18 plans, 128 commits
- 119 tests (70 core, 16 MCP, 9 types, 24 schema)
- 3 days from start to ship (2026-02-28 → 2026-03-02)

**Git range:** initial commit → `fix(10): address second review round`

**What's next:** v0.2.0 — TBD

---
