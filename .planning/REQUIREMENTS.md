# Requirements: Assay v0.3.0 Orchestration Foundation

## Orchestration

- [ ] **ORCH-01**: User can create an isolated git worktree for a spec with `assay worktree create <spec>`
- [ ] **ORCH-02**: User can list all active worktrees with `assay worktree list`
- [ ] **ORCH-03**: User can check worktree status (branch, dirty, behind/ahead) with `assay worktree status <spec>`
- [ ] **ORCH-04**: User can clean up a worktree with `assay worktree cleanup <spec>`
- [ ] **ORCH-05**: MCP tools `worktree_create`, `worktree_status`, `worktree_cleanup` available for agent use
- [ ] **ORCH-06**: Worktree paths are configurable (default: `.assay/worktrees/`)
- [ ] **ORCH-07**: Specs resolve from parent project when gates evaluate in worktree context

## CLI Polish

- [ ] **CLI-01**: `NO_COLOR` environment variable handled correctly per no-color.org spec (`var_os().is_none()`)
- [ ] **CLI-02**: Gate command help text consolidated (no duplication between top-level and subcommand)
- [ ] **CLI-03**: Enforcement check block deduplicated between `handle_gate_run_all` and `handle_gate_run`
- [ ] **CLI-04**: Spec show color branch duplication eliminated
- [ ] **CLI-05**: `StreamCounters` has doc comments, `tally()` method, and `gate_blocked()` method
- [ ] **CLI-06**: `StreamConfig` fields have doc comments
- [ ] **CLI-07**: Command column separator is data-driven (not hardcoded)
- [ ] **CLI-08**: `[srs]` magic string extracted to constant

## MCP Validation

- [ ] **MCP-01**: All MCP tools return specific error messages for missing required parameters
- [ ] **MCP-02**: All MCP tools return specific error messages for invalid parameter types
- [ ] **MCP-03**: Spec-not-found errors include list of available spec names
- [ ] **MCP-04**: MCP failure reason checks stdout in addition to stderr
- [ ] **MCP-05**: Unnecessary clone intermediaries removed from gate_run

## Types Hygiene

- [x] **TYPE-01**: All types without float fields derive `Eq` alongside `PartialEq`
- [x] **TYPE-02**: `Enforcement`, `GateKind`, and other key enums implement `Display`
- [x] **TYPE-03**: All public types and fields have doc comments
- [x] **TYPE-04**: `GateSection` derives `Default`
- [x] **TYPE-05**: `GateCriterion` / `Criterion` structural duplication reduced
- [x] **TYPE-06**: `EnforcementSummary` fields have doc comments

## Gate Output

- [ ] **GATE-01**: Gate command output captured with head+tail truncation and byte budget
- [ ] **GATE-02**: Truncation uses `[truncated: X bytes omitted]` marker between head and tail
- [ ] **GATE-03**: UTF-8 boundaries respected (no split multi-byte sequences)
- [ ] **GATE-04**: Independent stdout/stderr byte budgets
- [ ] **GATE-05**: Existing `truncated` and `original_bytes` fields on `GateResult` populated correctly

## Error Messages

- [ ] **ERR-01**: Command not found during gate run shows actionable message ("Command 'X' not found. Is it installed and in PATH?")
- [ ] **ERR-02**: Spec not found shows available spec names
- [ ] **ERR-03**: Invalid spec TOML shows file path, line number, and specific parse error

## Core Tech Debt

- [x] **CORE-01**: `AssayError` construction ergonomics improved
- [ ] **CORE-02**: Enforcement validation duplication eliminated between `validate()` and `validate_gates_spec()`
- [ ] **CORE-03**: `evaluate_all` and `evaluate_all_gates` shared logic extracted
- [ ] **CORE-04**: History `list()` handles unreadable directory entries with warning instead of silent drop
- [x] **CORE-05**: `serde_json` errors distinguished from I/O errors in `AssayError`
- [ ] **CORE-06**: `generate_run_id` visibility changed to `pub(crate)`
- [ ] **CORE-07**: Guard daemon PID file write followed by `fsync()`
- [ ] **CORE-08**: `try_save_checkpoint` uses stored project dir instead of `current_dir()`
- [ ] **CORE-09**: Spec parse errors logged/warned instead of silently ignored

---

## Future Requirements (Deferred)

- [ ] Claude Code Launcher (headless `--print` mode) — deferred to v0.4.0
- [ ] Session Record (worktree/agent/gate linkage) — deferred to v0.4.0
- [ ] Gate Evaluate (independent evaluation with diff context) — deferred to v0.4.0
- [ ] Minimal TUI gate results viewer — deferred to v0.4.0
- [ ] Composable gate definitions (`gate.extends`) — deferred
- [ ] Spec preconditions section — deferred
- [ ] Gate history summary with pass/fail rates — deferred
- [ ] Merge-back pipeline — deferred to v0.4.0+

## Out of Scope

- **tmux session management** — Requires interactive agent support, not headless
- **Multi-session orchestrator** — Requires session record + launcher first
- **Full TUI dashboard** — Requires orchestrator for real-time multi-session view
- **Spec Provider Trait** — One implementation = premature abstraction
- **SQLite for sessions** — JSON files follow existing history pattern
- **Agent launcher trait** — Concrete module only until second implementation exists

---

## Traceability

| Requirement | Phase | Phase Name |
|-------------|-------|------------|
| ORCH-01 | 28 | Worktree Manager |
| ORCH-02 | 28 | Worktree Manager |
| ORCH-03 | 28 | Worktree Manager |
| ORCH-04 | 28 | Worktree Manager |
| ORCH-05 | 28 | Worktree Manager |
| ORCH-06 | 28 | Worktree Manager |
| ORCH-07 | 28 | Worktree Manager |
| CLI-01 | 32 | CLI Polish |
| CLI-02 | 32 | CLI Polish |
| CLI-03 | 32 | CLI Polish |
| CLI-04 | 32 | CLI Polish |
| CLI-05 | 32 | CLI Polish |
| CLI-06 | 32 | CLI Polish |
| CLI-07 | 32 | CLI Polish |
| CLI-08 | 32 | CLI Polish |
| MCP-01 | 33 | MCP Validation |
| MCP-02 | 33 | MCP Validation |
| MCP-03 | 33 | MCP Validation |
| MCP-04 | 33 | MCP Validation |
| MCP-05 | 33 | MCP Validation |
| TYPE-01 | 27 | Types Hygiene |
| TYPE-02 | 27 | Types Hygiene |
| TYPE-03 | 27 | Types Hygiene |
| TYPE-04 | 27 | Types Hygiene |
| TYPE-05 | 27 | Types Hygiene |
| TYPE-06 | 27 | Types Hygiene |
| GATE-01 | 29 | Gate Output Truncation |
| GATE-02 | 29 | Gate Output Truncation |
| GATE-03 | 29 | Gate Output Truncation |
| GATE-04 | 29 | Gate Output Truncation |
| GATE-05 | 29 | Gate Output Truncation |
| ERR-01 | 31 | Error Messages |
| ERR-02 | 31 | Error Messages |
| ERR-03 | 31 | Error Messages |
| CORE-01 | 26 | Structural Prerequisites |
| CORE-02 | 30 | Core Tech Debt |
| CORE-03 | 30 | Core Tech Debt |
| CORE-04 | 30 | Core Tech Debt |
| CORE-05 | 26 | Structural Prerequisites |
| CORE-06 | 30 | Core Tech Debt |
| CORE-07 | 30 | Core Tech Debt |
| CORE-08 | 30 | Core Tech Debt |
| CORE-09 | 30 | Core Tech Debt |
