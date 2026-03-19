# Architecture Report: Assay Platform Expansion

**Date:** 2026-03-15
**Explorer:** explorer-architecture
**Challenger:** challenger-architecture
**Rounds:** 2 (converged)
**Status:** Consensus reached

---

## Decision: P1 (Core Expansion) with Callback Inversion

After exploring 5 proposals and 2 rounds of debate, the team converged on absorbing Smelt orchestration as modules in `assay-core` with a new `assay-harness` leaf crate for adapter implementations. Control inversion uses closures/callbacks, not traits.

### Final Dependency Graph

```
assay-cli ──→ assay-core ──→ assay-types
assay-tui ──→ assay-core ──→ assay-types
assay-mcp ──→ assay-core ──→ assay-types
assay-harness ──→ assay-core ──→ assay-types
```

One new crate (`assay-harness`) as a leaf. No changes to the existing dependency direction.

---

## Architecture Details

### 1. Orchestration in assay-core

New `orchestrate/` module in `assay-core`:

```
assay-core/src/
  orchestrate/
    mod.rs             — public API, OrchestrateError variants
    dag.rs             — dependency graph, topological sort, parallel wave computation
    merge.rs           — MergeRunner: sequential merge, AI conflict resolution, human fallback
    manifest.rs        — SessionManifest: multi-session coordination
    scope.rs           — scope isolation: worktree boundaries, env vars, resource limits
```

Extensions to existing modules:
- `worktree.rs` — orphan worktree detection, branch collision avoidance, bulk cleanup
- `work_session.rs` — session runner via callback-based agent invocation

### 2. assay-harness as a Leaf Crate

```
assay-harness/src/
  lib.rs               — HarnessProfile loading, public API
  prompt.rs            — layered prompt builder (project → spec → workflow → orchestration)
  settings.rs          — layered settings merger (project → spec → orchestration)
  hooks.rs             — hook contracts → harness-specific hook translation
  adapters/
    mod.rs             — adapter registry (enum dispatch, not traits)
    claude_code.rs     — Claude Code adapter: subprocess invocation, prompt formatting, hook translation
```

### 3. Callback Inversion Pattern

Agent invocation uses closures, not traits. This is consistent with the codebase's zero-trait convention:

```rust
// assay-core/src/orchestrate/session_runner.rs
pub async fn run_session<F, Fut>(
    session: &mut WorkSession,
    launch_agent: F,
    // ...
) -> Result<SessionOutcome>
where
    F: FnOnce(&WorkSession) -> Fut,
    Fut: Future<Output = Result<AgentHandle>>,
{
    // Core orchestration logic
    // Calls launch_agent(session) when ready
    // Handles lifecycle: monitor → gate → complete
}
```

Binary crates wire the callback at the integration point:

```rust
// In assay-cli or assay-mcp
let adapter = ClaudeCodeAdapter::new(profile);
run_session(&mut session, |ws| adapter.launch(ws)).await?;
```

### 4. Type Placement Rules

**In `assay-types` (cross-crate DTOs with serde + schemars):**
- `HarnessProfile` — crosses boundaries via config deserialization
- `SettingsLayer` — spec/config references
- `HookContract` — spec-level hook declarations
- `SessionManifest` — exposed by MCP tools
- `MergeStrategy` — config-level enum

**NOT in `assay-types` (implementation-internal):**
- `HarnessArtifacts` — internal to adapter implementations
- `PromptLayer` / `PromptLayerBuilder` — internal to prompt building
- Adapter-specific types (`ClaudeCodeProcessArgs`, etc.)

**Rule:** assay-types holds types that cross crate boundaries via serialization. Internal implementation types stay in their crate.

---

## Proposals Explored and Eliminated

| Proposal | Verdict | Reason |
|----------|---------|--------|
| P1: Core Expansion | **Selected** (with callbacks) | Lowest disruption, consistent with existing patterns |
| P2: Orchestrate Crate | Eliminated R1 | Premature — only 2 new modules + extensions, not enough for a crate. Trait proliferation violates codebase conventions |
| P3: Trait-in-Types | Eliminated R1 | Breaks pure-DTO invariant, pulls async into types crate, one implementation = premature abstraction |
| P4: Layered Sandwich | Eliminated R1 | Deepens dep chain to 4 levels, type placement tensions, evaluator split-brain |
| P5: Feature-Gated Core | Eliminated R1 | cfg complexity, IDE experience degradation, still a monolith organizationally |

---

## Key Debate Points Resolved

### "Orchestration is a separate domain" — Rejected

Smelt's functionality decomposes into ~2 genuinely new modules (DAG + merge) and ~4 extensions to existing modules (worktree, session). This doesn't justify crate-level separation. Smelt was a separate *project*, not a separate *domain*.

### "HarnessAdapter trait needed for dep graph" — Rejected

The callback/closure pattern achieves the same control inversion without introducing traits. The codebase has zero traits across 20k lines — this is a deliberate convention, not an oversight. Callbacks are the Rust-idiomatic alternative.

### "evaluator.rs should be unified with harness" — Deferred

`evaluator.rs` currently hardcodes Claude Code subprocess invocation. This is architecturally similar to what harness does, and is a future unification candidate. However, premature unification risks leaky abstractions — the evaluator pipeline (prompt → spawn → parse → map) has different lifecycle concerns than the session runner (create → launch → monitor → gate → complete). **Trigger for unification:** when a second harness adapter materializes that proves the abstraction.

### "HarnessProfile in assay-types without core consumer" — Resolved

Not a code smell. `HarnessProfile` will be deserialized by `assay-core`'s config loader when reading `[harness]` from `config.toml` — same pattern as `WorktreeConfig`. The placement rule is "crosses crate boundaries via serialization," not "consumed by core's business logic."

---

## Design Principles Preserved

1. **assay-types = pure DTOs.** Serialize/Deserialize/JsonSchema, no behavior, no traits, no async
2. **assay-core = all domain logic.** Specs, gates, worktrees, sessions, AND orchestration
3. **No traits.** Enum dispatch for variant selection, closures for control inversion
4. **Binaries are thin wrappers.** CLI, TUI, MCP server delegate to core; wire callbacks at integration points
5. **Leaf crates for implementations.** assay-harness depends on core, not the other way around

---

## Implementation Guidance

### Config Extension

Add to `assay-types/src/lib.rs`:
```rust
pub struct Config {
    // ... existing fields ...
    pub orchestrate: Option<OrchestrateConfig>,
    pub harness: Option<HarnessProfile>,
}
```

### Module Registration

New modules in `assay-core/src/lib.rs`:
```rust
pub mod orchestrate;  // DAG, merge, manifest, scope
```

### Workspace Dependency

Add to root `Cargo.toml`:
```toml
assay-harness = { path = "crates/assay-harness" }
```

### Migration Path for Existing Worktree/Session Code

No migration needed. Extend in place:
- `worktree.rs`: add `detect_orphans()`, `check_branch_collision()` functions
- `work_session.rs`: add `run_session()` with callback parameter

---

*Consensus reached after 2 rounds of explorer/challenger debate. Both sides aligned on all points.*
