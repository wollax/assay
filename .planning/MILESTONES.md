# Project Milestones: Assay

## v0.3.0 Orchestration Foundation (In Progress)

**Goal:** Build worktree isolation foundation and close tech debt from v0.2.0 — types hygiene, CLI polish, MCP validation, error messages, and gate output truncation.

**Phases planned:** 26-33 (8 phases)

**See:** `.planning/ROADMAP.md` for detailed phase breakdown, `.planning/REQUIREMENTS.md` for full requirement IDs.

---

## v0.2.0 Dual-Track Gates & Hardening (Shipped: 2026-03-08)

**Delivered:** Full dual-track quality gate platform with agent-evaluated criteria, run history persistence, enforcement levels, session diagnostics, team context protection, and comprehensive hardening.

**Phases completed:** 11-25 (38 plans total)

**Key accomplishments:**

- Dual-track gate system combining deterministic shell commands with agent-evaluated criteria via MCP gate_report/gate_finalize
- Required/advisory enforcement levels with separate tracking across CLI, MCP, and history
- Complete run history subsystem with atomic persistence, retention pruning, and query tools (CLI + MCP)
- Token-aware session diagnostics with JSONL parsing, bloat categorization, and context % visualization
- Composable pruning engine with 6 strategies, dry-run default, and team message protection
- Guard daemon with threshold-based pruning, circuit breaker, kqueue/inotify reactive recovery

**Stats:**

- 58 Rust files, 23,385 lines of Rust
- 15 phases, 38 plans
- 493 tests
- 6 days from start to ship (2026-03-03 → 2026-03-08)

**Git range:** `feat(11-01)` → `fix(25): address PR review findings`

---

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

---
