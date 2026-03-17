# Architecture Proposals: Assay Platform Expansion

**Explorer:** explorer-architecture
**Date:** 2026-03-15
**Scope:** Smelt orchestration absorption + assay-harness crate placement

---

## Current State Summary

**Existing dependency graph:**
```
assay-cli ──→ assay-core ──→ assay-types
assay-tui ──→ assay-core ──→ assay-types
assay-mcp ──→ assay-core ──→ assay-types
```

**Key observations from code exploration:**
- `assay-types` is a pure DTO crate: serde + schemars, no business logic, ~50 types across 12 modules
- `assay-core` handles all domain logic: spec loading/validation, gate evaluation, worktree lifecycle, work sessions, context diagnostics, guard daemon, evaluator subprocess
- `assay-core` already has worktree management (`worktree.rs`: create/list/status/cleanup + metadata) and work sessions (`work_session.rs`: ULID-based lifecycle with phase state machine: Created → AgentRunning → GateEvaluated → Completed → Abandoned)
- `assay-mcp` is a thin server layer: MCP tool handlers that delegate to `assay-core` functions
- The codebase has `inventory`-based schema registry for JSON Schema generation
- `WorkSession` links a spec + worktree + agent invocation + gate runs through a linear phase pipeline
- `WorktreeMetadata` persists base_branch + spec_slug at `<worktree>/.assay/worktree.json`
- `cupel` is already a workspace dependency (context engine, extracted from workspace)

**What Smelt brings that Assay doesn't have:**
1. Orphan worktree detection + branch collision avoidance
2. SessionManifest (multi-session coordination)
3. MergeRunner (sequential merge with AI conflict resolution + human fallback)
4. DAG executor (dependency-ordered spec execution)
5. Scope isolation (preventing cross-session contamination)
6. Session runner (agent lifecycle beyond create/transition)

**What assay-harness needs:**
1. HarnessProfile = prompt + settings + hooks per harness
2. HarnessAdapter trait (profile → harness-specific artifacts)
3. Layered prompt builder (project → spec → workflow → orchestration)
4. Layered settings (project → spec → orchestration)
5. Hook contracts + harness-specific translation
6. First adapter: Claude Code

---

## Proposal 1: "Core Expansion" — Everything in assay-core

### What
Absorb all Smelt functionality as new modules in `assay-core` and add `assay-harness` as a new crate at the same level.

```
assay-cli ──→ assay-core ──→ assay-types
assay-tui ──→ assay-core ──→ assay-types
assay-mcp ──→ assay-core ──→ assay-types
assay-harness ──→ assay-core ──→ assay-types
```

**Module layout in assay-core:**
```
assay-core/src/
  worktree.rs          → worktree.rs (extend: orphan detection, collision avoidance)
  work_session.rs      → work_session.rs (extend: session runner, manifest)
  orchestrate/
    mod.rs             → DAG executor, scope isolation
    merge.rs           → MergeRunner (sequential merge, AI conflict, human fallback)
    manifest.rs        → SessionManifest (multi-session coordination)
    dag.rs             → dependency graph + topological sort
```

**assay-harness layout:**
```
assay-harness/src/
  lib.rs               → HarnessAdapter trait, HarnessProfile
  prompt.rs            → layered prompt builder
  settings.rs          → layered settings merger
  hooks.rs             → hook contracts + translation
  adapters/
    mod.rs
    claude_code.rs     → Claude Code adapter
```

**Types in assay-types:**
```
assay-types/src/
  harness.rs           → HarnessProfile, HookContract, SettingsLayer (DTOs only)
  orchestrate.rs       → SessionManifest, MergeStrategy, DagNode (DTOs only)
```

### Why
- **Minimal disruption.** Existing worktree and session modules are extended in-place, not moved. Zero refactoring of existing code paths.
- **Preserves the "types = DTOs, core = logic" invariant.** The clear boundary stays clean.
- **assay-harness as a peer crate** keeps harness concerns out of core while having access to all domain logic.
- **Natural growth path.** assay-core is 33k lines — a few hundred more lines of orchestration won't strain it.

### Scope
- **Low refactoring.** Extend `worktree.rs` and `work_session.rs` in place. New `orchestrate/` module.
- **New crate creation.** `assay-harness` only.
- **Config extension.** Add `[orchestrate]` and `[harness]` sections to `Config`.

### Risks
- **assay-core becomes a monolith.** It's already the largest crate (all domain logic). Adding orchestration + merge + DAG could make compile times and cognitive load worse.
- **Tight coupling.** Orchestration logic directly importing `crate::worktree`, `crate::work_session`, `crate::gate` makes it hard to test in isolation.
- **HarnessAdapter in assay-harness can't be used by assay-core.** If orchestration needs to invoke a harness (e.g., session runner launches agent via harness), this creates a circular dependency or forces the trait into assay-types.

---

## Proposal 2: "Orchestration Layer" — New assay-orchestrate Crate

### What
Extract orchestration into a dedicated crate that sits between core and the binaries/MCP server.

```
assay-cli ──→ assay-orchestrate ──→ assay-core ──→ assay-types
assay-tui ──→ assay-orchestrate ──→ assay-core ──→ assay-types
assay-mcp ──→ assay-orchestrate ──→ assay-core ──→ assay-types
assay-harness ──→ assay-orchestrate ──→ assay-core ──→ assay-types
                 (also: assay-harness ──→ assay-types directly)
```

**assay-orchestrate layout:**
```
assay-orchestrate/src/
  lib.rs               → re-exports, OrchestrateError
  session_runner.rs    → runs agent lifecycle (create session → start agent → monitor → gate → complete)
  merge_runner.rs      → sequential merge with conflict strategies
  manifest.rs          → SessionManifest: multi-session coordination
  dag.rs               → dependency graph, topological sort, parallel wave computation
  scope.rs             → scope isolation (worktree boundaries, env vars, resource limits)
  worktree_ext.rs      → orphan detection, branch collision, bulk cleanup
```

**assay-harness layout:**
```
assay-harness/src/
  lib.rs               → HarnessAdapter trait, HarnessProfile
  prompt.rs            → layered prompt builder
  settings.rs          → layered settings merger
  hooks.rs             → hook contracts
  adapters/
    claude_code.rs     → Claude Code adapter
```

**Key trait:** `HarnessAdapter` lives in `assay-harness`, not `assay-types`. The orchestrate crate depends on harness for agent invocation:

```
assay-orchestrate ──→ assay-harness ──→ assay-types
                 └──→ assay-core    ──→ assay-types
```

### Why
- **Clean separation of concerns.** `assay-core` stays focused on single-spec/single-session domain logic. `assay-orchestrate` handles multi-session coordination.
- **Independent compile and test.** Orchestration logic can be tested with mock HarnessAdapters and mock worktree/session operations without pulling in all of core.
- **Mirrors the conceptual boundary.** Smelt was a separate tool because orchestration IS a distinct domain from spec/gate evaluation.
- **Future extraction.** If orchestration ever needs to run standalone (e.g., a CI-only orchestrator), it's already a clean crate.

### Scope
- **Moderate refactoring.** May need to extract some functions from `worktree.rs` and `work_session.rs` into traits so `assay-orchestrate` can depend on abstractions rather than concrete implementations.
- **Two new crates.** `assay-orchestrate` + `assay-harness`.
- **Config splitting.** Orchestration config in its own section, harness config in its own section.

### Risks
- **Trait proliferation.** To avoid concrete dependencies on assay-core internals, you'd need `WorktreeManager` trait, `SessionManager` trait, etc. This adds ceremony.
- **Deeper dep graph = slower full builds.** More crate boundaries means more linking steps.
- **assay-harness position is awkward.** If orchestrate depends on harness, and harness needs types from core (e.g., `Spec`, `Criterion`), the dep graph gets tangled unless harness only depends on assay-types.

---

## Proposal 3: "Trait-in-Types" — HarnessAdapter as a First-Class Abstraction

### What
Put the `HarnessAdapter` trait in `assay-types` alongside its DTOs. Orchestration stays in `assay-core`. Harness implementations live in `assay-harness`.

```
assay-cli     ──→ assay-core ──→ assay-types (has HarnessAdapter trait)
assay-tui     ──→ assay-core ──→ assay-types
assay-mcp     ──→ assay-core ──→ assay-types
assay-harness ──→ assay-types (implements HarnessAdapter)
```

**assay-types additions:**
```
assay-types/src/
  harness.rs           → HarnessProfile, HarnessAdapter trait, HookContract, SettingsLayer
  orchestrate.rs       → SessionManifest, MergeStrategy, DagSpec (DTOs)
```

**Key: HarnessAdapter trait in assay-types:**
```rust
pub trait HarnessAdapter: Send + Sync {
    fn translate_profile(&self, profile: &HarnessProfile) -> Result<HarnessArtifacts>;
    fn launch_agent(&self, artifacts: &HarnessArtifacts, session: &WorkSession) -> Result<AgentHandle>;
    fn build_prompt(&self, layers: &[PromptLayer]) -> Result<String>;
    fn translate_hooks(&self, contracts: &[HookContract]) -> Result<Vec<HarnessHook>>;
}
```

**Orchestration in assay-core uses `dyn HarnessAdapter`:**
```rust
// assay-core/src/orchestrate/session_runner.rs
pub fn run_session(
    session: &WorkSession,
    adapter: &dyn HarnessAdapter,
    // ...
) -> Result<SessionOutcome> { ... }
```

### Why
- **Maximum reuse.** Any crate in the workspace can accept `&dyn HarnessAdapter` without depending on `assay-harness`. Plugins, MCP server, CLI — all can work with harness adapters generically.
- **No circular dependencies.** The trait is in the bottom of the dep graph; implementations are in a leaf crate.
- **assay-types stays lightweight.** A trait with no impls and a few DTOs doesn't add significant weight. `assay-types` already has `inventory`-based registration; adding a trait is consistent.
- **Smelt absorption is clean.** Orchestration modules in `assay-core` can use `&dyn HarnessAdapter` to launch agents without knowing about Claude Code specifics.

### Scope
- **Low-moderate refactoring.** No existing code moves. Add trait + DTOs to `assay-types`, add orchestration modules to `assay-core`, create `assay-harness` with adapter implementations.
- **One new crate.** `assay-harness` only.
- **Trait design is the hard part.** Getting the `HarnessAdapter` API right is critical — it must be general enough for future harnesses without over-abstracting.

### Risks
- **Trait in types breaks the "pure DTO" rule.** `assay-types` has been strictly serializable types with no behavior. Adding a trait is a philosophical shift.
- **Async considerations.** If `launch_agent` needs to be async (likely for agent lifecycle management), the trait needs `async_trait` or manual `Pin<Box<dyn Future>>`. This pulls async runtime concerns into assay-types.
- **API stability pressure.** Since the trait is in the most-depended-upon crate, changing its signature ripples everywhere.

---

## Proposal 4: "Layered Sandwich" — assay-harness Between types and core

### What
Position `assay-harness` as a middle layer: it depends on `assay-types` and `assay-core` depends on it. This creates a 4-layer stack.

```
assay-cli ──→ assay-core ──→ assay-harness ──→ assay-types
assay-tui ──→ assay-core ──→ assay-harness ──→ assay-types
assay-mcp ──→ assay-core ──→ assay-harness ──→ assay-types
```

**assay-harness contains:**
- `HarnessAdapter` trait
- `HarnessProfile`, `SettingsLayer`, `HookContract` types
- Layered prompt builder
- Layered settings merger
- Built-in adapters (Claude Code, future: Codex, Gemini)

**assay-core gains:**
- `orchestrate/` module (DAG, merge runner, scope isolation)
- Extended worktree management (orphan detection, collision avoidance)
- Extended session management (session runner using `HarnessAdapter`)
- All orchestration logic can use `HarnessAdapter` trait directly

### Why
- **assay-harness as a building block.** Core logic naturally depends on harness abstractions — "how do I launch an agent?" is a question core must answer for orchestration.
- **Types stay pure.** No traits in assay-types.
- **Core has full access.** Session runner, DAG executor, merge runner all naturally import `HarnessAdapter` from their dependency.
- **Adapters ship with the library.** Users get Claude Code support out of the box without a separate dependency.

### Scope
- **Moderate refactoring.** Must carefully separate what's currently in `assay-core` that the harness layer would need. The evaluator subprocess logic (`assay-core/src/evaluator.rs`) might partially move to harness.
- **One new crate (assay-harness)**, inserted into the dep chain.
- **All binaries and assay-mcp automatically get harness support** through transitive dependency.

### Risks
- **Deeper dependency chain.** 4 levels means changes to assay-types trigger 3 levels of recompilation. Changes to assay-harness trigger 2 levels.
- **Where do harness types live?** If `HarnessProfile` is in assay-harness, then assay-types can't reference it. Any type that needs to be serialized alongside other assay-types (e.g., in `Config`) must live in assay-types even though the trait lives in assay-harness.
- **Built-in adapters in the middle layer** means adding a new adapter (e.g., for Cursor) requires modifying a core dependency, not a leaf crate. Feature flags could mitigate this but add complexity.
- **Existing evaluator subprocess logic in assay-core** is already a form of "agent invocation." Deciding whether to move it to assay-harness or keep it creates a split-brain.

---

## Proposal 5: "Feature-Gated Core" — Everything in assay-core Behind Feature Flags

### What
Keep the single assay-core crate but use Cargo feature flags to segment functionality. No new crates for orchestration; `assay-harness` is still a separate crate but only for adapter implementations.

```
assay-core features:
  - default = ["spec", "gate", "worktree", "session"]
  - orchestrate = ["default"]  → DAG, merge runner, session manifest
  - harness-support = []       → HarnessAdapter trait, profile types

assay-harness ──→ assay-core[harness-support] ──→ assay-types
assay-cli     ──→ assay-core[orchestrate]       ──→ assay-types
assay-mcp     ──→ assay-core[orchestrate]       ──→ assay-types
```

**Module layout:**
```
assay-core/src/
  worktree.rs              (always compiled)
  work_session.rs          (always compiled)
  #[cfg(feature = "orchestrate")]
  orchestrate/
    mod.rs
    dag.rs
    merge.rs
    manifest.rs
    scope.rs
  #[cfg(feature = "harness-support")]
  harness.rs               → HarnessAdapter trait, HarnessProfile
```

### Why
- **Zero new crates for orchestration.** Keeps workspace simple.
- **Compile time optimization.** Downstream crates that don't need orchestration (e.g., a hypothetical spec-validation-only tool) don't compile it.
- **Incremental adoption.** Can add orchestration behind a feature flag, test it thoroughly, then make it default once stable.
- **Natural for the Rust ecosystem.** tokio, serde, and many large crates use feature flags for optional modules.

### Scope
- **Minimal refactoring.** Just add new modules behind `cfg(feature)`. Existing code untouched.
- **One new crate** (`assay-harness`), small footprint.
- **Feature flag coordination.** Cargo.toml workspace deps need feature propagation.

### Risks
- **Feature flag complexity.** Feature-gated code is harder to reason about. `cargo test --all-features` vs default features can mask bugs.
- **Conditional compilation is infectious.** If `orchestrate` uses types from `harness-support`, features interact. `cfg(all(feature = "orchestrate", feature = "harness-support"))` is messy.
- **IDE experience degrades.** rust-analyzer sometimes struggles with heavily feature-gated code.
- **Still a monolith.** The organizational benefit of separate crates (clear interfaces, independent testing) is lost.

---

## Comparison Matrix

| Dimension | P1: Core Expansion | P2: Orchestrate Crate | P3: Trait-in-Types | P4: Layered Sandwich | P5: Feature Flags |
|-----------|-------------------|----------------------|-------------------|---------------------|------------------|
| New crates | 1 (harness) | 2 (orchestrate + harness) | 1 (harness) | 1 (harness, inserted) | 1 (harness) |
| Existing code changes | Low | Moderate | Low-Moderate | Moderate | Minimal |
| Compile time impact | Neutral | Slightly worse | Neutral | Worse (deeper chain) | Best (feature-gated) |
| Types purity preserved | Yes | Yes | No (trait in types) | Yes | Mostly (trait behind cfg) |
| Orchestration testability | Coupled to core | Independent | Coupled to core | Coupled to core | Coupled to core |
| Circular dep risk | High (harness ↔ core) | Low | None | None | Low |
| Cognitive complexity | Low | Moderate | Low | High | Moderate |
| Future extraction | Hard | Easy | Medium | Hard | Medium |

---

## Initial Recommendation

**Proposal 2 (Orchestration Layer) with elements of Proposal 3.**

Rationale:
- Orchestration is genuinely a separate domain from single-spec evaluation
- A dedicated crate forces clean interfaces (which the Smelt absorption needs anyway)
- The `HarnessAdapter` trait goes in `assay-types` (Proposal 3 element) to avoid the awkward three-way dependency between orchestrate/harness/core — but ONLY if we accept that assay-types gains one behavioral trait alongside its DTOs
- If the types-purity constraint is absolute, fall back to pure Proposal 2 where HarnessAdapter lives in assay-harness and orchestrate depends on harness

This is the explorer's opening position. Ready for challenger debate.
