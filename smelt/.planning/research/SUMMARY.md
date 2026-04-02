# Research Summary — Smelt v0.1.0 Orchestration PoC

**Date:** 2026-03-09

---

## 1. Executive Summary

Smelt is a multi-agent orchestration tool that uses git worktrees as isolation boundaries. Each agent session operates in its own worktree on a dedicated branch, and the orchestrator merges their outputs into a single target branch, resolving conflicts via AI-assisted resolution with human fallback. The product sits in the workflow orchestration space (analogous to CI/CD merge queues) but adapted for AI coding agents rather than human-authored PRs. The v0.1.0 milestone is a proof of concept: demonstrate that the create-worktrees, run-agents, merge-branches loop works end-to-end.

The recommended stack is **Rust**, primarily for ecosystem alignment with Assay (shared types via `serde` structs) and single-binary distribution. The git library landscape is fragmented across all languages — no library fully covers worktree management and merge operations — so a hybrid approach (shell-out to `git` CLI for writes, library for reads) is the pragmatic design regardless of language choice. TypeScript with `simple-git` has the strongest git operations story but introduces type drift with Assay and a runtime dependency.

The critical risks center on three areas: (1) **process lifecycle management** — zombie agents, orphaned worktrees, and signal propagation are day-one concerns that must be addressed in the foundation; (2) **semantic conflicts that merge cleanly** — git-level merge success does not imply code correctness, making post-merge verification essential even for the PoC; and (3) **AI conflict resolution quality** — LLM-resolved conflicts may introduce subtle bugs, requiring mandatory human review and post-resolution verification in v0.1.0. The architecture mitigates these through a five-component design with clear boundaries and a build order that prioritizes testability (scripted sessions before real agents, human fallback before AI resolution).

---

## 2. Key Findings

### Stack (STACK.md)

- **No git library in any language fully covers Smelt's core operations** (worktree add/remove + merge with conflict markers). Shell-out to the `git` CLI is required in every language. The design wraps this behind a `SmeltGitOps` trait so the implementation can migrate to pure-library calls as libraries mature.
- **Rust is recommended** for Assay alignment (shared types), single-binary distribution, and `tokio`-based async process orchestration. The velocity tradeoff is real but manageable with tight PoC scope and `anyhow` for error handling.
- **TypeScript is the runner-up** — `simple-git` is the only library across all languages that wraps `git worktree` operations directly. It would be the pick if Assay integration and binary distribution were not factors.
- All candidate languages (Rust, C#, TypeScript, Go) have excellent process orchestration and CLI framework stories. The git library quality is the differentiating axis.
- Key crates: `tokio` (async runtime), `clap` (CLI), `serde`/`serde_json` (Assay types), `anyhow` (errors), `tracing` (logging), `gix` (git reads where possible).

### Features (FEATURES.md)

- **7 table-stakes features** define the minimum viable orchestration loop: worktree lifecycle (TS-1), session manifest (TS-2), agent launcher (TS-3), sequential merge (TS-4), AI conflict resolution (TS-5), human fallback (TS-6), and git-native state (TS-7).
- **5 differentiators** separate Smelt from "just run agents in terminals": task graph (D-1), merge order intelligence (D-2), session output summary (D-3), scope isolation verification (D-4), and dry-run/simulation mode (D-5).
- **9 anti-features** are explicitly excluded: container isolation, workflow SDK, forge integration, Assay gate integration, distributed coordination, cost tracking, daemon mode, semantic/AST merge, and agent adapter abstraction.
- The critical path is linear: TS-1 through TS-6. TS-7 (git-native state) is a cross-cutting constraint.
- Highest-value differentiators for least effort: D-3 (session summary) and D-5 (simulation mode).

### Architecture (ARCHITECTURE.md)

- **Five components** with single responsibilities: Orchestrator (coordination), Worktree Manager (git worktree lifecycle), Session Controller (agent process lifecycle), Merge Orchestrator (branch merging), and Conflict Resolver (AI + human resolution).
- **Sequential merge strategy** (not octopus) — mirrors CI/CD merge queue patterns. Octopus merge aborts entirely on any conflict; sequential merge isolates conflicts to specific branch pairs.
- **Build order driven by dependencies and risk**: Worktree Manager first (everything depends on it), then scripted sessions (enables full-pipeline testing), then clean merges, then conflict resolution, then the top-level orchestrator, and real agent sessions last (highest risk, proven interface by that point).
- **Centralized orchestrator using git as the state layer** — no database, no message queue. Session state stored in `.smelt/` files, branches as work units, commits as completion signals.
- The architecture follows established patterns: supervisor (Erlang/OTP), branch-per-unit-of-work (merge queues), pipes-and-filters (conflict resolution), strategy pattern (session backends, merge strategies).

### Pitfalls (PITFALLS.md)

- **3 critical-severity pitfalls**: zombie agent processes (2.1), semantic conflicts that merge cleanly (3.1), and HEAD/index state corruption from wrong worktree context (4.1).
- **8 high-severity pitfalls** spanning worktree lifecycle (orphaned worktrees, branch collisions, gc contention), process management (signal handling), and merge operations (order-dependent outcomes, AI resolution bugs, ref races, state recovery).
- **Process group management is essential** — agents must be spawned in separate process groups so `kill(-pgid, SIGTERM)` cleans up entire process trees on crash.
- **Git gc must be disabled during orchestration** (`gc.auto 0`) to prevent object store corruption while agents are mid-commit.
- **Simulated sessions must be adversarial** — testing only with clean, successful simulated sessions will mask every real-world failure mode.
- All but 1 pitfall (submodule interactions) are tagged for v0.1.0 attention.

---

## 3. Implications for Roadmap

### Suggested Phase Structure

**Phase 1: Foundation** (Worktree Manager + Scripted Sessions)

Rationale: Every other component depends on worktree management, and scripted sessions enable full-pipeline testing without real AI costs. This phase also addresses the majority of critical pitfalls (orphaned worktrees, branch collisions, HEAD/index corruption, zombie processes, signal handling).

Deliverables:
- `WorktreeManager` with create/remove/list/cleanup/health
- `SessionController` with script backend (configurable: commit count, file patterns, conflict production, failure modes)
- Startup reconciliation (orphan detection, PID liveness checks)
- Signal handler for graceful shutdown
- Process group management for spawned sessions

**Phase 2: Merge Pipeline** (Merge Orchestrator + Conflict Resolution)

Rationale: The merge pipeline is the core value proposition. Building it after sessions exist means real inputs are available for testing. Human fallback is built before AI resolution (safety net first, optimization second).

Deliverables:
- `MergeOrchestrator` with sequential merge strategy
- Deterministic merge ordering (by completion time or configurable)
- `ConflictResolver` with human fallback (CLI interactive)
- AI-assisted resolution with confidence threshold and mandatory review
- Post-merge verification hook point (build/test command)
- Serialized merge operations (no concurrent merges)

**Phase 3: Orchestrator Integration + Real Agents**

Rationale: The top-level orchestrator is glue code — building it after components stabilize avoids constant refactoring. Real agent sessions are added last, slotting into a proven interface.

Deliverables:
- `Orchestrator` driving the full lifecycle (plan, execute, merge, report)
- Session output summary (D-3)
- Claude Code agent backend for `SessionController`
- Dry-run / simulation mode (D-5)
- Basic state journal for crash recovery

### Research Flags

- **Version verification required**: All crate versions in STACK.md are from training data (May 2025 cutoff). Run `cargo search` to confirm before `Cargo.toml` is finalized.
- **Claude Code headless mode**: The exact CLI flags and behavior for non-interactive Claude Code sessions need verification. The session controller design assumes `--print` or equivalent.
- **`gix` worktree support maturity**: The research notes `gix` has partial worktree support. Verify current status before deciding which read operations use `gix` vs shell-out.
- **macOS process death signals**: Linux has `PR_SET_PDEATHSIG`; macOS equivalent (`kqueue EVFILT_PROC`) needs implementation verification.

---

## 4. Confidence Assessment

| Area | Confidence | Notes |
|------|-----------|-------|
| **Architecture** | High | Five-component design follows proven patterns (CI/CD orchestrators, merge queues). Build order is well-justified by dependency analysis. |
| **Stack choice (Rust)** | Medium-High | Correct for ecosystem alignment with Assay. The velocity tradeoff is the main risk — mitigated by tight scope and shell-out-heavy git strategy. Would revisit if Assay alignment becomes less important. |
| **Git operations strategy** | High | Shell-out behind a trait is pragmatic and well-understood. No language has a library that eliminates this need. |
| **Feature scope** | High | Table stakes vs differentiator vs anti-feature classification is clear. The critical path (TS-1 through TS-6) is minimal and well-ordered. |
| **Pitfall catalog** | Medium-High | Comprehensive for known failure modes. Gaps exist around real-world Claude Code behavior (output format, error modes, timing) since no live testing was done. |
| **Merge strategy** | Medium | Sequential merge is the right starting point, but merge order effects on 3+ agent scenarios need empirical validation. The interaction between AI resolution quality and merge ordering is underexplored. |
| **AI conflict resolution** | Medium | The pipeline design is sound (parse, resolve, verify, fallback), but resolution quality depends on prompt engineering that has not been prototyped. Mandatory human review in v0.1.0 is the correct hedge. |
| **Crate versions** | Low | All versions are from training data. Must verify before use. |

---

## 5. Gaps to Address

1. **Claude Code integration specifics** — Exact CLI flags for headless/non-interactive mode, output format, exit codes, and error behavior. This directly affects `SessionController` design. Needs hands-on testing.

2. **AI conflict resolution prompt engineering** — The pipeline structure is defined but the actual prompts for conflict resolution are not. Resolution quality will depend heavily on context assembly (how much surrounding code, session descriptions, file history). Needs prototyping before Phase 2 is finalized.

3. **`gix` crate current state** — The research notes `gix` has partial worktree support and no merge engine, but this was based on May 2025 knowledge. Given gitoxide's rapid development, the current state may differ. Verify before finalizing the git operations layer.

4. **Empirical merge order analysis** — The research identifies merge order as a high-severity concern but does not provide data on how often order-dependent conflicts arise in practice. A small experiment with real codebases would inform whether merge order intelligence (D-2) should be promoted to table stakes.

5. **macOS process management** — `PR_SET_PDEATHSIG` (child-dies-with-parent) is Linux-only. The macOS equivalent needs research and implementation verification. This is critical for the zombie process pitfall.

6. **Post-merge verification** — The architecture includes a "hook point" for build/test verification, but the design of this hook (what commands to run, how to interpret results, retry behavior) is unspecified. This is the primary defense against semantic conflicts (pitfall 3.1, rated critical).

7. **Concurrent agent resource profiling** — The pitfall catalog flags resource exhaustion but does not provide data on actual resource consumption per Claude Code session. Profiling 2-3 concurrent sessions on target hardware would inform the default concurrency limit.

8. **Web search validation** — All four research files note that WebSearch, Context7, and Ref MCP tools were unavailable. A follow-up pass should check for new multi-agent orchestration tools released in late 2025/early 2026, and verify the current state of libraries mentioned in STACK.md.

---

*Synthesized from: STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md*
*Research date: 2026-03-09*
