# High-Value Feature Proposals for Assay v0.2

**Explorer:** explorer-highvalue
**Date:** 2026-03-02
**Context:** v0.1.0 shipped with deterministic gates, CLI, MCP server, and Claude Code plugin. These proposals target the features that define what Assay becomes next.

---

## 1. Agent-Evaluated Gates

### What
Add a new `GateKind::AgentEvaluate` variant that delegates criterion evaluation to an LLM. The agent receives context (file diffs, test output, spec description) and a natural-language assertion (e.g., "error messages are user-friendly and include recovery suggestions"), then returns a structured judgment: pass/fail with reasoning and confidence score. This completes the dual-track quality model — deterministic gates catch objective failures; agent gates enforce subjective quality standards that no shell command can express.

### Why
This is Assay's **category-defining feature**. Without it, Assay is a spec-organized test runner — competent but undifferentiated. Agent-evaluated gates are what make "quality gate" mean more than "CI passed." They enable assertions like "the API follows RESTful conventions," "error handling is comprehensive," or "the documentation matches the implementation." No other tool offers programmable AI-verified quality criteria as a first-class workflow primitive.

### Scope
**Phase 1 — Foundation (medium complexity):**
- Add `prompt: Option<String>` field to `Criterion`
- Add `AgentEvaluate { prompt: String }` to `GateKind` with `AgentGateResult { passed: bool, reasoning: String, confidence: f64 }`
- Define `AgentEvaluator` trait: `async fn evaluate(&self, context: AgentGateContext, prompt: &str) -> AgentGateResult`
- Ship a "passthrough" evaluator that formats the prompt + context and calls a configurable LLM endpoint (start with Anthropic API)

**Phase 2 — Context assembly (medium-high complexity):**
- Context collector: git diff, file contents, previous gate results, spec metadata
- Context windowing/truncation for token limits
- Configurable context sources per criterion

**Phase 3 — Trust calibration (high complexity):**
- Confidence thresholds: fail if confidence < 0.7, warn if < 0.9
- Human-in-the-loop override for low-confidence judgments
- Agent agreement (run multiple evaluators, require consensus)

### Risks
- **LLM reliability:** Agent judgments are non-deterministic. Same code, same prompt can produce different results across runs. Mitigation: confidence scores, consensus mode, and clear UX distinguishing deterministic vs. agent results.
- **Cost:** Every agent gate is an API call. A spec with 10 agent criteria is 10 LLM invocations per gate run. Mitigation: caching, batching, and clear cost visibility.
- **Prompt engineering burden:** Users need to write good prompts for gates to be useful. Bad prompts produce unreliable gates. Mitigation: ship prompt templates, provide examples, validate prompt clarity.
- **Latency:** LLM calls are 2-30 seconds each. Sequential agent gates on a 10-criterion spec could take minutes. Mitigation: parallel evaluation, deterministic gates first (fast-fail), streaming results.

### Dependencies
- Async runtime in gate evaluation (currently synchronous `spawn_blocking`)
- HTTP client dependency (reqwest) for LLM API calls
- API key management (config or env var)
- No hard dependency on other proposals — can ship standalone

---

## 2. Worktree Orchestrator (MVP)

### What
Build the minimum viable orchestration loop: Assay creates a git worktree per spec, launches an AI agent to implement against that spec in isolation, runs gates when the agent signals completion, and reports results. This is the "one agent, one spec, one worktree" loop — not yet multi-agent or merge-back, but the foundational unit of the orchestration vision. The orchestrator manages the lifecycle: create worktree → configure agent → launch in subprocess → monitor for completion signal → run gates → report.

### Why
This transforms Assay from a passive quality checker into an **active development orchestrator**. The spec-work-gate loop is the atomic unit of everything Assay aspires to be. Without it, specs and gates are tools an agent can optionally use. With it, Assay *drives* the development process. This also unlocks the key workflow: a human writes specs, Assay dispatches agents, gates verify quality, human reviews results. That's the "agentic development kit" promise.

### Scope
**Phase 1 — Worktree lifecycle (low-medium complexity):**
- `WorktreeManager` struct: create, list, cleanup worktrees
- Branch naming convention: `assay/<spec-slug>/<timestamp>`
- Integration with `git2` or shell `git worktree` commands
- Cleanup policy: auto-remove on gate pass, preserve on failure

**Phase 2 — Agent launcher (medium complexity):**
- `AgentLauncher` trait: `async fn launch(&self, worktree_path: &Path, spec: &Spec) -> AgentHandle`
- Initial implementation: spawn `claude` CLI as subprocess with spec injected via `CLAUDE.md` or stdin prompt
- `AgentHandle`: pid, stdout/stderr streams, completion signal
- Completion detection: process exit, file sentinel, or explicit signal

**Phase 3 — Orchestration loop (medium complexity):**
- `Orchestrator` struct tying WorktreeManager + AgentLauncher + gate evaluation
- CLI command: `assay run <spec-slug>` — create worktree, launch agent, wait, run gates, report
- Status tracking: pending → working → gating → passed/failed
- Basic retry: re-launch agent on gate failure (configurable max retries)

### Risks
- **Agent compatibility:** Different AI coding agents (Claude Code, Cursor, Aider, etc.) have very different invocation models, context injection methods, and completion signals. An abstraction that works for all is hard. Mitigation: start with Claude Code only, design the trait to be extensible.
- **Worktree conflicts:** Git worktrees share the same `.git` directory. Concurrent operations on the main repo while worktrees exist can cause issues. Mitigation: lock files, sequential operations initially.
- **Scope creep toward full daemon:** The MVP should resist becoming a full daemon. It runs one loop, synchronously, in the foreground. The daemon comes later.
- **Resource management:** Each agent subprocess consumes significant CPU/memory/API tokens. No built-in resource management yet.

### Dependencies
- Git worktree support (git2 crate or shell commands)
- Async subprocess management (tokio::process)
- Spec system (exists)
- Gate system (exists)

---

## 3. Gate Composition & Dual-Track Pipeline

### What
Extend the gate evaluation engine with composite logic: gates can be combined with AND (all must pass), OR (any must pass), and threshold (N of M must pass). Add a pipeline mode that runs deterministic gates first as a fast-fail filter, then runs agent-evaluated gates only if deterministic gates pass. This enables sophisticated quality policies like "tests pass AND (code review OR pair programming was done) AND at least 3 of 5 quality criteria score above 0.8."

### Why
Real quality policies are multi-dimensional. A single flat list of pass/fail criteria doesn't express policies like "all tests must pass, but style checks are advisory" or "either the security audit passed or a security team member reviewed." Gate composition makes Assay's quality model expressive enough for real engineering workflows. The dual-track pipeline (deterministic first, agent second) is also a critical efficiency optimization — why spend API tokens on agent evaluation if `cargo test` already failed?

### Scope
**Phase 1 — Criterion-level metadata (low complexity):**
- Add `required: bool` (default true) and `weight: Option<f64>` to `Criterion`
- `GateRunSummary` reports required vs. advisory failures separately
- Pass/fail logic: all required criteria must pass; advisory failures are warnings

**Phase 2 — Ordered evaluation (low-medium complexity):**
- Add `phase: Option<u32>` to `Criterion` for explicit ordering
- Criteria without a phase run in phase 0 (deterministic default)
- Short-circuit: if any required criterion in phase N fails, skip phase N+1
- Natural fit: deterministic criteria in phase 0, agent criteria in phase 1

**Phase 3 — Composite gates (medium complexity):**
- `CompositeGate` type: `And(Vec<Gate>)`, `Or(Vec<Gate>)`, `Threshold { min: usize, gates: Vec<Gate> }`
- Spec format extension for nested criteria groups
- `GateRunSummary` enhanced with group-level results

### Risks
- **Spec format complexity:** TOML doesn't naturally express deeply nested structures. Composite gates could make spec files hard to read and write. Mitigation: keep nesting shallow (max 2 levels), provide spec validation, consider YAML as alternative format.
- **Evaluation ordering subtleties:** Short-circuit logic with mixed required/advisory criteria across phases gets complex. Edge cases: what if a required criterion in phase 1 depends on context from phase 0? Mitigation: keep phases independent, document ordering semantics clearly.
- **Over-engineering risk:** Most users may never need composite gates. Start with required/advisory distinction (phase 1) and prove demand before building full composition.

### Dependencies
- Proposal #1 (Agent-Evaluated Gates) for the dual-track pipeline to be meaningful
- Current gate evaluation engine (exists, needs refactoring from free functions to trait-based)

---

## 4. Run History & Evidence Store

### What
Persist gate evaluation results to a local SQLite database (or structured file store). Each gate run is recorded with: timestamp, spec slug, per-criterion results, stdout/stderr evidence, duration, and pass/fail status. Provide CLI commands to query history: `assay history <spec>` shows recent runs, `assay history --trend` shows pass/fail rates over time, `assay history --diff <run1> <run2>` compares two runs. Expose history via MCP tool for agent consumption.

### Why
Currently, gate results are ephemeral — they exist only in terminal output and MCP tool responses. This means:
- No trend analysis: "are gates getting flakier?"
- No regression detection: "this gate was passing last week, what changed?"
- No audit trail: "prove this code passed quality gates before merge"
- No agent learning: agents can't see what failed before and adjust

A persistent evidence store transforms gates from one-shot checks into a **continuous quality signal**. It's also infrastructure for the orchestrator — the orchestrator needs to record and query run state across agent sessions.

### Scope
**Phase 1 — Local file store (low complexity):**
- JSON files in `.assay/runs/<spec-slug>/<timestamp>.json`
- `GateRunRecord` struct: spec slug, timestamp, summary, per-criterion details
- `assay gate run` automatically saves results
- `assay history <spec>` lists recent runs

**Phase 2 — Query and analysis (medium complexity):**
- `assay history --trend <spec>` shows pass rate over last N runs
- `assay history --failures` shows most frequently failing criteria
- Duration tracking: detect tests getting slower
- MCP tool: `run_history` for agent consumption

**Phase 3 — SQLite store (medium complexity):**
- Migrate from JSON files to SQLite for efficient querying
- Full-text search over evidence (stdout/stderr)
- Cross-spec analytics: "which specs fail most?"
- Export: JSON, CSV for external tooling

### Risks
- **Storage growth:** With evidence (stdout/stderr), each run could be megabytes. Over time, this accumulates. Mitigation: configurable retention policy, evidence compression, truncation matching current 64KB cap.
- **Complexity vs. value tradeoff:** Phase 1 (JSON files + list) delivers 80% of the value at 20% of the complexity. SQLite may be over-engineering for a local dev tool. Mitigation: start with files, upgrade to SQLite only if query performance or data volume demands it.
- **Git bloat:** If `.assay/runs/` is checked into the repo, it bloats the git history. Mitigation: `.gitignore` by default, optional flag to include.

### Dependencies
- Gate evaluation engine (exists)
- No hard dependency on other proposals — can ship standalone

---

## 5. SpecProvider Trait & Pluggable Spec Sources

### What
Define a `SpecProvider` trait that abstracts where specs come from. The current TOML-file-based spec loading becomes the `FileSpecProvider` — the default implementation. Additional providers can source specs from: a Kata project's roadmap (phases as specs), an external API, a Git branch's diff (auto-generate specs from PR description), or a conversation with an AI (interactive spec refinement). The trait surface: `list() -> Vec<SpecSummary>`, `get(name) -> Spec`, `watch() -> Stream<SpecChange>` (for live reload).

### Why
Specs are Assay's input — they define what work to do and what quality to enforce. If specs can only come from hand-written TOML files, adoption is limited to users willing to write specs upfront. Pluggable providers unlock:
- **Kata integration:** Your roadmap phases already *are* specs. `KataSpecProvider` reads `.planning/` and presents each phase as a spec with criteria derived from the plan.
- **PR-driven specs:** Extract spec from a PR description or issue body. Agent implements against the spec, gates verify, reviewer approves.
- **AI-assisted spec authoring:** `assay spec create` launches an interactive session where an AI helps decompose a high-level goal into concrete criteria.
- **External systems:** Jira tickets, Linear issues, or any system that defines "what to build."

### Scope
**Phase 1 — Trait definition + FileSpecProvider (low-medium complexity):**
- Define `SpecProvider` trait with `list`, `get`, `validate`
- Refactor current `spec::scan`/`spec::load` into `FileSpecProvider`
- Registry: `SpecProviderRegistry` supporting multiple providers with priority
- Config: `[spec_providers]` section in `.assay/config.toml`

**Phase 2 — Watch + live reload (medium complexity):**
- `watch() -> Stream<SpecChange>` for file-system watching (notify crate)
- TUI and daemon can react to spec changes without restart
- Hot-reload spec definitions during agent work

**Phase 3 — Kata provider (medium complexity):**
- `KataSpecProvider` reads `.planning/roadmap.md`, phase plans, and milestone definitions
- Maps phases to specs, plan steps to criteria
- Bi-directional: gate results flow back to Kata progress tracking

### Risks
- **Abstraction too early:** The trait surface designed now may not fit providers discovered later. The wrong abstraction is worse than no abstraction. Mitigation: start with a minimal trait (list + get), extend when real providers reveal needs.
- **Provider impedance mismatch:** Different spec sources have very different data models. A Linear issue is not a TOML spec — the mapping may lose information or add false precision. Mitigation: providers can return partial specs (missing criteria = descriptive only), validation catches mismatches.
- **Maintenance burden:** Each provider is its own integration surface with its own failure modes, auth requirements, and update cadence. Mitigation: ship FileSpecProvider built-in, all others as optional features behind cargo feature flags.

### Dependencies
- Current spec system (exists, needs refactoring from free functions to trait methods)
- Kata project (for KataSpecProvider — both are owned by the same user, so integration is natural)

---

## 6. TUI Dashboard: Live Session Monitor

### What
Replace the TUI skeleton with a functional dashboard for supervising agent work sessions. Core views: (1) **Session list** — shows all active/recent worktree sessions with status (working, gating, passed, failed), spec name, agent, and duration; (2) **Session detail** — live-streaming agent stdout/stderr, current gate results, spec criteria with pass/fail indicators; (3) **Gate history** — per-spec trend visualization using sparklines or bar charts. The TUI is the human supervision surface — where a developer watches multiple agents work and intervenes when needed.

### Why
The orchestrator needs a supervision interface. CLI output is insufficient for monitoring multiple concurrent agents — it's serial and ephemeral. The TUI provides:
- **Situational awareness:** See all agent sessions at a glance, spot failures immediately
- **Intervention point:** Pause an agent, re-run a gate, approve a low-confidence agent judgment, abort a runaway session
- **Confidence builder:** Humans trusting AI agents requires transparency. Watching an agent work, seeing gates pass/fail in real-time, and being able to intervene builds the trust needed for adoption
- **Differentiation:** Most AI coding tools have no supervision surface. The TUI is how Assay says "you're not giving up control, you're gaining leverage."

### Scope
**Phase 1 — Spec & gate viewer (medium complexity):**
- App state machine: SpecList → SpecDetail → GateRunning → GateResults
- SpecList: table of specs with name, criteria count, last run status
- SpecDetail: criteria list with pass/fail indicators
- GateRunning: live output streaming (stdout/stderr) with progress indicator
- Keyboard navigation: j/k scroll, Enter select, Esc back, q quit, g run gates

**Phase 2 — Session monitor (high complexity):**
- Session list view (requires orchestrator — proposal #2)
- Live agent output streaming (subprocess stdout piped to TUI)
- Session controls: pause, resume, abort, re-gate
- Split-pane layout: session list left, detail right

**Phase 3 — Analytics & intervention (high complexity):**
- Gate history sparklines (requires history store — proposal #4)
- Agent-evaluated gate details: show reasoning, confidence, allow override
- Multi-agent comparison view: side-by-side session progress

### Risks
- **Premature without orchestrator:** Phase 2-3 require the worktree orchestrator to exist. Phase 1 (spec/gate viewer) is standalone. Building the full dashboard before the orchestrator exists means building UI for imaginary data flows.
- **TUI complexity:** Rich TUI applications in ratatui are significantly more complex than CLI output. State management, layout, event handling, async data updates — each adds substantial code. Mitigation: start with Phase 1 as a useful standalone tool, prove the architecture before Phase 2.
- **Terminal compatibility:** TUI rendering varies across terminals, SSH sessions, tmux, screen. Testing all combinations is expensive. Mitigation: test on iTerm2 + tmux initially, document known issues.

### Dependencies
- Proposal #2 (Worktree Orchestrator) for Phase 2-3
- Proposal #4 (Run History) for Phase 3
- assay-core gate evaluation (exists)
- Phase 1 is standalone — depends only on existing spec/gate system

---

## Prioritization Recommendation

**Must-build for v0.2 (defines the product):**
1. **Agent-Evaluated Gates (#1)** — Without this, Assay is a test runner. With it, it's a new category.
2. **Gate Composition (#3, Phase 1 only)** — Required/advisory distinction is table-stakes for the dual-track model.

**Should-build for v0.2 (enables the vision):**
3. **Run History (#4, Phase 1)** — JSON file store + list command. Low effort, high infrastructure value.
4. **TUI Dashboard (#6, Phase 1)** — Spec/gate viewer makes the TUI useful standalone.

**Build for v0.3 (the orchestrator milestone):**
5. **Worktree Orchestrator (#2)** — The big investment. Needs agent-evaluated gates and history store as prerequisites.
6. **SpecProvider Trait (#5)** — Enables ecosystem. Build when real providers are needed (Kata integration).
