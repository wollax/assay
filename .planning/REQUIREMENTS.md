# Requirements: v0.6.2 P0 Cleanup

## Process Safety

- [ ] **SAFE-01**: `kill_agent_subprocess` uses `killpg` (negative PID) for process group termination instead of single-process `kill` (WOL-464)
- [ ] **SAFE-02**: Auto-promote path handles TOCTOU race between session status check and promotion (WOL-471)
- [ ] **SAFE-03**: Pipeline crash error messages include stderr content after streaming migration (WOL-465)
- [ ] **SAFE-04**: Relay thread panics are logged instead of silently swallowed (WOL-466)
- [ ] **SAFE-05**: TUI strips ANSI/control characters from TextDelta/TextBlock to prevent terminal injection (WOL-348)

## Type Correctness

- [ ] **TYPE-01**: `Criterion.when: Option<When>` representational ambiguity resolved — `None` vs `Some(SessionEnd)` disambiguated (WOL-453)
- [ ] **TYPE-02**: `review::SessionPhase` renamed to `CheckpointPhase` to avoid name collision with `work_session::SessionPhase` (WOL-454)
- [ ] **TYPE-03**: `When::AfterToolCalls { n: 0 }` rejected by validation (nonsensical value) (WOL-456)
- [ ] **TYPE-04**: `evaluate_checkpoint` respects CLI/config timeout overrides instead of silently dropping them (WOL-457)
- [ ] **TYPE-05**: `review::SessionPhase` includes `OnEvent` variant for event-triggered diagnostics (WOL-458)
- [ ] **TYPE-06**: `CriterionKind` serde tagging made consistent with `When` enum (WOL-482)
- [ ] **TYPE-07**: `evaluate_checkpoint` at `SessionEnd` phase documents no-op behavior with warning (WOL-455)

## Review Findings (S04)

- [ ] **S04-01**: `test_auto_promote_already_verified_is_noop` test name corrected to match actual behavior (WOL-473)
- [ ] **S04-02**: Session lookup in spec review avoids iterating all sessions (WOL-472)

## Review Findings (S05)

- [ ] **S05-01**: `close-the-loop` README inaccuracies fixed (WOL-479)
- [ ] **S05-02**: `ManifestSession.prompt` renamed to clarify distinction from `prompt_layers` (WOL-478)
- [ ] **S05-03**: Low-severity review findings batch addressed (WOL-477)
- [ ] **S05-04**: `ManifestSession.prompt` supports file-path references (WOL-481)
- [ ] **S05-05**: Parse test added for example `gates.toml` against `GatesSpec` (WOL-480)

## Test Coverage

- [ ] **TEST-01**: `gate_sessions` directory has eviction/cleanup to prevent unbounded growth (WOL-117)
- [ ] **TEST-02**: `find_context_for_spec` test covers corrupted/unreadable session file being skipped (WOL-118)
- [ ] **TEST-03**: `gate_run` tracing field naming standardized (`spec_name` across handlers) (WOL-116)
- [ ] **TEST-04**: 3 pipeline integration tests added with synthetic provider (WOL-463)
- [ ] **TEST-05**: `claude_stream` test for non-`text_delta` `content_block_delta` (e.g. `input_json_delta`) (WOL-345)
- [ ] **TEST-06**: `claude_stream` test for mixed stream with TextDelta + TextBlock for same content (WOL-346)
- [ ] **TEST-07**: TextDelta text length cap to prevent unbounded per-token allocations (WOL-347)
- [ ] **TEST-08**: `pipeline_checkpoint` tests: Windows portability + OnEvent driver coverage (WOL-467)

---

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SAFE-01 | — | Planned |
| SAFE-02 | — | Planned |
| SAFE-03 | — | Planned |
| SAFE-04 | — | Planned |
| SAFE-05 | — | Planned |
| TYPE-01 | — | Planned |
| TYPE-02 | — | Planned |
| TYPE-03 | — | Planned |
| TYPE-04 | — | Planned |
| TYPE-05 | — | Planned |
| TYPE-06 | — | Planned |
| TYPE-07 | — | Planned |
| S04-01 | — | Planned |
| S04-02 | — | Planned |
| S05-01 | — | Planned |
| S05-02 | — | Planned |
| S05-03 | — | Planned |
| S05-04 | — | Planned |
| S05-05 | — | Planned |
| TEST-01 | — | Planned |
| TEST-02 | — | Planned |
| TEST-03 | — | Planned |
| TEST-04 | — | Planned |
| TEST-05 | — | Planned |
| TEST-06 | — | Planned |
| TEST-07 | — | Planned |
| TEST-08 | — | Planned |
