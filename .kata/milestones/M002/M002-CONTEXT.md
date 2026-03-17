# M002: Multi-Agent Orchestration — Context

**Gathered:** 2026-03-16
**Status:** Future milestone — detail planning deferred until M001 completes

## Project Description

M002 extends the single-agent harness from M001 into a multi-agent orchestration system. Multiple agents work in parallel on independent specs, with dependency-aware scheduling (DAG executor) and sequential merge ordering.

## Why This Milestone

Single-agent execution (M001) proves the pipeline works. Real-world projects have multiple specs that can be worked on concurrently by different agents. M002 enables this — a manifest declares multiple sessions with dependencies, and Assay orchestrates them: launching agents in parallel where possible, waiting for dependencies, and merging results in the correct order.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Write a multi-session manifest with dependency declarations between sessions
- Have Assay launch multiple agents in parallel on independent specs
- See orchestration status (which agents are running, waiting, completed)
- Have completed sessions merged sequentially in dependency order

### Entry point / environment

- Entry point: `assay run <manifest.toml>` (same as M001, but manifest has multiple `[[sessions]]`)
- Environment: local dev
- Live dependencies involved: `git` CLI, `claude` CLI, potentially multiple concurrent agent processes

## Completion Class

- Contract complete means: DAG executor correctly orders sessions, parallel launch works, merge ordering is correct
- Integration complete means: multiple real agents work concurrently on different specs in different worktrees
- Operational complete means: handles agent failures mid-orchestration, partial completion, and recovery

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- A multi-session manifest with dependencies launches agents in correct order (parallel where independent, sequential where dependent)
- If one agent fails, dependent sessions are skipped but independent sessions continue
- Completed sessions merge in topological order without conflicts

## Risks and Unknowns

- Concurrent git operations (multiple worktrees being created/merged) may have race conditions
- Process management for N concurrent agents is more complex than single-agent
- MergeRunner sequential ordering needs careful handling of partial failures

## Existing Codebase / Prior Art

- Everything from M001 (harness, pipeline, worktree enhancements)
- `crates/assay-core/src/work_session.rs` — WorkSession lifecycle (will be composed into OrchestratorSession)
- Brainstorm architecture report recommends `assay-core::orchestrate` module with feature gate

> See `.kata/DECISIONS.md` for all architectural and pattern decisions.

## Relevant Requirements

- R020: Multi-agent orchestration (OrchestratorSession, DAG executor)
- R021: orchestrate_* MCP tools
- R022: Harness orchestration layer (scope enforcement, multi-agent prompts)
- R023: MergeRunner with sequential merge

## Scope

### In Scope

- `OrchestratorSession` composing `Vec<WorkSession>`
- DAG executor for dependency-aware session scheduling
- Parallel agent launching
- MergeRunner with sequential merge ordering
- `orchestrate_*` MCP tools (additive)
- Harness orchestration layer (scope enforcement per session)
- `assay-core::orchestrate` module (feature-gated with `cfg(feature = "orchestrate")`)

### Out of Scope / Non-Goals

- AI conflict resolution (M003)
- Additional harness adapters (M003)
- SessionCore type unification (M003)

## Technical Constraints

- Feature-gated module: `cfg(feature = "orchestrate")` on `assay-core::orchestrate`
- OrchestratorSession composes `Vec<WorkSession>` — does not extend WorkSession
- New MCP tools are additive — existing tools unchanged
- Closures/callbacks for orchestration control, not traits

## Integration Points

- M001's pipeline as the single-session primitive
- WorkSession lifecycle from assay-core
- Worktree module for parallel worktree management
- Merge module for sequential merge execution
- MCP server for orchestrate_* tools

## Open Questions

- Should the DAG executor be sync (thread pool) or async (tokio tasks)? — Decide during M002 planning based on M001 experience
- How should partial orchestration failure be represented to the user? — Design during M002 discuss phase
