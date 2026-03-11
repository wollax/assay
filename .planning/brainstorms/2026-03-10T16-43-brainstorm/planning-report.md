# Planning Primitives & Spec Authoring — Consolidated Report

**Explorer:** explorer-planning | **Challenger:** challenger-planning
**Date:** 2026-03-10
**Rounds of debate:** 2 (converged)

---

## Executive Summary

7 proposals were explored for closing the "discuss/plan" gap at the start of Assay's orchestration loop. After 2 rounds of challenge, the brainstorm converged on **2 proposals for v0.4.0** (both fitting existing Phase 37), **2 deferred to v0.5.0**, and **3 killed permanently**. A documented convention replaces one killed proposal with zero code.

**Key insight:** Agents with LLM capabilities handle spec decomposition, markdown parsing, and file manipulation natively. Assay should provide *validation and structure* (spec_create, dependency validation), not duplicate what LLMs already do well.

---

## v0.4.0 Recommendations (Phase 37: Spec Validation)

### 1. `spec_create` MCP Tool — SPEC-05

**What:** An MCP tool that creates a directory-based spec from structured parameters. Generates `gates.toml` only (no `spec.toml`). Validates the spec before writing to disk.

**Schema:**
```json
{
  "name": "auth-flow",                          // required
  "description": "User authentication flow",    // optional, defaults to ""
  "enforcement": "required",                     // optional, defaults to "required"
  "criteria": [                                  // required, at least one
    {
      "name": "compiles",
      "description": "Code compiles without errors",
      "cmd": "cargo build"
    },
    {
      "name": "security-review",
      "description": "Agent reviews for security issues",
      "kind": "AgentReport",
      "prompt": "Check for SQL injection"
    }
  ]
}
```

**Minimal call:** `{"name": "foo", "criteria": [{"name": "check", "description": "...", "cmd": "echo ok"}]}`

**Why it fits v0.4.0:** Write-side complement to `spec_validate`'s read-side checks. Enables tight MCP-only authoring loop: agent discusses with user → `spec_create` → `gate_run` → no file system dance. Directly serves headless orchestration theme.

**Implementation notes:**
- Generates `gates.toml` only — agents write `spec.toml` by hand if they need full IEEE 830 structure
- Runs `validate()` on the generated spec before writing to disk
- Returns error if spec directory already exists (same as CLI `spec new`)
- Reuses existing `handle_spec_new` template logic from `crates/assay-cli/src/commands/spec.rs:279-342`
- Optional fields exist in schema to teach agents what's possible (description, enforcement)

**Scope:** 1-2 days
**Risk:** Low — straightforward MCP params → file writes + validation

### 2. Requirement-level `depends_on` — Partial SPEC-04

**What:** Add `depends_on: Vec<String>` to the `Requirement` type in `feature_spec.rs`. Enables intra-spec requirement ordering with cycle detection during validation.

**Type change:**
```rust
// In crates/assay-types/src/feature_spec.rs, Requirement struct
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub depends_on: Vec<String>,
```

**Why it fits v0.4.0:** Phase 37 SPEC-04 already calls for "Cross-spec dependency validation with cycle detection." This implements the intra-spec portion. Spec-level cross-spec dependencies are deferred (see below).

**Implementation notes:**
- Intra-spec only: `depends_on` references requirement IDs within the same spec
- Validation: referential integrity (all referenced IDs must exist in the spec) + cycle detection (topological sort)
- Integrates naturally with `spec_validate` (SPEC-01/SPEC-02)
- `deny_unknown_fields` on `Requirement` means this is an additive schema change — existing specs without `depends_on` continue to parse

**Scoping decision for Phase 37 planners:** SPEC-04 is *partially* satisfied by this. Cross-spec dependency validation (spec A depends on spec B) is deferred because it breaks `load_spec_entry`'s isolation — currently each spec loads independently without knowledge of other specs. Cross-spec deps would require `scan()` to load all specs for any single validation, which is architecturally wrong at this stage.

**Scope:** 1 day
**Risk:** Low — additive type change + single-pass validation

---

## v0.5.0 Candidates

### 3. Criteria Libraries with `include` Field

**What:** A `.assay/criteria/` directory containing reusable criterion definitions. Specs reference these via an `include` field on `GatesSpec`, inheriting criteria without duplication.

**Why deferred:**
1. **Phase 37 scope:** Adding this makes Phase 37 the largest phase in v0.4.0 (6 requirements). Scope smell.
2. **Doesn't serve orchestration theme:** Criteria libraries serve DRY (developer experience), not headless orchestration.
3. **Design coupling with `extends`:** The v0.4.0 brainstorm identified `extends:` spec inheritance as a seed. `include` (criteria-only merge) and `extends` (full parent-child inheritance) have different merge semantics. They should be designed together to avoid migration pain.
4. **Technical surface area:** Touches GatesSpec type, spec loading pipeline, validation, MCP server, and CLI display — four crates for one feature.

**v0.4.0 workaround — Baseline Spec Convention (zero code, first-class recommendation):** Criteria reuse is solved by convention in v0.4.0: put shared criteria in `.assay/specs/baseline/gates.toml`. Agent reads baseline via `spec_get("baseline")`, then passes those criteria to `spec_create` for new specs. This is formalized with `include` in v0.5.0 once the `extends` design is settled. Same outcome for the orchestration loop, no new infrastructure.

**When to build:** v0.5.0, alongside `extends` design. Scope: 3-5 days. Constraints: single-level includes only, directory specs only, error on name conflict (not silent override).

### 4. `spec_diff` — Git-based Spec Comparison

**What:** An MCP tool that compares current spec to a previous git revision, producing structural diff (added/removed/modified requirements and criteria).

**Why deferred:** Requires either spec snapshots in history (storage bloat, schema changes) or git integration (shelling out, handling renames/format migration). Neither is justified until review workflows demand it.

**When to build:** v0.5.0, git-based approach. Start with `git show <ref>:path` comparison. Scope: 3-5 days.

---

## Killed Permanently

### 5. `spec_update` — Iterative Spec Refinement via MCP

**Why killed:** The TOML round-trip problem is crate-wide (`toml` crate strips comments/reorders; `toml_edit` requires API migration). The operation DSL (add/remove/update for requirements and criteria) is a mini state machine with ordering semantics, conflict resolution, and rollback on partial failure. Agents already excel at read-modify-write cycles on small TOML files — this duplicates their core competency at high implementation cost.

**If narrow need arises:** Build `spec_set_status(name, status)` — 20 lines, no operation enum, no round-trip issues.

### 6. `spec_decompose` — Plan Decomposition Primitive

**Why killed:** The LLM IS the decomposition engine. An agent with `spec_get` output (requirements with priorities, obligations, traceability) and full project context will always produce better execution plans than a static grouping algorithm. The only unique value would be cross-spec dependency closure, which requires infrastructure we shouldn't build yet.

### 7. `spec_from_issue` — Issue-to-Spec Heuristic Scaffold

**Why killed:** Heuristic regex-based markdown parsing in Rust will always be inferior to an LLM reading the same markdown. The agent can: read issue (GitHub MCP / Linear CLI) → read example spec (`spec_get`) → generate draft (LLM core competency) → write spec (file tools or `spec_create`). Document this pattern; don't build a worse version.

---

## Architectural Decisions

1. **`spec_create` generates `gates.toml` only.** Agents that need full IEEE 830 `spec.toml` write it by hand after reading an example via `spec_get`. This keeps the MCP input schema minimal.

2. **Requirement dependencies are intra-spec only in v0.4.0.** Cross-spec dependencies break `load_spec_entry` isolation and are deferred until there's a real use case.

3. **`include` and `extends` are architecturally distinct.** `include` = criteria-only library merge across unrelated specs. `extends` = parent-child spec inheritance (criteria + metadata + enforcement). Design both together in v0.5.0.

4. **"Baseline spec" convention is the v0.4.0 criteria reuse strategy.** Criteria reuse is solved by convention now (`spec_get` baseline → `spec_create` with copied criteria), then formalized with `include` in v0.5.0 once `extends` design is settled. Zero infrastructure, same outcome for the orchestration loop.

5. **LLMs handle decomposition, markdown parsing, and plan generation.** Assay provides validation and structure, not intelligence. Don't build deterministic alternatives to things LLMs do natively.

---

## Impact on v0.4.0 Roadmap

| Phase | Current Requirements | Proposed Addition | Net Change |
|-------|---------------------|-------------------|------------|
| Phase 37 (Spec Validation) | SPEC-01, SPEC-02, SPEC-03, SPEC-04 | SPEC-05 (`spec_create`) | +1 requirement |
| Phase 37 (Spec Validation) | SPEC-04 (cross-spec deps) | Partial: intra-spec only | Scoping clarification |

**Total v0.4.0 impact:** +1 net-new requirement (SPEC-05). SPEC-04 scoped down to intra-spec. No new phases added.

---

*Consolidated from 2 rounds of explorer/challenger debate — 2026-03-10*
