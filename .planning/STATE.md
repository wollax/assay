# State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-02)

**Core value:** Dual-track quality gates (deterministic + agent-evaluated) for AI coding agents
**Current focus:** v0.2.0 Dual-Track Gates & Hardening (COMPLETE)

## Current Position

Phase: 25 — Tech Debt Cleanup (IN PROGRESS)
Plan: 02 of 02
Status: Plan 25-02 complete
Last activity: 2026-03-07 — Completed 25-02-PLAN.md

Progress: v0.2.0 [██████████████] ~98%

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 100% (shipped) |
| v0.2.0 | 15 (11-25) | 52 | ~93% |

## Accumulated Context

### Decisions

v0.1.0 decisions archived to .planning/milestones/v0.1.0-ROADMAP.md

v0.2.0 decisions (from brainstorm + research):
- Agent gates receive evaluations via MCP, not call LLMs directly
- Self-evaluation + audit trail for v0.2; independent evaluator deferred to v0.3
- Keep core types domain-agnostic
- No built-in LLM client, no SpecProvider trait yet
- Pipeline semantics for future orchestrator design
- Type relocation (GateRunSummary -> assay-types) is highest-churn change — do first
- Agent-reported gates default to advisory enforcement (trust asymmetry)
- Per-spec subdirectory layout for results (.assay/results/{spec-name}/)
- Timestamp + 6-char random hex suffix for run IDs (no new crate)
- Include assay_version in GateRunRecord for future schema migration
- Two-tier enforcement only (required/advisory) — SonarQube validates no warnings tier
- Cozempic-inspired features (token diagnostics, team protection) added to v0.2.0 as phases 20-23
- Session JSONL parsing in Rust (not Python) — full feature parity with Cozempic, native performance
- Phases 20-23 are independent of 11-19 — can be worked in parallel or after gates
- Guard daemon uses kqueue (macOS) / inotify (Linux) for sub-second reactive recovery
- Pruning strategies compose sequentially, dry-run by default, team messages always protected

v0.2.0 decisions (from 11-01 execution):
- Clean break for type relocation: no re-exports from assay-core, all consumers import from assay_types
- Output types (GateRunSummary, CriterionResult) do NOT use deny_unknown_fields
- Schema registry entries added for both relocated types

v0.2.0 decisions (from 11-02 execution):
- Backward-compat test verifies GateRunSummary deserializes from minimal JSON without results field
- Skipped criterion test (result: None) verifies skip_serializing_if works correctly

v0.2.0 decisions (from 12-01 execution):
- cmd takes precedence over path when both set (simpler than mutual exclusivity validation)
- path field uses same serde attributes as cmd (skip_serializing_if + default)
- evaluate_file_exists implementation unchanged — already correct

v0.2.0 decisions (from 13-01 execution):
- Enforcement enum uses Copy trait (two-variant fieldless, read frequently during evaluation)
- Input types use Option<Enforcement> (None = inherit from gate section default); output types use concrete Enforcement
- GateSection uses deny_unknown_fields (user-authored input); EnforcementSummary does not (output type)

v0.2.0 decisions (from 13-02 execution):
- Backward compat preserved: passed/failed/skipped counts compute as before; EnforcementSummary is additive
- resolve_enforcement() is public for reuse by CLI/MCP pass/fail logic
- Validation enforces at-least-one-required at parse time, not evaluation time
- Descriptive-only criteria (no cmd/path) do not count as executable for the required check

v0.2.0 decisions (from 14-01 execution):
- GateRunRecord wraps GateRunSummary via summary field (no field duplication)
- spec_name accessed via record.summary.spec_name (already embedded, not duplicated at record level)
- save() takes assay_dir and derives results path internally
- No new error variants needed — existing AssayError::Io covers all history operations
- generate_run_id() is public for caller flexibility

v0.2.0 decisions (from 14-02 execution):
- PartialEq derived on GateRunRecord, GateRunSummary, CriterionResult, GateResult (non-breaking, enables structural equality assertions)

v0.2.0 decisions (from 15-01 execution):
- max_history defaults to None (no pruning); CLI will apply default_max_history() (1000) when absent
- Some(0) and None both skip pruning — zero is treated as unlimited
- prune() is private to the history module — only save() calls it
- SaveResult replaces PathBuf as save() return type

v0.2.0 decisions (from 15-02 execution):
- save_run_record() helper centralizes record construction and save logic
- Streaming mode records have empty results vec and zero total_duration_ms (no per-criterion timing)
- handle_gate_run_all() streaming path tracks per-spec counters via before/after delta

v0.2.0 decisions (from 16-01 execution):
- CriterionKind enum is simple (not internally tagged) — single variant AgentReport for now
- Agent fields on GateResult are all Option<T> with skip_serializing_if for backward compat
- AgentSession uses HashMap/HashSet for flexible keying (O(1) criterion lookup)
- EvaluatorRole::SelfEval serializes as "self" via serde rename
- Downstream compile errors (assay-core, assay-mcp) deferred to Plan 02

v0.2.0 decisions (from 16-02 execution):
- evaluate() returns InvalidCriterion error for AgentReport (not evaluable standalone)
- evaluate_all/evaluate_all_gates skip AgentReport criteria as pending (result: None)
- finalize_as_timed_out() does NOT save — caller decides persistence
- Unevaluated advisory criteria in timed-out sessions are skipped (not failed)
- AgentReport criteria count as "executable" for at-least-one-required validation
- Evaluator priority: Human > Independent > SelfEval (highest wins)

v0.2.0 decisions (from 16-03 execution):
- AssayServer holds Arc<Mutex<HashMap<String, AgentSession>>> for in-memory session state
- gate_run auto-creates sessions when spec contains AgentReport criteria
- Session timeout is 30 minutes; timed-out sessions are auto-finalized and persisted
- CriterionSummary gains kind_label field (cmd/file/agent) for agent-aware formatting
- GateRunResponse gains optional session_id/pending_criteria for session-aware responses

v0.2.0 decisions (from 16-04 execution):
- CLI streaming output prepends [cmd]/[file]/[agent]/[auto] labels to criterion names
- AgentReport criteria show as "pending" in streaming mode (not skipped)
- History detail view displays evaluator_role, confidence, evidence (200 chars), reasoning (200 chars) when present
- 6 new schema snapshots added for Phase 16 types (total: 23)

v0.2.0 decisions (from 17-01 execution):
- gate_run timeout defaults to 300s, returns CallToolResult error (not McpError) on expiry
- working_dir validation via is_dir() check before spawn_blocking — early return with domain error
- spec_list uses SpecListResponse envelope with skip_serializing_if on errors vec
- GateRunResponse gains required_passed, advisory_passed, blocked fields computed from EnforcementSummary

v0.2.0 decisions (from 17-02 execution):
- gate_history loads config for validation consistency, prefixes _config (not directly needed)
- List mode reverses history::list() output for most-recent-first agent ergonomics
- Detail mode passes through full GateRunRecord JSON without mapping
- Default limit is 10 runs; unreadable entries skipped with tracing::warn

v0.2.0 decisions (from 18-01 execution):
- assay_mcp::serve() error wrapped via anyhow::anyhow! (Box<dyn Error> lacks Send+Sync for anyhow)
- Bare invocation outside project returns Ok(1) not Err (expected condition, not error)
- Gate business logic failures return Ok(1) (exit code) not Err (error propagation)

v0.2.0 decisions (from 18-02 execution):
- counters.failed tracks only required failures; counters.warned tracks advisory failures
- Advisory criteria always labeled [advisory] in streaming output (pass or fail)
- Summary line includes warned category between failed and skipped
- Post-hoc has_required_failure tracking removed; exit code driven by counters.failed > 0

v0.2.0 decisions (from 19-02 execution):
- MCP handler methods and param types made pub for integration test access
- Integration tests require --test-threads=1 due to CWD dependency
- Insta snapshots use manual normalization (no redactions feature)

v0.2.0 decisions (from 20-02 execution):
- `sessions_from_history()` and `estimate_tokens_from_bytes()` are public API marked `#[allow(dead_code)]` until consumed by CLI/MCP
- Stale read detection uses HashSet of file_path strings; second read of same path counts as stale
- `deny.toml` updated: MPL-2.0 allowed (option-ext via dirs-sys), getrandom@0.2 skipped (redox_users via dirs-sys)
- Context module constants (`DEFAULT_CONTEXT_WINDOW`, `SYSTEM_OVERHEAD_TOKENS`) widened to `pub(crate)` for guard daemon access

v0.2.0 decisions (from 21-01 execution):
- ParsedEntry imported via pub re-export (`crate::context::ParsedEntry`), not private parser module
- `merge_team_config` is a no-op until team config.json format stabilizes; session-extracted state is authoritative
- JSON frontmatter (not YAML) between `---` delimiters for checkpoint files
- Archive filenames use ISO 8601 with colons replaced by dashes for filesystem compatibility
- Context health uses fixed 200K context window (same as tokens module)

v0.2.0 decisions (from 22-01 execution):
- PruneStrategy::label() lives on enum in assay-types; apply_strategy() is a free function in assay-core (orphan rule)
- ParsedEntry::update_content() re-serializes entry for content-modifying strategies
- Protection set uses stable line_number identifiers (no re-indexing between strategies)

### Pending Issues

19 open issues (reduced from 38 after triaging 19 test-related issues in 19-02)

### Blockers

None.

v0.2.0 decisions (from 23-03 execution):
- Context tokens module widened from `pub(super)` to `pub(crate)` for guard daemon access
- `estimate_tokens_from_bytes` changed from `pub` + `#[allow(dead_code)]` to `pub(crate)` (consumed by daemon)
- SessionWatcher watches both file (Modify via kqueue) and parent directory (Create for atomic writes)
- Guard daemon event loop uses `tokio::select!` with 1s debounce on watcher events
- Hard threshold enforces minimum Standard tier (overrides Gentle from circuit breaker)

### Pending Issues

19 open issues (reduced from 38 after triaging 19 test-related issues in 19-02)

### Blockers

None.

v0.2.0 decisions (from 24-01 execution):
- format_gate_response takes &GateRunSummary; no clone needed before history save
- Command-only gate_run save failures are non-fatal (tracing::warn, not error return)

### Next Actions

Plan 25-01 complete. Next: 25-02-PLAN.md (remaining tech debt items).

### Session Continuity

Last session: 2026-03-07T15:58Z
Stopped at: Completed 25-01-PLAN.md
Resume file: None
