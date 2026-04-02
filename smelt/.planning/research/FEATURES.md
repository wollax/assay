# FEATURES — v0.1.0 Orchestration PoC

Research dimension: Features
Milestone: v0.1.0 — Multi-agent worktree orchestration proof of concept
Date: 2026-03-09

---

## How Multi-Agent Orchestration Typically Works

### Session Coordination Patterns

Multi-agent orchestration systems generally follow one of three coordination models:

1. **Centralized orchestrator** — A coordinator process owns the task graph, assigns work to agents, monitors progress, and collects outputs. This is the dominant pattern in workflow engines (Temporal, Airflow, Prefect) and the natural fit for Smelt. The orchestrator holds the session manifest (who is working on what, in which worktree) and drives the lifecycle.

2. **Blackboard / shared-state** — Agents read from and write to a shared data store. Coordination is implicit: agents pick up tasks when preconditions are met. Git itself can serve as this blackboard (branches as task slots, commits as completion signals). Projects like git-bug and git-notes demonstrate using git's object store and ref namespace as a coordination substrate without external databases.

3. **Message-passing / event-driven** — Agents communicate through events. More common in distributed systems (Kafka, NATS). Overkill for single-machine orchestration and explicitly out of scope for v0.1.0.

For Smelt v0.1.0, the **centralized orchestrator using git as the state layer** is the right model. The orchestrator creates worktrees, launches agent sessions, monitors completion, and drives the merge.

### Git Worktree Mechanics

`git worktree` allows multiple working trees attached to a single repository. Key behaviors relevant to orchestration:

- **Shared object store** — All worktrees share `.git/objects`. Commits made in one worktree are immediately visible to others via refs. This is the coordination primitive: an agent commits in its worktree, and the orchestrator can read that commit from the main working tree.
- **Branch exclusivity** — A branch can only be checked out in one worktree at a time. Each agent session needs its own branch. The orchestrator must manage branch naming/assignment.
- **Lock files** — Git uses lock files in `.git/worktrees/<name>/` to prevent concurrent operations on the same worktree. Smelt must respect these and not run multiple agents in the same worktree.
- **Pruning** — `git worktree prune` cleans up stale worktree references. Smelt must handle cleanup on session completion or failure.
- **Performance** — Worktree creation is near-instant (no network, no clone). This is a significant advantage over container-based isolation for the PoC.

### Merge Strategies in Existing Tools

Git's built-in merge strategies relevant to multi-agent output merging:

- **Recursive/ort (default)** — Three-way merge. Works well when agents modify different files. This should be the default path.
- **Octopus** — Merges multiple branches simultaneously. Useful when 3+ agents complete work and none conflict. Git refuses octopus merges when conflicts exist, so it serves as a fast path for the clean case.
- **Sequential merge** — Merge agent branches one at a time into a target branch. Order matters when agents touch overlapping files. The orchestrator can order merges by scope (smaller changes first) or by dependency.
- **Rebase-then-merge** — Rebase each agent branch onto the accumulating target before merging. Produces linear history but requires conflict resolution at rebase time.

For the PoC, **sequential merge with fallback to AI-assisted conflict resolution** is the pragmatic choice. Octopus merge as a fast path for the clean case is a nice optimization but not required.

### Conflict Resolution Approaches

Existing approaches to automated/AI-assisted conflict resolution:

- **Semantic merge tools** (SemanticMerge, now part of PlasticSCM/Unity) — Parse code into ASTs and merge at the semantic level rather than text level. Handles structural changes (method moves, renames) that text-based merge cannot. Complex to implement, language-specific.
- **AI-powered merge** — Feed both sides of a conflict plus surrounding context to an LLM and ask it to produce the resolved version. This is the approach Smelt should take. The key insight: the orchestrator already knows what each agent was trying to do (it assigned the tasks), so it can provide richer context than a generic merge tool.
- **git rerere** (reuse recorded resolution) — Git caches conflict resolutions and auto-applies them if the same conflict recurs. Useful for repeated merge patterns but not for novel conflicts. Smelt could maintain its own resolution cache over time.
- **Human fallback** — When automated resolution fails or confidence is low, present the conflict to a human. The standard escape hatch. The quality of the human experience (clear diff presentation, context about what each agent intended) is what differentiates good tools from bad ones.

### Existing Tools and What They Provide

**Axon (axon-core/axon)** — K8s controller for Claude Code in ephemeral Pods. Single-agent-per-task, single-repo. No orchestration, no merge, no conflict resolution. Validates the market but is architecturally different (container-centric vs worktree-centric).

**git-bug** — Distributed bug tracker using git's object store (custom refs under `refs/bugs/`). Proves that git can be a coordination substrate: no external database, offline-capable, mergeable state. Relevant pattern: using custom refs to store orchestration metadata.

**git-notes** — Git's built-in mechanism for attaching metadata to commits without modifying them. Could be used for session metadata, merge decisions, conflict resolution records. Stored under `refs/notes/` and travel with push/fetch.

**OpenDevin / SWE-agent / Aider** — AI coding agents that operate on codebases. All single-session. None provide multi-session orchestration. They are potential "agent backends" for Smelt, not competitors to the orchestration layer.

**Claude Code** — CLI-based AI coding agent. Supports headless mode (`--print` flag), can be scripted, reads/writes files. The primary agent target for Smelt v0.1.0. Key integration points: launching sessions with prompts, monitoring for completion, reading output.

---

## Feature Catalog — v0.1.0

### Table Stakes (Must Have for Orchestration PoC)

These are the minimum features required to prove the orchestration thesis. Without any one of these, the PoC does not demonstrate coordinated multi-agent work.

#### TS-1: Worktree Lifecycle Management

Create, track, and clean up git worktrees for agent sessions.

- Create a worktree per agent session with a dedicated branch
- Track worktree state (created, active, completed, failed, cleaned)
- Clean up worktrees on session completion or failure
- Handle abnormal termination (agent crash, SIGKILL) gracefully

**Complexity:** Low-medium. Git worktree operations are simple; the state tracking and error handling add complexity.
**Dependencies:** None — foundational primitive.

#### TS-2: Session Manifest and Assignment

The orchestrator's model of what work is being done and by whom.

- Define a set of tasks (initially: prompt + target files/scope)
- Assign tasks to sessions, each mapped to a worktree
- Track session state (pending, running, completed, failed)
- Support minimum 2 concurrent sessions (target: N)

**Complexity:** Medium. The data model is straightforward; the lifecycle management and error states add complexity.
**Dependencies:** TS-1 (worktree lifecycle).

#### TS-3: Agent Session Launcher

Start agent sessions in worktrees. Support both real agents and scripted/simulated sessions.

- Launch Claude Code in headless mode in a worktree directory
- Launch scripted sessions (shell scripts that make predefined changes) for testing
- Capture session output (stdout/stderr) for debugging
- Detect session completion (process exit)

**Complexity:** Medium. Claude Code's headless mode is straightforward; robust process management (timeouts, output capture, failure detection) adds work.
**Dependencies:** TS-1, TS-2.

#### TS-4: Sequential Branch Merge

Merge completed agent branches into a single target branch.

- Create a merge target branch from the base commit
- Merge each completed agent branch sequentially
- Detect and classify merge results: clean merge, conflict, or failure
- Produce a merged branch that contains all agents' work (when conflict-free)

**Complexity:** Medium. Git merge operations are well-understood; handling edge cases (empty commits, divergent histories, partial failures) adds complexity.
**Dependencies:** TS-1, TS-2, TS-3 (needs completed sessions to merge).

#### TS-5: AI-Assisted Conflict Resolution

When merges conflict, use an AI agent to resolve them.

- Detect conflicts from merge output
- Extract conflict context: both sides, common ancestor, surrounding code
- Present conflict to an LLM with context about what each agent intended
- Apply the resolved version and continue the merge
- Track resolution confidence and decisions made

**Complexity:** High. The conflict extraction is mechanical; the quality of AI resolution depends on context presentation, prompt engineering, and knowing when to give up.
**Dependencies:** TS-4 (merge must produce conflicts to resolve).

#### TS-6: Human Fallback Escalation

When AI resolution fails or confidence is too low, escalate to a human.

- Define a confidence threshold for AI resolution
- When threshold is not met, pause the merge and present the conflict to the user
- Accept human resolution input (initially: CLI-based, user edits the file)
- Resume merge after human resolution
- Log the escalation and resolution for future reference

**Complexity:** Medium. The CLI interaction model is simple; the merge pause/resume lifecycle adds state management complexity.
**Dependencies:** TS-5 (AI resolution must fail/be uncertain to trigger fallback).

#### TS-7: Git-Native State (No External Dependencies)

All orchestration state stored in git — no database, no message queue.

- Session manifest stored as files in the repo (e.g., `.smelt/sessions/`)
- Merge decisions and conflict resolutions recorded in git
- State is inspectable with standard git tools (`git log`, `git show`)
- State survives process restart (orchestrator can resume from git state)

**Complexity:** Medium. File-based state is simple; making it robust (atomic writes, crash recovery, concurrent access) takes care.
**Dependencies:** Cuts across all features — this is a constraint, not a feature to build separately.

---

### Differentiators (Competitive Advantage Over Manual Agent Usage)

These features make Smelt meaningfully better than "I'll just run agents in separate terminals and merge by hand." They are strongly recommended for v0.1.0 but could be descoped if timeline pressure demands it.

#### D-1: Orchestration Plan / Task Graph

Define the set of tasks, their dependencies, and assignment strategy before execution begins.

- Declarative task definition (what each agent should work on)
- Dependency ordering (agent B starts after agent A completes)
- Parallel fan-out for independent tasks
- Plan validation before execution (detect impossible assignments, circular deps)

**Complexity:** Medium. Simple DAG representation. The value is in making the orchestration intentional rather than ad-hoc.
**Dependencies:** TS-2 (session manifest).
**Why differentiating:** Manual agent usage has no plan — you just start agents and hope. A plan makes orchestration reproducible and debuggable.

#### D-2: Merge Order Intelligence

Choose the optimal order to merge agent branches to minimize conflicts.

- Analyze file change overlap between agent branches before merging
- Merge non-overlapping changes first (guaranteed clean)
- Order overlapping merges by scope (smaller changeset first)
- Report expected conflict zones before attempting merge

**Complexity:** Medium. File-level overlap analysis is cheap (`git diff --stat`); the ordering heuristic is simple but valuable.
**Dependencies:** TS-4 (sequential merge).
**Why differentiating:** Manual merging is typically done in arbitrary order. Smart ordering can eliminate conflicts that arise purely from merge order.

#### D-3: Session Output Summary

After all sessions complete, produce a structured summary of what happened.

- Per-session: files changed, lines added/removed, duration, success/failure
- Aggregate: total changes, conflict count, resolution method (AI/human), merge result
- Machine-readable output (JSON) for downstream tooling
- Human-readable output (terminal) for operator awareness

**Complexity:** Low. Data collection is straightforward; formatting is polish work.
**Dependencies:** TS-2, TS-3, TS-4.
**Why differentiating:** Running agents manually gives you no aggregate view. Understanding "what just happened across all sessions" is the orchestration value proposition.

#### D-4: Scope Isolation Verification

Verify that each agent stayed within its assigned scope.

- Compare agent's actual file changes against its assigned scope
- Flag out-of-scope modifications (files the agent was not expected to touch)
- Optionally reject out-of-scope changes before merging
- Report scope violations in the session summary

**Complexity:** Low-medium. File-level scope checking is simple; deciding what to do about violations requires policy decisions.
**Dependencies:** TS-2 (session manifest defines scope), TS-3 (session output to check).
**Why differentiating:** When running agents manually, there is no concept of scope — agents change whatever they want. Scope isolation is a prerequisite for trusting parallel agent work.

#### D-5: Dry-Run / Simulation Mode

Run the full orchestration pipeline with scripted agents to validate the plan and merge strategy without invoking real AI agents.

- Execute the orchestration plan with scripted sessions that apply predefined changes
- Exercise the full merge pipeline including conflict detection and resolution
- Validate that the orchestration machinery works before spending agent tokens
- Enable repeatable testing of the orchestration layer itself

**Complexity:** Low (if TS-3 already supports scripted sessions). The simulation mode is a composition of existing primitives.
**Dependencies:** TS-3 (scripted session support).
**Why differentiating:** No manual workflow supports simulation. This makes orchestration development and testing dramatically faster.

---

### Anti-Features (Deliberately NOT Building in v0.1.0)

These are features that might seem natural to include but would add scope, complexity, or wrong-level abstraction for the PoC. Each has a clear rationale for exclusion.

#### AF-1: Container/Docker Isolation

**What it is:** Running agent sessions in Docker containers rather than bare worktrees.
**Why not:** The brainstorm explored Docker-based isolation, but the milestone scope pivoted to worktrees as the coordination primitive. Container isolation adds significant complexity (image management, volume mounting, credential handling) without advancing the orchestration thesis. Worktrees provide sufficient isolation for the PoC. Container isolation can be layered on later as an execution backend.

#### AF-2: Workflow SDK / Workflow-as-Code

**What it is:** A typed SDK (C#, Rust, etc.) for defining orchestration workflows programmatically.
**Why not:** The v0.1.0 goal is proving the orchestration loop works, not building a developer-facing SDK. The PoC can use configuration files or CLI arguments. The SDK is a v1.0 concern, informed by what works in the PoC.

#### AF-3: Forge Integration (PR Creation, Issue Comments)

**What it is:** Automatically creating PRs, commenting on issues, or posting status updates to GitHub/ADO/GitLab.
**Why not:** Explicitly deferred in the milestone definition. The PoC produces a merged branch. What happens to that branch (PR, deploy, etc.) is out of scope. Forge integration is a v0.2+ feature.

#### AF-4: Assay Integration (Quality Gates)

**What it is:** Reading Assay gate run records to inform merge/reject/retry decisions.
**Why not:** Explicitly deferred. The PoC merges agent outputs based on git-level conflict analysis, not quality gate results. Assay integration adds a dependency on Assay's output format and requires decision logic that is premature for the PoC.

#### AF-5: Multi-Machine / Distributed Coordination

**What it is:** Coordinating agent sessions across multiple machines via git push/pull.
**Why not:** Explicitly out of scope. Single-machine orchestration must work first. Git's architecture supports this extension naturally (push/fetch to synchronize state), but the complexity of distributed coordination (network failures, partial state, clock skew) is inappropriate for a PoC.

#### AF-6: Cost/Token Tracking

**What it is:** Tracking token usage and costs across orchestrated sessions.
**Why not:** Explicitly deferred. The PoC focuses on coordination and merge correctness. Cost tracking requires provider-specific token counting, which is a separate concern. The session output summary (D-3) can record duration and basic metrics without full cost attribution.

#### AF-7: Persistent Orchestration Daemon

**What it is:** A long-running process that watches for triggers and automatically starts orchestration runs.
**Why not:** The PoC is invoked explicitly (CLI command). A daemon requires process management, logging infrastructure, and trigger configuration — all v1.0 concerns.

#### AF-8: Semantic / AST-Level Merge

**What it is:** Parsing code into ASTs and merging at the semantic level rather than text level.
**Why not:** Language-specific, complex to implement, and not required for the PoC. Git's text-based merge handles the common case. AI-assisted resolution (TS-5) handles the conflict case. Semantic merge is a potential optimization for a future version.

#### AF-9: Agent Adapter Abstraction

**What it is:** A generic interface that supports multiple agent backends (Claude Code, Codex, Aider, etc.).
**Why not:** Premature abstraction from a sample size of 1. Build for Claude Code directly. Extract the interface when adding agent #2, informed by real observed differences. This aligns with the brainstorm's "no premature abstractions" principle.

---

## Feature Dependencies

```
TS-1 Worktree Lifecycle
 ├── TS-2 Session Manifest ──── D-1 Task Graph
 │    ├── TS-3 Agent Launcher ── D-4 Scope Isolation
 │    │    └── D-5 Simulation Mode
 │    └── D-3 Session Summary
 └── TS-4 Sequential Merge ──── D-2 Merge Order Intelligence
      └── TS-5 AI Conflict Resolution
           └── TS-6 Human Fallback

TS-7 Git-Native State (cross-cutting constraint on all features)
```

## Complexity Summary

| ID | Feature | Complexity | Category |
|----|---------|-----------|----------|
| TS-1 | Worktree Lifecycle | Low-Medium | Table Stakes |
| TS-2 | Session Manifest | Medium | Table Stakes |
| TS-3 | Agent Session Launcher | Medium | Table Stakes |
| TS-4 | Sequential Branch Merge | Medium | Table Stakes |
| TS-5 | AI Conflict Resolution | High | Table Stakes |
| TS-6 | Human Fallback | Medium | Table Stakes |
| TS-7 | Git-Native State | Medium | Table Stakes |
| D-1 | Task Graph | Medium | Differentiator |
| D-2 | Merge Order Intelligence | Medium | Differentiator |
| D-3 | Session Output Summary | Low | Differentiator |
| D-4 | Scope Isolation Verification | Low-Medium | Differentiator |
| D-5 | Dry-Run / Simulation Mode | Low | Differentiator |

## Critical Path

The minimum viable orchestration loop:

```
TS-1 → TS-2 → TS-3 → TS-4 → TS-5 → TS-6
```

Worktree management, session assignment, agent launch, merge, conflict resolution, human fallback. This is the end-to-end path that proves the thesis.

TS-7 (git-native state) is a cross-cutting concern that should inform implementation decisions throughout, not a feature built in isolation.

D-3 (session summary) and D-5 (simulation mode) are the highest-value differentiators for the least effort and should be included if timeline permits. D-2 (merge order intelligence) and D-4 (scope isolation) provide meaningful quality improvements. D-1 (task graph) is the most complex differentiator and could be simplified to a flat task list for the PoC.

---

## Research Limitations

WebSearch, Ref MCP, and Context7 tools were unavailable during this research. The analysis is based on:
- Project context (PROJECT.md, brainstorm reports, STATE.md)
- Domain knowledge of git internals (`git worktree`, merge strategies, `git-notes`, `git-bug` patterns)
- Domain knowledge of multi-agent orchestration patterns (Temporal, Airflow, centralized vs. blackboard vs. message-passing)
- Domain knowledge of AI-assisted conflict resolution approaches
- Knowledge of existing tools (Axon, SemanticMerge, SWE-agent, OpenDevin, Aider, Claude Code)

A follow-up research pass with web search enabled would be valuable for:
- Discovering new multi-agent coding tools released in late 2025/early 2026
- Finding specific git worktree coordination libraries and their APIs
- Reviewing recent papers on AI-assisted merge conflict resolution
- Checking Axon's current feature set (last known: v0.4.0)
