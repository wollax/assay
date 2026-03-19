# Radical Ideas: Assay v0.4.0+

> Explorer: radical paradigm shifts. No self-censorship.

---

## Idea 1: The Gate Oracle — Predictive Failure Prevention

**Name**: Gate Oracle

**What**: Transform gate history from a passive archive into an active prediction engine. Before running gates, the Oracle queries historical run records and computes a "failure probability" for each criterion based on: code diff fingerprints, session context patterns, time-of-day, and historical co-failure correlations. High-risk gates get pre-flight warnings; the CI pipeline can optionally abort early.

The shift: gates stop being post-hoc validators and become pre-flight risk advisors. You know *before* `gate_run` whether you're likely to fail.

**Why This Changes the Game**:
- Most CI waste is running a full suite only to fail on the same gate that always fails when `auth.rs` changes. Oracle eliminates this.
- Developers get actionable signal before committing: "Based on 47 prior runs, this diff pattern fails `security-audit` 89% of the time."
- Transforms Assay from a reporter into a predictor — a fundamentally different value proposition.

**Scope**: Seed in v0.4.0 (co-failure correlation from history). Full ML-backed model: v1.0.

**Risks**:
- Predictions are wrong → false confidence. Need to be explicit this is probabilistic.
- History cold-start: new projects have no data. Must be opt-in after N runs.
- "Prediction" as feature adds complexity to what should be a simple tool.

**Seeds for v0.4.0**:
- Add a `gate_predict` MCP tool that queries history and returns co-failure correlations.
- Simple heuristic: "this criterion failed in 3 of last 5 runs" → emit warning.
- No ML required. Just frequency counting over history records.

---

## Idea 2: Recursive Quality Contracts — Gates Evaluating Gates

**Name**: Fractal Gates

**What**: Allow gate specs to reference other specs as sub-gates. A top-level `release-readiness` spec composes `unit-tests`, `security-audit`, `performance-baseline`, and `docs-coverage` — each of which is itself a fully evaluated spec. Enforcement is hierarchical: if a required child spec fails, the parent fails.

Crucially, a parent spec can override child enforcement levels (e.g., "treat `performance-baseline`'s advisory criteria as required for release"). This enables environment-specific quality profiles without duplicating specs.

**Why This Changes the Game**:
- Eliminates spec duplication. Today teams copy-paste spec variants. Composition replaces inheritance.
- Enables true quality abstraction: `senior-dev-release` vs `intern-PR` specs that compose the same base checks with different enforcement.
- Makes Assay the "quality dependency graph" tool — specs can declare `depends_on` and Assay resolves the DAG.

**Scope**: v0.4.0 for simple composition (spec references other specs). Full DAG resolution: v0.5.0.

**Risks**:
- Circular dependencies → infinite loops. Need cycle detection.
- Evaluation order matters; parent specs must wait for all children.
- Complex failure attribution: which child made the parent fail?

**Seeds for v0.4.0**:
- Add a `uses: [spec-name]` field in spec TOML.
- `gate_run` resolves `uses` specs first, inlines their criteria.
- Enforcement override syntax: `uses: { spec: "security-audit", override_enforcement: required }`.

---

## Idea 3: Context Counterfactuals — What Would Have Prevented This Failure?

**Name**: Causal Context Engine

**What**: Invert the context compression problem. Instead of "given a budget, what's optimal?", ask: "given this gate failure, what context was missing that would have led to a pass?"

After a gate failure, the Causal Context Engine diffs the session's pruned context against a full context, then uses the agent to identify the "causal gap" — the specific context that was absent. It outputs a `context_prescription`: next time, protect these files/sections.

Over time, prescriptions accumulate into a project-level `context policy` that the pruning engine consults. This makes pruning adaptive: files that correlate with gate passes are protected; files that correlate with failures-to-pass get elevated budget.

**Why This Changes the Game**:
- Today's pruning is dumb (structural heuristics). Counterfactual analysis makes it semantically aware.
- Closes the loop between gate outcomes and context decisions — currently a completely manual process.
- This is the "Postgres query planner" insight for context: instead of "always keep N rows", "keep rows that matter for this query".

**Scope**: Conceptual seed in v0.4.0 (failure diff logging). Full prescription engine: v0.5.0-v1.0.

**Risks**:
- Requires running an agent after failure, which costs tokens/time.
- "Causal" is a strong claim; what we actually have is correlation.
- Context prescriptions can become stale as code evolves.

**Seeds for v0.4.0**:
- On gate failure, log the pruning report alongside the gate result in history.
- Add a `gate_diagnose` MCP tool that shows "what context was pruned before this failing run?"
- Manual prescription: let users annotate files with `assay: protect` to override pruning.

---

## Idea 4: Spec Mutation Testing — Are Your Gates Actually Guarding?

**Name**: Gate Mutation Testing

**What**: Systematically verify that your spec gates catch real problems by introducing deliberate, targeted mutations and verifying gates fail. Borrowed from software mutation testing (`cargo mutants`), applied to quality gates.

Assay generates a set of targeted code mutations based on the spec's criteria descriptions (using an agent to translate natural-language criteria into mutation strategies), applies them in isolated worktrees, runs the spec, and reports which gates caught the mutation and which didn't.

A gate that fails to catch any mutation is a "surviving mutant" — it's testing the wrong thing, or it's too weak. The output is a **Gate Coverage Score**.

**Why This Changes the Game**:
- Most CI gates are never validated. They pass until they don't, and nobody knows if they'd catch real bugs.
- Gate Coverage Score is a new primary metric: "your quality gates have 73% mutation coverage."
- Gives teams objective evidence that their specs are meaningful vs. theater.
- Natural complement to code coverage: "code coverage tells you what was run; gate coverage tells you what was caught."

**Scope**: v0.5.0 or later. Needs stable worktree isolation (v0.3.0 ✓) + agent mutation generation.

**Risks**:
- Mutation generation is expensive (agent calls for each criterion).
- False positives: mutations that are syntactically valid but semantically equivalent.
- May feel adversarial to teams who've invested in gates ("your gates are weak").

**Seeds for v0.4.0**:
- Add a `gate_mutate` experimental command (worktree-isolated, no agent — just random comment removal).
- Prove the worktree isolation pattern works for mutation testing.
- Gather feedback before investing in agent-driven mutation generation.

---

## Idea 5: Evaluation Archaeology — Gate History as Semantic Database

**Name**: Quality Archaeology

**What**: Make gate history queryable as a semantic time series, not just a file system. The current history system stores JSON blobs; Archaeology indexes them into a lightweight embedded database (SQLite or equivalent) with structured queries:

- "Show me all runs where `security-audit` failed within 48 hours of a dependency update"
- "What's the correlation between context utilization % and gate pass rate?"
- "Which criteria has degraded most over the last 30 days?"

The MCP tool surfaces this as a natural-language query interface: `gate_query "show quality trends for the last 2 weeks"`.

Crucially: history isn't just per-project. With opt-in, Assay can contribute anonymous quality patterns to a **federated quality index** — "community benchmarks" for what passing rates look like for test gates, security gates, docs gates across the ecosystem.

**Why This Changes the Game**:
- Quality trends are invisible today. Teams know a gate is failing *now*; they don't see the slide.
- Federated benchmarks answer "is 73% pass rate for security gates normal?" with real data.
- Transforms Assay into infrastructure that compounds in value over time, not just a per-run tool.
- The "Postgres of agent quality" insight: Postgres is valuable because it stores + makes queryable, not just stores.

**Scope**: Local SQLite index in v0.4.0. Federation in v1.0.

**Risks**:
- SQLite adds a new dependency and migration concerns.
- Federated data collection is a privacy/trust problem. Opt-in must be explicit and verifiable.
- Query interface complexity creep — might fight the "simple CLI" product positioning.

**Seeds for v0.4.0**:
- Index existing JSON history into SQLite on first `gate_query` call.
- Implement 3 basic queries: trend over time, co-failure correlation, worst performing criteria.
- No federation yet. Prove local value first.

---

## Idea 6: Living Specs — Self-Amending Quality Contracts

**Name**: Adaptive Specs

**What**: Specs that propose their own amendments based on observed gate behavior. When a required criterion fails consistently (>60% failure rate over 10+ runs) without corresponding code fixes, Assay proposes: "This criterion appears too strict for current velocity. Suggest: downgrade to advisory, or refine criterion description to be more specific."

Conversely: when advisory criteria fail alongside critical bugs (detected via gate co-failure patterns), Assay proposes: "This advisory criterion correlates strongly with `build-failure`. Suggest: promote to required."

The spec never auto-modifies — it opens a PR-style diff showing the suggested amendment. Humans approve.

**Why This Changes the Game**:
- Specs get stale. Teams add criteria optimistically, then ignore them when they're inconvenient. Living Specs surfaces this instead of letting it rot silently.
- The direction reversal is radical: instead of teams maintaining specs, specs maintain themselves with human oversight.
- Creates a feedback loop between quality outcomes and quality policy — the core missing link in most CI/CD systems.

**Scope**: v0.5.0. Needs history indexing (Idea 5) + spec diff generation.

**Risks**:
- "Auto-amending specs" is scary. Risk of slowly weakening quality standards.
- The amendment logic could be gamed if teams optimize for "stop the nagging."
- Requires trust in the correlation analysis — which could be spurious.

**Seeds for v0.4.0**:
- Add `gate_health` MCP tool that shows per-criterion pass rate from history.
- Flag criteria with >60% failure rate as "potentially miscalibrated."
- No auto-proposals yet. Just surface the data.

---

## Cross-Cutting Theme

All six ideas share a unifying insight: **gates are data, not just checks**. The paradigm shift is from Assay as a *validator* (pass/fail) to Assay as a *quality intelligence system* (predict, explain, adapt, correlate). The v0.4.0 seeds are all about instrumenting the right data — the payoff is v1.0 when that data drives decisions.

The minimal v0.4.0 investment across all ideas:
1. `gate_predict`: frequency-based failure prediction from history
2. `uses:` spec composition field
3. Pruning report logged alongside gate failures
4. Worktree mutation testing (no agent)
5. Local SQLite history index + 3 queries
6. `gate_health` pass-rate per criterion
