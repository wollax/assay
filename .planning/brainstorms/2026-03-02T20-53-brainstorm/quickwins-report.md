# Quick Wins Report — Assay v0.2.0

*Written by team lead after explorer-quickwins stalled. Based on analysis of 38 open issues and codebase review.*

## Surviving Proposals

### 1. Issue Burn-Down: Error Handling & CLI Polish

**What:** Address ~15 open issues in a single focused phase: CLI error propagation (#cli-error-propagation), bare invocation exit code, RUST_LOG feedback, .assay path constant extraction, help duplication, color branch simplification, version semantics clarification.

**Why:** These are individually trivial (1-2 hours each) but collectively make the CLI feel unfinished. Fixing them removes noise from the issue backlog and makes Assay credible for early adopters. A polished CLI is the first thing users touch.

**Scope:** Small — 1 phase, ~200-300 lines changed across assay-cli.

**Risks:** Low. Each fix is independent and testable. Risk is only in batch size — if too many changes land at once, debugging regressions is harder. Mitigate with per-fix commits.

---

### 2. MCP Hardening

**What:** Address the 9 MCP-specific issues: missing timeout parameter for agents, working_dir validation, spec_list error handling, failure reason stdout fallback, unnecessary clones, tool description accuracy, response struct docs, SpecNotFound dead code.

**Why:** The MCP surface is how agents interact with Assay. Every rough edge here is multiplied by every agent session. The timeout parameter issue is particularly critical — agents can't control gate timeouts, which is a usability gap.

**Scope:** Small — 1 phase, ~150-250 lines across assay-mcp.

**Risks:** Low. Most fixes are straightforward. The timeout parameter requires a schema change (MCP tool input struct), which touches the API contract — needs care with backwards compatibility.

---

### 3. Type System Hygiene

**What:** Address type-level issues: serde skip_serializing_if on all domain types, OutputDetail enum, truncation metadata on GateResult, type invariant enforcement. Also wire the existing `FileExists` gate kind (dead code).

**Why:** Types are the foundation — every surface (CLI, MCP, TUI) consumes them. Getting types right now prevents costly migrations later. The `skip_serializing_if` fix alone improves JSON output quality for every consumer.

**Scope:** Small-medium — 1 phase, ~200-400 lines in assay-types + assay-core.

**Risks:** Medium. Type changes propagate everywhere. The OutputDetail enum and truncation metadata are design decisions that need to be right the first time. Wire format vs display format types (issue #wire-format-types) should be evaluated but likely deferred — it's a bigger refactor.

---

### 4. Test Coverage Gaps

**What:** Address the two test coverage gap issues (Phase 3 and Phase 6) plus add MCP tool handler tests (currently zero). Target: integration tests for all 3 MCP tool handlers, edge case tests for gate evaluation, error path tests for config/spec loading.

**Why:** 119 tests is good for v0.1 but the MCP layer has zero direct tests — the most critical surface is the least tested. Adding tests now provides a safety net for all the v0.2 feature work.

**Scope:** Small-medium — 1 phase, ~300-500 lines of test code.

**Risks:** Low. Tests don't change behavior. The MCP handler tests may need test infrastructure (mock MCP client or direct function calls) — choose the simpler approach.

---

### 5. Tooling Tightening

**What:** Tighten cargo-deny from warn to deny for multiple-versions and source controls. Add CI plugin schema validation. Document ANSI overhead assumptions.

**Why:** Warnings that aren't enforced are ignored. Making these deny-level catches real problems before they ship. Minimal effort, prevents future headaches.

**Scope:** Small — half a phase, ~50 lines of config changes.

**Risks:** Very low. If deny fails the build, that's the point — fix the violation, don't weaken the check.

---

### 6. Dogfooding Checkpoint

**What:** Use Assay's own gate system to enforce quality on Assay itself. Create a spec with criteria like "clippy passes," "tests pass," "no cargo-deny violations" — then run `assay gate run` as part of the dev workflow.

**Why:** This is the ultimate validation that the tool works. If we can't use Assay to build Assay, something is wrong. It also surfaces real UX issues that synthetic testing misses.

**Scope:** Small — 1 spec file + integration into `just ready`. No code changes.

**Risks:** Low. The main risk is that dogfooding reveals problems that expand scope — but that's a feature, not a bug.

---

## Recommended Sequencing

1. **Dogfooding checkpoint** (do first — surfaces issues for everything else)
2. **Test coverage gaps** (safety net before feature work)
3. **Type system hygiene** (foundation for v0.2 features)
4. **MCP hardening** (agent-facing surface)
5. **CLI polish** (human-facing surface)
6. **Tooling tightening** (guard rails)

Items 1-2 should precede v0.2 feature work. Items 3-6 can be interleaved with or precede feature phases.

## Dropped Ideas

- **Wire format vs display format types** — Too large for a quick win. Evaluate during type hygiene but likely defer to v0.3.
- **Streaming capture with byte budget** — Interesting but changes gate evaluation architecture. Better as part of a feature phase.
- **Progressive gate disclosure (two-tool MCP pattern)** — Design change to MCP surface. Should be considered alongside gate_report tool, not independently.
