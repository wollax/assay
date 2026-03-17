# High-Value Features Report: Assay v0.3.0

**Explorer:** explorer-highvalue
**Challenger:** challenger-highvalue
**Date:** 2026-03-08
**Rounds:** 3 (initial proposals → detailed critique → convergence)
**Status:** Converged — 5 features accepted, 2 killed, all scoped and sequenced

---

## Executive Summary

From 7 initial proposals, debate produced a focused 5-feature v0.3.0 scope totaling **8.5-9.5 weeks** of development. The milestone delivers: worktree isolation, headless agent launching, session tracking, independent quality evaluation, and a minimal TUI — the first concrete steps toward Assay's orchestration vision.

Two proposals were killed: **Merge-Back Pipeline** (premature without full orchestration) and **Spec Provider Trait** (premature abstraction with insufficient real-world variance). One proposal was moved to the quick-wins track: **`assay spec import --from-linear`** as a CLI command.

---

## The v0.3.0 Workflow

A critical insight from debate: v0.3.0 targets a **headless, sequential workflow**, not the full MCP-integrated iterative workflow. The distinction matters for setting expectations.

### v0.3.0 Workflow (what we're building)

```
1. Human creates spec with criteria
2. Assay creates worktree from base branch
3. Assay launches Claude Code in --print mode with spec prompt
4. Agent implements in isolation, exits when done
5. Assay runs command gates against worktree (deterministic)
6. Assay runs gate_evaluate with a SEPARATE agent (independent evaluation)
7. Human reviews gate results + evaluation in TUI
8. Human merges manually (git merge from worktree branch)
```

### v0.4.0 Workflow (what comes next)

```
Same as above, but:
- Agent runs interactively (tmux) with MCP access
- Agent calls gate_run/gate_report iteratively during implementation
- Merge-back pipeline handles branch strategy + conflict resolution
- Orchestrator manages N concurrent sessions
- TUI supervises all sessions in real-time
```

This distinction avoids overselling v0.3.0. The headless workflow is genuinely useful — it's "automated code generation with quality gates" — but it's not the full orchestration vision.

---

## Accepted Proposals

### 1. Worktree Manager — Git Worktree Lifecycle Engine

| Attribute | Value |
|-----------|-------|
| **Scope** | 2 weeks |
| **Estimate** | ~500-700 lines core + ~200 lines CLI + ~150 lines MCP + ~500 lines tests (incl. git test fixtures) |
| **Dependencies** | None |
| **Risk** | Medium — path threading refactor, spec resolution design |

**What:** A `worktree` module in `assay-core` managing git worktree creation, listing, status, and cleanup.

**Key design decisions from debate:**

1. **Specs live in the parent project, not the worktree.** The worktree is for implementation only. `spec::scan()` needs a separate `specs_origin: &Path` parameter for resolving specs from the parent while evaluating gates in the worktree.

2. **Manual lifecycle only.** Create/list/status/cleanup — no auto-reaping, no session-tied lifecycle. Stale worktree cleanup is a v0.4 concern tied to the orchestrator.

3. **Git test infrastructure required.** Current tests are in-memory or tempdir-based. Worktree operations need real git repos with branches. Budget ~200 lines of test fixtures (repo creation, branch setup, commit helpers).

**Surfaces:**
- CLI: `assay worktree create <spec> [--base-branch main]`, `assay worktree list`, `assay worktree status <spec>`, `assay worktree cleanup <spec>`
- MCP: `worktree_create`, `worktree_status`, `worktree_cleanup`

**Why this is THE foundation:** Every subsequent feature depends on "agents work in isolated worktrees." Without this, there's no isolation story, no independent evaluation, no multi-agent future. It's also the simplest piece to build and test independently — pure git operations with well-understood semantics.

---

### 2. Claude Code Launcher — Concrete Agent Launch Module

| Attribute | Value |
|-----------|-------|
| **Scope** | 1.5 weeks |
| **Estimate** | ~300-400 lines core + ~100 lines CLI + ~300 lines tests |
| **Dependencies** | Worktree Manager |
| **Risk** | Low-Medium — subprocess management is well-understood, but prompt injection strategy needs research |

**What:** A `claude_code` module in `assay-core` that launches Claude Code in `--print` (headless) mode within a worktree.

**Key design decisions from debate:**

1. **Concrete module, NOT a trait.** With exactly one real agent (Claude Code), a trait would be shaped by Claude Code's quirks and useless for the `GenericCliLauncher` that has no real user. Extract the `AgentLauncher` trait when a second concrete implementation materializes (Codex, Aider).

2. **Headless only (`--print` mode).** Interactive agent support (tmux sessions) is v0.4. This dramatically simplifies the launcher to: spawn subprocess → monitor PID → capture exit code + output.

3. **Prompt injection via CLI args + file.** The launcher writes the spec content to a file in the worktree and passes it via `--print` argument. No MCP registration needed for headless mode.

**Important limitation:** `--print` mode Claude Code does NOT use MCP tools. The agent won't call `gate_run` or `gate_report` during implementation. Gates run AFTER the agent exits. This is the v0.3.0 sequential workflow — iterative MCP-integrated workflow requires interactive mode (v0.4).

**Surfaces:**
- CLI: `assay launch <spec> [--timeout 30m]`
- MCP: `session_launch` (creates worktree + launches agent + creates session record)

---

### 3. Session Record — Minimal Session Tracking

| Attribute | Value |
|-----------|-------|
| **Scope** | 1 week |
| **Estimate** | ~200 lines core + ~100 lines CLI + ~50 lines MCP + ~150 lines tests |
| **Dependencies** | Worktree Manager (needs worktree path) |
| **Risk** | Low — deliberately minimal schema |

**What:** A `session` module that persists session metadata as JSON files in `.assay/sessions/`.

**Key design decisions from debate:**

1. **Session is a bookmark, not a workflow engine.** No state machine, no transition validation, no event sourcing. Just a record of what's happening.

2. **Schema defined by consumers, not future orchestrator.** The minimal schema serves exactly what v0.3 features need:

```rust
struct SessionRecord {
    id: SessionId,
    spec_name: String,
    worktree_path: PathBuf,
    agent_pid: Option<u32>,
    status: SessionStatus, // Active, Completed, Failed, Cancelled
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    gate_run_ids: Vec<String>, // links to existing run history
}
```

3. **Scoping constraint:** If you find yourself adding `retry_count`, `feedback_history`, `parent_session`, or `merge_strategy` — stop. Those are orchestrator concerns for v0.4.

**Why include it:** Without the session record, the worktree manager and launcher are disconnected utilities. The session is the join table: "this worktree belongs to this spec being worked on by this agent." It enables crash recovery (know what was in-flight), TUI display, and gate_evaluate context assembly.

**Surfaces:**
- CLI: `assay session list`, `assay session show <id>`
- MCP: `session_status`

---

### 4. Gate Evaluate — Context-Controlled Independent Evaluation

| Attribute | Value |
|-----------|-------|
| **Scope** | 3 weeks |
| **Estimate** | ~500-600 lines diff/context assembly + ~200 lines MCP tool + ~100 lines types + ~400 lines tests |
| **Dependencies** | Worktree Manager (for diffs), Session Record (for context) |
| **Risk** | High — diff assembly quality determines evaluation quality |

**What:** A new MCP tool `gate_evaluate` that enables independent (non-self) evaluation of agent work by assembling context and presenting it to a separate evaluating agent.

**Key design decisions from debate:**

1. **Evaluator isolation is the point.** Gate Evaluate REQUIRES worktree isolation. Without it, the evaluator can see the implementer's reasoning (tool call history, CLAUDE.md modifications), undermining independence. This is why it depends on the Worktree Manager — not just for diffs, but for context separation.

2. **Diff assembly is its own module.** `assay-core/src/context/diff.rs` (or `assay-core/src/diff/`) — a pure function `fn assemble_evaluation_context(worktree: &Path, base: &str, spec: &Spec) -> EvaluationContext`. Reusable by TUI (show diffs), merge pipeline (v0.4), and future tooling. Testable independently of MCP.

3. **Spec-level evaluation config.** New field on `GateSection` or `Criterion`:
   ```toml
   [gate]
   evaluation = "independent"  # "self" | "independent" | "both"
   ```
   This lets specs declare their trust requirements. `gate_run` handles `self` (current behavior). `gate_evaluate` handles `independent`. `both` requires both to pass.

4. **Token budget integration.** Before assembling context, query `estimate_tokens` for the evaluating agent's context utilization. Truncate diff intelligently — keep test files and spec-referenced files, summarize generated/boilerplate code.

**The trust model progression:**
- v0.2: Self-evaluation with structured rigor + audit trail
- v0.3: Independent evaluation with context isolation (THIS FEATURE)
- v0.4: Orchestrator-enforced evaluation policies

**Surfaces:**
- MCP: `gate_evaluate` (spec_name, session_id, evaluator config)
- CLI: `assay gate evaluate <spec> --session <id>` (runs evaluation via CLI-spawned agent)

---

### 5. Minimal TUI — Gate Results Viewer

| Attribute | Value |
|-----------|-------|
| **Scope** | 1 week |
| **Estimate** | ~400-600 lines TUI + ~100 lines data layer |
| **Dependencies** | Session Record, gate results (existing) |
| **Risk** | Low — ratatui already in workspace, scope deliberately narrow |

**What:** A single-purpose TUI screen for viewing gate results interactively.

**Key design decisions from debate:**

1. **Gate results viewer, not multi-session dashboard.** The full dashboard (multiple panels, real-time updates, session supervision) requires the orchestrator. The v0.3 TUI is a better `cat results.json | jq`.

2. **Scope:**
   - Navigable table of criteria results (pass/fail, duration, evidence preview)
   - Detail pane for selected criterion (full stdout/stderr, evidence, reasoning)
   - Keyboard nav (j/k to navigate, Enter for detail, q to quit)
   - Color-coded status (green pass, red fail, yellow advisory)
   - Session list sidebar (if sessions exist)

3. **Extensible to dashboard.** The layout, event loop, and data layer patterns established here will be reused when the full dashboard is built in v0.4.

**Surfaces:**
- CLI: `assay tui` (launches gate results viewer)

---

## Killed Proposals

### Merge-Back Pipeline — Deferred to v0.4.0

**Original scope:** 2-3 weeks for end-to-end merge workflow.

**Why killed:** Without automatic session management, a human is already doing `git merge` manually. The delta between `assay merge` and `git merge && assay gate run` is only meaningful with full orchestration. Building it now means building it twice — once speculatively, once when real integration surfaces.

**When to revisit:** After v0.3.0 ships and the worktree→launch→gate workflow is proven. At that point, merge-back becomes a natural 1-week addition.

### Spec Provider Trait + LinearSpecProvider — Killed

**Original scope:** 2-3 weeks for trait + HTTP API client.

**Why killed:**
1. Linear issues and TOML specs have fundamentally different data models — the abstraction is lossy and arbitrary
2. `reqwest` pulls ~50 transitive dependencies for a non-core feature
3. The v0.2 brainstorm correctly identified this as premature — one real provider doesn't justify a trait

**Replacement:** `assay spec import --from-linear NDI-34` moved to quick-wins track. Shell out to `linear` CLI, parse JSON, write TOML. No new dependencies, no trait, no runtime API calls.

---

## Sequencing

```
Week 1-2:   Worktree Manager (foundation, blocks everything)
Week 2-3:   Claude Code Launcher + Session Record (parallel, both need worktree)
Week 4-6:   Gate Evaluate (needs worktree + session context, diff assembly is complex)
Week 7:     Minimal TUI (consumes all other features' data)
Week 8-9:   Integration Testing + Polish
```

**Integration week is critical.** The five features must compose into a smooth end-to-end workflow:

```
assay worktree create my-spec
  → assay launch my-spec
    → assay session show <id>
      → assay gate run my-spec --working-dir <worktree>
        → assay gate evaluate my-spec --session <id>
          → assay tui
```

If this flow doesn't work cleanly, individual features don't matter.

**Parallelization:** Launcher and Session Record can be built concurrently (weeks 2-3). Everything else is sequential.

---

## Cross-Cutting Concerns

### Path Threading

The worktree manager forces a pervasive (but valuable) change: all domain functions must accept explicit working directory parameters rather than assuming CWD. Functions already accepting `&Path` (like `evaluate()`) are fine. Functions using implicit CWD need refactoring. This is background work throughout weeks 1-3.

### Schema Evolution

Three new types enter `assay-types`: `SessionRecord`, `SessionStatus`, `EvaluationContext`. One existing type gains a field: `GateSection.evaluation`. All must follow established patterns: `serde(skip_serializing_if)`, `schemars(JsonSchema)`, `deny_unknown_fields` (with the caveat that this is a known tech debt issue for forward compatibility).

### Test Strategy

- **Worktree:** Real git repo fixtures (~200 lines infrastructure)
- **Launcher:** Mock subprocess or test with actual `claude` binary if available
- **Session:** In-memory + tempdir persistence (follows existing history test patterns)
- **Gate Evaluate:** Snapshot tests for diff assembly output
- **TUI:** Manual testing (ratatui doesn't have a great test story)

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Diff assembly produces poor evaluation context | Medium | High | Invest in semantic truncation; test with real-world diffs from v0.2 development |
| `--print` mode limitations surface late | Medium | Medium | Research Claude Code `--print` behavior early (week 1); pivot to `--message` or API if needed |
| Path threading refactor is larger than expected | Medium | Medium | Start with worktree module; refactor incrementally, not big-bang |
| Integration week reveals composability gaps | Low-Medium | High | Design shared types (SessionId, WorktreePath) upfront; avoid coupling via filesystem paths only |
| Scope creep from orchestrator features leaking into v0.3 | Medium | Medium | Hard scoping constraint: if a feature requires managing >1 concurrent session, it's v0.4 |

---

## What v0.3.0 Enables for v0.4.0

With v0.3.0 complete, v0.4.0 can build:

1. **tmux Session Management** → upgrade launcher from headless to interactive
2. **Merge-Back Pipeline** → ~1 week addition on top of worktree + session + gates
3. **Multi-Session Orchestrator** → session record becomes session manager; TUI viewer becomes dashboard
4. **MCP-Integrated Iterative Workflow** → agents call gate tools during implementation
5. **Agent Launcher Trait** → extract from concrete Claude Code + second implementation

The v0.3.0 features are designed as foundations, not endpoints.

---

*Consolidated from 3 rounds of explorer/challenger debate — 2026-03-08*
