# High-Value Feature Proposals for Assay v0.3.0

**Explorer:** explorer-highvalue
**Date:** 2026-03-08
**Context:** v0.2.0 shipped (dual-track gates, run history, enforcement, diagnostics, guard daemon). 23,385 LOC, 493 tests. Next step: move toward orchestration vision.

---

## Proposal 1: Worktree Manager — Git Worktree Lifecycle Engine

### What

A `worktree` module in `assay-core` that manages git worktree creation, isolation, and cleanup for agent work. Provides:

- `worktree::create(spec_name, base_branch) -> WorktreeHandle` — creates `.assay/worktrees/<spec-slug>/` with feature branch
- `worktree::cleanup(handle)` — removes worktree and optionally deletes branch
- `worktree::list()` — inventory of active worktrees with metadata (spec, branch, age, disk usage)
- `worktree::status(handle)` — git status/diff summary for a worktree
- CLI: `assay worktree create/list/status/cleanup`
- MCP: `worktree_create`, `worktree_status`, `worktree_cleanup` tools

### Why

**This is THE foundation for everything else in the vision.** Without worktree isolation, there's no multi-agent story, no merge-back workflow, no concurrent sessions. Every other v0.3 feature depends on "agents work in isolated worktrees." It's also the simplest piece of the orchestrator to build independently — pure git operations, no agent abstraction needed, testable in isolation.

The worktree manager also immediately improves single-agent workflows: an agent can create a worktree for a spec, work in isolation, run gates against the worktree, and the human can inspect/merge at leisure. This delivers value even without the full orchestrator.

### Scope

**1.5-2 weeks.** ~400-600 lines core + ~200 lines CLI + ~150 lines MCP + ~300 lines tests.

Core complexity: git worktree commands are straightforward, but handling edge cases (dirty worktrees, orphaned branches, concurrent creation, cleanup on crash) requires careful error handling. Need to thread `working_dir` through existing gate evaluation functions.

### Dependencies

- None — pure git operations, no new external dependencies
- Benefits from the `working_dir: PathBuf` refactor (currently `Option<String>` in some types), but can work around it

### Risks

- **Path threading is pervasive**: Gate evaluation, spec scanning, history persistence all assume CWD or explicit path. Worktrees need *all* of these to accept arbitrary paths. This is a medium-sized refactor of existing interfaces.
- **Platform edge cases**: Windows worktree symlinks behave differently from Unix. macOS case-insensitive filesystem could cause slug collisions.
- **Stale worktree accumulation**: If agents crash or sessions are abandoned, worktrees linger. Need a reaping strategy (age-based, or tied to session lifecycle).

---

## Proposal 2: Session Lifecycle Manager — Single-Agent Spec Execution

### What

A `session` module that tracks the full lifecycle of one agent working on one spec in one worktree. This is the state machine at the heart of the orchestrator:

```
Init → Worktree Created → Agent Launched → Implementing → Gates Running →
  → [Pass] Review → Merge Ready → Merged → Cleanup
  → [Fail] Feedback → Re-implementing (loop back)
```

Concrete deliverables:
- `Session` type with state machine semantics (state enum, transition validation)
- Session persistence: `.assay/sessions/<session-id>.json` with crash recovery
- Session events: structured log of state transitions with timestamps
- Agent completion detection: exit code monitoring, output markers, timeout
- CLI: `assay session list/show/cancel`
- MCP: `session_status`, `session_cancel` tools

### Why

The orchestrator is too big to build in one shot. A single-session manager is the vertical slice that proves the lifecycle model works before scaling to N concurrent sessions. It also delivers immediate value: even managing ONE agent through the spec→gate→merge pipeline with structured state tracking is a massive improvement over ad-hoc workflows.

This forces critical design decisions about agent interaction that must be settled before multi-session work: How does the agent signal completion? How do gates trigger after implementation? How is merge approval gated?

### Scope

**2-3 weeks.** ~600-800 lines core (state machine, persistence, event log) + ~200 lines CLI + ~150 lines MCP + ~400 lines tests.

### Dependencies

- **Worktree Manager (Proposal 1)**: Sessions create worktrees for agents
- Design decision needed: agent launcher abstraction (see Proposal 3)

### Risks

- **State machine complexity**: Real-world sessions don't follow happy paths. Cancellation, timeout, crash recovery, partial gate results — each adds states and transitions. Risk of over-engineering the state machine before understanding real failure modes.
- **Agent completion detection is hard**: Different agents signal completion differently. Claude Code exits the process. Cursor stays running. Aider exits. Need a flexible completion model.
- **Persistence format**: Getting the session schema right is critical — it's the audit trail for the entire workflow. Schema changes are painful once sessions are in-flight.

---

## Proposal 3: Agent Launcher Trait — Pluggable Agent Abstraction

### What

Define the `AgentLauncher` trait that abstracts how agents are started, monitored, and stopped:

```rust
trait AgentLauncher {
    fn launch(&self, config: LaunchConfig) -> Result<AgentHandle>;
    fn status(&self, handle: &AgentHandle) -> AgentStatus;
    fn stop(&self, handle: &AgentHandle) -> Result<()>;
}

struct LaunchConfig {
    spec: Spec,
    working_dir: PathBuf,
    prompt_injection: PromptStrategy,  // env var, file, stdin, MCP
    timeout: Option<Duration>,
}
```

Ship with two implementations:
1. **`ClaudeCodeLauncher`** — launches `claude` CLI with `--print` or interactive mode, MCP registration, spec injection via CLAUDE.md or prompt
2. **`GenericCliLauncher`** — launches any CLI tool with configurable command template, env vars, working directory

### Why

This is the hardest design problem in v0.3 and the one most likely to be wrong on first attempt. Building it early with two concrete implementations (Claude Code + generic CLI) forces the abstraction to be grounded in reality rather than theoretical.

The trait also unlocks the competitive multi-agent story (Swarm Forge from the radical track): if you can launch any agent, you can launch N agents on the same spec and pick the best result. But the immediate value is simpler — any user can plug in their preferred agent.

### Scope

**2-3 weeks.** ~300-400 lines for trait + types, ~300-400 lines per launcher implementation, ~400 lines tests.

The trait design is the hard part — getting the abstraction level right. Too specific and it only works for Claude Code. Too generic and it's useless. The sweet spot is: launch a process in a directory with a prompt, monitor its lifecycle, detect completion.

### Dependencies

- **Worktree Manager (Proposal 1)**: Agents launch into worktrees
- Research needed: How Claude Code, Aider, and Codex handle prompt injection and completion signaling

### Risks

- **Premature abstraction**: With only Claude Code as a real user, the trait may be designed around Claude Code's quirks and break for other agents. Mitigation: build GenericCliLauncher simultaneously to force generality.
- **tmux dependency**: The vision includes tmux session management for agent lifecycle. This may need to be baked into the launcher or layered on top. Unclear which is correct.
- **Prompt injection variance**: Claude Code uses MCP tools + CLAUDE.md. Aider uses file context + CLI args. Cursor uses IDE state. The `PromptStrategy` enum could explode in complexity.

---

## Proposal 4: Gate Evaluate — Context-Controlled Independent Evaluation

### What

A new MCP tool `gate_evaluate` that implements independent (non-self) evaluation of agent work. Unlike `gate_report` (agent submits its own evaluation), `gate_evaluate` has Assay assemble the evaluation context:

1. Assay gathers: git diff (worktree vs base), spec criteria, file listing, test results
2. Assay constructs an evaluation prompt with this context (NOT the agent's implementation reasoning)
3. The evaluating agent receives the prompt and returns structured assessment
4. Assay records the result with `evaluator_role: Independent`

This is the architectural solution to the self-evaluation trust problem identified in the v0.2 brainstorm.

### Why

Self-evaluation is v0.2's pragmatic answer. But for Assay to be trusted for autonomous merge decisions, independent evaluation is non-negotiable. This is the feature that makes `autonomous: true` viable.

The key insight: Assay doesn't need an LLM client. It needs to *assemble context* and present it to an evaluating agent via MCP. The evaluating agent already has LLM access. Assay just controls what information flows to the evaluator — preventing the implementer's rationale from biasing the evaluation.

This directly enables the v0.3 trust model: implementer runs gates via `gate_report` (self-eval), then a separate evaluator runs via `gate_evaluate` (independent eval). Both produce `GateResult` with different `evaluator_role` values. The human or orchestrator can require independent evaluation for merge approval.

### Scope

**2-3 weeks.** ~400-500 lines for context assembly + diff generation, ~200 lines MCP tool, ~300 lines tests.

The diff generation and context assembly are the core complexity. Need to handle: large diffs (truncation), binary files, new files, deleted files, test output integration.

### Dependencies

- **Worktree Manager (Proposal 1)**: Need git diff between worktree and base branch
- **Session Manager (Proposal 2)**: Need to know which agent is the implementer vs evaluator
- Existing `gate_report` infrastructure (reuse `AgentEvaluation`, `GateResult` types)

### Risks

- **Context assembly quality**: Garbage in, garbage out. If the assembled context is too large (token limits), too small (missing key files), or poorly structured, evaluations will be unreliable.
- **Diff explosion**: Large refactors or generated code can produce diffs that exceed any reasonable context window. Need truncation + summarization strategy.
- **Evaluator gaming**: An agent evaluating its own team's work may still be biased. True independence requires a completely separate agent instance with no shared context. This is an orchestrator concern, not a tool concern.

---

## Proposal 5: Merge-Back Pipeline — Feature Branch to Main with Gate Enforcement

### What

The end-to-end workflow for getting agent work from a worktree into the main branch:

1. **Branch creation**: Feature branch from worktree (`feat/<spec-slug>`)
2. **Conflict detection**: Rebase or merge from main, detect conflicts
3. **Conflict resolution**: If conflicts exist, either auto-resolve (trivial) or escalate to agent/human
4. **Pre-merge gate**: Run all spec gates against the merged result
5. **Merge approval**: Human approval (default) or autonomous (if enabled)
6. **Merge execution**: Fast-forward or merge commit to main
7. **Post-merge cleanup**: Remove worktree, archive session

CLI: `assay merge <session-id> [--auto] [--strategy rebase|merge]`
MCP: `merge_prepare`, `merge_execute` tools

### Why

This completes the core loop: spec → agent → gate → **merge**. Without this, agents produce code in worktrees that humans must manually cherry-pick or merge. The merge-back pipeline is the second core differentiator (after dual-track gates) that separates Assay from agtx.

The key value: **gate enforcement at merge time.** Even if gates passed during development, the merged result must pass gates again (since main may have diverged). This is the quality guarantee that makes autonomous merging safe.

### Scope

**2-3 weeks.** ~500-700 lines core (branch strategy, conflict detection, merge execution) + ~200 lines CLI + ~200 lines MCP + ~400 lines tests.

### Dependencies

- **Worktree Manager (Proposal 1)**: Source of feature branches
- **Session Manager (Proposal 2)**: Tracks merge state
- **Gate evaluation**: Already exists — just needs to run against merge result
- Git operations: rebase, merge, conflict detection — well-understood domain

### Risks

- **Conflict resolution is an open problem**: Auto-resolving merge conflicts reliably is extremely hard. Starting with "escalate to human" for any conflict is pragmatic but limits the autonomous story.
- **Branch strategy proliferation**: Trunk-based, feature branches, release branches — each requires different merge logic. Risk of building a mini git workflow engine.
- **Race conditions**: Two agents merging to main simultaneously could create conflicts with each other. Need serialization or optimistic locking.

---

## Proposal 6: TUI Dashboard — Multi-Session Supervision Interface

### What

Transform the TUI skeleton into a real-time dashboard for supervising agent sessions:

- **Session list panel**: All active sessions with status (implementing, gates running, merge ready), spec name, agent type, duration, health
- **Session detail panel**: Selected session's gate results, recent events, context utilization
- **Log stream**: Live tail of agent output (if accessible) or session events
- **Action bar**: Approve merge, cancel session, re-run gates, open in editor
- **Keyboard navigation**: vim-style keybindings, tab between panels

### Why

The TUI is Assay's supervision surface — it's how humans maintain oversight of N concurrent agent sessions. Without it, the orchestrator is a headless daemon that's hard to monitor. The TUI transforms Assay from a CLI tool into a control center.

Building the TUI now (even before the full orchestrator) is valuable because:
1. It forces the session/event data model to be designed for real-time consumption
2. It provides immediate value for single-session supervision (gate results, context health)
3. It's the most visible demonstration of Assay's vision — screenshots of a TUI dashboard supervising agents are compelling

### Scope

**3-4 weeks.** ~800-1200 lines TUI (ratatui widgets, event handling, layout) + ~200-300 lines data layer (polling sessions, events).

Ratatui is already in the workspace. The complexity is in layout design, responsive resizing, and real-time data updates.

### Dependencies

- **Session Manager (Proposal 2)**: TUI displays session state
- Data layer: needs session list, event stream, gate results accessible via API
- No external dependencies — ratatui + crossterm already in workspace

### Risks

- **Premature without orchestrator**: If there's only one session to supervise, the TUI is overkill. Counter-argument: building the TUI forces the right data model for multi-session.
- **UX design complexity**: Good TUI design is hard. Layout, keybindings, information density, color usage — easy to build something confusing.
- **Real-time data**: Polling vs push for session updates. Polling is simple but laggy. Push (via channels) is responsive but adds complexity.

---

## Proposal 7: Spec Provider Trait — Pluggable Spec Sources

### What

Abstract the spec loading system behind a trait:

```rust
trait SpecProvider {
    fn list(&self) -> Result<Vec<SpecSummary>>;
    fn get(&self, name: &str) -> Result<Spec>;
    fn status(&self, name: &str) -> Result<SpecStatus>;
}
```

Ship with two implementations:
1. **`FileSpecProvider`** — current behavior, reads from `.assay/specs/` directory (refactored from free functions)
2. **`LinearSpecProvider`** — reads specs from Linear issues (the user already uses Linear for project management)

### Why

The pluggable spec provider is a core architectural principle from PROJECT.md: "teams already use Kata, Spec Kit, OpenSpec, or custom workflows; they shouldn't have to abandon them to get orchestration." The trait is the extension point.

Building `LinearSpecProvider` alongside the trait forces the abstraction to handle real-world variance: Linear issues have different fields than TOML files, need API calls instead of filesystem reads, have status semantics (todo/in-progress/done) that TOML files don't.

The Linear integration is also directly useful to the project owner (who uses Linear with `.linear.toml` configs). Dogfooding creates the tightest feedback loop.

### Scope

**2-3 weeks.** ~150-200 lines for trait + types, ~100 lines to refactor FileSpecProvider, ~400-500 lines for LinearSpecProvider (API client, mapping), ~300 lines tests.

### Dependencies

- HTTP client dependency (reqwest) for Linear API
- Linear API key configuration
- The previous brainstorm explicitly deferred this: "wait for a second concrete provider." Linear IS that second provider.

### Risks

- **Premature abstraction**: The v0.2 brainstorm warned against this. Counter: with Linear as the concrete second provider, it's no longer premature — there are two real implementations driving the design.
- **API client complexity**: HTTP client, auth, pagination, error handling, rate limiting — this is a non-trivial subsystem. Could scope-creep.
- **Spec mapping fidelity**: Linear issues don't naturally map to Assay's spec format. Mapping decisions (which fields? how to handle missing criteria? bidirectional sync?) introduce design complexity.

---

## Summary: Proposed Sequencing

```
Phase 1 (Foundation):
  [1] Worktree Manager         — THE prerequisite for everything

Phase 2 (Core Loop):
  [3] Agent Launcher Trait      — parallel with session work
  [2] Session Lifecycle Manager — depends on worktree + launcher

Phase 3 (Trust & Quality):
  [4] Gate Evaluate             — independent evaluation
  [5] Merge-Back Pipeline       — completes the loop

Phase 4 (Surfaces & Extensions):
  [6] TUI Dashboard             — supervision surface
  [7] Spec Provider Trait        — extensibility
```

Total estimated scope: **15-20 weeks** of focused development for the full set. Realistically, 2-3 of these would constitute a strong v0.3.0 milestone (Worktree Manager + Session Manager + Agent Launcher or Merge-Back Pipeline).
