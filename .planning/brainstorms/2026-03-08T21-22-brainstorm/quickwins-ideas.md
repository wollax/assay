# Quick Wins Ideas — Assay v0.3.0

*Explorer: explorer-quickwins*
*Date: 2026-03-08*

---

## 1. Types Hygiene Batch

**What:** Batch-fix all `assay-types` issues in a single sweep: add `Eq` derives alongside `PartialEq`, add `#[serde(default)]` to optional fields, add `#[deny(unknown_fields)]` where appropriate, add missing doc comments, and implement `Display` for `Enforcement` and other enums. There are ~18 open issues in `assay-types` alone — most are mechanical derive/attribute additions.

**Why:** These are individually trivial but collectively create a "death by a thousand cuts" maintenance burden. Batching them eliminates 15-18 issues in one PR, dramatically shrinking the open issue count (106 → ~88) and making the types crate a clean foundation for v0.3.0 work. Every downstream crate benefits from cleaner types.

**Scope:** 1 day. Mechanical changes with high test coverage already in place (493 tests). Low cognitive load per change.

**Risks:** Serde defaults could subtly change deserialization behavior for existing stored gate run records. Need to verify backwards compatibility with existing `.assay/` history files. `deny_unknown_fields` could break forward compatibility if agents add unexpected fields.

---

## 2. CLI Polish Sprint

**What:** Bundle the 14 CLI issues into one focused sprint: consolidate help text duplication (gate help, spec show color branches), fix `NO_COLOR` to use `var_os()`, extract ANSI overhead to a computed constant, add `StreamCounters` doc comments and `tally()`/`gate_blocked()` methods, rename `failed` → `failing` field, fix enforcement check duplication, add command separator constant.

**Why:** The CLI is the primary human interface — every rough edge here is multiplied by every user interaction. These are all well-scoped, independently testable changes. Completing them gives the CLI a "polished" feel for v0.3.0. Several of these (NO_COLOR fix, help duplication) are correctness issues, not just cosmetics.

**Scope:** 2 days. Each individual fix is 15-60 minutes. Can be parallelized across files since changes are mostly independent.

**Risks:** Help text changes need manual review to ensure nothing important is lost. `StreamCounters` rename (`failed` → `failing`) needs coordinated update across CLI and any consumers. Low blast radius overall.

---

## 3. `assay doctor` Command

**What:** Add an `assay doctor` (or `assay check`) command that validates the project setup: checks `.assay/` directory structure exists, validates spec TOML syntax for all specs, verifies gate commands are executable, checks history directory permissions, validates config against schema. Reports issues as a checklist with pass/fail/warn status.

**Why:** Currently, users discover problems only when they run a specific command that hits the broken thing. A doctor command provides proactive diagnostics — especially valuable for onboarding new projects or debugging "why isn't my gate running?" Common pattern in mature CLIs (brew doctor, flutter doctor, rustup check). Also useful for CI — `assay doctor` as a pipeline step catches config rot.

**Scope:** 1-2 days. Most validation logic already exists in `assay-core` (spec scanning, config loading). The command is primarily orchestration — call existing functions, format results. Can start minimal and grow.

**Risks:** Scope creep — "doctor" commands tend to accumulate checks endlessly. Need to define a clear initial checklist and resist adding more in the same PR. False positives could erode trust.

---

## 4. `gate_run` Output Truncation (Streaming Capture Lite)

**What:** Implement a lightweight version of the Phase 7 streaming capture issue — add a configurable `max_output_bytes` (default 32KB) to gate command execution. When stdout/stderr exceeds the limit, capture first N + last N bytes with a `[truncated: X bytes omitted]` marker. No fancy marker scanning yet — just head+tail truncation.

**Why:** This is the single highest-impact DX improvement for agent users. Unbounded command output is the #1 source of token waste and context window blowouts. The full Phase 7 solution (marker scanning, exit-code-aware budgets) is complex, but the 80/20 version — simple byte-limited capture — solves the immediate pain. Can land in v0.3.0 and be refined later.

**Scope:** 2-3 days. Requires changing `Command::output()` to `Command::spawn()` + `BufReader` with byte counting. Need to handle both stdout and stderr independently. Config plumbing through `gate.defaults` in spec TOML.

**Risks:** Truncation could hide the actual error message if it's in the middle of output. Head+tail is a reasonable heuristic but not perfect. Could break snapshot tests that assert on full output. Need to ensure truncation doesn't split multi-byte UTF-8 sequences.

---

## 5. Guard Daemon Hardening Batch

**What:** Batch-fix the 15 guard daemon issues: add `Debug` derives, implement `Display` for `GuardStatus`, extract duplicated `context_pct` helper, add `fsync` to PID file writes, document temp file suffixes in watcher, derive `Ord` for threshold levels, fix circuit breaker reset logic, replace `eprintln!` with `tracing`, add `#[instrument]` annotations.

**Why:** The guard daemon is the newest and least-polished component (shipped in v0.2.0's final phase). These issues are all low-risk, mechanical improvements that make the daemon production-ready. The daemon runs as a background process — reliability and observability (tracing, proper error handling) matter more here than in interactive commands.

**Scope:** 2 days. Similar to the types batch — each individual fix is small, and they can be grouped by file for efficient review. The `fsync` and circuit breaker fixes are the most important for correctness.

**Risks:** Guard daemon changes are harder to test manually (requires running daemon, simulating sessions). The circuit breaker reset logic fix needs careful design — incorrect reset could cause either over-pruning or no pruning. `tracing` replacement of `eprintln!` changes the output channel, which could surprise users who grep stderr.

---

## 6. MCP Tool Input Validation & Error Messages

**What:** Add proper input validation to all MCP tools with clear, actionable error messages. Currently, invalid inputs (missing spec name, malformed parameters) produce generic or internal errors. Add: spec name existence check with "did you mean?" suggestions (fuzzy matching against known specs), parameter type validation with specific messages, and document all tool parameters in the MCP tool descriptions.

**Why:** AI agents are the primary MCP consumers, and they learn from error messages. A vague error like "spec not found" wastes an agent retry cycle. "Spec 'auth-flw' not found. Available specs: auth-flow, data-pipeline" lets the agent self-correct in one turn. This is high-leverage DX for the agentic workflow — every saved retry is saved tokens and time.

**Scope:** 1-2 days. The MCP server is only 5 tools with simple parameter types. Fuzzy matching can use `strsim` crate or simple Levenshtein. Most work is in crafting good error message templates.

**Risks:** Adding a fuzzy matching dependency increases compile time slightly. Over-eager suggestions could confuse agents. Need to keep error messages concise — agents have limited context budgets too.

---

## 7. Issue Triage & Consolidation

**What:** Triage the 106 open issues: merge duplicates (several issues reference the same underlying problem from different angles), close issues that were fixed as side effects of other work, group related issues into "batch fix" meta-issues, and add priority labels. Target: reduce open issue count by 30-40% through consolidation alone, without writing any code.

**Why:** 106 open issues is psychologically overwhelming and makes prioritization impossible. Many issues are micro-observations from PR reviews that should be grouped (e.g., "all StreamCounters improvements" is one unit of work, not 5 separate issues). Consolidation makes the backlog actionable and helps identify which batches to tackle in v0.3.0 vs defer.

**Scope:** 0.5 days. Pure organizational work — read each issue, identify duplicates and groups, update or close files. No code changes.

**Risks:** Over-aggressive consolidation could lose nuance from individual issues. Closing issues prematurely (thinking they're fixed) could let bugs slip. Should verify "fixed as side effect" claims with a quick code check.
