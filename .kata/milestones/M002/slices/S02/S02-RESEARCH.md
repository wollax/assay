# S02: Parallel Session Executor — Research

**Date:** 2026-03-17

## Summary

S02 builds the core DAG-driven parallel executor that dispatches manifest sessions to threads via `std::thread::scope`, respecting dependency ordering from S01's `DependencyGraph`. The executor loop consumes `ready_set()` to find dispatchable sessions, launches them on bounded worker threads (default: `min(sessions, 8)`), serializes worktree creation through a `Mutex` (D018), and propagates failures via `mark_skipped_dependents()`. Each thread calls the existing `run_session()` pipeline function unchanged — the new code is purely orchestration glue.

The primary implementation challenge is the executor loop: a condvar-based dispatch loop that waits for thread completions, updates the DAG tracking sets (completed/in_flight/skipped/failed), and dispatches newly-ready sessions until all sessions are resolved. This is a classic bounded-concurrency work-stealing pattern, straightforward in ~200 lines of Rust with `std::thread::scope` + `Mutex` + `Condvar`.

New types are needed in `assay-types` for orchestrator state persistence (`OrchestratorStatus`, `SessionRunState`, `FailurePolicy`) and in `assay-core` for executor results (`SessionOutcome`, `OrchestratorResult`). State persistence to `.assay/orchestrator/<run_id>/state.json` enables `orchestrate_status` queries (S06) without holding an executor reference.

## Recommendation

Build `assay-core::orchestrate::executor` as a single public function `run_orchestrated()` that takes a `RunManifest`, `PipelineConfig`, `HarnessWriter` closure, and an `OrchestratorConfig` (concurrency limit, failure policy). The function:

1. Builds `DependencyGraph::from_manifest()` (validates DAG)
2. Generates a `run_id` (ULID)
3. Creates `.assay/orchestrator/<run_id>/` directory
4. Enters `std::thread::scope` with a dispatch loop:
   - Acquires lock on shared state (`Mutex<ExecutorState>`)
   - Calls `ready_set()` to find dispatchable sessions
   - For each ready session (up to concurrency limit): marks in_flight, spawns a scoped thread
   - Each thread: acquires worktree mutex → calls `run_session()` → records result → signals condvar
   - Main thread waits on condvar when no work is dispatchable and sessions remain
   - On failure: calls `mark_skipped_dependents()`, records skipped sessions
   - Persists state snapshot after each completion (for status queries)
5. Returns `OrchestratorResult` with per-session outcomes and timing

This matches the Smelt executor pattern but uses closures and sync primitives per Assay conventions (D001, D007, D017).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Thread scoping | `std::thread::scope` (Rust 1.63+) | Zero-dependency, guarantees all threads join before scope exits. Perfect for bounded concurrency with shared references. |
| Unique run IDs | `ulid` crate (already in workspace) | Already used for session IDs in `work_session.rs`. Consistent, sortable, no new dep. |
| Atomic JSON writes | `tempfile` + rename pattern (already in `work_session.rs`) | Proven pattern for crash-safe state persistence. Reuse directly. |
| Pipeline execution | `run_session()` from `pipeline.rs` | The existing single-session pipeline is the unit of work. No changes needed — each thread calls it as-is. |

## Existing Code and Patterns

- `crates/assay-core/src/orchestrate/dag.rs` — `DependencyGraph` with `ready_set()`, `mark_skipped_dependents()`, `topological_groups()`. S02 consumes all three. **Critical from S01 forward intelligence:** `mark_skipped_dependents()` does NOT insert the failed session itself — caller must record failure separately before calling it. Also: skipped deps count as satisfied in `ready_set()`, so the dispatch loop should call `mark_skipped_dependents()` first, then `ready_set()`.
- `crates/assay-core/src/pipeline.rs` — `run_session()` is the single-session pipeline primitive. Takes `&ManifestSession`, `&PipelineConfig`, `&HarnessWriter`. Returns `Result<PipelineResult, PipelineError>`. This is the function each worker thread calls. `PipelineConfig` contains `project_root`, `worktree_base`, `specs_dir`, `timeout_secs`, `base_branch`. All references — safe to share across threads via scoped references.
- `crates/assay-core/src/pipeline.rs` — `HarnessWriter` is `dyn Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String>`. This is `Fn` (not `FnMut`), so it's safe to share via `&HarnessWriter` across scoped threads without wrapping in a Mutex.
- `crates/assay-core/src/work_session.rs` — `save_session()` uses tempfile-then-rename atomic pattern. The orchestrator state persistence should follow the same pattern.
- `crates/assay-core/src/worktree.rs` — `create()` does collision detection internally (checks for existing worktrees with active sessions). The Mutex serialization (D018) wraps this function to prevent concurrent `git worktree add` races, not to replace its internal collision check.
- `crates/assay-types/src/manifest.rs` — `ManifestSession` has `depends_on: Vec<String>`. `RunManifest` has `sessions: Vec<ManifestSession>`. Both derive `Clone`.
- `crates/assay-core/src/error.rs` — `AssayError` is `#[non_exhaustive]` with feature-gated variants. New orchestrator error variants need `#[cfg(feature = "orchestrate")]`.

## Constraints

- **Feature gate (D002):** All new `orchestrate::executor` code must be behind `cfg(feature = "orchestrate")`.
- **Sync core (D007/D017):** Use `std::thread::scope`, not tokio tasks. The executor is a sync function.
- **Zero traits (D001):** No `trait Executor` or `trait SessionRunner`. The harness writer is a closure, and the executor is a plain function.
- **Closure-based HarnessWriter (D015):** `HarnessWriter = dyn Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String>`. It's `Fn` (shared reference), so it's thread-safe to pass as `&HarnessWriter` into scoped threads without `Arc` or `Mutex`.
- **Worktree creation serialized (D018):** Wrap `worktree::create()` calls in a `Mutex` to prevent concurrent `git worktree add` races. Agent execution (the slow part) runs outside the mutex.
- **Failure semantics (D020):** Skip dependents of failed sessions, continue independent sessions. Do not abort the entire run on a single failure (unless `FailurePolicy::Abort` is specified).
- **JSON persistence (D009/D022):** State snapshots go to `.assay/orchestrator/<run_id>/state.json`.
- **PipelineError wraps String (D016):** `PipelineError` is `Clone`, which is necessary for collecting results across threads.
- **Bounded concurrency:** Default `min(sessions, 8)`. Configurable via `OrchestratorConfig`.

## Common Pitfalls

- **Thread panic propagation in `std::thread::scope`** — If a scoped thread panics, `std::thread::scope` propagates the panic when the scope exits. `run_session()` returns `Result` and should not panic, but if the harness writer or any downstream code panics, the entire orchestrator aborts. **Mitigation:** Wrap each thread's body in `std::panic::catch_unwind()` and convert panics to `SessionOutcome::Failed`. This is defensive but important for robustness.
- **Condvar spurious wakeups** — `Condvar::wait` can return spuriously. **Mitigation:** Always re-check the termination/dispatch condition after waking. Standard practice.
- **Deadlock between worktree mutex and shared state mutex** — If the shared state lock is held while acquiring the worktree mutex (or vice versa), deadlock is possible. **Mitigation:** Never hold both locks simultaneously. The worktree mutex is acquired only inside worker threads; the shared state mutex is acquired by the dispatcher and briefly by workers to record results.
- **State persistence frequency** — Writing state.json after every single session completion could be slow with many sessions. **Mitigation:** At ≤20 sessions (typical), this is negligible (<1ms per write). Don't over-optimize.
- **`run_session` modifies session state on disk** — Each `run_session()` call creates a WorkSession, creates a worktree, and potentially abandons the session on failure. These are per-session side effects that don't interfere across threads as long as worktree creation is serialized. The only shared resource is the `.assay/sessions/` directory, and `save_session()` uses atomic writes with unique filenames (ULID-based), so concurrent session creation is safe.
- **ManifestSession borrowing** — `run_session()` takes `&ManifestSession`. Since `std::thread::scope` allows borrowing from the enclosing scope, we can pass `&manifest.sessions[idx]` directly — no cloning needed.

## Open Risks

- **`run_session()` uses hardcoded "claude" binary name in `launch_agent()`** — For testing the executor, we need mock harness writers that don't actually spawn `claude`. The existing `HarnessWriter` closure pattern allows this, but `run_session()` still calls `launch_agent()` internally which spawns `claude`. Unit tests for the executor should test the dispatch/scheduling logic with a mock session runner, not the full pipeline. This means the executor should accept a session runner function (not `run_session` directly) to enable unit testing. However, keeping the `run_session` call hardcoded in the public API and providing an internal test seam is the pragmatic path.
- **`run_session()` is not easily mockable** — It's a free function that calls worktree::create, launch_agent, etc. The executor tests need to verify scheduling/ordering/failure behavior without running real agents. **Mitigation:** Accept a generic session runner closure `Fn(&ManifestSession, &PipelineConfig, &HarnessWriter) -> Result<PipelineResult, PipelineError>` as a parameter, defaulting to `run_session` in production. This is consistent with D001 (closures not traits) and enables clean unit testing.

## New Types Needed

### In `assay-types/src/orchestrate.rs` (new file)

```rust
/// Run state of a single session in the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionRunState {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Failure policy for orchestrated runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailurePolicy {
    /// Skip dependents of failed sessions, continue independent sessions.
    SkipDependents,
    /// Abort the entire run on first failure.
    Abort,
}

/// Per-session status snapshot for observability.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionStatus {
    pub name: String,
    pub spec: String,
    pub state: SessionRunState,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_secs: Option<f64>,
    pub error: Option<String>,
    pub skip_reason: Option<String>,
}

/// Orchestrator-level status snapshot, persisted for status queries.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrchestratorStatus {
    pub run_id: String,
    pub phase: OrchestratorPhase,
    pub failure_policy: FailurePolicy,
    pub sessions: Vec<SessionStatus>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrchestratorPhase {
    Running,
    Completed,
    PartialFailure,
    Aborted,
}
```

### In `assay-core/src/orchestrate/executor.rs` (new file)

```rust
/// Outcome of a single orchestrated session.
pub enum SessionOutcome {
    Completed {
        result: PipelineResult,
        worktree_path: PathBuf,
        branch_name: String,
        changed_files: Vec<String>,
    },
    Failed {
        error: PipelineError,
        stage: PipelineStage,
    },
    Skipped {
        reason: String,
    },
}

/// Configuration for orchestrated runs.
pub struct OrchestratorConfig {
    pub pipeline: PipelineConfig,
    pub max_concurrency: usize,
    pub failure_policy: FailurePolicy,
}

/// Result of an orchestrated run.
pub struct OrchestratorResult {
    pub run_id: String,
    pub outcomes: Vec<(String, SessionOutcome)>, // (session_name, outcome)
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub failure_policy: FailurePolicy,
}
```

## Executor Loop Pseudocode

```
fn run_orchestrated(manifest, config, harness_writer, session_runner) -> OrchestratorResult:
    graph = DependencyGraph::from_manifest(manifest)?
    run_id = Ulid::new()
    create .assay/orchestrator/<run_id>/
    
    shared_state = Mutex::new(ExecutorState { completed, in_flight, skipped, failed, outcomes })
    worktree_lock = Mutex::new(())
    condvar = Condvar::new()
    
    thread::scope(|s| {
        loop {
            let state = shared_state.lock()
            if all_resolved(state): break
            
            let ready = graph.ready_set(&state.completed, &state.in_flight, &state.skipped)
            let available_slots = config.max_concurrency - state.in_flight.len()
            let batch = ready[..min(ready.len(), available_slots)]
            
            if batch.is_empty() && !state.in_flight.is_empty():
                // Wait for a thread to complete
                condvar.wait(state)
                continue
            
            for idx in batch:
                state.in_flight.insert(idx)
                s.spawn(move || {
                    // Serialize worktree creation
                    let _lock = worktree_lock.lock()
                    // worktree::create happens inside run_session, but the mutex
                    // ensures only one thread is in the create phase at a time
                    // Actually: run_session does create internally, so we need the
                    // session_runner to be called while holding the worktree lock
                    // for the create portion only. 
                    //
                    // Better approach: split into create + execute, or accept that
                    // the entire run_session is serialized during the create phase.
                    // Since create is ~100ms and execution is minutes, hold the lock
                    // during the entire run_session call but release after create.
                    //
                    // Cleanest: pass worktree_lock into a wrapper that locks during
                    // worktree creation only.
                    
                    let result = catch_unwind(|| session_runner(&session, &config.pipeline, harness_writer))
                    
                    let mut state = shared_state.lock()
                    state.in_flight.remove(idx)
                    match result {
                        Ok(Ok(pipeline_result)) => state.completed.insert(idx); record outcome
                        Ok(Err(pipeline_error)) => {
                            state.failed.insert(idx)
                            graph.mark_skipped_dependents(idx, &mut state.skipped)
                            if config.failure_policy == Abort: mark all remaining as cancelled
                        }
                        Err(panic) => same as failure
                    }
                    persist_state_snapshot()
                    condvar.notify_all()
                })
        }
    })
    
    return OrchestratorResult { run_id, outcomes, timing }
```

### Worktree Mutex Refinement

The key design question is how to serialize only the worktree creation phase while letting agent execution run in parallel. `run_session()` does both — it calls `worktree::create()` internally in stage 2, then proceeds to agent launch in stage 4.

**Option A: Hold the worktree mutex for the entire `run_session()` call.** This serializes everything, losing parallelism during agent execution. Unacceptable.

**Option B: Accept a worktree lock in the session runner.** The executor passes a `&Mutex<()>` to the session runner closure, which acquires it around the worktree creation call. This requires a modified session runner signature or a wrapper.

**Option C (recommended): Use a two-phase approach.** The executor itself calls `worktree::create()` under the mutex, then calls a reduced session runner that takes an already-created worktree. This cleanly separates the serialized phase (create) from the parallel phase (harness + agent + gate + merge).

However, Option C requires refactoring `run_session()` into two parts, which is more invasive. **Pragmatic choice: Option B with a thin wrapper.** The executor provides a `SessionRunner` closure that wraps `run_session()` by acquiring the worktree mutex before calling it and releasing after the WorktreeCreate stage completes. But `run_session()` doesn't expose per-stage hooks.

**Final approach:** Accept that worktree creation (~100ms) serialized via a mutex around the worktree `create()` call — but this requires the executor to call `worktree::create()` before `run_session()`. Since `run_session()` also calls `worktree::create()` internally, this would double-create.

**Simplest correct approach:** The session runner closure wraps the entire `run_session()` call. The worktree mutex is held for the full call duration of the worktree create step only — but since we can't hook into `run_session()`'s internals, we accept holding the worktree lock for the entire call. At this scale (≤8 concurrent sessions, with ~100ms for create vs minutes for execution), the brief serialization during create is acceptable because git's own lock files on `.git/worktrees/` would serialize it anyway. The mutex prevents confusing error messages from concurrent git access.

**Revised simplest approach:** Don't hold the mutex during `run_session()`. Instead, serialize only the worktree creation calls. The executor should split the pipeline into: (1) worktree create (serialized), (2) rest of pipeline (parallel). This means we need a `run_session_with_worktree()` variant or split the function. Given the executor is new code, creating a two-phase pipeline function is clean and correct:

- Phase 1 (serialized): `worktree::create()` + `work_session::start_session()` — under worktree mutex
- Phase 2 (parallel): harness config + agent launch + gate evaluate + merge check — no mutex

This is a small refactor: extract stages 1-2 of `run_session()` into a setup function, and stages 3-6 into an execute function. The executor calls setup under the mutex, then spawns execute in parallel.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | none relevant | No skills needed — std::thread::scope, Mutex, Condvar are stdlib |

No external skills needed. The work is standard Rust concurrency with `std::thread::scope`.

## Sources

- `std::thread::scope` documentation — guarantees all spawned threads join before the scope exits, allowing borrowed references. Available since Rust 1.63.
- Smelt's `orchestrate/executor.rs` (referenced in roadmap) — uses `tokio::task::JoinSet` and `trait ConflictHandler`. Assay ports the same ready_set/dispatch loop pattern but with sync primitives and closures.
- S01 forward intelligence — `mark_skipped_dependents()` does NOT insert the failed session; `ready_set()` treats skipped as satisfied; all returns are sorted for determinism.
