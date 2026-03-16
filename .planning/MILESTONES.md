# Project Milestones: Assay

## v0.4.0 Headless Orchestration (Shipped: 2026-03-15)

**Delivered:** Headless agent evaluation with `gate_evaluate` capstone MCP tool, `WorkSession` persistence, `spec_validate` static health checking, `cupel` context engine integration for token-budgeted diff slicing, observability improvements, and 120+ tech debt fixes.

**Phases completed:** 35-45 (30 plans total)

**Key accomplishments:**

- `gate_evaluate` MCP tool: single-call headless agent evaluation with diff computation, subprocess orchestration, structured per-criterion results, and automatic persistence
- `WorkSession` type with JSON persistence, phase transitions, session MCP tools (create/get/update/list), and startup recovery for stale sessions
- `spec_validate` MCP tool with structured diagnostics: TOML parse errors, criterion uniqueness, prompt validation, cross-spec dependency cycle detection
- Context engine integration via external `cupel` crate: token-budgeted context windowing with passthrough optimization and DiffTruncation metadata
- Observability: warnings field on all MCP responses, outcome-filtered gate_history, spec_get resolved config, growth rate metrics
- 120+ tech debt issues resolved in batch sweep across all crates

**Stats:**

- 356 files changed, 31,825 insertions, 582 deletions
- 33,462 lines of Rust across 5 crates
- 11 phases, 30 plans
- 836 tests
- 5 days from start to ship (2026-03-11 → 2026-03-15)

**Git range:** `docs: start milestone v0.4.0` → `Merge pull request #131`

**What's next:** v0.4.1 — Merge tools (merge_check, merge_propose, worktree fixes, gate evidence formatting)

---

## v0.3.0 Orchestration Foundation (Shipped: 2026-03-10)

**Delivered:** Worktree isolation foundation, gate output truncation, actionable error messages, types hygiene, CLI polish, MCP validation hardening, and core tech debt cleanup.

**Phases completed:** 26-34 (22 plans total)

**Key accomplishments:**

- Git worktree lifecycle management: create, list, status, cleanup across CLI and MCP surfaces
- Gate output head+tail truncation with independent 32 KiB per-stream byte budgets, UTF-8 safety, and MCP truncation metadata visibility
- Actionable error messages: Levenshtein fuzzy matching for spec-not-found, exit code classification (127/126), TOML parse error formatting with source-line carets
- Types hygiene: Eq derives on all safe types, Display impls for all public enums, deny(missing_docs), Criterion/GateCriterion dedup
- CLI polish: NO_COLOR/TTY handling, help text dedup, enforcement dedup, StreamCounters API, constants extraction
- Core tech debt: shared evaluate_criteria/validate_criteria helpers, history save_run() API, guard daemon persistence hardening

**Stats:**

- 180 files changed, 20,883 insertions, 5,071 deletions
- 27,067 lines of Rust across 5 crates
- 9 phases, 22 plans, 143 commits
- 603 tests
- 2 days from start to ship (2026-03-08 → 2026-03-10)

**Git range:** `docs: start milestone v0.3.0` → `Merge pull request #103`

**What's next:** v0.4.0 — Headless sequential orchestration (Claude Code launcher, session record, gate_evaluate)

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
