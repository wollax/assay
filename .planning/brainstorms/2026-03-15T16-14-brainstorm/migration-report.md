# Migration & Risk Mitigation — Consolidated Report

**Participants**: explorer-migration, challenger-migration
**Date**: 2026-03-15
**Rounds**: 3 (initial proposals → critique → convergence)

---

## Executive Summary

After analyzing Assay's codebase (worktree.rs, work_session.rs, gate/session.rs, server.rs, assay-types) and debating 5 migration strategies across 3 rounds, we converged on a **4-phase incremental plan** that prioritizes zero blast radius and rollback safety.

The key insight: Assay's existing `WorkSession` state machine and `SessionPhase` enum are fundamentally incompatible with DAG-style orchestration (the serde boundary rejects unknown phase variants). Rather than force-extending existing types, the plan introduces a parallel orchestration layer that coexists with, and eventually subsumes, the current abstractions.

---

## Recommended Plan: 4 Phases

### Phase 0: AgentSession Persistence (Prerequisite)

**What**: Make the in-memory `AgentSession` store (`Arc<Mutex<HashMap>>` in server.rs) crash-recoverable by adding write-through persistence.

**Why**: The MCP server currently loses all active gate evaluation sessions on restart. Orchestrating N parallel evaluations amplifies this from "annoying" to "unacceptable." This must be fixed before building on top.

**How**:
- Serialize AgentSession to `.assay/agent-sessions/{session_id}.json` using the existing `NamedTempFile` + rename pattern from `work_session.rs`
- Convert the in-memory HashMap to a write-through cache: reads from memory, writes to memory + disk
- Load persisted sessions on server startup, prune expired ones
- `AgentSession` already derives `Serialize`/`Deserialize` — no type changes needed

**Scope**: 1 phase. Small. Pattern is proven.

**Risk**: Concurrent MCP requests writing the same session file. Mitigation: the `Mutex` already serializes access; write-through preserves this guarantee.

---

### Phase 1: Orchestrator Crate (Proof of Concept)

**What**: Create `crates/assay-orchestrator` as a workspace member that wraps Smelt's capabilities behind Assay's domain model.

**Why**: This proves orchestration works without changing any existing crate. If it fails, `cargo remove assay-orchestrator` reverts to v0.4.0 cleanly.

**How**:
- New crate: `assay-orchestrator` depends on `assay-core` and `assay-types`
- Implements Smelt's worktree enhancements (orphan detection, collision prevention, session-aware lifecycle) using `assay-core::worktree` functions internally
- Introduces `OrchestrationManifest` type for DAG sessions (separate from `WorkSession`)
- Feature-gated optional dependency from `assay-mcp`: `assay-mcp --features orchestrator`
- **Time-boxed**: if the adapter survives 2 milestone cycles without being inlined, it's tech debt. Plan the inline from day one.

**Scope**: Medium. New crate, but delegating to existing internals.

**Risks**:
- Semantic impedance mismatch: Smelt returns session-aware handles, Assay returns `WorktreeInfo` without session context. The adapter must bridge this gap, and every translation is a bug surface.
- 124 open tech debt issues may conflict with adapter assumptions about internal APIs. Mitigate by auditing issues tagged `worktree` and `session` before starting.

**Key design decision**: Name the crate for what it *does* (assay-orchestrator), not what it *wraps* (smelt-adapter).

---

### Phase 2: Parallel MCP Tools (Additive)

**What**: Add `orchestrate_*` MCP tools that expose the orchestrator. Existing tools remain completely untouched.

**Why**: From the agent consumer perspective, new namespaced tools are the clearest evolution path. Agents already distinguish `gate_run` sessions from `session_create` sessions. Adding `orchestrate_*` is the same pattern.

**New tools**:
- `orchestrate_create_worktree` — session-bound worktree creation (`session_id: String` required, spec inferred from session)
- `orchestrate_start` — launch an orchestration manifest (DAG of sessions)
- `orchestrate_status` — check orchestration progress
- `orchestrate_merge_check` / `orchestrate_merge_propose` — merge workflow tools

**Critical decision: Do NOT add optional params to existing tools.** Adding `session_id: Option<String>` to `WorktreeCreateParams` creates interfield validation ambiguity (does `name` still mean spec slug when session_id is set? what if they disagree?). New tools with required params are unambiguous.

**Forward-compatibility**: Use the existing `assay_version` field (semver comparison) for version gating, not a new `schema_version` integer. Semver is more expressive and already exists on both `WorkSession` and `GateRunRecord`.

**Scope**: Medium. Follows existing MCP server patterns (Parameter struct + schemars + handler).

**Risks**:
- Tool proliferation: 18 existing tools + 4-5 orchestrate tools = 22-23 total. Manageable but approaching the limit of what agents can reason about. Consider tool grouping in descriptions.
- Feature flag management: `assay-mcp --features orchestrator` gates the new tools. Must test both with and without.

---

### Phase 3: Composition-Based Unification

**What**: Extract shared fields into a `SessionCore` struct embedded in both `WorkSession` and `OrchestrationManifest`. Deprecate (don't remove) old tool names.

**Why**: Once the orchestrator API surface stabilizes through real usage, unify by promoting the most-used patterns into assay-types/assay-core.

**How**:
```rust
// In assay-types
pub struct SessionCore {
    pub id: String,
    pub spec_name: String,
    pub worktree_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub assay_version: String,
}

// WorkSession embeds it (backward compatible via #[serde(flatten)])
pub struct WorkSession {
    #[serde(flatten)]
    pub core: SessionCore,
    pub phase: SessionPhase,        // Linear state machine, untouched
    pub transitions: Vec<PhaseTransition>,
    pub agent: AgentInvocation,
    pub gate_runs: Vec<String>,
}

// OrchestrationManifest embeds it too
pub struct OrchestrationManifest {
    #[serde(flatten)]
    pub core: SessionCore,
    pub phase: OrchestrationPhase,  // DAG-aware: Scheduling, Running, Merging, etc.
    pub sessions: Vec<String>,      // Child WorkSession IDs
    pub dag: DependencyGraph,
    pub merge_strategy: MergeStrategy,
}
```

**Critical decision: Use struct composition, NOT a trait.** A `SessionCore` trait would require `dyn SessionCore` compatibility, which breaks `#[serde(flatten)]` and forces dynamic dispatch. Struct embedding gives shared *data* without shared *behavior*.

**Existing `SessionPhase` is untouched.** The linear `Created → AgentRunning → GateEvaluated → Completed` state machine stays exactly as-is for single-spec workflows. `OrchestrationPhase` is a separate enum for DAG workflows.

**Scope**: Medium-High. Type changes propagate through the codebase.

**Risks**:
- `#[serde(flatten)]` changes the JSON layout of `WorkSession`. Existing persisted sessions won't deserialize unless migration code is added. Mitigation: version-gate on `assay_version >= 0.6.0`.
- The deprecation of old `session_*` tools may never complete if agents depend on them. Set a removal timeline (2 major versions).

---

## Rejected Strategies (with reasons)

| Strategy | Why Rejected |
|----------|-------------|
| **Bottom-Up Type Unification (alone)** | `SessionPhase` serde boundary rejects unknown variants. Can't additively add DAG phases without breaking the `session_phase_unknown_variant_errors` test contract. |
| **Worktree-First (alone)** | Branch naming `assay/{spec_slug}` is hardcoded throughout list/cleanup/MCP tools. Multi-worktree-per-spec (needed for DAG) requires renaming, which is a breaking change. |
| **Adapter as permanent layer** | Semantic impedance mismatch accumulates. Session-aware handles ↔ session-unaware WorktreeInfo translation is a persistent bug surface. |
| **Schema-first MCP stubs** | Risk of over-designing contracts before the domain model is understood through the adapter proof-of-concept. |

---

## Cross-Cutting Risks & Mitigations

### 1. Tech Debt Interference
**Risk**: 124 open tech debt issues may conflict with orchestrator assumptions.
**Mitigation**: Audit `worktree`/`session`-tagged issues before Phase 1. Close or defer conflicting issues.

### 2. Error Variant Proliferation
**Risk**: `AssayError` has 25+ variants. Orchestration adds DAG cycle detection, merge conflict, scope violation, manifest parse errors.
**Mitigation**: Introduce error domains. `OrchestratorError` as a separate enum, wrapped by `AssayError::Orchestrator(OrchestratorError)`. Feature-gated.

### 3. Config Evolution
**Risk**: `Config` struct needs orchestration config. Where?
**Mitigation**: `orchestration: Option<OrchestratorConfig>` as a top-level key in `config.toml`. Feature-gated. Only parsed when `orchestrator` feature is enabled.

### 4. Plugin/Hook Compatibility
**Risk**: Existing plugins/hooks may depend on worktree/session event shapes.
**Mitigation**: Audit all plugins before Phase 2. Existing events unchanged; orchestrate events are additive.

### 5. Test Regression
**Risk**: 836 tests must pass at every phase boundary.
**Mitigation**: Existing tests are the regression gate. New orchestrator tests are additive, never replacements. CI runs both `cargo test` and `cargo test --features orchestrator`.

---

## Sequencing Summary

```
Phase 0: AgentSession Persistence
   ↓ (prerequisite — crash recovery for parallel sessions)
Phase 1: assay-orchestrator crate (proof of concept, feature-gated)
   ↓ (proves Smelt capabilities map to Assay domain)
Phase 2: orchestrate_* MCP tools (additive, no existing tool changes)
   ↓ (agents opt in to orchestration)
Phase 3: SessionCore composition + type unification (deprecate old paths)
```

Each phase is independently releasable. Each phase passes all 836 existing tests. Rollback at any point is: disable the `orchestrator` feature flag.
