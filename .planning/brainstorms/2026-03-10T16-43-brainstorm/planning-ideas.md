# Planning Primitives & Spec Authoring — Explorer Proposals

**Explorer:** explorer-planning
**Date:** 2026-03-10
**Focus:** Closing the "discuss/plan" gap at the start of the orchestration loop

---

## Context

The orchestration loop is: **discuss → research → plan → execute → verify/test → complete/merge**

Current Assay coverage starts at "execute" (worktree isolation) and "verify" (gate_run, gate_report, gate_finalize). The v0.4.0 brainstorm focused on headless evaluation (gate_evaluate, WorkSession, context engine). But there's nothing for the **left side** of the loop — no tooling for spec authoring, plan decomposition, or iterative refinement.

Today, an agent wanting to work through a spec must:
1. Manually create TOML files (or use `assay spec new` for a bare skeleton)
2. Know the full IEEE 830-style schema
3. Have no way to decompose a spec into sub-tasks or plan execution order
4. Have no way to refine criteria iteratively through the MCP interface

---

## Proposal 1: `spec_create` — MCP-driven Spec Scaffolding

**What:** An MCP tool that creates a new directory-based spec from structured parameters. Unlike `assay spec new` (CLI-only, static template), `spec_create` accepts a name, description, requirements list, and criteria list — and generates both `spec.toml` and `gates.toml` in one call.

**Why:** Agents currently can't create specs through MCP. They'd need to use file writes to construct valid TOML, which is error-prone and requires deep schema knowledge. `spec_create` makes spec authoring a first-class agent operation — the agent describes *what* it wants to build, and Assay handles the file structure.

**Interface sketch:**
```json
{
  "name": "auth-flow",
  "description": "User authentication flow",
  "requirements": [
    { "id": "REQ-FUNC-001", "title": "Login with credentials", "statement": "..." }
  ],
  "criteria": [
    { "name": "compiles", "description": "...", "cmd": "cargo build" },
    { "name": "security-review", "description": "...", "kind": "AgentReport", "prompt": "..." }
  ]
}
```

**Scope:** Small-medium (1-2 days). The `spec new` CLI command already has the template logic. This is largely wiring MCP params → file writes + validation.

**Risks:**
- Agents may generate low-quality specs (garbage-in). Mitigation: run `validate()` on the generated spec before writing.
- Parameter surface area is large (FeatureSpec has many optional fields). Mitigation: start with required fields only, add optional sections in v0.4.1.

---

## Proposal 2: `spec_update` — Iterative Spec Refinement via MCP

**What:** An MCP tool for incremental spec modifications: add/remove/update requirements, add/remove criteria, update metadata (status, version). Operates on an existing spec by name, applying a delta rather than rewriting.

**Why:** Specs evolve. In practice, an agent discusses requirements with a user, then iterates — "add a security criterion," "change this requirement to 'should'," "mark the spec as 'planned'." Without `spec_update`, the agent must read → parse → modify → serialize → write the whole TOML file, risking formatting loss and parse errors.

**Interface sketch:**
```json
{
  "name": "auth-flow",
  "operations": [
    { "op": "add_criterion", "criterion": { "name": "tests-pass", "description": "...", "cmd": "cargo test" } },
    { "op": "update_status", "status": "planned" },
    { "op": "remove_requirement", "id": "REQ-FUNC-003" }
  ]
}
```

**Scope:** Medium (3-5 days). Requires designing an operation enum, implementing apply logic per operation, and re-serializing TOML preserving comments where possible.

**Risks:**
- TOML comment preservation is hard; `toml` crate's serializer doesn't preserve comments. Mitigation: use `toml_edit` for round-trip fidelity, or accept comment loss and document it.
- Operation conflicts (e.g., removing a requirement that criteria trace to). Mitigation: validate cross-references after applying all operations.

---

## Proposal 3: `spec_decompose` — Plan Decomposition Primitive

**What:** An MCP tool that takes a spec and produces a structured execution plan: an ordered list of work items with dependencies, estimated scope, and suggested worktree strategy. The tool doesn't generate the plan itself (no LLM) — it extracts structure from the spec's requirements and criteria to produce a decomposition *template* that the agent fills in.

**Why:** Large specs (10+ requirements, 15+ criteria) need decomposition before execution. Currently agents have to reason about this from raw TOML. `spec_decompose` provides a structured scaffold: "here are your requirements grouped by priority, here are the criteria that trace to each group, here's a suggested execution order based on dependency hints."

**Interface sketch:**
```json
{
  "name": "auth-flow",
  "strategy": "by-priority"  // or "by-requirement-group", "by-criterion-type"
}
```
Returns:
```json
{
  "phases": [
    {
      "name": "core-auth",
      "requirements": ["REQ-FUNC-001", "REQ-FUNC-002"],
      "criteria": ["compiles", "tests-pass"],
      "suggested_order": 1
    },
    {
      "name": "security-hardening",
      "requirements": ["REQ-SEC-001"],
      "criteria": ["security-review"],
      "suggested_order": 2,
      "depends_on": ["core-auth"]
    }
  ]
}
```

**Scope:** Medium (3-4 days). Grouping by priority/obligation is straightforward. Dependency inference from requirement IDs and traceability links is the hard part.

**Risks:**
- Over-engineering: agents are good at planning; this tool may provide structure they don't need. Mitigation: keep it simple — group and sort, don't try to be smart about dependencies.
- Dependency inference may be unreliable without explicit `depends_on` fields on requirements. Mitigation: make dependencies agent-supplied, not auto-inferred.

---

## Proposal 4: Spec Dependencies & Ordering (`depends_on` field)

**What:** Add an optional `depends_on: Vec<String>` field to both `Requirement` and `FeatureSpec` types. At the requirement level, it expresses "implement REQ-002 after REQ-001." At the spec level, it expresses "this feature depends on auth-flow being verified first."

**Why:** The orchestration loop needs ordering. Today, specs and requirements are flat lists with no declared ordering. For `spec_decompose` to produce meaningful plans, and for `gate_evaluate` to know which gates to run first, dependency information must be in the data model.

**Changes:**
- `FeatureSpec`: add `depends_on: Vec<String>` (spec names)
- `Requirement`: add `depends_on: Vec<String>` (requirement IDs)
- Validation: cycle detection, referential integrity (all referenced IDs/names must exist)
- `spec_validate` (v0.4.0 planned): include dependency graph validation

**Scope:** Small (1-2 days). Type changes + serde + validation. The interesting part is making `scan_directory` resolve cross-spec dependencies efficiently.

**Risks:**
- Cross-spec dependencies create coupling that makes independent spec development harder. Mitigation: keep this optional and advisory, not blocking.
- Cycle detection in large spec sets could be complex. Mitigation: topological sort with cycle reporting — well-understood algorithm.

---

## Proposal 5: Criteria Libraries — Reusable Criterion Templates

**What:** A `.assay/criteria/` directory containing reusable criterion definitions (e.g., `rust-basics.toml` with "compiles", "clippy-clean", "tests-pass", "fmt-check"). Specs reference these via an `extends` or `include` field, inheriting criteria without duplicating them.

**Why:** Every Rust spec in the project has near-identical criteria: "compiles", "clippy", "tests pass", "formatted." This is DRY violation at the spec level. Criteria libraries let you define common quality baselines once and include them everywhere. This also aligns with the v0.4.0 brainstorm's `extends:` spec inheritance seed.

**Format:**
```toml
# .assay/criteria/rust-basics.toml
[[criteria]]
name = "compiles"
description = "Code compiles without errors"
cmd = "cargo build"

[[criteria]]
name = "clippy-clean"
description = "No clippy warnings"
cmd = "cargo clippy -- -D warnings"
```

Spec references:
```toml
# .assay/specs/auth-flow/gates.toml
name = "auth-flow"
include = ["rust-basics"]

[[criteria]]
name = "auth-specific-test"
description = "Auth integration tests pass"
cmd = "cargo test -p auth"
```

**Scope:** Medium (3-5 days). Requires changes to spec loading (resolve includes before validation), GatesSpec type (add `include` field), and scan logic.

**Risks:**
- Include resolution order matters (spec-local criteria override library ones? Or error on conflict?). Mitigation: explicit override semantics — spec-local wins, with a warning.
- Transitive includes (`rust-basics` includes `general-basics`) add complexity. Mitigation: single-level includes only for v0.4.0, matching the brainstorm's decision.

---

## Proposal 6: `spec_from_issue` — Issue-to-Spec Generation Scaffold

**What:** An MCP tool that accepts a Linear/GitHub issue body (markdown) and produces a draft spec structure. It doesn't use an LLM — it applies heuristics: extract headings as requirements, checklists as acceptance criteria, labels as priority hints. The output is a `spec_create`-compatible parameter set that the agent can review and refine.

**Why:** Many features start as issues. The gap between "issue filed" and "spec written" is a manual translation step. `spec_from_issue` provides a starting point, reducing the authoring effort from "write a spec from scratch" to "review and refine a generated draft."

**Interface sketch:**
```json
{
  "source": "markdown",
  "content": "## Login Flow\n\n- [ ] Email/password auth\n- [ ] OAuth2 support\n...",
  "name_hint": "auth-flow"
}
```

Returns a draft spec structure (not written to disk — agent decides whether to pass it to `spec_create`).

**Scope:** Medium (3-4 days). Markdown parsing heuristics, mapping to FeatureSpec structure.

**Risks:**
- Heuristic quality may be poor for unstructured issues. Mitigation: return a minimal skeleton with `TODO` markers rather than guessing.
- Feature creep toward NLP/LLM territory. Mitigation: keep it purely structural (headings, lists, checkboxes) — no semantic analysis.

---

## Proposal 7: `spec_diff` — Spec Evolution Tracking

**What:** An MCP tool that compares two versions of a spec (current vs. a previous gate run's snapshot, or current vs. git HEAD) and produces a structured diff: added/removed/modified requirements, criteria changes, status transitions.

**Why:** As specs evolve through the loop (draft → proposed → planned → in-progress → verified), agents and humans need to understand *what changed*. This is especially important for `gate_evaluate` — the evaluator needs to know if criteria changed since the last run, and for review workflows where a reviewer needs to see what's new.

**Interface sketch:**
```json
{
  "name": "auth-flow",
  "compare_to": "last_run"  // or "git:HEAD", "git:abc123"
}
```

Returns:
```json
{
  "requirements": {
    "added": ["REQ-SEC-002"],
    "removed": [],
    "modified": [{ "id": "REQ-FUNC-001", "changes": ["statement", "priority"] }]
  },
  "criteria": {
    "added": ["security-review"],
    "removed": ["manual-check"],
    "modified": []
  },
  "status_transition": { "from": "draft", "to": "planned" }
}
```

**Scope:** Medium (3-5 days). Requires snapshotting specs at gate run time (or reading from git), then structural comparison.

**Risks:**
- Git integration adds complexity (shelling out to `git show`, handling missing commits). Mitigation: start with "last gate run snapshot" comparison only, add git in v0.4.1.
- Structural diffs can be noisy (reformatting counts as "modified"). Mitigation: compare parsed structures, not raw TOML text.

---

## Summary Priority Matrix

| # | Proposal | Scope | Strategic Value | Implementation Risk |
|---|----------|-------|-----------------|---------------------|
| 1 | `spec_create` | Small-Med | **High** — enables agent-driven spec authoring | Low |
| 2 | `spec_update` | Medium | **High** — enables iterative refinement | Medium (TOML round-trip) |
| 3 | `spec_decompose` | Medium | **Medium** — agents can plan without this | Medium (usefulness unclear) |
| 4 | Spec dependencies | Small | **High** — foundational for ordering | Low |
| 5 | Criteria libraries | Medium | **High** — reduces duplication, aligns with `extends:` seed | Medium |
| 6 | `spec_from_issue` | Medium | **Medium** — nice-to-have convenience | Medium (heuristic quality) |
| 7 | `spec_diff` | Medium | **Medium** — important for review/evolution | Medium (snapshotting) |

**Recommended v0.4.0 inclusions:** Proposals 1, 2, 4, 5 (the authoring toolkit + structural foundations).
**Defer to v0.4.1:** Proposals 3, 6, 7 (decomposition and tracking — useful but not blocking).
