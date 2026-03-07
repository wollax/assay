# Roadmap: Assay

## Milestones

<details>
<summary>✅ v0.1.0 Proof of Concept — SHIPPED 2026-03-02</summary>

**Goal:** Prove Assay's dual-track gate differentiator through a thin vertical slice — foundation types, spec-driven gates, MCP server, and Claude Code plugin.

- [x] Phase 1: Workspace Prerequisites (1 plan) — 2026-02-28
- [x] Phase 2: MCP Spike (1 plan) — 2026-02-28
- [x] Phase 3: Error Types and Domain Model (2 plans) — 2026-02-28
- [x] Phase 4: Schema Generation (1 plan) — 2026-02-28
- [x] Phase 5: Config and Initialization (3 plans) — 2026-03-01
- [x] Phase 6: Spec Files (2 plans) — 2026-03-01
- [x] Phase 7: Gate Evaluation (2 plans) — 2026-03-01
- [x] Phase 8: MCP Server Tools (2 plans) — 2026-03-01
- [x] Phase 9: CLI Surface Completion (2 plans) — 2026-03-02
- [x] Phase 10: Claude Code Plugin (2 plans) — 2026-03-02

[Full archive](milestones/v0.1.0-ROADMAP.md)

</details>

### 🔄 v0.2.0 Dual-Track Gates & Hardening

**Goal:** Ship agent-evaluated gates (via MCP `gate_report` tool), run history persistence, required/advisory gate enforcement, and comprehensive hardening of the v0.1 foundation.

- [x] Phase 11: Type System Foundation (2 plans) — 2026-03-04
  - [x] 11-01: Type relocation + serde hygiene
  - [x] 11-02: Schema snapshots, roundtrip tests, schema regeneration
- [x] Phase 12: FileExists Gate Wiring (1 plan) — 2026-03-04
  - [x] 12-01: Add path field, wire dispatch, update tests and snapshots
- [x] Phase 13: Enforcement Levels — 2026-03-04
  - [x] 13-01: Enforcement type layer (Enforcement enum, GateSection, EnforcementSummary)
  - [x] 13-02: Enforcement evaluation logic (resolve_enforcement, enforcement-aware evaluation, validation)
  - [x] 13-03: CLI enforcement integration (exit codes, spec template, schema snapshots)
- [x] Phase 14: Run History Core — 2026-03-05
  - [x] 14-01: GateRunRecord type and history persistence module
  - [x] 14-02: History persistence integration tests
- [x] Phase 15: Run History CLI — 2026-03-05
  - [x] 15-01: Config extension, pruning, and save() update
  - [x] 15-02: CLI history command, table/detail views, gate run integration
- [x] Phase 16: Agent Gate Recording — 2026-03-05
  - [x] 16-01: Agent gate types and criterion extension
  - [x] 16-02: Core evaluation dispatch and validation
  - [x] 16-03: MCP gate_report and gate_finalize tools
  - [x] 16-04: Visual distinction, schema snapshots, quality gate
- [x] Phase 17: MCP Hardening & Agent History — 2026-03-05
  - [x] 17-01: MCP tool hardening (timeout, path validation, error envelope, enforcement counts)
  - [x] 17-02: gate_history tool and response struct documentation
- [x] Phase 18: CLI Hardening & Enforcement Surface — 2026-03-05
  - [x] 18-01: CLI error propagation foundation (anyhow, ASSAY_DIR_NAME, run() pattern)
  - [x] 18-02: Enforcement-aware streaming output (warned counter, advisory labels)
- [x] Phase 19: Testing & Tooling — 2026-03-06
  - [x] 19-01: Tighten cargo-deny policies (multiple-versions deny, sources deny)
  - [x] 19-02: MCP handler tests & open issue triage
  - [x] 19-03: Dogfooding spec (self-check.toml)
- [x] Phase 20: Session JSONL Parser & Token Diagnostics — 2026-03-06
  - [x] 20-01: Context types in assay-types (SessionEntry, UsageData, DiagnosticsReport, etc.)
  - [x] 20-02: Core session parser, discovery, token extraction, diagnostics engine
  - [x] 20-03: CLI `assay context diagnose` and `assay context list` commands
  - [x] 20-04: MCP `context_diagnose` and `estimate_tokens` tools
  - [x] 20-05: Quality gate verification and smoke tests
- [x] Phase 21: Team State Checkpointing — 2026-03-06
  - [x] 21-01: Checkpoint types (assay-types) and core extraction + persistence (assay-core)
  - [x] 21-02: CLI `assay checkpoint save|show|list` commands
  - [x] 21-03: Plugin checkpoint hook script and hooks.json integration
- [x] Phase 22: Pruning Engine — 2026-03-06
  - [x] 22-01: Foundation types, raw_line preservation, protection set, module skeleton
  - [x] 22-02: Line-deletion strategies (progress-collapse, stale-reads)
  - [x] 22-03: Content-modification strategies (thinking-blocks, metadata-strip, tool-output-trim, system-reminder-dedup)
  - [x] 22-04: Pipeline executor, backup/restore, dry-run report
  - [x] 22-05: CLI `assay context prune` integration
- [x] Phase 23: Guard Daemon & Recovery — 2026-03-07
  - [x] 23-01: Guard config types, PID file management, threshold evaluation, error variants
  - [x] 23-02: Circuit breaker state machine and escalating prescriptions
  - [x] 23-03: Daemon event loop, file system watcher, public API (start/stop/status)
  - [x] 23-04: CLI `assay context guard` commands, schema snapshots, quality gate
- [ ] Phase 24: MCP History Persistence Fix
  - [ ] 24-01: Add history save to MCP gate_run for command-only specs, integration test
- [ ] Phase 25: Tech Debt Cleanup
  - [ ] 25-01: Missing VERIFICATION.md backfill (phases 16, 19, 20)
  - [ ] 25-02: Open issues triage and cleanup

---

## Progress Summary

| Milestone | Status | Phases | Requirements | Complete |
|-----------|--------|--------|--------------|----------|
| v0.1.0 Proof of Concept | Shipped | 10 | 43 | 100% |
| v0.2.0 Dual-Track Gates & Hardening | In Progress | 15 | 52 | ~87% |
| v0.3.0 | Planned | — | — | — |

---

## v0.2.0 Phase Details

### Phase 11: Type System Foundation

**Goal:** Relocate result types from assay-core to assay-types and enforce serde hygiene across all domain types — the highest-churn refactor that must land before any feature work.

**Depends on:** None (first phase of v0.2.0)

**Requirements:**
- TYPE-01: `GateRunSummary` and `CriterionResult` relocated from assay-core to assay-types with `Deserialize` + `JsonSchema`
- TYPE-02: All domain types use `#[serde(skip_serializing_if)]` on optional fields
- TYPE-03: New fields use `#[serde(default)]` for backward compatibility with existing spec/config files

**Success Criteria:**
1. `GateRunSummary` and `CriterionResult` import from `assay_types` in all consuming crates and `just ready` passes
2. Serializing a type with `None` optional fields produces JSON without those keys
3. A v0.1.0-era spec file (without new fields) parses successfully under v0.2.0 code
4. JSON Schema snapshots reflect the relocated types with `Deserialize` + `JsonSchema` derives

---

### Phase 12: FileExists Gate Wiring

**Goal:** Connect the existing `FileExists` gate kind to the evaluation dispatch so it produces real results instead of being dead code.

**Depends on:** Phase 11

**Requirements:**
- TYPE-04: `FileExists` gate kind is wired into `evaluate()` dispatch (connect dead code)

**Success Criteria:**
1. A spec with a `FileExists` criterion pointing to an existing file evaluates to `passed: true` with evidence
2. A spec with a `FileExists` criterion pointing to a missing file evaluates to `passed: false` with a clear reason
3. `GateKind::FileExists` is no longer reachable as an unhandled match arm in gate evaluation

---

### Phase 13: Enforcement Levels

**Goal:** Add required/advisory enforcement to criteria so gate evaluation distinguishes blocking failures from informational warnings.

**Depends on:** Phase 11

**Requirements:**
- ENFC-01: Criterion has an `enforcement` field with values `required` (default) and `advisory`
- ENFC-02: Gate evaluation summary separates required failures from advisory failures

**Success Criteria:**
1. A criterion without an explicit `enforcement` field deserializes with `required` as the default
2. `evaluate_all()` returns a summary where required and advisory failure counts are reported separately
3. A gate with only advisory failures reports an overall `passed: true` status
4. A gate with any required failure reports `passed: false` regardless of advisory results

---

### Phase 14: Run History Core

**Goal:** Persist gate run results to disk as JSON files with atomic writes and retention policy, providing the audit trail infrastructure for all surfaces.

**Depends on:** Phase 11, Phase 13

**Requirements:**
- HIST-01: Gate run results are persisted to `.assay/results/<spec>/<timestamp>.json` after every evaluation
- HIST-04: Run history files use atomic write (tempfile-then-rename) to prevent corruption from concurrent writes

**Success Criteria:**
1. After `evaluate_all()`, a JSON file appears in `.assay/results/<spec-name>/` with the complete `GateRunRecord`
2. Two concurrent saves for the same spec produce two distinct files (no clobbering or corruption)
3. A partially-written file (simulated crash) does not leave a corrupt JSON file in the results directory
4. The persisted `GateRunRecord` deserializes back to the same logical content that was saved

---

### Phase 15: Run History CLI

**Goal:** Users can view gate run history from the command line, and results are automatically pruned to prevent unbounded accumulation.

**Depends on:** Phase 14

**Requirements:**
- HIST-02: User can view recent gate run history for a spec via `assay history <spec>`
- HIST-03: Run history has a configurable retention policy (per-spec file count limit) enforced on save

**Success Criteria:**
1. `assay history <spec>` displays a table of recent gate runs with timestamp, pass/fail, and criterion counts
2. When more than N results exist (N = configured retention limit), the oldest files are pruned on the next save
3. A user can configure the retention limit in `.assay/config.toml` and see it take effect on the next gate run

---

### Phase 16: Agent Gate Recording

**Goal:** Agents can submit gate evaluations via the MCP `gate_report` tool with structured reasoning, creating the second track of Assay's dual-track quality gates.

**Depends on:** Phase 13, Phase 14

**Requirements:**
- AGNT-01: MCP `gate_report` tool accepts agent-submitted pass/fail evaluations with structured reasoning
- AGNT-02: `GateKind::AgentReport` variant exists for criteria evaluated by agents (not shell commands)
- AGNT-03: Agent evaluations include `evaluator_role` metadata (`self`, `independent`, `human`)
- AGNT-04: Agent evaluation results are persisted to run history (same store as command gate results)

**Success Criteria:**
1. An agent calling `gate_report` with a spec name, criterion name, pass/fail, and reasoning receives a structured confirmation response
2. The submitted evaluation appears in the run history directory as a persisted `GateRunRecord`
3. Agent-reported results carry `evaluator_role` metadata distinguishing self-evaluation from independent evaluation
4. Agent-reported gates default to `advisory` enforcement unless explicitly overridden
5. `GateKind::AgentReport` criteria are visually distinct from `Command` criteria in gate summaries

---

### Phase 17: MCP Hardening & Agent History

**Goal:** Harden the MCP surface with timeout support, path validation, error handling, and documentation — then expose gate history to agents.

**Depends on:** Phase 14, Phase 16

**Requirements:**
- MCP-01: `gate_run` tool accepts a `timeout` parameter for agent-controlled timeouts
- MCP-02: `resolve_working_dir` validates that the path exists before evaluation
- MCP-03: `spec_list` handles scan errors gracefully instead of silently discarding them
- MCP-04: Tool descriptions are accurate and field-level documentation exists on response structs
- AGNT-05: MCP `gate_history` tool allows agents to query past gate run results for a spec
- ENFC-04: MCP `gate_run` response distinguishes required vs advisory results

**Success Criteria:**
1. An agent calling `gate_run` with a `timeout` parameter gets results within that timeout (or a timeout error)
2. An agent calling `gate_run` with a non-existent `working_dir` receives a clear error before any gate execution starts
3. An agent calling `gate_history` for a spec receives a list of recent run results with timestamps and outcomes
4. MCP `gate_run` responses include separate counts for required and advisory pass/fail results
5. `spec_list` returns partial results with error annotations when some spec files fail to parse

---

### Phase 18: CLI Hardening & Enforcement Surface

**Goal:** Polish the CLI with proper error propagation, exit codes, constants, and surface enforcement-level awareness in gate run output.

**Depends on:** Phase 13, Phase 15

**Requirements:**
- CLI-01: `main()` returns `Result` for proper error propagation
- CLI-02: Bare `assay` invocation exits with non-zero code
- CLI-03: `.assay` directory path is extracted to a named constant
- CLI-04: Gate command help duplication is resolved
- ENFC-03: CLI exit code reflects only required criterion failures; advisory failures are warnings

**Success Criteria:**
1. `assay` with no subcommand prints help and exits with a non-zero exit code
2. A gate run where all required criteria pass but advisory criteria fail exits with code 0 (success)
3. A gate run where any required criterion fails exits with a non-zero code regardless of advisory results
4. CLI error messages display the underlying cause chain (no silent swallowing of errors)
5. The `.assay` directory path appears as a named constant, not a string literal, in CLI code

---

### Phase 19: Testing & Tooling

**Goal:** Fill test coverage gaps, add comprehensive tests for all new v0.2.0 features, tighten cargo-deny policies, and dogfood Assay on itself.

**Depends on:** Phase 16, Phase 17, Phase 18

**Requirements:**
- TEST-01: MCP tool handlers have direct unit/integration tests (currently zero coverage)
- TEST-02: Test coverage gaps from Phase 3 and Phase 6 PR reviews are addressed
- TEST-03: All new features (history, enforcement, gate_report) have comprehensive tests
- TOOL-01: cargo-deny `multiple-versions` policy tightened from warn to deny
- TOOL-02: cargo-deny `source-controls` policy tightened from warn to deny
- TOOL-03: Dogfooding spec exists — Assay uses its own gates to enforce quality on itself

**Success Criteria:**
1. MCP tool handlers (`spec_get`, `spec_list`, `gate_run`, `gate_report`, `gate_history`) each have at least one direct test
2. `cargo deny check` passes with `multiple-versions` and `source-controls` set to `deny`
3. An `.assay/specs/self-check.toml` spec exists that runs Assay's own quality gates (fmt, clippy, tests, deny)
4. `just ready` passes and `assay gate run self-check` passes on a clean build

---

### Phase 20: Session JSONL Parser & Token Diagnostics

**Goal:** Parse Claude Code session files to provide exact token-aware diagnostics — the foundation for all context management features. Inspired by [Cozempic](https://github.com/Ruya-AI/cozempic).

**Depends on:** None (independent of phases 11-19)

**Requirements:**
- SDIAG-01: JSONL parser reads Claude Code session files
- SDIAG-02: Extract exact token counts from `usage` fields
- SDIAG-03: Calculate context window utilization % per model
- SDIAG-04: Categorize bloat sources (progress ticks, thinking blocks, stale reads, tool output, metadata, system reminders)
- SDIAG-05: CLI `assay context diagnose` shows token usage, bloat breakdown, context %
- SDIAG-06: CLI `assay context list` shows sessions with sizes and token counts
- SDIAG-07: MCP `context_diagnose` tool exposes full diagnostics to agents
- SDIAG-08: MCP `estimate_tokens` tool for quick token count + context %

**Success Criteria:**
1. Parser successfully reads a real Claude Code session JSONL file and extracts message-level token counts
2. `assay context diagnose` shows total tokens used, context window %, and a categorized bloat breakdown
3. `assay context list` displays all sessions with file size, token count, and message count columns
4. MCP `context_diagnose` returns structured JSON with the same data available to CLI
5. MCP `estimate_tokens` returns token count and context % within 100ms (reads only session tail)

---

### Phase 21: Team State Checkpointing

**Goal:** Extract and persist agent team state from session files and config.json, with hook-driven and manual checkpoint triggers.

**Depends on:** Phase 20 (uses session JSONL parser)

**Requirements:**
- TPROT-01: Team state extractor reads JSONL session + `~/.claude/teams/*/config.json`
- TPROT-02: Checkpoint persists team state to markdown file
- TPROT-03: CLI `assay checkpoint` command for on-demand snapshots
- TPROT-04: Plugin hooks trigger checkpoints on PostToolUse[Task|TaskCreate|TaskUpdate], PreCompact, Stop

**Success Criteria:**
1. Extractor correctly identifies all agent spawns, task state, and coordination messages from a session JSONL
2. Config.json fields (team name, lead agent, member models, working directories) are merged as authoritative
3. `assay checkpoint` writes a human-readable markdown file with agent list, task list, and coordination summary
4. Plugin hooks fire on every Task/TaskCreate/TaskUpdate tool use, PreCompact, and Stop — checkpoint file is updated
5. Checkpoint round-trips: state extracted from JSONL matches state in checkpoint file

---

### Phase 22: Pruning Engine

**Goal:** Composable, team-aware pruning strategies that safely reduce session bloat while preserving critical coordination messages.

**Depends on:** Phase 20 (session parser), Phase 21 (team extractor for protection)

**Requirements:**
- TPROT-05: Composable pruning strategies (progress-collapse, metadata-strip, thinking-blocks, tool-output-trim, stale-reads, system-reminder-dedup)
- TPROT-06: Team-aware pruning preserves coordination messages

**Success Criteria:**
1. Each strategy runs independently and reports bytes/tokens saved and messages removed/modified
2. Strategies compose sequentially — each runs on the output of the previous, savings are accurate
3. Team coordination messages (Task, TeamCreate, SendMessage, TaskCreate, TaskUpdate) are never removed by any strategy
4. Dry-run is the default — `--execute` required to modify files
5. Automatic timestamped backup before any modification
6. A prescription (gentle/standard/aggressive) applies the correct strategy subset with expected savings range

---

### Phase 23: Guard Daemon & Recovery

**Goal:** Background daemon with tiered threshold response, reactive overflow recovery, and circuit breaker — the full context protection system.

**Depends on:** Phase 20, Phase 21, Phase 22

**Requirements:**
- TPROT-07: Guard daemon polls session file at configurable interval
- TPROT-08: Soft threshold triggers gentle pruning without session reload
- TPROT-09: Hard threshold triggers full prune + team-protect + optional session reload
- TPROT-10: Token-based thresholds alongside file-size thresholds
- TPROT-11: Reactive overflow recovery with file system watcher (kqueue on macOS, inotify on Linux)
- TPROT-12: Circuit breaker prevents infinite recovery loops
- TPROT-13: Escalating prescriptions on repeated recoveries (gentle → standard → aggressive)

**Success Criteria:**
1. Guard daemon runs as a background process with PID file preventing double-starts
2. Soft threshold crossing triggers a gentle prune without restarting the session
3. Hard threshold crossing triggers full prune with team-protect and optional session reload
4. Token-based and file-size thresholds work independently — whichever fires first triggers action
5. Reactive watcher detects session file growth within sub-second latency (kqueue) and triggers recovery
6. Circuit breaker trips after configurable max recoveries in time window, halts with final checkpoint
7. Escalating prescriptions: recovery #1=gentle, #2=standard, #3=aggressive before breaker trips
8. `Ctrl+C` on guard writes a final checkpoint before exiting

---

### Phase 24: MCP History Persistence Fix

**Goal:** Fix the integration asymmetry where MCP `gate_run` does not persist history for command-only specs, unlike the CLI which always saves.

**Depends on:** Phase 14, Phase 17

**Gap Closure:** Closes integration gap from v0.2.0 audit

**Success Criteria:**
1. An agent calling `gate_run` on a command-only spec sees the run appear in `gate_history` results
2. MCP `gate_run` calls `history::save()` for all specs, not just those with agent criteria
3. Integration test verifies history persistence for command-only MCP gate runs

---

### Phase 25: Tech Debt Cleanup

**Goal:** Address accumulated tech debt from v0.2.0 development: backfill missing verification documents and triage open issues.

**Depends on:** None

**Success Criteria:**
1. Phases 16, 19, and 20 each have a VERIFICATION.md document
2. Open issues in `.planning/issues/open/` are triaged: resolved issues closed, remaining categorized
3. Issue count reduced to actionable items only

---

## Requirement Coverage

| Requirement | Phase | Description |
|-------------|-------|-------------|
| TYPE-01 | 11 | Relocate GateRunSummary/CriterionResult to assay-types |
| TYPE-02 | 11 | skip_serializing_if on optional fields |
| TYPE-03 | 11 | serde(default) for backward compat |
| TYPE-04 | 12 | Wire FileExists into evaluate() |
| ENFC-01 | 13 | Enforcement field on Criterion |
| ENFC-02 | 13 | Summary separates required/advisory |
| ENFC-03 | 18 | CLI exit code reflects required only |
| ENFC-04 | 17 | MCP response distinguishes required/advisory |
| HIST-01 | 14 | Persist results to .assay/results/ |
| HIST-02 | 15 | CLI history command |
| HIST-03 | 15 | Retention policy |
| HIST-04 | 14 | Atomic writes |
| AGNT-01 | 16 | gate_report MCP tool |
| AGNT-02 | 16 | GateKind::AgentReport variant |
| AGNT-03 | 16 | evaluator_role metadata |
| AGNT-04 | 16 | Agent results persisted to history |
| AGNT-05 | 17 | gate_history MCP tool |
| MCP-01 | 17 | gate_run timeout parameter |
| MCP-02 | 17 | Working dir validation |
| MCP-03 | 17 | spec_list error handling |
| MCP-04 | 17 | Tool description accuracy |
| CLI-01 | 18 | main() returns Result |
| CLI-02 | 18 | Bare invocation exit code |
| CLI-03 | 18 | .assay path constant |
| CLI-04 | 18 | Help duplication fix |
| TEST-01 | 19 | MCP handler tests |
| TEST-02 | 19 | Phase 3/6 review gaps |
| TEST-03 | 19 | New feature tests |
| TOOL-01 | 19 | cargo-deny multiple-versions → deny |
| TOOL-02 | 19 | cargo-deny source-controls → deny |
| TOOL-03 | 19 | Dogfooding spec |
| SDIAG-01 | 20 | JSONL session file parser |
| SDIAG-02 | 20 | Token count extraction from usage fields |
| SDIAG-03 | 20 | Context window utilization % |
| SDIAG-04 | 20 | Bloat source categorization |
| SDIAG-05 | 20 | CLI context diagnose command |
| SDIAG-06 | 20 | CLI context list command |
| SDIAG-07 | 20 | MCP context_diagnose tool |
| SDIAG-08 | 20 | MCP estimate_tokens tool |
| TPROT-01 | 21 | Team state extractor (JSONL + config.json) |
| TPROT-02 | 21 | Checkpoint markdown persistence |
| TPROT-03 | 21 | CLI checkpoint command |
| TPROT-04 | 21 | Plugin checkpoint hooks |
| TPROT-05 | 22 | Composable pruning strategies |
| TPROT-06 | 22 | Team-aware pruning protection |
| TPROT-07 | 23 | Guard daemon polling loop |
| TPROT-08 | 23 | Soft threshold pruning |
| TPROT-09 | 23 | Hard threshold + team-protect + reload |
| TPROT-10 | 23 | Token-based thresholds |
| TPROT-11 | 23 | Reactive overflow recovery (kqueue/inotify) |
| TPROT-12 | 23 | Circuit breaker |
| TPROT-13 | 23 | Escalating prescriptions |

**Coverage:** 52/52 requirements mapped (100%)
