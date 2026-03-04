# Features Research -- v0.2.0 (Run History, Gate Enforcement, Agent Evaluation, Hardening)

**Research Date:** 2026-03-02
**Scope:** How do run history, gate enforcement levels, and agent-submitted evaluations typically work in quality tools?
**Existing Base:** v0.1.0 ships gate evaluation (command execution, timeout, streaming, structured evidence), MCP server with 3 tools (spec_list, spec_get, gate_run), CLI commands, Claude Code plugin, and serializable domain types.

---

## Executive Summary

Research across CI/CD quality gate systems (SonarQube, GitHub Actions, GitLab CI), spec-driven agent tools (Kiro, agtx, agentic-orchestration), test result persistence patterns (nextest, dbt, Jest), and MCP ecosystem patterns reveals clear guidance for Assay's v0.2 features.

**Key findings:**

1. **Run history:** The industry has converged on NDJSON/compressed JSON Lines for local file-based persistence. SQLite is overkill for single-project tools. Nextest's model (append-only event log + bounded cache + parent-child run relationships) is the gold standard for Rust CLI tools.

2. **Gate enforcement levels:** The ecosystem splits cleanly into two models -- SonarQube abandoned warnings (binary pass/fail since v7.6), while GitLab CI's `allow_failure` pattern (required/advisory with visual differentiation) is the dominant UX. For agent workflows, the rjmurillo/ai-agents framework demonstrates a three-tier model (MUST/SHOULD/MAY) that maps well to Assay's dual-track design.

3. **Agent evaluation submissions:** No MCP-native `gate_report` pattern exists yet. The closest analogs are agentic-orchestration's critic pattern (structured pass/fail + retry logic) and the evaluation-rubrics MCP skill pattern. Assay's `gate_report` tool would be genuinely novel in the MCP ecosystem.

4. **Hardening:** Table stakes are actionable error messages, test coverage for all error paths, serde hygiene on all public types, and bounded MCP responses. The existing open issues inventory provides a ready-made hardening backlog.

---

## 1. Run History Persistence

### How Existing Tools Persist Run Results

| Tool | Storage Format | Location | Metadata Per Run | Retention |
|------|---------------|----------|-----------------|-----------|
| nextest | Zstd-compressed JSON Lines | `~/.cache/cargo-nextest/` | Run ID, timing, CLI args, env vars, parent run ID, pass/fail counts, compression stats | Bounded cache with auto-prune (daily, or 1.5x limit) |
| dbt | JSON (`run_results.json`) | `target/` directory | elapsed_time, per-node status/timing/thread_id, compiled_code, adapter_response | Overwritten each run (single file) |
| Jest | JSON (`--json` flag) | stdout or file | numPassedTestSuites, numFailedTests, startTime, testResults array | No built-in persistence |
| SonarQube | PostgreSQL database | Server-side | Analysis timestamp, quality gate status, metric values per condition, new vs overall code | Full history (server-managed) |
| agtx | SQLite database | `~/.local/share/agtx/` or `~/Library/Application Support/` | Task metadata, phase completion, worktree paths | Application-managed |
| agentic-orchestration | JSON files (state.json, actions.log) | Project `.agentic/` directory | Codebase snapshots, workflow progress, step execution, timestamps | Append-only log + state snapshot |

### Key Design Patterns

**Pattern A: Single Summary File (dbt model)**
- Overwrite `run_results.json` on each invocation
- Pro: Simple, no cache management, easy to parse
- Con: No history, no trend analysis, no parent-child runs
- Confidence: HIGH -- well-proven for "last run" queries

**Pattern B: Append-Only Event Log (nextest model)**
- NDJSON/JSON Lines with optional compression
- Each run appends entries; cache bounded by count/size limits
- Parent-child relationships enable rerun chains
- Pro: Full history, crash recovery (partial events preserved), streaming-friendly
- Con: Requires cache management, more complex implementation
- Confidence: HIGH -- nextest proves this works for Rust CLI tools at scale

**Pattern C: Database (agtx/SonarQube model)**
- SQLite or PostgreSQL for structured queries
- Pro: Complex queries, aggregation, joins
- Con: Binary dependency, migration management, overkill for single-project local tool
- Confidence: MEDIUM -- appropriate for server tools, overly complex for Assay's use case

### Recommended Approach for Assay

**Phase 1 (v0.2): Pattern A with history extension** -- Write each gate run as a single JSON file in `.assay/runs/`. Filename encodes timestamp and spec name (e.g., `2026-03-02T14-30-00_auth-flow.json`). This is simpler than NDJSON and provides natural filesystem-based history without cache management.

**Phase 2 (v0.3+): Migrate to Pattern B** -- If run history grows large enough to matter, migrate to NDJSON with bounded cache. The JSON-per-run pattern makes migration straightforward (each file becomes a line).

### Run Record Schema (Proposed)

```rust
pub struct GateRunRecord {
    /// Unique run identifier (ULID or UUID v7 for time-ordered IDs).
    pub run_id: String,
    /// Which spec was evaluated.
    pub spec_name: String,
    /// When the run started.
    pub started_at: DateTime<Utc>,
    /// When the run completed.
    pub completed_at: DateTime<Utc>,
    /// Total wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Aggregate pass/fail/skip counts.
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    /// Per-criterion results (reuse existing CriterionResult).
    pub criteria: Vec<CriterionResult>,
    /// What triggered this run (cli, mcp, hook).
    pub trigger: RunTrigger,
    /// Optional parent run ID (for reruns).
    pub parent_run_id: Option<String>,
    /// Git context at time of run (optional).
    pub git_ref: Option<String>,
}

pub enum RunTrigger {
    Cli,
    Mcp,
    Hook,
}
```

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| JSON file per run in `.assay/runs/` | Table Stakes | Medium | GateRunRecord type in assay-types, file I/O in assay-core | Natural filesystem history; `.gitignore` the runs dir |
| Run ID generation (ULID or UUID v7) | Table Stakes | Low | ulid or uuid crate | Time-ordered for natural sorting |
| `gate_run` persists results automatically | Table Stakes | Low | Run persistence in assay-core | Hook into existing evaluate_all |
| `assay history` CLI command (list recent runs) | Table Stakes | Medium | Run file scanning, display formatting | Show last N runs with pass/fail summary |
| `assay history show <run-id>` (detail view) | Table Stakes | Low | Run file loading | Display full criterion results |
| `gate_history` MCP tool (query run history) | Differentiator | Medium | Run persistence, MCP tool registration | Agents can ask "did this pass before?" |
| Run trigger tracking (cli/mcp/hook) | Differentiator | Low | RunTrigger enum | Useful for usage analytics |
| Git ref capture per run | Differentiator | Low | `git rev-parse HEAD` | Correlate runs with commits |
| Parent-child run relationships | Differentiator | Low | parent_run_id field | Track rerun chains |
| NDJSON migration with bounded cache | Anti-Feature (v0.2) | High | Compression, cache pruning | Defer until run volume justifies complexity |
| SQLite storage | Anti-Feature | High | rusqlite dependency, migrations | Overkill for local file-based tool |
| Run diffing (compare two runs) | Anti-Feature (v0.2) | Medium | Diff logic | Defer; the brainstorm noted agent context eviction makes delta refs unreliable |

---

## 2. Gate Enforcement Levels (Required vs Advisory)

### How Existing Tools Handle Enforcement Levels

**SonarQube (pass/fail only, since v7.6):**
- Removed warning threshold in v7.6 -- all conditions are either met or failed
- Quality gate is binary: "Passed" or "Failed"
- No intermediate states. The community has requested warnings repeatedly (discussion #43998) but SonarQube has deliberately refused
- Rationale: Warning thresholds were routinely ignored, creating false confidence
- Confidence: HIGH

**GitLab CI (`allow_failure` model):**
- Jobs default to `allow_failure: false` (required/blocking)
- Setting `allow_failure: true` makes a job advisory
- Advisory jobs that fail show an orange/yellow warning icon (not red)
- Pipeline status: "passed with warnings" when advisory jobs fail
- Pipeline continues to subsequent stages regardless of advisory failures
- Use cases: linters, style checks, flaky tests, non-critical environments
- Confidence: HIGH

**GitHub Actions (required status checks):**
- Branch protection rules define which checks are "required"
- Non-required checks can fail without blocking merge
- Annotations support three levels: `notice`, `warning`, `failure`
- Community has requested a "warning" status for checks (discussion #11592) -- not implemented
- Check runs have conclusion values: `success`, `failure`, `neutral`, `skipped`, `timed_out`, `action_required`
- The `neutral` conclusion is the closest to "advisory" -- it does not block merge
- Confidence: HIGH

**rjmurillo/ai-agents (three-tier RFC 2119 model):**
- **MUST (blocking):** Pre-commit hooks, pre-push phases, CI required checks -- exit code 1 halts workflow
- **SHOULD (advisory):** Memory export, retrospective documentation -- recommended but non-blocking
- **MAY (informational):** Security warnings, suggestions -- displayed but ignored for pass/fail
- Each requirement has a verification mechanism (file existence, tool output, exit code)
- Confidence: HIGH

**agentic-orchestration (critic + retry model):**
- Critic returns pass/fail per step
- On fail, step retries up to 3 times automatically
- After max retries, human approval gate activates
- No advisory/warning tier -- everything is blocking or escalated
- Confidence: MEDIUM

**Kiro (spec-driven with hooks):**
- Hooks enforce consistency and quality automatically
- Agent hooks run on specific triggers (file changes, code commits)
- Standards documented in steering docs rather than explicit enforcement tiers
- Quality is baked into spec structure (EARS format requirements) rather than runtime enforcement
- Confidence: MEDIUM

### Enforcement Level Design Patterns

| Pattern | Used By | Pros | Cons |
|---------|---------|------|------|
| Binary pass/fail | SonarQube, agentic-orchestration | Simple, unambiguous, no "warning fatigue" | All-or-nothing can frustrate incremental adoption |
| Required + Advisory | GitLab CI, GitHub Actions | Flexible, supports incremental adoption, visual differentiation | Advisory gates often get permanently ignored |
| Three-tier (MUST/SHOULD/MAY) | rjmurillo/ai-agents | Nuanced, maps to RFC 2119, good for documentation | Complex to implement, may confuse simple use cases |
| Critic + retry | agentic-orchestration | Self-healing, reduces false negatives | Only works with agent-evaluated gates, not deterministic |

### Recommended Approach for Assay

**Two-tier enforcement: `required` (default) and `advisory`.**

Rationale:
- SonarQube's experience shows three-tier (with warnings) creates noise. Two tiers is the sweet spot.
- GitLab CI's `allow_failure` pattern is the industry-proven UX for this.
- For agent workflows, `required` gates block completion (Stop hook prevents agent from finishing), `advisory` gates report but don't block.
- The default should be `required` -- making advisory opt-in prevents "everything is optional" drift.

### Spec-Level Configuration

```toml
# .assay/specs/auth-flow.toml
name = "auth-flow"

[[criteria]]
name = "tests-pass"
description = "All tests pass"
cmd = "cargo test"
# level defaults to "required" when omitted

[[criteria]]
name = "lint-clean"
description = "No clippy warnings"
cmd = "cargo clippy -- -D warnings"
level = "advisory"  # Fails won't block the gate

[[criteria]]
name = "docs-reviewed"
description = "API documentation is complete and accurate"
# No cmd -- agent-evaluated criterion (v0.2)
level = "required"
```

### UX Implications

| Scenario | Gate Result | CLI Display | MCP Response |
|----------|------------|-------------|--------------|
| All required pass, advisory pass | PASSED | Green checkmark per criterion | `"overall": "passed"` |
| All required pass, advisory fail | PASSED (with warnings) | Green overall, orange/yellow for advisory | `"overall": "passed_with_warnings"` |
| Any required fail | FAILED | Red for failed required criteria | `"overall": "failed"` |
| No required criteria defined | PASSED (trivially) | Warning: "no required criteria" | `"overall": "passed"`, `"warning": "no required criteria"` |

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| `level` field on Criterion (`required` / `advisory`) | Table Stakes | Low | Criterion type change in assay-types | Default to `required`; `#[serde(default)]` |
| GateRunSummary includes advisory pass/fail counts | Table Stakes | Low | Summary type changes | Separate counts: `advisory_passed`, `advisory_failed` |
| Overall gate status enum (Passed, PassedWithWarnings, Failed) | Table Stakes | Low | New enum in assay-types | Three-value rather than boolean |
| CLI displays advisory failures as warnings (different color) | Table Stakes | Medium | CLI display logic, color support | Orange/yellow for advisory, red for required |
| MCP response includes overall status + per-criterion level | Table Stakes | Low | Response struct changes | Agents need to know what's blocking vs informational |
| `deny_unknown_fields` compatibility for `level` | Table Stakes | Low | Serde configuration | Existing specs without `level` must still parse |
| Stop hook respects enforcement levels | Differentiator | Medium | Plugin hook logic | Only block on required failures |
| Spec-level default enforcement | Differentiator | Low | Spec type extension | `default_level = "advisory"` on spec |
| Enforcement override via CLI (`--all-required`) | Differentiator | Low | CLI flag, gate evaluation | Treat everything as required for CI mode |
| Per-run enforcement level snapshot | Differentiator | Low | Run record extension | Record effective levels at time of run |
| Three-tier enforcement (required/advisory/informational) | Anti-Feature | -- | -- | SonarQube's experience: warning fatigue. Two tiers is the sweet spot. |
| Enforcement as separate config (not on criterion) | Anti-Feature | -- | -- | Co-locate enforcement with what it applies to. Separate config files create drift. |

---

## 3. Agent-Submitted Evaluations (gate_report MCP Tool)

### How Existing Tools Handle Agent/External Evaluation Submissions

**agentic-orchestration (critic pattern):**
- After each workflow step, a critic agent validates the output
- Returns structured pass/fail with scoring
- Retry logic: on fail, step retries up to 3 times; after max retries, escalates to human
- State persisted in `state.json` and `actions.log`
- No explicit "submission" API -- the critic is an integral workflow step
- Confidence: HIGH

**rjmurillo/ai-agents (session logs):**
- Session creates `.agents/sessions/YYYY-MM-DD-session-NN.json`
- Contains RFC 2119 compliance evidence (tool outputs, file modifications)
- Checklist completion status for all MUST requirements
- Exit code semantics (0=success, 1=logic error, 2=config error, 3=external failure, 4=auth failure)
- Verification via `validate_session_json.py` script
- Confidence: HIGH

**MCPx-eval (structured evaluation):**
- Judge prompt structured with `<settings>`, `<prompt>`, `<output>`, `<check>`, `<expected-tools>`
- Evaluation criteria provided in `<check>` section
- Judge reviews agent output against criteria and expected tool usage
- Returns structured assessment
- Confidence: MEDIUM

**Kiro (specification-driven quality):**
- Quality baked into spec structure (EARS format: "when X, system shall Y")
- Hooks enforce quality on triggers (file changes, commits)
- No explicit submission API -- enforcement is environmental
- Confidence: MEDIUM

**MCP ecosystem patterns for receiving structured data:**
- MCP tools receive structured input via JSON Schema-validated `inputSchema`
- Return `CallToolResult` with content array (text, images, embedded resources)
- Error responses use `isError: true` with descriptive text
- No standard "submission" or "report" pattern exists in the MCP spec
- Confidence: HIGH

### The gate_report Pattern (Novel)

No existing MCP tool implements a "gate_report" submission pattern where an agent evaluates criteria and reports results back to the tool server. This makes Assay's `gate_report` genuinely novel.

The closest analogs are:
1. agentic-orchestration's critic pattern (but integrated, not submitted externally)
2. MCPx-eval's judge pattern (but evaluation-focused, not quality-gate-focused)
3. GitHub's Check Run API (external services submit check results, but via REST, not MCP)

### Proposed gate_report MCP Tool Design

```json
{
  "name": "gate_report",
  "description": "Submit an agent evaluation for criteria that require manual/AI assessment. Use this after reviewing code, documentation, or behavior against a criterion's description. Provide your assessment as passed/failed with evidence explaining your reasoning.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "spec_name": {
        "type": "string",
        "description": "Spec being evaluated (filename without .toml)"
      },
      "criterion_name": {
        "type": "string",
        "description": "Criterion within the spec being assessed"
      },
      "passed": {
        "type": "boolean",
        "description": "Whether the criterion is met in your assessment"
      },
      "reasoning": {
        "type": "string",
        "description": "Explanation of your assessment with specific evidence (what you checked, what you found)"
      },
      "confidence": {
        "type": "string",
        "enum": ["high", "medium", "low"],
        "description": "How confident you are in this assessment"
      }
    },
    "required": ["spec_name", "criterion_name", "passed", "reasoning"]
  }
}
```

### Agent Evaluation Record

```rust
pub struct AgentEvaluation {
    /// Which criterion was evaluated.
    pub spec_name: String,
    pub criterion_name: String,
    /// Agent's assessment.
    pub passed: bool,
    pub reasoning: String,
    pub confidence: Option<EvalConfidence>,
    /// When the evaluation was submitted.
    pub timestamp: DateTime<Utc>,
    /// How this evaluation was submitted.
    pub source: EvalSource,
}

pub enum EvalConfidence {
    High,
    Medium,
    Low,
}

pub enum EvalSource {
    /// Submitted via gate_report MCP tool.
    Mcp,
    /// Submitted via CLI (future).
    Cli,
}
```

### Integration with Gate Runs

When `gate_run` encounters a criterion without `cmd`:
- **v0.1 behavior (current):** Skipped, counted in `skipped` total
- **v0.2 behavior (proposed):** Check if a recent `gate_report` exists for this criterion
  - If yes and passed: count as passed
  - If yes and failed: count as failed with agent's reasoning as evidence
  - If no report: count as "pending" (new status, distinct from skipped)

This creates the dual-track evaluation model: deterministic gates auto-evaluate, agent-evaluated gates require explicit submission.

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| `gate_report` MCP tool (submit evaluation) | Table Stakes | Medium | AgentEvaluation type, run persistence | Core v0.2 differentiator |
| AgentEvaluation type in assay-types | Table Stakes | Low | Serde + JsonSchema derives | Flat struct, no nested objects |
| Evaluation persistence (JSON file per eval) | Table Stakes | Medium | File I/O in assay-core | Store in `.assay/evaluations/` |
| gate_run integrates agent evaluations | Table Stakes | Medium | Evaluation lookup, gate evaluation changes | Check for recent reports when criterion has no cmd |
| "pending" criterion status (no cmd, no report) | Table Stakes | Low | CriterionStatus changes | Distinct from "skipped" |
| Confidence field on evaluations | Differentiator | Low | EvalConfidence enum | Agents self-report uncertainty |
| Evaluation staleness (expire after N minutes/commits) | Differentiator | Medium | Timestamp comparison, optional git integration | Prevent stale evaluations from persisting |
| `assay eval list` CLI command | Differentiator | Low | Evaluation file scanning | Show recent evaluations |
| Evaluation history per criterion | Differentiator | Low | File naming convention | Track evaluation changes over time |
| `prompt` field on Criterion (LLM-evaluated) | Anti-Feature (v0.2) | High | LLM API dependency, subprocess | v0.3+ feature; gate_report is the v0.2 mechanism |
| Automated re-evaluation triggers | Anti-Feature | High | File watching, event system | YAGNI; agent decides when to re-evaluate |
| Multi-agent consensus (multiple evaluations per criterion) | Anti-Feature (v0.2) | High | Aggregation logic | Interesting but complex; defer |

---

## 4. Foundation Hardening

### What Quality Gate Tools Need for Production Readiness

Research across CI/CD tools, Rust CLI best practices, and the existing open issues inventory reveals consistent hardening requirements.

**Error Message Quality (from quality gate UX research):**
- Actionable: Tell the user what went wrong AND what to do about it
- Contextual: Include file paths, line numbers, command that failed
- Structured: Machine-parseable for agents, human-readable for developers
- SonarQube: "detailed dashboards provide actionable insights"
- JetBrains Qodana: "comprehensive reports highlighting potential issues"

**Test Coverage (from Rust ecosystem):**
- cargo-tarpaulin or grcov for coverage measurement
- Coverage targets: 80%+ for libraries, 70%+ for CLI (integration tests)
- Focus on error path coverage (the code that matters most is the code that fails)
- nextest: "even at default Zstandard level 3, compression is very efficient" -- they test compression

**Serde Hygiene (from compression brainstorm, already identified):**
- `#[serde(skip_serializing_if)]` on all Option/String/Vec fields
- `#[serde(deny_unknown_fields)]` on all input types
- `#[serde(default)]` on optional fields for forward compatibility
- Estimated 10-30% token savings on MCP responses

**Error Type Completeness (from existing open issues):**
- The `AssayError` enum needs variants for all failure modes
- Error context chaining via thiserror `#[source]`
- `#[non_exhaustive]` already present (good)

### Existing Open Issues Relevant to Hardening

From `.planning/issues/open/`:
- `2026-03-01-core-error-types.md` -- Error type refinements
- `2026-03-01-cli-error-propagation.md` -- CLI error handling
- `2026-03-01-test-coverage-gaps-phase3.md` -- Phase 3 test gaps
- `2026-03-01-test-coverage-gaps-phase6.md` -- Phase 6 test gaps
- `2026-03-01-type-invariant-enforcement.md` -- Type safety improvements
- `2026-03-01-error-ergonomics.md` -- Error UX improvements
- `2026-03-01-serde-hygiene.md` (from compression brainstorm) -- Serialization cleanup
- `2026-03-02-gate-pr-review-suggestions.md` -- Gate module quality improvements

### Hardening Checklist

| Area | Current State | v0.2 Target | Notes |
|------|--------------|-------------|-------|
| Error messages | Functional but inconsistent | All errors include context + suggestion | "Did you mean...?" for spec not found |
| Test coverage | Good for happy paths, gaps in error paths | 80%+ overall, all error paths tested | Pipe read errors, thread panics, spawn failures |
| Serde hygiene | Mostly done (`skip_serializing_if` present) | 100% coverage on all public types | Audit every struct in assay-types |
| Type invariants | `deny_unknown_fields` on most types | All input types validated | Trim-then-validate for all string fields |
| CLI error display | color-eyre | Consistent formatting, no raw panics | Error codes for scripting |
| MCP error responses | `isError: true` with text | Structured error codes + suggestions | Agents need to know which errors are retryable |

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| Audit and fix all error messages (context + suggestion) | Table Stakes | Medium | AssayError variants | "spec `foo` not found in .assay/specs/ -- did you mean `foo-bar`?" |
| Test error paths: pipe read, thread panic, spawn failure | Table Stakes | Medium | Test infrastructure | From gate PR review suggestions |
| Test truncation with multi-byte UTF-8 | Table Stakes | Low | Test case | From gate PR review suggestions |
| Serde hygiene audit on all public types | Table Stakes | Low | Mechanical review | skip_serializing_if, default, deny_unknown_fields |
| MCP error codes (retryable vs permanent) | Table Stakes | Medium | Error classification | Agents need to know if retry will help |
| CLI exit codes (0=ok, 1=gate-fail, 2=config-error) | Table Stakes | Low | CLI main() changes | Standard Unix convention |
| `GateRunSummary.total` convenience field | Table Stakes | Low | Type change | passed + failed + skipped |
| CriterionStatus enum (Passed/Failed/Skipped) | Differentiator | Low | Type redesign | Stronger than Option<GateResult> |
| Fuzzy spec name matching ("did you mean...?") | Differentiator | Medium | String similarity (strsim crate) | High UX value for agents |
| Error telemetry / diagnostics dump | Anti-Feature | Medium | Privacy concerns | YAGNI for local tool |
| Automated error reporting | Anti-Feature | -- | -- | Local tool, no telemetry |

---

## 5. Agent Workflow Quality Enforcement Patterns

### How Agent-First Tools Enforce Quality

**agtx (artifact-based enforcement):**
- Phase completion requires artifact file existence (e.g., `.agtx/plan.md`)
- Polling-based detection: spinner shows checkmark when artifact appears
- No structured quality assessment -- binary "artifact exists or not"
- Worktree isolation per task prevents cross-contamination
- Confidence: HIGH

**Kiro (spec-driven enforcement):**
- EARS format requirements ("when X, system shall Y") enable formal verification
- Hooks run automatically on triggers (file changes, commits)
- Steering docs define standards -- enforced by agent behavior, not runtime gates
- Three autonomous agents: coding, security, DevOps -- each with quality focus
- Confidence: HIGH

**rjmurillo/ai-agents (defense-in-depth):**
- Four layers: protocol docs, PreToolUse hooks, git hooks, CI/CD
- Routing gates embedded in agent instructions (cannot be skipped unlike `--no-verify`)
- Transcript evidence requirements: agent must show tool outputs proving compliance
- Session validation script produces JSON compliance reports
- Confidence: HIGH

**Addy Osmani's "Agents Need a Manager" pattern:**
- Mandatory test execution: agents must run tests and include output
- Structured PR packets: changes, reasoning, files, tests, risks
- Two-agent pattern: implementer + reviewer
- Delegation levels: fully delegate / delegate with checkpoints / retain human ownership
- Confidence: HIGH

**agentic-orchestration (critic loop):**
- Critic validates structure + semantic quality after each step
- Pass/fail scoring with automatic retry (up to 3 attempts)
- Human approval gates for high-impact decisions (architecture, tech stack)
- Resumable execution from `state.json` -- no lost work on failure
- Confidence: HIGH

### Patterns Relevant to Assay v0.2

| Pattern | Applicable? | How It Maps to Assay |
|---------|------------|---------------------|
| Artifact-based completion (agtx) | Partially | FileExists gate already exists; could extend to "spec completion = all gates pass" |
| Hooks as enforcement (Kiro, rjmurillo) | Yes | PostToolUse and Stop hooks in Claude Code plugin |
| Transcript evidence (rjmurillo) | Yes | gate_report reasoning field IS transcript evidence |
| Critic + retry (agentic-orchestration) | Future | v0.3+ with LLM integration; v0.2 uses manual agent submission |
| Two-agent pattern (Osmani) | Future | Assay could support reviewer agent as evaluator; not v0.2 scope |
| Structured PR packets (Osmani) | Yes | GateRunSummary IS a structured quality report |
| Resumable execution (agentic-orchestration) | Partially | Run history enables "resume from last gate" pattern |

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| Stop hook blocks on required gate failures | Table Stakes | Medium | Plugin hook, gate_run integration | Core agent enforcement mechanism |
| PostToolUse hook runs gates after code changes | Differentiator | Medium | Plugin hook, trigger detection | Auto-quality-check on file writes |
| `gate_status` MCP tool (current pass/fail state) | Differentiator | Medium | Run history, evaluation persistence | Agents query "am I done?" without re-running |
| Run history enables "last known state" queries | Differentiator | Low | Run persistence | "Did tests pass last time?" |
| Structured quality summary in gate_run response | Table Stakes | Low | Already exists | GateRunSummary is the structured PR packet |
| Agent evaluation as transcript evidence | Differentiator | Low | gate_report reasoning field | Agent must explain its assessment |
| Auto-retry on gate failure (re-run after fix) | Anti-Feature (v0.2) | High | Workflow orchestration | Assay is a quality tool, not an orchestrator |
| Multi-agent coordination | Anti-Feature (v0.2) | High | Agent management | Orchestration is agtx's job, not Assay's |
| Workflow state machine | Anti-Feature (v0.2) | High | State management | Deferred; spec-driven, not workflow-driven |

---

## Cross-Cutting Themes

### 1. Two-Tier Enforcement Is The Industry Standard

SonarQube tried three tiers (pass/warning/fail) and removed warnings. GitLab CI uses two tiers (required + advisory). GitHub Actions uses two tiers (required + non-required). Every tool that shipped warnings eventually regretted it. Assay should ship `required` (default) and `advisory`, nothing more.

### 2. File-Based History Beats Databases for Local Tools

Every local-first tool (nextest, dbt, agentic-orchestration) uses JSON/NDJSON files. Every server tool (SonarQube) uses a database. Assay is a local-first tool. JSON files in `.assay/runs/` is the correct choice.

### 3. Agent Evaluation Is Genuinely Novel

No MCP tool in the ecosystem implements a "gate_report" submission pattern. The closest analogs are integrated critic loops (agentic-orchestration) and evaluation frameworks (MCPx-eval). Assay's gate_report creates a new pattern: **externalized quality assessment with structured evidence, submitted by the same agent that did the work**. This is the v0.2 differentiator.

### 4. Hardening Is Not Optional

The existing open issues inventory contains 13+ hardening items. Every quality gate tool's credibility depends on its own quality. An error-prone quality tool is a contradiction. v0.2 must ship hardening alongside new features, not defer it.

### 5. Enforcement Must Be Environmental, Not Trust-Based

The rjmurillo/ai-agents framework's key insight: "labels like 'MANDATORY' are insufficient. Each requirement MUST have a verification mechanism." For Assay, this means:
- Required gates must actually block (Stop hook, CLI exit code)
- Advisory failures must be visually distinct (color, status text)
- Agent evaluations must include reasoning (not just pass/fail)
- Absence of evaluation is "pending," not "passed"

---

## Dependency Map

```
Run History Persistence ────────────────────────────────┐
     │                                                   │
     ├─── GateRunRecord type (assay-types)               │
     │         │                                         │
     │         └─── Run file I/O (assay-core)            │
     │               │                                   │
     ├─── gate_run auto-persists ◄───────────────────────┤
     │                                                   │
     ▼                                                   │
Enforcement Levels ──────────────────────────────────┐   │
     │                                               │   │
     ├─── `level` field on Criterion                 │   │
     │         │                                     │   │
     │         ├─── GateRunSummary changes            │   │
     │         │                                     │   │
     │         └─── OverallGateStatus enum            │   │
     │               │                               │   │
     │               ├─── CLI display (colors)        │   │
     │               └─── MCP response changes        │   │
     │                                               │   │
     ▼                                               │   │
Agent Evaluation (gate_report) ──────────────────┐   │   │
     │                                           │   │   │
     ├─── AgentEvaluation type                   │   │   │
     │         │                                 │   │   │
     │         ├─── Evaluation persistence        │   │   │
     │         │                                 │   │   │
     │         └─── gate_run integrates evals     │   │   │
     │                                           │   │   │
     ▼                                           │   │   │
Hardening ◄──────────────────────────────────────┘───┘───┘
     │    (applies to all above)
     ├─── Error message audit
     ├─── Test coverage for error paths
     ├─── Serde hygiene audit
     ├─── MCP error codes
     └─── CLI exit codes
```

### Recommended Phase Ordering

1. **Hardening first** -- Fix error types, test gaps, serde hygiene. This builds confidence for the type changes that follow.
2. **Enforcement levels** -- Add `level` field to Criterion, update GateRunSummary. Small type change, big behavioral impact.
3. **Run history** -- Add GateRunRecord, persistence, CLI commands. Depends on enforcement levels for correct status recording.
4. **Agent evaluation** -- Add gate_report MCP tool, AgentEvaluation type, integration with gate_run. Depends on run history for persistence pattern.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `level` field breaks existing TOML specs | Low | Medium | `#[serde(default = "default_required")]` -- omitted field = required |
| Agent evaluations are unreliable/hallucinated | Medium | Medium | Confidence field + reasoning requirement + staleness expiry |
| Run history grows unbounded | Low | Low | Cache limits (count/size), or `.gitignore` the runs dir |
| `gate_report` UX confuses agents | Medium | Medium | Clear tool description, schema validation, examples in SKILL.md |
| Hardening scope creep delays features | Medium | Medium | Fixed hardening budget (e.g., 1 week), prioritize by open issues |
| Breaking changes to GateRunSummary serialization | Low | Medium | Additive changes only (new fields with defaults), no field removals |

---

## Summary: v0.2.0 Feature Set

### Table Stakes (Must Ship)

1. **Enforcement levels** -- `level` field on Criterion (required/advisory), OverallGateStatus enum
2. **Run history persistence** -- JSON file per run in `.assay/runs/`, GateRunRecord type
3. **gate_report MCP tool** -- Agent submits structured evaluation for non-command criteria
4. **AgentEvaluation type** -- Flat struct with passed/reasoning/confidence/timestamp
5. **gate_run integrates evaluations** -- Non-command criteria check for recent agent reports
6. **"pending" criterion status** -- Distinct from "skipped" for unevaluated criteria
7. **Error message audit** -- All errors include context + suggestion
8. **Test error paths** -- Pipe read, thread panic, spawn failure, multi-byte truncation
9. **Serde hygiene audit** -- 100% coverage on public types
10. **CLI exit codes** -- 0=ok, 1=gate-fail, 2=config-error
11. **`assay history` CLI command** -- List recent runs
12. **MCP response includes enforcement level** -- Agents know what's blocking

### Differentiators (Should Ship If Time Allows)

1. **gate_history MCP tool** -- Agents query "did this pass before?"
2. **gate_status MCP tool** -- Current pass/fail state without re-running
3. **Evaluation staleness** -- Agent evaluations expire after N minutes/commits
4. **Git ref capture per run** -- Correlate runs with commits
5. **Fuzzy spec name matching** -- "did you mean...?" for typos
6. **CriterionStatus enum** -- Replace Option<GateResult> with Passed/Failed/Skipped/Pending
7. **Stop hook respects enforcement levels** -- Only block on required failures
8. **PostToolUse hook auto-gates** -- Auto-quality-check after code changes
9. **Confidence field on evaluations** -- Agents self-report uncertainty
10. **CLI `--all-required` flag** -- Treat everything as required for CI mode

### Anti-Features (Explicitly Out of Scope for v0.2)

1. **SQLite storage** -- Overkill for local file-based tool
2. **NDJSON with compression** -- Premature optimization; JSON-per-run sufficient
3. **Three-tier enforcement** -- Warning fatigue; two tiers is the industry lesson
4. **`prompt` field on Criterion** -- LLM API dependency; gate_report is the v0.2 mechanism
5. **Automated re-evaluation triggers** -- Agent decides when to re-evaluate
6. **Multi-agent consensus** -- Multiple evaluations per criterion is v0.3+ complexity
7. **Workflow state machine** -- Assay is a quality tool, not an orchestrator
8. **Auto-retry on gate failure** -- Orchestration is agtx's job
9. **Run diffing** -- Agent context eviction makes delta refs unreliable
10. **Error telemetry** -- Local tool, no phone-home

---

## Sources

- [SonarQube Quality Gates (2025.3)](https://docs.sonarsource.com/sonarqube-server/2025.3/quality-standards-administration/managing-quality-gates/introduction-to-quality-gates/)
- [SonarQube Warning Threshold Discussion](https://community.sonarsource.com/t/quality-gate-condition-warning/43998)
- [GitLab CI allow_failure](https://www.bestdevops.com/gitlab-pipeline-allow_failure-what-is-allow_failure-in-gitlab-ci-cd/)
- [GitHub Actions Status Checks](https://docs.github.com/articles/about-status-checks)
- [GitHub Warning Status Discussion](https://github.com/orgs/community/discussions/11592)
- [GitHub Required Checks for Conditional Jobs](https://devopsdirective.com/posts/2025/08/github-actions-required-checks-for-conditional-jobs/)
- [nextest Recording Runs](https://nexte.st/docs/design/architecture/recording-runs/)
- [dbt Run Results JSON](https://docs.getdbt.com/reference/artifacts/run-results-json)
- [agtx (GitHub)](https://github.com/fynnfluegge/agtx)
- [agentic-orchestration (GitHub)](https://github.com/gbFinch/agentic-orchestration)
- [rjmurillo/ai-agents Quality Gates](https://deepwiki.com/rjmurillo/ai-agents/7.1-skill-architecture-and-frontmatter)
- [Kiro Spec-Driven Development (AWS)](https://www.infoq.com/news/2025/08/aws-kiro-spec-driven-agent/)
- [Addy Osmani: Your AI Coding Agents Need a Manager](https://addyosmani.com/blog/coding-agents-manager/)
- [MCPx-eval Evaluation Framework](https://docs.mcp.run/blog/2025/03/03/introducing-mcpx-eval/)
- [MCP-Bench: Benchmarking Tool-Using Agents](https://arxiv.org/pdf/2508.20453)
- [Augment Code: Autonomous Quality Gates](https://www.augmentcode.com/guides/autonomous-quality-gates-ai-powered-code-review)
- [Quality Gates in Software Development (CEUR)](https://ceur-ws.org/Vol-3845/paper06.pdf)
- [Rust Error Handling Guide 2025](https://markaicode.com/rust-error-handling-2025-guide/)
- [cargo-tarpaulin (Rust Coverage)](https://github.com/xd009642/tarpaulin)
- [NDJSON Serialization with Serde](https://users.rust-lang.org/t/serializing-to-ndjson-with-serde/35330)
- [ndjson-stream Crate](https://docs.rs/ndjson-stream)

---

*Research completed: 2026-03-02*
