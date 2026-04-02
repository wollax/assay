# Architecture Research — Smelt v0.1.0

**Dimension:** Architecture
**Date:** 2026-03-09
**Scope:** Multi-agent worktree orchestration system design for the v0.1.0 PoC milestone.

---

## 1. Component Boundaries

Smelt v0.1.0 decomposes into five components. Each has a single responsibility and a defined interface boundary.

### 1.1 Orchestrator (core)

**Responsibility:** Top-level coordination. Accepts a plan (which sessions to run, on what branches, with what inputs), executes the plan by delegating to other components, and drives the merge pipeline when sessions complete.

**Owns:**
- Plan execution lifecycle (init → run sessions → await completion → merge → report)
- Session scheduling (parallel vs sequential, dependency ordering)
- Error escalation and retry policy
- The "run" as a first-class entity with ID, state, and result

**Does not own:** Git operations, session internals, merge strategy, conflict resolution logic.

**Interface:** The orchestrator is the entry point. In v0.1.0 this is a CLI command (`smelt run`). Internally it coordinates the other four components through direct function calls (no IPC, no message bus).

**Analogues:** Tekton's `PipelineRun` controller, Argo Workflows' workflow controller, Concourse's `scheduler`. All CI/CD orchestrators separate "what to run" from "how to run it" — the orchestrator is the "what."

### 1.2 Worktree Manager

**Responsibility:** Git worktree lifecycle — creation, configuration, health checking, and cleanup. Abstracts `git worktree` operations behind a clean interface.

**Owns:**
- `git worktree add` / `git worktree remove` lifecycle
- Branch creation for each worktree (naming convention: `smelt/<run-id>/<session-id>`)
- Worktree path management (deterministic paths under a configurable root)
- Lock management (`git worktree lock/unlock`) for crash recovery
- Cleanup on failure (no orphaned worktrees)

**Key constraints from git worktree:**
- Each worktree must be on a different branch — two worktrees cannot check out the same branch simultaneously.
- Worktrees share the object store but have independent index/HEAD — this is the isolation model.
- The main worktree's `.git` directory is shared; linked worktrees get a `.git` file pointing back to it.
- Worktree paths must not be nested inside each other or the main worktree.
- `git worktree list --porcelain` provides machine-readable state for health checks.

**Interface:**

```
WorktreeManager:
  create(runId, sessionId, baseBranch) → WorktreeHandle
  remove(handle) → void
  list() → WorktreeHandle[]
  health(handle) → WorktreeHealth
  cleanup(runId) → void   // remove all worktrees for a run
```

`WorktreeHandle` carries: path, branch name, session ID, creation timestamp.

### 1.3 Session Controller

**Responsibility:** Agent session lifecycle within a worktree. Starts a session, monitors it, captures its output, and reports completion.

**Owns:**
- Starting an agent session (real or simulated) in a specific worktree
- Session process lifecycle (spawn, monitor, kill, timeout)
- Output capture (stdout/stderr streaming, structured result extraction)
- Session state machine: `pending → running → completed | failed | timed_out`
- Exit signal: determining *what changed* in the worktree after the session completes

**Two session backends (v0.1.0):**

1. **Real agent session:** Spawns Claude Code (or another agent) as a subprocess pointed at the worktree directory. The agent operates on the worktree's working tree. Session controller monitors the process and captures exit status.

2. **Simulated/scripted session:** Executes a script (shell script, Node script, etc.) that makes changes to the worktree. Used for development, testing, and deterministic replay. Same interface as a real session — the orchestrator doesn't know the difference.

**Interface:**

```
SessionController:
  start(handle: WorktreeHandle, config: SessionConfig) → Session
  status(session: Session) → SessionState
  cancel(session: Session) → void
  result(session: Session) → SessionResult

SessionConfig:
  type: "agent" | "script"
  command: string          // agent command or script path
  args: string[]
  timeout: Duration
  env: Record<string, string>

SessionResult:
  state: "completed" | "failed" | "timed_out"
  exitCode: number
  changedFiles: string[]   // from git diff --name-only
  commitSha: string | null // if session committed
  duration: Duration
```

**Design decision — commit strategy:** Sessions should commit their work to their worktree branch before completion. This gives the merge orchestrator clean commit SHAs to work with. If a session doesn't commit (e.g., it crashes), the session controller should auto-commit any staged/unstaged changes as a recovery mechanism, clearly marked as `smelt: auto-committed uncommitted work from session <id>`.

### 1.4 Merge Orchestrator

**Responsibility:** Combining work from multiple session worktrees into a single target branch. This is the core value proposition of Smelt.

**Owns:**
- Merge strategy selection and execution
- Ordering of merges (which session's work gets merged first)
- Conflict detection
- Delegating conflict resolution to the Conflict Resolver
- Final merged branch state verification

**Merge strategy (v0.1.0 — sequential octopus):**

Rather than attempting a single octopus merge (which fails entirely on any conflict), merge session branches sequentially into the target branch:

```
target ← session-1 (merge or fast-forward)
target ← session-2 (merge, may conflict)
target ← session-3 (merge, may conflict)
```

This is the same strategy CI/CD merge queues (GitHub merge queue, Bors, Mergify) use — sequential integration with conflict detection at each step.

**Why sequential over octopus:**
- Octopus merge (`git merge A B C`) aborts entirely if *any* conflict exists. No partial progress.
- Sequential merge isolates conflicts to the specific pair of branches that conflict.
- Merge order can be deterministic (by session completion time) or priority-based.
- If session-2 conflicts with session-1's merged result, only that pair needs resolution. Session-3 may merge cleanly.

**Interface:**

```
MergeOrchestrator:
  merge(sessions: SessionResult[], targetBranch: string) → MergeResult

MergeResult:
  status: "clean" | "conflicts_resolved" | "conflicts_unresolved" | "failed"
  targetSha: string
  mergedSessions: string[]       // session IDs successfully merged
  conflictingSessions: string[]  // session IDs that had conflicts
  resolutions: ConflictResolution[]
```

### 1.5 Conflict Resolver

**Responsibility:** Resolving merge conflicts, either automatically (AI-assisted) or by escalating to a human.

**Owns:**
- Parsing conflict markers from git merge output
- Assembling context for AI resolution (both sides of the conflict, surrounding code, session descriptions)
- Invoking AI to propose a resolution
- Applying the resolution and verifying the result compiles/parses
- Human fallback: presenting the conflict for manual resolution when AI confidence is low or AI resolution fails verification

**Resolution pipeline:**

```
Conflict detected
  → Parse conflict hunks (git diff with conflict markers)
  → For each conflicted file:
      → Extract ours/theirs/base content
      → Gather context: file purpose, session descriptions, surrounding code
      → Ask AI for resolution with confidence score
      → If confidence >= threshold:
          → Apply resolution
          → Verify (syntax check at minimum)
          → If verify passes: accept
          → If verify fails: fall through to human
      → If confidence < threshold:
          → Fall through to human
  → If any conflicts remain: prompt human via CLI interactive mode
  → Return resolution result
```

**Interface:**

```
ConflictResolver:
  resolve(conflict: MergeConflict, context: ResolutionContext) → Resolution

MergeConflict:
  file: string
  base: string
  ours: string
  theirs: string
  oursSession: string   // which session produced "ours"
  theirsSession: string // which session produced "theirs"

ResolutionContext:
  sessionDescriptions: Record<string, string>
  fileHistory: string[]  // recent commits touching this file

Resolution:
  status: "ai_resolved" | "human_resolved" | "unresolved"
  content: string
  confidence: number
  explanation: string
```

---

## 2. Data Flow

### 2.1 Happy Path (no conflicts)

```
User invokes `smelt run` with plan
  │
  ▼
Orchestrator validates plan, generates run ID
  │
  ▼
Worktree Manager creates N worktrees (one per session)
  │  branch: smelt/<run-id>/session-1
  │  branch: smelt/<run-id>/session-2
  │  ... all branched from the same base commit
  │
  ▼
Session Controller starts N sessions in parallel
  │  Each session works in its own worktree
  │  Sessions are independent — no inter-session communication
  │
  ▼
Session Controller reports completion for each session
  │  Captures: changed files, commit SHA, exit status
  │
  ▼
Merge Orchestrator merges session branches sequentially into target
  │  target ← session-1 (fast-forward or merge commit)
  │  target ← session-2 (merge commit)
  │
  ▼
Orchestrator reports result
  │  "Run <id> complete: 2 sessions merged cleanly into <target-branch>"
  │
  ▼
Worktree Manager cleans up all session worktrees
```

### 2.2 Conflict Path

```
... (same as above through session completion)
  │
  ▼
Merge Orchestrator: target ← session-2 produces conflicts
  │
  ▼
Conflict Resolver receives conflicted files + context
  │
  ├─→ AI proposes resolution (high confidence)
  │     → Apply, verify syntax → Accept
  │     → Merge continues with session-3
  │
  └─→ AI proposes resolution (low confidence) OR verification fails
        → Present to human via CLI
        → Human resolves interactively
        → Merge continues with session-3
```

### 2.3 State Storage (git-native)

All coordination state lives in git. No database, no files outside the repo.

| State | Storage Mechanism |
|-------|-------------------|
| Run metadata | `.smelt/runs/<run-id>/run.json` (committed to a smelt-internal branch or written to the main worktree) |
| Session branches | `smelt/<run-id>/<session-id>` branch refs |
| Session results | Captured in-memory during execution; persisted to `.smelt/runs/<run-id>/sessions/<session-id>.json` |
| Merge result | The target branch itself — its HEAD commit *is* the result |
| Conflict resolutions | Committed as merge commits with structured commit messages documenting the resolution |

**Open question for implementation:** Should `.smelt/` state files be committed to the target branch, stored on a separate `smelt/meta` branch, or kept as local-only files? For v0.1.0, local-only files are simplest. Git-committed state becomes important for multi-machine coordination in future milestones.

---

## 3. Architectural Patterns

### 3.1 Supervisor Pattern (from Erlang/OTP, adopted by orchestrators)

The orchestrator is a supervisor that owns the lifecycle of its children (sessions). If a session fails, the supervisor decides whether to retry, skip, or abort the entire run. This is the same pattern Kubernetes controllers, Erlang supervisors, and CI/CD pipeline controllers use.

**For v0.1.0:** Simple policy — if a session fails, mark it as failed and continue merging the successful sessions. Report the failure in the run result. No automatic retry.

### 3.2 Branch-per-Unit-of-Work (from merge queues, feature branch workflows)

Each session gets its own branch. Branches are cheap, isolated, and the native git unit of parallel work. This is the same pattern GitHub merge queues, Bors-ng, and GitLab merge trains use — each unit of work is isolated on a branch, merged sequentially into the target.

**Key insight from merge queue systems:** The merge order matters. Merge queues typically use FIFO (first-ready, first-merged). Smelt can use the same — sessions that complete first get merged first. This gives a natural priority to faster sessions and means the merge orchestrator doesn't need to wait for all sessions to complete before starting to merge.

### 3.3 Pipes and Filters (conflict resolution pipeline)

Conflict resolution is a pipeline: parse → contextualize → resolve (AI) → verify → fallback (human). Each step has a clear input/output contract. This makes it easy to add new resolution strategies (e.g., rule-based resolution for known conflict patterns) without changing the pipeline structure.

### 3.4 Strategy Pattern (session backends, merge strategies)

Both session execution and merge behavior use the strategy pattern:
- `SessionBackend`: "agent" or "script" — same interface, different implementations
- `MergeStrategy`: "sequential" for v0.1.0, extensible to "octopus", "rebase", or custom strategies later

### 3.5 Handle Pattern (worktree references)

Worktrees are referenced by opaque handles, not raw paths. The handle carries metadata (session ID, branch, state) and ensures the worktree manager controls the lifecycle. Direct path access is available via the handle but callers don't construct paths themselves.

---

## 4. Precedent Analysis

### 4.1 CI/CD Orchestrators

**Tekton (Kubernetes-native CI/CD):**
- Separates `Task` (unit of work) from `Pipeline` (composition of tasks) from `PipelineRun` (execution instance).
- Smelt parallel: `Session` ≈ Task, `Plan` ≈ Pipeline, `Run` ≈ PipelineRun.
- Tekton's controller watches for PipelineRun resources and reconciles them. Smelt's orchestrator is the equivalent — it watches session state and drives the next step.

**Argo Workflows:**
- DAG-based workflow execution on Kubernetes.
- Each workflow step runs in a container with a shared volume for artifact passing.
- Smelt parallel: Each session runs in a worktree (instead of a container) with the git repo as the shared artifact store.

**Concourse CI:**
- Resources (inputs/outputs), tasks (execution), and jobs (composition).
- Resources are versioned — Concourse tracks which version of each resource was used.
- Smelt parallel: The base commit is the "resource version." Each session branch is a derived version. The merge result is the composed version.

**Key takeaway from CI/CD:** The separation of *plan* (what to do), *execution* (doing it), and *reconciliation* (combining results) is universal. Smelt should follow this three-phase structure.

### 4.2 Git-Based Coordination Tools

**Bors-ng / merge queues:**
- Sequential merge testing — each PR is rebased/merged onto the target, tested, then integrated.
- If a merge-and-test fails, the PR is rejected and the next one is tried.
- Smelt's sequential merge strategy directly mirrors this.

**git-bug / git-appraise:**
- Store issue/review data in git refs (not files in the working tree).
- Use `refs/bugs/*` or `refs/notes/*` namespaces.
- Validates that git can store arbitrary coordination data without polluting the working tree.
- Smelt could use `refs/smelt/*` for run metadata in future milestones.

**git-branchless:**
- Tracks commit "stacks" and their relationships.
- Uses a hidden SQLite database for performance, with git as the source of truth.
- Lesson: Pure git storage is correct but slow for complex queries. A local cache/index is acceptable as long as git remains authoritative.

### 4.3 Multi-Agent Systems

**CrewAI / AutoGen / LangGraph:**
- Agent coordination through message passing and shared state.
- Typically use a central "manager" agent that delegates tasks and synthesizes results.
- Smelt's orchestrator fills the manager role, but coordination is through git state (branches, commits) rather than message passing.

**Key difference:** Multi-agent frameworks assume agents communicate during execution. Smelt's v0.1.0 model is *embarrassingly parallel* — sessions are independent, with no inter-session communication. Coordination happens only at the merge phase. This is simpler, more robust, and maps naturally to git's branching model.

---

## 5. Build Order

Based on component dependencies, the recommended build order is:

### Phase 1: Foundation (Worktree Manager + Session Controller scaffolding)

**Build first:** Worktree Manager

No other component can be tested without worktrees. The worktree manager is a pure git abstraction with no dependency on other Smelt components. It can be built and tested in isolation with unit tests that create/remove worktrees.

**Build second:** Session Controller (script backend only)

The scripted session backend enables all downstream testing without requiring a real AI agent. A script that writes files and commits is sufficient to produce the inputs the merge orchestrator needs. Build the "agent" backend later.

**Deliverable:** Can create worktrees, run scripts in them, and report what changed.

### Phase 2: Merge Pipeline (Merge Orchestrator + basic Conflict Resolver)

**Build third:** Merge Orchestrator (conflict-free path)

Implement the sequential merge strategy for the happy path (no conflicts). This requires session results as input and produces a merged branch. Test with scripted sessions that touch disjoint files.

**Build fourth:** Conflict Resolver (AI-assisted + human fallback)

Add conflict handling to the merge orchestrator. Start with the human fallback (interactive CLI resolution), then add AI-assisted resolution on top. This order ensures the system always works — AI is an optimization, human resolution is the safety net.

**Deliverable:** Can merge multiple session branches, resolve conflicts (AI or human), and produce a single branch.

### Phase 3: Orchestrator + Real Sessions

**Build fifth:** Orchestrator (plan execution)

Wire everything together. The orchestrator drives the full lifecycle: create worktrees → start sessions → await completion → merge → cleanup. This is mostly coordination logic — the hard work is in the components it delegates to.

**Build sixth:** Session Controller (real agent backend)

Add the Claude Code agent backend. This is the riskiest component (subprocess management, output parsing, timeout handling for a long-running AI process) but by this point the entire pipeline is testable with scripts, so the agent backend only needs to conform to the existing interface.

**Deliverable:** Full v0.1.0 — coordinate real agent sessions, merge their outputs, resolve conflicts.

### Dependency Graph

```
Phase 1                    Phase 2                    Phase 3
─────────────────────────────────────────────────────────────────
Worktree Manager ──┐
                   ├──→ Merge Orchestrator ──┐
Session Controller ┘    (clean merges)       │
(script backend)         │                   ├──→ Orchestrator
                         ▼                   │
                   Conflict Resolver ────────┘
                   (human, then AI)          │
                                             ▼
                                       Session Controller
                                       (agent backend)
```

### Rationale

1. **Worktree Manager first** — everything depends on it, it depends on nothing.
2. **Script sessions before agent sessions** — enables testing the entire pipeline without real AI costs or complexity. This is the "simulated session" requirement from the milestone.
3. **Clean merges before conflict resolution** — the happy path is the common case and proves the core value proposition. Conflict resolution is an enhancement.
4. **Human fallback before AI resolution** — human resolution always works. AI resolution is an optimization. Build the safety net first.
5. **Orchestrator late** — it's glue code. Building it too early means constant refactoring as component interfaces evolve. Building it after components are stable means it wires together proven interfaces.
6. **Real agent backend last** — highest risk, highest complexity, but the entire system is testable without it. When it's added, it only needs to match an already-proven interface.

---

## 6. Open Questions for Implementation

| Question | Options | Recommendation |
|----------|---------|----------------|
| **Language** | Rust (Assay alignment), TypeScript (git lib ecosystem, rapid prototyping), C# (developer comfort) | Assess git library quality in each ecosystem as the deciding factor. `libgit2` bindings exist for all three. TypeScript's `simple-git` (wrapper around git CLI) may be more practical than libgit2 for worktree operations. |
| **Git interaction model** | Shell out to `git` CLI vs libgit2 bindings | Shell out for v0.1.0. The git CLI is the most complete and well-tested interface. libgit2 has gaps in worktree support. Wrap shell calls behind the Worktree Manager interface so the implementation can be swapped later. |
| **State persistence** | Local files only vs git-committed vs git refs | Local files (`.smelt/runs/`) for v0.1.0. Revisit for multi-machine coordination. |
| **Merge target branch** | Create new branch vs merge into existing | Create a new `smelt/result/<run-id>` branch. Never mutate the user's branches without explicit instruction. |
| **AI for conflict resolution** | In-process LLM call vs spawn agent | In-process API call (e.g., Anthropic API directly). Spawning an agent for conflict resolution is overkill — this is a structured prompt with code context, not an open-ended coding task. |
| **Session completion detection** | Process exit vs file sentinel vs git hook | Process exit for subprocess-based sessions. The session controller spawns the process and `await`s its exit. Polling or sentinels add unnecessary complexity. |

---

## 7. Summary

Smelt v0.1.0 is five components with clear boundaries:

| Component | Depends On | Risk Level |
|-----------|-----------|------------|
| Worktree Manager | git CLI | Low — well-understood git operations |
| Session Controller (script) | Worktree Manager | Low — subprocess management |
| Merge Orchestrator | Session results | Medium — merge semantics, edge cases |
| Conflict Resolver | Merge conflicts | Medium-High — AI integration, UX for human fallback |
| Orchestrator | All of the above | Low — glue code |
| Session Controller (agent) | Worktree Manager | High — real AI subprocess management |

The architecture follows established patterns from CI/CD orchestrators (supervisor, branch-per-unit-of-work, sequential merge) adapted to the multi-agent context. The build order is driven by dependencies and risk — foundation first, integration last, with scripted sessions enabling full-pipeline testing before real agents are introduced.
