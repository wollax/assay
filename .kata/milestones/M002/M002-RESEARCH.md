# M002: Multi-Agent Orchestration — Research

**Date:** 2026-03-16

## Summary

M002 extends the proven single-agent pipeline (`run_session`) into a multi-agent orchestrator that runs independent sessions in parallel and merges results in topological order. The core technical challenges are: (1) a DAG executor that respects inter-session dependencies while maximizing parallelism, (2) safe concurrent git worktree operations, and (3) a sequential MergeRunner that handles partial failures gracefully.

The existing codebase is well-positioned for this. `run_session` is already a self-contained, sync function that can be spawned on threads. `run_manifest` iterates sessions sequentially — the upgrade path is to replace that sequential loop with a DAG-driven scheduler. The manifest type (`RunManifest` with `[[sessions]]` array) was designed forward-compatible for multi-session (D004/R016). The MCP server already uses `tokio::task::spawn_blocking` for sync core functions, so the async surface is proven.

The primary recommendation is: **thread-pool-based DAG executor in `assay-core::orchestrate`** (feature-gated per D002), composing `Vec<WorkSession>` as `OrchestratorSession`. Use `std::thread` with a bounded pool (not tokio tasks) to match the sync-core convention (D007). The DAG is small (tens of nodes max), so a simple topological-sort + ready-queue approach is sufficient — no need for a general-purpose DAG library. Sequential merge ordering uses the existing `merge_check` as a pre-check, then adds `merge_execute` for the actual `git merge --no-ff`.

## Recommendation

Build a minimal DAG executor as a plain Rust module (`assay-core::orchestrate`) behind `cfg(feature = "orchestrate")`. The executor takes a dependency graph (adjacency list derived from manifest session declarations), maintains a ready queue of sessions whose dependencies have all completed, and dispatches ready sessions to a bounded thread pool (default: number of sessions, capped at 8). Each thread runs `run_session` — the existing pipeline function — unchanged.

This approach reuses 100% of the M001 pipeline. The new code is purely orchestration glue: dependency parsing, ready-queue management, result collection, and merge sequencing. The MergeRunner is a separate sequential pass after all sessions complete (or fail), merging completed sessions in topological order.

New MCP tools (`orchestrate_run`, `orchestrate_status`) are additive per D005. The CLI entry point is the same `assay run <manifest.toml>` — it detects multi-session manifests and routes to the orchestrator.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Topological sort | `petgraph::algo::toposort` or hand-roll (~30 lines for Kahn's algorithm) | The graph is tiny (≤20 nodes). Kahn's algorithm is 30 lines of Rust with no dependencies. Adding `petgraph` (200+ transitive deps) is overkill. Hand-roll. |
| Thread pool | `std::thread::scope` (Rust 1.63+) or `rayon::ThreadPool` | `std::thread::scope` is zero-dependency and guarantees all threads join before returning. Rayon adds a dependency for no gain at this scale. Use `std::thread::scope`. |
| Process management | `std::process::Command` (already used in `launch_agent`) | Proven pattern in M001. No change needed. |
| Git merge execution | `git merge --no-ff` via `std::process::Command` | Same shell-out pattern as all other git operations (D008). Extends `merge.rs`. |

## Existing Code and Patterns

- `crates/assay-core/src/pipeline.rs` — `run_session()` is the single-session primitive. `run_manifest()` is the sequential multi-session wrapper to be replaced by the DAG executor. `PipelineError` has stage-tagged errors with recovery guidance — extend for orchestration-level failures. `HarnessWriter` type alias shows the closure-based control inversion pattern (D001/D015).
- `crates/assay-core/src/work_session.rs` — `WorkSession` lifecycle with phase state machine. `start_session()`, `abandon_session()`, `complete_session()` are the building blocks. Recovery scan (`recover_stale_sessions`) already handles startup cleanup — will need to handle orchestrator-level recovery too.
- `crates/assay-core/src/worktree.rs` — `create()` has collision detection (rejects if spec already has active worktree). Each parallel session needs a unique spec, so collisions should not occur in valid manifests — but validation must enforce this.
- `crates/assay-core/src/merge.rs` — `merge_check()` is read-only (uses `git merge-tree --write-tree`). M002 needs `merge_execute()` that actually performs the merge. Sequential ordering matters: merge A then B, not interleaved.
- `crates/assay-types/src/manifest.rs` — `ManifestSession` needs a `depends_on: Vec<String>` field for dependency declarations. `RunManifest` needs no structural change.
- `crates/assay-mcp/src/server.rs` — `#[tool_router]` macro with `spawn_blocking` pattern. New tools follow same pattern. Currently 20 tools (19 original + run_manifest).
- `crates/assay-core/src/lib.rs` — Module registry. New `pub mod orchestrate;` goes here (feature-gated).
- `crates/assay-core/src/error.rs` — `AssayError` is `#[non_exhaustive]`, so adding variants is non-breaking. Need `OrchestratorError` variants for DAG validation, partial failure, and merge execution.

## Constraints

- **Feature gate required (D002):** `assay-core::orchestrate` must be behind `cfg(feature = "orchestrate")` for rollback safety. The `assay-cli` and `assay-mcp` crates enable the feature; `assay-types` does not need it (types are always available).
- **Zero traits (D001):** No trait objects for session dispatch. The executor uses closures/callbacks, same as `HarnessWriter`.
- **Sync core (D007):** Core orchestration logic is sync. Async surfaces (`assay-mcp`) wrap with `spawn_blocking`. The DAG executor itself runs on `std::thread::scope`, not tokio tasks.
- **Additive MCP tools only (D005/R031):** Existing 20 tools are untouched. New `orchestrate_*` tools are namespaced additions.
- **Composition not inheritance (context):** `OrchestratorSession` composes `Vec<WorkSession>`, does not extend `WorkSession`.
- **Shell out to git (D008):** Merge execution uses `git merge --no-ff` via `Command`, same as all other git ops.
- **JSON file persistence (D009):** Orchestrator state persists as JSON, consistent with session/history modules.
- **`deny_unknown_fields` on new persisted types:** Required by codebase convention for immutable-after-creation types.
- **ManifestSession must remain backward-compatible:** Adding `depends_on` field must use `#[serde(default)]` so existing single-session manifests still parse.

## Common Pitfalls

- **Concurrent git worktree creation race condition** — Two threads calling `git worktree add` simultaneously for different specs can race on the shared `.git/worktrees/` directory. Git itself handles this with lock files (`$GIT_DIR/worktrees/<name>/locked`), but error messages are confusing. **Mitigation:** Serialize worktree creation through a mutex, then parallelize only the agent execution phase. Worktree creation is fast (~100ms); agent execution is slow (minutes). The mutex costs nothing.
- **Circular dependency in manifest** — Users can declare `A depends_on B, B depends_on A`. **Mitigation:** Validate the dependency graph is a DAG at manifest load time (cycle detection during toposort). Reject with actionable error naming the cycle.
- **Partial failure cascade** — If session A fails, should dependent session B be skipped or retried? **Mitigation:** Skip dependents of failed sessions. Independent sessions continue. Report which sessions were skipped and why.
- **Merge ordering sensitivity** — Merging A then B can succeed, but B then A can conflict. **Mitigation:** Always merge in topological order. Each merge re-checks for conflicts against the updated base. If a merge introduces conflicts for a later session, that session's merge fails with a clear error.
- **Orphaned worktrees on orchestrator crash** — If the orchestrator process dies, multiple worktrees and sessions may be left in `AgentRunning`. **Mitigation:** The existing `recover_stale_sessions` sweep handles this. Ensure orchestrator sets meaningful session metadata so recovery notes are actionable.
- **Thread pool exhaustion** — With N sessions and M threads (M < N), the ready queue must be serviced as threads complete. **Mitigation:** Use `std::thread::scope` with a simple condvar-based work-stealing loop. The bounded pool prevents resource exhaustion.
- **MergeRunner vs MergeCheck confusion** — `merge_check` is read-only; `merge_execute` has side effects. **Mitigation:** Clear naming and separate functions. MergeRunner calls `merge_check` first, then `merge_execute` only if clean.

## Open Risks

- **Concurrent git operations beyond worktree add:** Even with serialized worktree creation, concurrent `git` operations in different worktrees (e.g., `git status`, `git merge-tree`) share the same `.git` object store. Git is designed for this (object store is content-addressed and append-only), but unusual edge cases (pack-file repacking during concurrent reads) could surface. Low probability but worth monitoring.
- **Agent process management at scale:** Spawning 8 concurrent `claude` processes each consuming significant memory and API quota. No current mechanism to throttle based on system resources or API rate limits. May need a concurrency limit configuration in the manifest or config.
- **ManifestSession `depends_on` referencing:** Sessions are currently identified by `spec` name. If two sessions reference the same spec (different overrides), dependency references become ambiguous. May need to require `name` for sessions with dependencies, or use array index.

## Candidate Requirements

The following are findings from research that may warrant new requirements. They are advisory — the discuss/planning phase decides which become real requirements.

| Finding | Category | Recommendation |
|---------|----------|----------------|
| Manifest dependency declaration syntax (`depends_on` field on ManifestSession) | type-extension | Likely table-stakes for R020. Add as part of manifest type evolution. |
| Cycle detection at manifest validation time | validation | Table-stakes for R020. Bad dependency graphs must fail fast with actionable errors. |
| Serialized worktree creation (mutex around `git worktree add`) | concurrency-safety | Should be explicit requirement. Race condition is real and git error messages are confusing. |
| `merge_execute` function (actual `git merge --no-ff`) | core-capability | Required for R023. Currently only `merge_check` exists (read-only). |
| Orchestrator state persistence (which sessions are running/waiting/done) | observability | Needed for R021 (`orchestrate_status` tool). JSON file under `.assay/orchestrator/`. |
| Session concurrency limit configuration | operational | Optional but valuable. Default to min(sessions, 8) with manifest override. |
| Dependent session skip-on-failure semantics | failure-visibility | Table-stakes for R020. Must be explicitly defined: skip dependents, continue independents. |

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust async/concurrency | `wshobson/agents@rust-async-patterns` (4.2K installs) | available — potentially useful for tokio patterns in MCP layer, but core is sync |
| Rust general | `oimiragieo/agent-studio@rust-expert` (45 installs) | available — generic, low relevance |
| Tokio concurrency | `geoffjay/claude-plugins@tokio-concurrency` (34 installs) | available — low relevance since core uses std::thread |

No skills are directly relevant enough to recommend installing. The core work is sync Rust with `std::thread::scope` — no specialized skill needed.

## Sources

- DAG execution patterns: Kahn's algorithm for topological sort is textbook (~30 lines). No external source needed — the graph is trivially small.
- `git merge-tree --write-tree` behavior confirmed from existing `merge.rs` implementation and git documentation.
- `std::thread::scope` stabilized in Rust 1.63 (edition 2024 is Rust 1.85+, so fully available).
- Thread-safety of git object store under concurrent access: git design documentation confirms content-addressed object store is safe for concurrent reads. Worktree lock files (`$GIT_DIR/worktrees/<name>/locked`) prevent concurrent adds to the same worktree name.
- Existing codebase patterns (D001-D016) from `.kata/DECISIONS.md`.
