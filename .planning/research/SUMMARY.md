# Project Research Summary

**Project:** Assay v0.7.0 Gate Composability
**Domain:** Developer tooling — quality gate composition and reuse
**Researched:** 2026-04-11
**Confidence:** HIGH

## Executive Summary

Assay v0.7.0 adds reuse and composition primitives to an existing, production-quality gate system. The pattern is well-understood from direct analogues (GitLab CI `extends`, GitHub Actions composite actions, Terraform preconditions) and the implementation is entirely additive — no existing types, tools, or commands change shape. All capabilities can be built with zero new workspace dependencies.

The recommended approach is dependency-first: types layer, then resolution logic, then I/O, then wizard core, then surfaces in parallel. The entire type layer change is backward-compatible because every new field on `GatesSpec` uses `#[serde(default, skip_serializing_if)]`, matching existing precedent. The main risks are correctness risks, not technology risks: criteria merge semantics, precondition state ambiguity, path traversal via `extends` references, and cross-surface validation drift.

Spec preconditions are a genuine differentiator — no quality gate tool in the CI space implements "skip my gate run unless a prior gate passed." This is stronger than ordering-only `depends` and supports the multi-spec orchestration model Assay is building toward.

## Key Findings

### Recommended Stack

Zero new workspace dependencies required. All composability features build on existing deps.

**Core technologies (existing, reused):**
- **indexmap 2:** Name-keyed deterministic criterion merge for `gate.extends` resolution
- **dialoguer 0.12.0:** `Select` + `MultiSelect` prompts for CLI wizard (already imported)
- **ratatui 0.30:** TUI wizard follows existing `WizardState`/`WizardAction` pattern
- **HashSet (stdlib):** DFS cycle detection for `extends` chains, following `milestone/cycle.rs` pattern

### Expected Features

**Must have (table stakes):**
- `gate.extends` criteria inheritance (load-time static resolution)
- Criteria libraries with `include` field (`.assay/criteria/<slug>.toml`)
- Validation of inheritance chains in `spec_validate`
- Wizard composability steps on all three surfaces (CLI, MCP, TUI)
- `criteria_list`/`criteria_get`/`spec_resolve` MCP tools

**Should have (competitive):**
- Spec preconditions (`[preconditions]` with `requires` + `commands`)
- Override semantics with per-criterion source annotation in gate run output
- `criteria_create` MCP tool

**Defer (v2+):**
- Multi-level inheritance > 2 levels
- Runtime/dynamic composition
- Parameterized/template criteria
- Cross-repo library references
- GUI library browser

### Architecture Approach

Pure load-time resolution layer. `spec::compose::resolve()` takes raw `GatesSpec` + two closures (for loading libraries and parent specs, consistent with zero-trait convention) and returns `ResolvedGatesSpec` with flattened `effective_criteria`. The existing `gate::evaluate_all()` already takes `&[Criterion]` — feeding it `effective_criteria` is the only evaluation callsite change. `GateKind` never gains a new variant; composability is invisible to the evaluation layer.

**Major components:**
1. **`assay-types` extensions** — `CriteriaLibrary`, `SpecPreconditions`, `PreconditionStatus` types; 3 additive fields on `GatesSpec`
2. **`assay-core::spec::compose`** — `resolve()` function with cycle detection and name-keyed merge
3. **`assay-core::criteria_library`** — Load/save/scan for `.assay/criteria/` directory
4. **`assay-core::wizard` extensions** — `apply_gate_wizard()` shared across all surfaces
5. **Surface implementations** — CLI subcommand, MCP tools, TUI state machine (parallelizable)

### Critical Pitfalls

1. **`deny_unknown_fields` backward compat (P-66)** — Every new `GatesSpec` field must have `#[serde(default, skip_serializing_if)]`; add backward-compat roundtrip test with pre-v0.7.0 fixture
2. **`GateKind` contamination (P-77)** — Composability must never add a `GateKind` variant; enforce with test asserting no parent ref in evaluated criteria
3. **Missing cycle detection (P-68)** — Existing `detect_cycles()` covers `depends` only; `extends` needs its own DFS; cap at depth limit
4. **Cross-surface validation drift (P-73)** — All validation in `assay-core::wizard`, never in surfaces; enforce with surface-parity test
5. **Path traversal (P-81)** — Slug-format validation on `extends`/`include` values + path containment assertion

## Implications for Roadmap

### Phase 1: Type Foundation
**Rationale:** Types unblock everything; must come first
**Delivers:** `CriteriaLibrary`, `SpecPreconditions`, `PreconditionStatus` types; 3 new fields on `GatesSpec`; schema snapshots; backward-compat tests
**Addresses:** Type foundation for all composability features
**Avoids:** P-66 (backward compat) by establishing `serde(default)` pattern immediately

### Phase 2: Criteria Library I/O + Resolution Core
**Rationale:** Resolution logic must be complete and tested before any surface touches it
**Delivers:** `criteria_library` module (load/save/scan); `spec::compose::resolve()` with cycle detection and name-keyed merge
**Addresses:** `include` field resolution, `gate.extends` resolution
**Avoids:** P-68 (missing cycle detection), P-77 (wrong abstraction layer)

### Phase 3: Gate Evaluation Integration + Validation
**Rationale:** Wire resolution into evaluation pipeline; extend spec_validate
**Delivers:** `evaluate_preconditions()` in gate run dispatch; `PreconditionFailed` result; composability diagnostics in `spec_validate`
**Addresses:** Spec preconditions, validation of inheritance chains
**Avoids:** P-81 (path traversal) via slug validation

### Phase 4: Wizard Core + CLI Surface
**Rationale:** Establish shared wizard logic before parallel surface work
**Delivers:** `apply_gate_wizard()` in core; `assay gate wizard` and `assay criteria list/new` CLI commands; surface-parity test
**Addresses:** CLI interactive wizard, criteria management commands
**Avoids:** P-73 (validation drift) by establishing core-first pattern

### Phase 5: MCP Surface
**Rationale:** Agent-facing tools, independent of CLI after wizard core exists
**Delivers:** `gate_wizard`, `criteria_list`, `criteria_get`, `criteria_create`, `spec_resolve` MCP tools
**Addresses:** Agent-driven gate composition
**Avoids:** P-74 (overloaded tools) via discrete tool design

### Phase 6: TUI Surface
**Rationale:** Human dashboard, independent of other surfaces after wizard core exists
**Delivers:** `GateWizardState`/`GateWizardAction`/`handle_gate_wizard_event()`/`draw_gate_wizard()`
**Addresses:** TUI wizard for human supervision
**Avoids:** P-73 by delegating all validation to core

### Phase Ordering Rationale

- Types → resolution → evaluation mirrors the data flow: define types, compose them, evaluate them
- Wizard core before surfaces prevents validation drift
- Phases 5 and 6 are independent after Phase 4 completes (parallelizable)

### Research Flags

All 6 phases use standard, well-documented patterns from the existing codebase. No phase requires `/kata:research-phase`.

Risk areas (criteria merge semantics, precondition state model) are design decisions to be recorded in type doc comments, not research gaps.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Direct codebase inspection, all dep versions confirmed in root Cargo.toml |
| Features | HIGH | Table stakes confirmed via GitLab CI, GitHub Actions, Terraform, pytest-dependency docs |
| Architecture | HIGH | All findings from direct codebase analysis with specific file paths and function names |
| Pitfalls | HIGH | 13 pitfalls (P-66 through P-81), each grounded in actual codebase patterns |

**Overall confidence:** HIGH

### Gaps to Address

- **Criteria merge conflict resolution:** "Own wins silently" is the recommended rule; must be documented in type comments and surfaced as per-criterion source annotation
- **Precondition temporal semantics:** "Last recorded gate run passed" is the simplest definition; staleness handling deferred
- **TOML edit round-trip:** Wizard writes new files; comment preservation deferred (consider `toml_edit` only if it becomes a hard requirement)

---
*Research completed: 2026-04-11*
*Ready for roadmap: yes*
