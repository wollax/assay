# Requirements: Assay v0.2.0 — Dual-Track Gates & Hardening

## Run History

- [ ] **HIST-01**: Gate run results are persisted to `.assay/results/<spec>/<timestamp>.json` after every evaluation
- [ ] **HIST-02**: User can view recent gate run history for a spec via `assay history <spec>`
- [ ] **HIST-03**: Run history has a configurable retention policy (per-spec file count limit) enforced on save
- [ ] **HIST-04**: Run history files use atomic write (tempfile-then-rename) to prevent corruption from concurrent writes

## Gate Enforcement

- [x] **ENFC-01**: Criterion has an `enforcement` field with values `required` (default) and `advisory`
- [x] **ENFC-02**: Gate evaluation summary separates required failures from advisory failures
- [ ] **ENFC-03**: CLI exit code reflects only required criterion failures; advisory failures are warnings
- [ ] **ENFC-04**: MCP `gate_run` response distinguishes required vs advisory results

## Agent Gate Recording

- [ ] **AGNT-01**: MCP `gate_report` tool accepts agent-submitted pass/fail evaluations with structured reasoning
- [ ] **AGNT-02**: `GateKind::AgentReport` variant exists for criteria evaluated by agents (not shell commands)
- [ ] **AGNT-03**: Agent evaluations include `evaluator_role` metadata (`self`, `independent`, `human`)
- [ ] **AGNT-04**: Agent evaluation results are persisted to run history (same store as command gate results)
- [ ] **AGNT-05**: MCP `gate_history` tool allows agents to query past gate run results for a spec

## Foundation — Type System

- [x] **TYPE-01**: `GateRunSummary` and `CriterionResult` relocated from assay-core to assay-types with `Deserialize` + `JsonSchema`
- [x] **TYPE-02**: All domain types use `#[serde(skip_serializing_if)]` on optional fields
- [x] **TYPE-03**: New fields use `#[serde(default)]` for backward compatibility with existing spec/config files
- [x] **TYPE-04**: `FileExists` gate kind is wired into `evaluate()` dispatch (connect dead code)

## Foundation — Testing

- [ ] **TEST-01**: MCP tool handlers have direct unit/integration tests (currently zero coverage)
- [ ] **TEST-02**: Test coverage gaps from Phase 3 and Phase 6 PR reviews are addressed
- [ ] **TEST-03**: All new features (history, enforcement, gate_report) have comprehensive tests

## Foundation — MCP Hardening

- [ ] **MCP-01**: `gate_run` tool accepts a `timeout` parameter for agent-controlled timeouts
- [ ] **MCP-02**: `resolve_working_dir` validates that the path exists before evaluation
- [ ] **MCP-03**: `spec_list` handles scan errors gracefully instead of silently discarding them
- [ ] **MCP-04**: Tool descriptions are accurate and field-level documentation exists on response structs

## Foundation — CLI Hardening

- [ ] **CLI-01**: `main()` returns `Result` for proper error propagation
- [ ] **CLI-02**: Bare `assay` invocation exits with non-zero code
- [ ] **CLI-03**: `.assay` directory path is extracted to a named constant
- [ ] **CLI-04**: Gate command help duplication is resolved

## Foundation — Tooling

- [ ] **TOOL-01**: cargo-deny `multiple-versions` policy tightened from warn to deny
- [ ] **TOOL-02**: cargo-deny `source-controls` policy tightened from warn to deny
- [ ] **TOOL-03**: Dogfooding spec exists — Assay uses its own gates to enforce quality on itself

## Session Diagnostics

*Inspired by [Cozempic](https://github.com/Ruya-AI/cozempic) — token-aware context management for AI agent sessions.*

- [ ] **SDIAG-01**: JSONL parser reads Claude Code session files from `~/.claude/projects/*/sessions/`
- [ ] **SDIAG-02**: Extract exact token counts from `usage` fields in assistant messages
- [ ] **SDIAG-03**: Calculate context window utilization % for model's max context (200K tokens for Opus)
- [ ] **SDIAG-04**: Categorize bloat sources (progress ticks, thinking blocks, stale reads, tool output overflow, metadata, system reminders)
- [ ] **SDIAG-05**: CLI `assay context diagnose` shows token usage, bloat breakdown, context %
- [ ] **SDIAG-06**: CLI `assay context list` shows sessions with sizes and token counts
- [ ] **SDIAG-07**: MCP `context_diagnose` tool exposes full diagnostics to agents
- [ ] **SDIAG-08**: MCP `estimate_tokens` tool for quick token count + context %

## Agent Team Context Protection

*Inspired by [Cozempic](https://github.com/Ruya-AI/cozempic) — prevent context loss that orphans agent teams during auto-compaction.*

- [ ] **TPROT-01**: Team state extractor reads JSONL session + `~/.claude/teams/*/config.json`
- [ ] **TPROT-02**: Checkpoint persists team state (agents, tasks, coordination messages) to markdown file
- [ ] **TPROT-03**: CLI `assay checkpoint` command for on-demand state snapshots
- [ ] **TPROT-04**: Plugin hooks trigger checkpoints on PostToolUse[Task|TaskCreate|TaskUpdate], PreCompact, Stop
- [ ] **TPROT-05**: Composable pruning strategies (progress-collapse, metadata-strip, thinking-blocks, tool-output-trim, stale-reads, system-reminder-dedup)
- [ ] **TPROT-06**: Team-aware pruning preserves coordination messages (Task, TeamCreate, SendMessage, TaskCreate, TaskUpdate)
- [ ] **TPROT-07**: Guard daemon polls session file at configurable interval
- [ ] **TPROT-08**: Soft threshold triggers gentle pruning without session reload
- [ ] **TPROT-09**: Hard threshold triggers full prune + team-protect + optional session reload
- [ ] **TPROT-10**: Token-based thresholds alongside file-size thresholds
- [ ] **TPROT-11**: Reactive overflow recovery with file system watcher (kqueue on macOS, inotify on Linux)
- [ ] **TPROT-12**: Circuit breaker prevents infinite recovery loops (configurable max recoveries in time window)
- [ ] **TPROT-13**: Escalating prescriptions on repeated recoveries (gentle → standard → aggressive)

---

## Future Requirements (deferred)

- Run comparison/diffing between gate results — v0.3
- Trend analysis and flaky criterion detection — v0.3
- Context-controlled agent evaluation (`gate_evaluate`) — v0.3
- Independent evaluator enforcement (requires orchestrator) — v0.3
- OutputDetail enum for semantic verbosity control — v0.3
- Wire format vs display format type separation — v0.3
- Streaming capture with byte budget for gate evaluation — v0.3

## Out of Scope

- Built-in LLM client — agents already have LLM access; `gate_report` eliminates the need
- SpecProvider trait — premature abstraction with one implementation
- SQLite or database storage — file-per-run is sufficient at this scale
- Three-tier enforcement (required/warning/advisory) — SonarQube removed warnings; two tiers is validated
- Composite gate logic (AND/OR/threshold) — required/advisory delivers the value
- TUI dashboard — no orchestrator to visualize yet
- Trust calibration / confidence scores — research problem, not engineering problem

## Traceability

<!-- Updated by roadmapper after phase assignment — 2026-03-02 -->

| Requirement | Phase | Status |
|-------------|-------|--------|
| TYPE-01 | 11 | Complete |
| TYPE-02 | 11 | Complete |
| TYPE-03 | 11 | Complete |
| TYPE-04 | 12 | Complete |
| ENFC-01 | 13 | Complete |
| ENFC-02 | 13 | Complete |
| ENFC-03 | 18 | Pending |
| ENFC-04 | 17 | Pending |
| HIST-01 | 14 | Pending |
| HIST-02 | 15 | Pending |
| HIST-03 | 15 | Pending |
| HIST-04 | 14 | Pending |
| AGNT-01 | 16 | Pending |
| AGNT-02 | 16 | Pending |
| AGNT-03 | 16 | Pending |
| AGNT-04 | 16 | Pending |
| AGNT-05 | 17 | Pending |
| MCP-01 | 17 | Pending |
| MCP-02 | 17 | Pending |
| MCP-03 | 17 | Pending |
| MCP-04 | 17 | Pending |
| CLI-01 | 18 | Pending |
| CLI-02 | 18 | Pending |
| CLI-03 | 18 | Pending |
| CLI-04 | 18 | Pending |
| TEST-01 | 19 | Pending |
| TEST-02 | 19 | Pending |
| TEST-03 | 19 | Pending |
| TOOL-01 | 19 | Pending |
| TOOL-02 | 19 | Pending |
| TOOL-03 | 19 | Pending |
| SDIAG-01 | 20 | Pending |
| SDIAG-02 | 20 | Pending |
| SDIAG-03 | 20 | Pending |
| SDIAG-04 | 20 | Pending |
| SDIAG-05 | 20 | Pending |
| SDIAG-06 | 20 | Pending |
| SDIAG-07 | 20 | Pending |
| SDIAG-08 | 20 | Pending |
| TPROT-01 | 21 | Pending |
| TPROT-02 | 21 | Pending |
| TPROT-03 | 21 | Pending |
| TPROT-04 | 21 | Pending |
| TPROT-05 | 22 | Pending |
| TPROT-06 | 22 | Pending |
| TPROT-07 | 23 | Pending |
| TPROT-08 | 23 | Pending |
| TPROT-09 | 23 | Pending |
| TPROT-10 | 23 | Pending |
| TPROT-11 | 23 | Pending |
| TPROT-12 | 23 | Pending |
| TPROT-13 | 23 | Pending |
