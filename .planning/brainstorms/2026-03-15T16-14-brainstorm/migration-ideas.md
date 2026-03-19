# Migration & Risk Mitigation Strategies

## Codebase Analysis Summary

**Assay's current abstractions:**
- **Worktree** (`worktree.rs`): Basic CRUD — create/list/status/cleanup. Worktrees are keyed by `spec_slug`, branches follow `assay/<slug>` convention. Metadata is minimal: `{base_branch, spec_slug}`. No awareness of sessions, no orphan detection, no collision prevention.
- **WorkSession** (`work_session.rs`): Linear state machine `Created → AgentRunning → GateEvaluated → Completed` (+ Abandoned escape). Stores `worktree_path` as a passive PathBuf — no bidirectional link. ULID-based IDs. Gate runs linked by string IDs.
- **AgentSession** (`gate/session.rs`): In-memory accumulate-then-commit pattern for gate evaluations. Distinct from WorkSession — this is ephemeral, WorkSession is persistent.
- **MCP Server** (`server.rs`): 18+ tools exposed. Tools reference worktree/session by slug/ID. Parameter structs use `schemars` for schema generation.

**Key gaps Smelt fills:**
1. Worktree↔session lifecycle binding (Assay has no link from worktree to owning session)
2. Orphan detection (worktrees without sessions, sessions pointing at removed worktrees)
3. Collision prevention (two sessions targeting the same worktree)
4. Session-aware cleanup (cleanup worktree → abandon session)
5. Multi-session orchestration (DAG executor, N concurrent sessions)
6. Merge workflow (sequential merge, AI conflict resolution, human fallback)

---

## Strategy 1: Adapter-First (Façade Pattern)

### What
1. **Phase A** — Create an `assay-smelt` adapter crate that imports Smelt's worktree manager and session runner as dependencies. The adapter implements Assay's existing `worktree::*` and `work_session::*` function signatures by delegating to Smelt internally.
2. **Phase B** — Wire the adapter behind a feature flag (`smelt-backend`). Default compilation uses the current code. When the flag is on, the adapter replaces the implementations.
3. **Phase C** — Once the adapter passes all 836 tests, flip the default. Remove the old code paths.
4. **Phase D** — Inline the adapter (move Smelt logic directly into `assay-core`), removing the intermediate crate.

### Why
- Zero disruption to MCP tools — the function signatures and return types don't change until Phase D.
- Feature flag means both code paths coexist and can be A/B tested.
- The adapter acts as a translation layer for Smelt's richer `SessionManifest` → Assay's simpler `WorkSession`.
- If Smelt's abstractions turn out to be incompatible, we can revert by disabling the flag.

### Scope
- **Effort**: Medium-High. Writing the adapter is significant work, but it's isolated.
- **Blast radius**: Minimal per phase. Each phase is independently releasable.

### Risks
- The adapter may paper over semantic differences (e.g., Smelt sessions have DAG-aware lifecycle, Assay's is linear). Long-lived adapters accumulate translation debt.
- Feature flag combinatorics: testing both paths doubles CI time.
- Smelt's types may not map cleanly to `assay-types` — especially if Smelt uses different serde representations.

---

## Strategy 2: Bottom-Up Type Unification

### What
1. **Phase A** — Unify the worktree metadata. Extend `WorktreeMetadata` with optional fields: `session_id: Option<String>`, `created_at: Option<DateTime<Utc>>`, `collision_lock: Option<String>`. These are all additive, backward-compatible with existing worktree.json files.
2. **Phase B** — Extend `WorkSession` with Smelt's lifecycle fields: `merge_status`, `parent_session_id` (for DAG), `scope_checks: Vec<ScopeViolation>`. WorkSession already tolerates unknown fields (`no deny_unknown_fields`), so old sessions deserialize fine.
3. **Phase C** — Add new `assay-core` functions alongside existing ones: `worktree::create_with_session()`, `worktree::detect_orphans()`, `worktree::check_collision()`. Existing functions remain unchanged.
4. **Phase D** — Add MCP tools (`worktree_create_session`, `session_orchestrate`) that use the new functions. Existing tools keep working.
5. **Phase E** — Deprecate old worktree-only creation path in favor of session-bound creation.

### Why
- Types are the foundation. Getting them right means all higher layers compose naturally.
- Additive changes to types are the safest possible migration — nothing breaks, new capabilities appear incrementally.
- Assay already designed `WorkSession` for forward-compatible evolution (the comment on line 127 of work_session.rs explicitly calls this out).
- MCP backward compatibility is guaranteed because old tools keep their exact signatures.

### Scope
- **Effort**: Medium. Each phase is small. Type changes are cheap.
- **Blast radius**: Very small per phase. No existing code is modified until Phase E.

### Risks
- Optional fields create "stringly typed" coupling — `session_id` in metadata must match an actual session. No compile-time guarantee.
- Incremental growth may lead to a "God struct" WorkSession with too many optional fields.
- Without a clear cutover, both old and new paths may persist indefinitely.

---

## Strategy 3: Parallel Session Model (Shadow Mode)

### What
1. **Phase A** — Introduce `OrchestratedSession` as a new type in `assay-types` alongside `WorkSession`. It encapsulates Smelt's richer model: DAG dependencies, merge strategy, scope isolation config, multi-worktree binding.
2. **Phase B** — Add an `assay-core::orchestrator` module that manages `OrchestratedSession` lifecycle. It uses the existing `worktree::*` functions for git operations but adds session-binding, orphan detection, and collision prevention on top.
3. **Phase C** — New MCP tools (`orchestrate_*` namespace) expose the orchestrator. Existing `session_*` and `worktree_*` tools are untouched.
4. **Phase D** — Add a migration function: `OrchestratedSession::from_work_session(ws: &WorkSession)` to upgrade in-flight sessions.
5. **Phase E** — Once stable, deprecate `WorkSession` tools and alias them to orchestrated equivalents.

### Why
- Clean separation means the new model can evolve independently without breaking existing workflows.
- Agents already using `session_create`/`session_update` keep working. New agents can opt into the richer orchestrator.
- The migration function (Phase D) provides a safe upgrade path for in-flight sessions.
- This mirrors how Assay already separates `AgentSession` (ephemeral gate eval) from `WorkSession` (persistent lifecycle) — adding a third "orchestrated" layer follows the same pattern.

### Scope
- **Effort**: High. New type, new module, new MCP tools — this is the most code.
- **Blast radius**: Near-zero for existing code. All new code, all additive.

### Risks
- Three session types (`AgentSession`, `WorkSession`, `OrchestratedSession`) is confusing. Naming matters.
- The migration function may not handle edge cases (sessions mid-transition, sessions with linked gate runs).
- If the deprecation never happens, you maintain two parallel session systems indefinitely.

---

## Strategy 4: Worktree-First, Session Follows

### What
1. **Phase A** — Absorb Smelt's worktree enhancements directly into `assay-core::worktree`: add `detect_orphans()`, `check_collision(session_id)`, and `bind_session(worktree_path, session_id)`. These are pure additions — no existing function changes.
2. **Phase B** — Update `worktree::create()` to accept an optional `session_id` parameter. When provided, it writes the session binding into metadata. When `None`, behavior is identical to today.
3. **Phase C** — Update `worktree::cleanup()` to check for session bindings and transition bound sessions to Abandoned.
4. **Phase D** — Layer merge capabilities: `worktree::merge_check()` and `worktree::merge_propose()` as new functions that combine ahead/behind status with Smelt's merge strategy.
5. **Phase E** — Extend the existing `session_create` MCP tool to auto-create a bound worktree (combining two current tool calls into one). Keep the two-step path working for backward compatibility.

### Why
- Worktree is the simpler, more stable abstraction. Starting there means each change is small and testable.
- Session binding is the critical missing piece — once worktrees know their session, orphan detection and collision prevention fall out naturally.
- `WorktreeMetadata` is already Assay-owned JSON, so extending it is safe.
- Merge capabilities layer cleanly on top of the existing `status()` function that already computes ahead/behind.

### Scope
- **Effort**: Medium. Incremental additions to an existing module.
- **Blast radius**: Low. Only cleanup() gets modified behavior (Phase C), and only when a session binding exists.

### Risks
- Worktree-centric view may not capture session-level concerns well (e.g., DAG dependencies between sessions are not worktree-level concepts).
- Phase C changes cleanup behavior — existing code that calls `cleanup()` may not expect session transitions as a side effect.
- The merge layer (Phase D) is complex and may not fit cleanly as worktree-level functions.

---

## Strategy 5: Contract-First MCP Evolution

### What
1. **Phase A** — Define the target MCP tool contracts first (input schemas, output shapes, error codes) for all new capabilities: `session_orchestrate`, `worktree_bind`, `merge_check`, `merge_propose`, `scope_check`. Write them as schema-only stubs that return "not implemented" errors.
2. **Phase B** — Implement the stubs one by one, each behind a feature flag. Each implementation pulls in the necessary Smelt logic.
3. **Phase C** — For existing tools that need enhanced behavior (e.g., `worktree_create` gaining session awareness), add optional parameters. Old calls without the parameter work identically.
4. **Phase D** — Add integration tests that exercise the new tools via the MCP protocol (not just unit tests).
5. **Phase E** — Remove stubs and feature flags. All tools are live.

### Why
- MCP backward compatibility is the #1 risk. Starting from the contract ensures agents never break.
- Schema-first design means the team (and agents consuming the tools) can review the API surface before implementation begins.
- Feature flags mean partially-implemented tools don't leak into production.
- Existing tools gain capabilities via additive optional parameters — the safest possible evolution for MCP schemas.

### Scope
- **Effort**: Medium. Schema design is fast; implementation follows the existing MCP server patterns.
- **Blast radius**: Zero until feature flags are enabled. Stubs are inert.

### Risks
- Schema-first can lead to over-design if the contracts are defined before the domain model is understood.
- Feature flag per-tool is fine-grained but creates many flags to manage.
- Stubs that linger too long become technical debt (agents may code against "not implemented" and not notice).

---

## Cross-Cutting Recommendations

1. **Manifest format**: Define a unified `SessionManifest` in `assay-types` that works for both single-session (current WorkSession) and multi-session (Smelt's DAG) use cases. Use `#[serde(flatten)]` for backward-compatible embedding.
2. **Test strategy**: Run existing 836 tests as a regression gate at every phase boundary. Add Smelt-specific tests in parallel, not as replacements.
3. **MCP versioning**: Consider adding `api_version` to the MCP server's `ServerInfo` so agents can detect capabilities.
4. **Plugin/hook audit**: Before any behavioral change, enumerate all plugins/hooks that depend on worktree/session events and verify they'll survive the migration.
