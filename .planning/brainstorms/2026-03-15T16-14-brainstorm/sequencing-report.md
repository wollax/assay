# Sequencing Report: Assay Platform Expansion

**Date:** 2026-03-15
**Participants:** explorer-sequencing, challenger-sequencing
**Rounds:** 3 (full convergence)

## Executive Summary

After evaluating 5 sequencing strategies and 3 rounds of debate, we recommend **"Thin Vertical Slices"** — three Assay milestones (v0.5.0, v0.6.0, v0.6.1) that deliver complete, demo-able capabilities at each step. The Smelt infrastructure pivot is tracked separately on Smelt's roadmap.

Key architectural decisions that emerged from the debate:
1. **Worktrees stay spec-scoped** — session linkage is additive, not a model replacement
2. **OrchestratorSession composes WorkSessions** — the linear state machine is preserved; graph structure lives above it
3. **Session vocabulary cleanup before absorption** — `AgentSession → GateEvalContext`, Smelt's manifest → `RunManifest`
4. **Minimal HarnessAdapter trait** — explicitly designed for v0.6.0 extension via default methods
5. **`[[sessions]]` array from day one** — avoids breaking schema change when multi-agent arrives

---

## Recommended Sequencing

### v0.5.0 — Single-Agent Harness End-to-End

**Goal:** Prove the harness abstraction through one complete flow: manifest → worktree → agent → gate → merge proposal, for a single agent.

**Scope:**

1. **Session vocabulary cleanup** (first commit, separate from feature work)
   - `AgentSession` → `GateEvalContext` (it's an in-memory gate evaluation context, not a session)
   - Establish naming conventions for incoming Smelt concepts: `RunManifest`, `RunExecutor`
   - Mechanical rename across `assay-types` and `assay-mcp`

2. **`assay-harness` crate**
   - `HarnessProfile`: prompt template + settings + hooks configuration
   - `HarnessAdapter` trait (minimal — 3 methods):
     - `fn build_prompt(&self, profile: &HarnessProfile, spec: &Spec) -> String`
     - `fn apply_settings(&self, profile: &HarnessProfile) -> Result<()>`
     - `fn configure_hooks(&self, profile: &HarnessProfile) -> Result<()>`
   - Doc comments noting expected v0.6.0 extensions (`apply_orchestration_context`, `enforce_scope`)
   - Claude Code adapter (first concrete implementation)
   - Layered prompt builder: project → spec layers (no orchestration layer yet)

3. **Worktree enhancements** (additive, not breaking)
   - Add `session_id: Option<String>` to `WorktreeMetadata`
   - Orphan detection: query for worktrees with no active WorkSession linked
   - Collision prevention: check for spec with active worktree + in-progress session
   - Absorbed from Smelt as validation/query layers on existing spec-scoped model

4. **RunManifest** (single-session, forward-compatible schema)
   ```toml
   [[sessions]]
   spec = "auth-flow"
   agent = "claude"
   model = "sonnet"
   ```
   - Uses `[[sessions]]` array even for single-agent (extends to multi-agent without breaking)

5. **End-to-end flow**
   - manifest → worktree create → agent launch → gate evaluate → merge propose
   - Exercises the full single-agent pipeline through harness

**Key constraints:**
- `HarnessAdapter` is intentionally minimal — will grow in v0.6.0
- Worktree ownership model unchanged (spec-scoped, `assay/{spec_slug}` branches)
- All existing MCP tools continue working unmodified

**Estimated scope:** ~8-10 phases, 6-8 days

---

### v0.6.0 — Multi-Agent Orchestration

**Goal:** Manage N concurrent agents working on related specs with dependency ordering, parallel execution, and sequential merge.

**Scope:**

1. **OrchestratorSession type** (composition, not extension)
   ```rust
   struct OrchestratorSession {
       id: String,
       manifest: RunManifest,           // parsed DAG definition
       sessions: Vec<WorkSession>,      // one per DAG node
       phase: OrchestratorPhase,        // Planned → Running → GatesComplete → Merging → Done
       created_at: DateTime<Utc>,
   }
   ```
   - WorkSession state machine unchanged (Created → AgentRunning → GateEvaluated → Completed)
   - OrchestratorPhase manages the graph lifecycle above individual sessions
   - Persistence as JSON alongside WorkSession files

2. **DAG executor** (absorbed from Smelt)
   - Dependency ordering from `RunManifest` `depends_on` fields
   - Parallel execution of independent nodes
   - Blocking on dependencies before starting downstream sessions

3. **Multi-session RunManifest extension**
   ```toml
   [[sessions]]
   name = "auth"
   spec = "auth-flow"
   agent = "claude"
   depends_on = []

   [[sessions]]
   name = "payments"
   spec = "payments"
   agent = "claude"
   depends_on = ["auth"]
   ```

4. **MergeRunner** (absorbed from Smelt)
   - Sequential merge with conflict detection
   - Integration with existing `merge_check` / `merge_propose` MCP tools
   - Merge ordering respects DAG topology

5. **Scope isolation as gate checks**
   - Scope violations detected as gate criteria
   - Agent work validated against spec boundaries

6. **Harness orchestration layer**
   - Extend `HarnessAdapter` with default methods:
     - `fn apply_orchestration_context(&self, ...) -> Result<()>` (default: no-op)
     - `fn enforce_scope(&self, ...) -> Result<()>` (default: no-op)
   - Claude Code adapter implements orchestration methods
   - Prompt builder gains orchestration layer: project → spec → workflow → orchestration

**Key constraints:**
- WorkSession is not modified — OrchestratorSession wraps it
- HarnessAdapter extensions use default methods (existing adapters don't break)
- v0.5.0 RunManifest files work unchanged (single-session = array with one element)

**Estimated scope:** ~8-10 phases, 7-10 days

---

### v0.6.1 — Conflict Resolution + Integration

**Goal:** Handle the hard merge cases and integrate supporting systems.

**Scope:**

1. **Conflict resolution strategies**
   - AI conflict resolution via evaluator subprocess
   - Human fallback escalation (configurable)
   - Strategy selection per OrchestratorSession

2. **Cupel integration**
   - Context optimization for long-running orchestrated sessions
   - Token budget awareness in OrchestratorSession lifecycle
   - Growth rate tracking across multi-agent runs

3. **Additional adapters**
   - Codex adapter stub
   - OpenCode adapter stub
   - Adapter discovery/registration mechanism

4. **End-to-end validation**
   - Multi-agent workflow integration tests
   - Merge pipeline stress testing (conflict scenarios)

**Estimated scope:** ~5-6 phases, 3-5 days

---

## Proposals Considered and Rejected

| Proposal | Why Rejected |
|----------|--------------|
| **1. Bottom-Up** (user's original) | v0.5.0 monolith risk; harness delayed to week 4; Smelt pivot inflates Assay's milestone count |
| **2. Parallel Tracks** | HarnessAdapter designed without orchestration context produces wrong trait; harness without orchestration is half-story |
| **4. Types-First** | Types for Smelt's domain can't be designed without implementation; precedent from Assay's own types doesn't transfer |
| **5. Migration Sprint** | 12-15 phase monolith; no validation until harness ships; big-bang integration risk |

## Architectural Decisions from Debate

### Decision 1: Worktrees stay spec-scoped

**Context:** Smelt's worktree manager is session-aware. Assay's current model ties worktrees to specs (`create(project_root, spec_slug, ...)`), with `assay/{spec_slug}` branch naming and `WorktreeMetadata { base_branch, spec_slug }`.

**Decision:** Keep spec-scoped ownership. Add `session_id: Option<String>` to `WorktreeMetadata` for session linkage. Smelt's lifecycle features (orphan detection, collision prevention) become query/validation layers on the existing model.

**Rationale:** Changing to session-scoped worktrees would break 4 MCP tools and the branch naming convention. Additive linkage achieves the same goal without breakage. Orphan detection = "worktrees with no active WorkSession linked." Collision prevention = "spec already has active worktree with in-progress session."

### Decision 2: OrchestratorSession composes WorkSessions

**Context:** WorkSession has a linear state machine (Created → AgentRunning → GateEvaluated → Completed) with hardcoded transitions in `can_transition_to()`. DAG execution needs parallel/branching phases.

**Decision:** Create `OrchestratorSession` that contains `Vec<WorkSession>`. Each DAG node is a WorkSession with its own linear lifecycle. The graph structure, dependency tracking, and merge coordination live in OrchestratorSession.

**Rationale:** Extending SessionPhase for DAG would require rewriting transition logic and updating every MCP tool that reads/writes sessions. Composition preserves the proven linear model and adds the graph above it.

### Decision 3: Session vocabulary cleanup

**Context:** Codebase has `AgentSession`, `WorkSession`, `SessionPhase`, and Smelt adds `SessionManifest`, `SessionRunner` — 5 "session" concepts.

**Decision:** Rename before absorption:
- `AgentSession` → `GateEvalContext` (in-memory gate evaluation context)
- Smelt's manifest → `RunManifest`
- Smelt's runner → `RunExecutor`
- `WorkSession` stays (it IS the work session)

**Rationale:** Naming clarity prevents confusion when working across the unified codebase. Committed separately for clean git blame.

### Decision 4: HarnessAdapter starts minimal

**Context:** Full adapter interface needs orchestration context (multi-agent prompts, scope constraints), but this doesn't exist until v0.6.0.

**Decision:** v0.5.0 trait has 3 methods (`build_prompt`, `apply_settings`, `configure_hooks`). v0.6.0 adds orchestration methods with default implementations. Doc comments on the trait explicitly call out expected extensions.

**Rationale:** Designing against incomplete requirements produces wrong abstractions (P2's flaw). Rust's default methods allow additive extension without breaking existing adapters.

### Decision 5: `[[sessions]]` array from day one

**Context:** Single-session manifest could use `[session]` (singular) but multi-session needs `[[sessions]]` (array).

**Decision:** Use `[[sessions]]` even for single-agent in v0.5.0.

**Rationale:** Costs nothing (array with one element). Avoids breaking schema change or backward-compat shim in v0.6.0.

### Decision 6: Smelt pivot is not an Assay milestone

**Context:** User's original phasing included "Phase 4: Smelt infrastructure pivot" as an Assay milestone.

**Decision:** Remove from Assay's roadmap. Track on Smelt's roadmap as work unblocked by v0.5.0 absorption.

**Rationale:** Smelt pivot doesn't touch Assay's codebase. Including it inflates Assay's milestone count and creates false dependency chains.

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| v0.5.0 E2E crosses too many boundaries (harness, worktree, manifest, merge) | Medium | High | Each boundary change is additive/minimal; no model replacements |
| HarnessAdapter trait needs significant rework in v0.6.0 | Medium | Medium | Kept minimal with 3 methods; default method extension is non-breaking |
| Partial Smelt absorption creates fork maintenance | Low | Medium | v0.5.0 absorbs worktree features completely; DAG/merge deferred cleanly to v0.6.0 |
| OrchestratorSession design discovered wrong in v0.6.0 | Low | High | v0.5.0 doesn't introduce it — design happens when DAG requirements are concrete |
| RunManifest schema evolves beyond `[[sessions]]` + `depends_on` | Low | Low | TOML is flexible; additional fields are additive |

---

## Timeline Summary

| Milestone | Phases | Duration | Cumulative |
|-----------|--------|----------|------------|
| v0.4.1 Merge Tools | 5 | 2-3 days | Week 1 |
| v0.5.0 Single-Agent E2E | 8-10 | 6-8 days | Weeks 2-3 |
| v0.6.0 Multi-Agent Orchestration | 8-10 | 7-10 days | Weeks 4-5 |
| v0.6.1 Conflict Resolution + Integration | 5-6 | 3-5 days | Week 6 |

**Total expansion: ~4-5 weeks after v0.4.1 ships.**
