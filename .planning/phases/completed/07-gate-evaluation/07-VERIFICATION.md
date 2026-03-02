---
phase: 07-gate-evaluation
status: gaps_found
verified: 12/14 must_haves
---

# Phase 7: Gate Evaluation — Verification

## Must-Haves Verification

### GATE-01: Command gate execution via `std::process::Command` with exit code evaluation
- **Status:** verified
- **Evidence:**
  - `crates/assay-core/src/gate/mod.rs:232–254` — `evaluate_command` spawns `sh -c <cmd>` via `std::process::Command`, captures stdout/stderr via piped threads, polls `try_wait` for completion.
  - `crates/assay-core/src/gate/mod.rs:323–337` — exit code extracted via `exit_status.code()`, `passed` set from `exit_status.success()`.
  - Test: `gate::tests::evaluate_echo_hello` (line 406) — asserts `passed: true`, `exit_code: Some(0)`, `stdout` contains `"hello"`.
  - Test: `gate::tests::evaluate_failing_command` (line 430) — asserts `passed: false`, `exit_code: Some(1)`, `stderr` contains `"fail"`.

### GATE-02: Structured `GateResult` with stdout/stderr evidence capture
- **Status:** verified
- **Evidence:**
  - `crates/assay-types/src/gate.rs:42–79` — `GateResult` struct with fields: `passed`, `kind`, `stdout`, `stderr`, `exit_code`, `duration_ms`, `timestamp`, `truncated`, `original_bytes`.
  - `crates/assay-core/src/gate/mod.rs:297–360` — stdout and stderr are read via reader threads, decoded via `String::from_utf8_lossy`, truncated if > 64 KB, stored in `GateResult`.
  - Test: `gate::tests::evaluate_echo_hello` — stdout capture verified.
  - Test: `gate::tests::evaluate_failing_command` — stderr capture verified.
  - Test: `gate::tests::gate_result_json_skips_empty_fields` and `gate_result_json_includes_populated_fields` in `assay-types/src/gate.rs`.

### GATE-03: Timeout enforcement on gate commands with configurable default (300s)
- **Status:** verified
- **Evidence:**
  - `crates/assay-core/src/gate/mod.rs:276–295` — polling loop with `try_wait`, kills process on `start.elapsed() >= timeout`, reaps zombie via `child.wait()`.
  - `crates/assay-core/src/gate/mod.rs:339–359` — on timeout (`status == None`), produces `GateResult { passed: false, exit_code: None, stderr: "[timed out after Xs]" }`.
  - `crates/assay-core/src/gate/mod.rs:184–194` — `resolve_timeout` returns 300s when all three inputs are `None`.
  - `crates/assay-types/src/lib.rs:109–130` — `GatesConfig.default_timeout` serde-defaults to 300.
  - Test: `gate::tests::evaluate_timeout` (line 452) — asserts `passed: false`, `exit_code: None`, `stderr` contains `"timed out"`.
  - Test: `gate::tests::resolve_timeout_default_300s` (line 597) — asserts `Duration::from_secs(300)` when all None.

### GATE-04: Explicit `working_dir` parameter on `gate::evaluate()` — never inherit
- **Status:** verified
- **Evidence:**
  - `crates/assay-core/src/gate/mod.rs:85–94` — `evaluate(criterion, working_dir: &Path, timeout)` signature requires explicit `working_dir`.
  - `crates/assay-core/src/gate/mod.rs:237` — `command.current_dir(working_dir)` sets CWD explicitly.
  - Doc comment at line 76: "working_dir is required — this function never inherits the process CWD."
  - `crates/assay-cli/src/main.rs:323–334` — CLI resolves `working_dir` to `config.gates.working_dir` or `project_root()`, never implicitly inheriting.
  - Test: `gate::tests::evaluate_working_dir_is_respected` (line 525) — verifies `pwd` output matches the tempdir path.

### GATE-05: `assay gate run <spec>` CLI command running all executable criteria
- **Status:** verified
- **Evidence:**
  - `crates/assay-cli/src/main.rs:33–37` — `Gate` variant in `Command` enum with `GateCommand` subcommand.
  - `crates/assay-cli/src/main.rs:59–75` — `GateCommand::Run { name, timeout, verbose, json }` with all flags.
  - `crates/assay-cli/src/main.rs:523–531` — wired in `main()` match arm.
  - `crates/assay-cli/src/main.rs:367–418` — streaming loop iterates all criteria, skipping those with no `cmd`.
  - No CLI integration tests exist (see Gaps section).

### GATE-06: `GateKind::FileExists { path }` variant for file existence checks
- **Status:** verified
- **Evidence:**
  - `crates/assay-types/src/gate.rs:23–27` — `GateKind::FileExists { path: String }` variant defined.
  - `crates/assay-core/src/gate/mod.rs:200–222` — `evaluate_file_exists(path, working_dir)` checks `working_dir.join(path).exists()`, returns `GateResult` with `kind: GateKind::FileExists`.
  - `crates/assay-core/src/gate/mod.rs:211–214` — on missing file, stderr is `"file not found: <full_path>"`.
  - Tests: `gate::tests::evaluate_file_exists_present` (line 494) and `gate::tests::evaluate_file_exists_missing` (line 508).
  - Tests: `gate::tests::gate_kind_file_exists_toml_roundtrip` in `assay-types/src/gate.rs:114`.

### GATE-07: Aggregate gate results — summary showing "N/M criteria passed" per spec
- **Status:** verified
- **Evidence:**
  - `crates/assay-core/src/gate/mod.rs:47–60` — `GateRunSummary` struct with `passed`, `failed`, `skipped` counts.
  - `crates/assay-core/src/gate/mod.rs:112–178` — `evaluate_all` accumulates all counts.
  - `crates/assay-cli/src/main.rs:420–429` — summary printed: `"Results: N passed, M failed, K skipped (of T total)"`.
  - Test: `gate::tests::evaluate_all_mixed_criteria` (line 611) — asserts `passed: 1, failed: 1, skipped: 1`.
  - Note: The plan truth states "N/M criteria passed" as the format, but the actual output is "Results: N passed, M failed, K skipped (of T total)". The format is richer than stated but satisfies the intent (see Gaps section).

### GATE-08: Gate evaluation is sync with documented async guidance (`spawn_blocking`)
- **Status:** verified
- **Evidence:**
  - `crates/assay-core/src/gate/mod.rs:1–19` — module-level doc comment states all functions are synchronous, provides `spawn_blocking` example for async callers.
  - `crates/assay-core/src/gate/mod.rs:79–93` — `evaluate()` doc comment: "This function is synchronous."
  - `crates/assay-core/src/gate/mod.rs:106–117` — `evaluate_all()` doc comment: "This function is synchronous."
  - No `async fn` or `await` anywhere in `gate/mod.rs` — only `async` appears in comments/doc strings.

---

## Plan 01 Truths

### Truth 1: Command gate runs `echo hello` → `passed: true`, `stdout` contains `'hello\n'`, `exit_code: 0`, non-zero `duration_ms`
- **Status:** gap
- **Evidence:**
  - `gate::tests::evaluate_echo_hello` (line 406–427) verifies `passed: true`, `stdout.contains("hello")`, `exit_code: Some(0)`, `kind: GateKind::Command`.
  - **Gap:** The test does NOT assert `non-zero duration_ms`. Comment at line 424 explicitly states "duration_ms is populated (may be 0 on very fast machines)". No assertion on `duration_ms > 0` exists. The truth requires non-zero duration_ms but the test does not enforce it.
  - **Gap:** The test asserts `stdout.contains("hello")`, not `stdout == "hello\n"`. The actual stdout from `echo hello` is `"hello\n"` which satisfies `contains("hello")`, but the test is weaker than the stated truth.

### Truth 2: Failing command → `passed: false`, stderr evidence, correct non-zero exit code
- **Status:** verified
- **Evidence:**
  - `gate::tests::evaluate_failing_command` (line 429–448) asserts `passed: false`, `stderr.contains("fail")`, `exit_code: Some(1)`.

### Truth 3: Command exceeding timeout → killed, `exit_code: None`, timeout message in stderr
- **Status:** verified
- **Evidence:**
  - `gate::tests::evaluate_timeout` (line 452–470) asserts `passed: false`, `exit_code: None`, `stderr.contains("timed out")`.
  - Implementation at `gate/mod.rs:339–359` confirms this behavior.

### Truth 4: `gate::evaluate()` requires explicit `working_dir` — no default, no inheritance
- **Status:** verified
- **Evidence:** Function signature at `gate/mod.rs:85` requires `working_dir: &Path`. No default value, no fallback to `std::env::current_dir()`.

### Truth 5: `GateKind::FileExists` checks file existence relative to `working_dir`
- **Status:** verified
- **Evidence:**
  - `gate/mod.rs:200–222` — `working_dir.join(path).exists()`.
  - Tests `evaluate_file_exists_present` and `evaluate_file_exists_missing` both use explicit `tempdir` as working_dir.

### Truth 6: Gate evaluation is sync with no async code in the gate module
- **Status:** verified
- **Evidence:** No `async fn`, `await`, `.await`, or `tokio::` invocations in `gate/mod.rs`. Only documentation references to `spawn_blocking`.

### Truth 7: No CLI/criterion/config timeout set → `resolve_timeout` returns exactly 300 seconds
- **Status:** verified
- **Evidence:**
  - `gate::tests::resolve_timeout_default_300s` (line 597–600): `resolve_timeout(None, None, None) == Duration::from_secs(300)`.
  - Implementation: `gate/mod.rs:189–193` — `.unwrap_or(300)`.

---

## Plan 02 Truths

### Truth 1: `assay gate run <spec>` prints summary showing 'N/M criteria passed' and individual criterion results
- **Status:** gap
- **Evidence:**
  - Individual criterion results: printed per-criterion in the streaming loop (`main.rs:386–416`).
  - Summary: printed at `main.rs:427–429` as `"Results: N passed, M failed, K skipped (of T total)"`.
  - **Gap:** The stated truth says "N/M criteria passed" but the actual format is "Results: N passed, M failed, K skipped (of T total)". This is a more informative format, but the plan truth literally says "N/M criteria passed" which is not the output format. The plan's own success criteria (`07-02-PLAN.md:315`) states "1 passed, 0 failed, 1 skipped" confirming the actual format is intentional and acceptable. The truth statement uses an abbreviated description, not the literal output. No CLI integration test verifies the actual output format.

### Truth 2: Streaming progress shows criterion name while running, then pass/fail on completion (cargo-test style)
- **Status:** verified
- **Evidence:**
  - `main.rs:375–379` — prints `"  <name> ... running"` to stderr before evaluation.
  - `main.rs:395–398` — replaces with `"  <name> ... ok"` or `"  <name> ... FAILED"` on completion.
  - Uses `\r\x1b[K` (carriage return + clear line) for overwrite behavior, matching cargo-test style.

### Truth 3: Failing criteria show stdout/stderr evidence automatically; passing criteria hide evidence unless --verbose
- **Status:** verified
- **Evidence:**
  - `main.rs:401–404` — `if !result.passed || verbose { print_evidence(...) }`.
  - `print_evidence` at `main.rs:440–464` prints stdout and stderr indented.

### Truth 4: `--json` flag emits structured JSON with all results (consistent with `spec show --json`)
- **Status:** verified
- **Evidence:**
  - `main.rs:340–352` — `--json` path calls `evaluate_all()`, serializes `GateRunSummary` via `serde_json::to_string_pretty()`, prints to stdout.
  - `GateRunSummary` is `#[derive(Serialize)]` at `gate/mod.rs:46`.
  - `spec show --json` uses `serde_json::to_string_pretty(&spec)` — same pattern.

### Truth 5: `--timeout` flag overrides the global default timeout for all criteria
- **Status:** verified
- **Evidence:**
  - `main.rs:381–382` — `resolve_timeout(cli_timeout, criterion.timeout, config_timeout)` where `cli_timeout` is from `--timeout`.
  - `resolve_timeout` precedence: CLI > criterion > config > 300s.
  - Test: `gate::tests::resolve_timeout_cli_wins` (line 579) verifies CLI timeout wins.

### Truth 6: Descriptive-only criteria are skipped and shown separately in the summary
- **Status:** gap
- **Evidence:**
  - `main.rs:369–372` — descriptive criteria (`cmd.is_none()`) are silently skipped during streaming (no progress line printed).
  - `main.rs:421–429` — skipped count is included in the summary line: "K skipped".
  - **Gap:** The truth says "shown separately in the summary" — they appear in the summary count but are NOT listed individually by name in the streaming output. During streaming, skipped criteria produce no output at all. Under `--json`, the `GateRunSummary.results` includes them as `CriterionResult { result: None }`. So the "shown separately" claim is only partially true: the count appears in the summary, but individual skipped criterion names are not listed in the streaming display.

### Truth 7: CLI exits with code 1 when any criterion fails; exit 0 when all pass
- **Status:** verified
- **Evidence:**
  - `main.rs:431–433` — `if failed > 0 { std::process::exit(1); }` after summary.
  - `main.rs:348–350` — JSON path also exits 1 if `summary.failed > 0`.
  - Implicit: falls through to normal process exit (0) when no failures.

---

## Test Results

All 103 tests pass with no failures or ignored tests (except 3 doc-tests intentionally marked `ignore`):

```
assay-core: 70 tests — ok
assay-types: 9 tests — ok
assay-types/tests/schema_roundtrip: 15 tests — ok
assay-types/tests/schema_snapshots: 9 tests — ok
assay-cli: 0 tests (no test module)
assay-mcp: 0 tests
assay-tui: 0 tests
```

Gate-specific tests in `assay-core`:
- `gate::tests::evaluate_echo_hello` — ok
- `gate::tests::evaluate_failing_command` — ok
- `gate::tests::evaluate_timeout` — ok
- `gate::tests::evaluate_always_pass_criterion` — ok
- `gate::tests::evaluate_file_exists_present` — ok
- `gate::tests::evaluate_file_exists_missing` — ok
- `gate::tests::evaluate_working_dir_is_respected` — ok
- `gate::tests::evaluate_all_mixed_criteria` — ok
- `gate::tests::evaluate_all_captures_spawn_failure` — ok
- `gate::tests::resolve_timeout_cli_wins` — ok
- `gate::tests::resolve_timeout_criterion_wins_over_config` — ok
- `gate::tests::resolve_timeout_config_used` — ok
- `gate::tests::resolve_timeout_default_300s` — ok
- `gate::tests::resolve_timeout_minimum_floor` — ok
- `gate::tests::truncate_output_within_budget` — ok
- `gate::tests::truncate_output_over_budget` — ok

---

## Gaps Found

### Gap 1: `duration_ms > 0` not asserted in `evaluate_echo_hello` test (Plan 01, Truth 1)
- **Severity:** Minor — test comment acknowledges "may be 0 on very fast machines".
- **Detail:** `gate::tests::evaluate_echo_hello` does not assert `result.duration_ms > 0`. The truth states "non-zero duration_ms" but the implementation allows 0. On fast machines the elapsed time can round to 0 ms. The behavior is correct (duration_ms is measured), but the test does not enforce the non-zero requirement.
- **File:** `crates/assay-core/src/gate/mod.rs:406–427`

### Gap 2: Skipped criteria not listed by name in streaming output (Plan 02, Truth 6)
- **Severity:** Minor — skipped count appears in the summary but individual skipped criterion names are not displayed anywhere in the streaming output.
- **Detail:** The truth says "shown separately in the summary" — the summary line shows "K skipped" as a count but does not enumerate skipped criterion names. This is a cosmetic limitation. Under `--json`, skipped criteria appear in `results` with `result: null`.
- **File:** `crates/assay-cli/src/main.rs:369–372, 420–429`

### Gap 3: No CLI integration tests (applies to Plan 02, Truths 1–7)
- **Severity:** Moderate — the `assay-cli` crate has zero tests. All CLI behaviors (streaming output format, exit codes, `--json` output, `--verbose` flag, `--timeout` override, summary format) are verified only by the plan's manual UAT checkpoint (Task 2), not by automated tests.
- **Detail:** `crates/assay-cli/src/` contains only `main.rs` with no `#[cfg(test)]` module. No `tests/` directory exists under `assay-cli`. The manual UAT in `07-02-PLAN.md` covers 8 scenarios but these are not repeatable automated tests.
- **File:** `crates/assay-cli/` — no test files present

### Gap 4: Summary format mismatch from stated truth (Plan 02, Truth 1)
- **Severity:** Informational — the truth states "N/M criteria passed" but the actual output is "Results: N passed, M failed, K skipped (of T total)". The plan's own success criteria section clarifies the correct format, confirming the truth description was an abbreviation. The implementation is correct and matches the plan's success criteria.
- **File:** `crates/assay-cli/src/main.rs:427–429`
