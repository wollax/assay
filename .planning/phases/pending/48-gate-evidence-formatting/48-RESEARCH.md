# Phase 48: Gate Evidence Formatting - Research

**Completed:** 2026-03-16
**Confidence:** HIGH (all findings from direct codebase investigation)

---

## Standard Stack

Use only what is already in the workspace. No new dependencies needed.

- **String formatting:** `std::fmt::Write` / `format!` macros — build markdown strings via `String` and `write!`/`writeln!`
- **Serialization (report file):** Already available `serde_json` for structured data; the markdown report file is plain text written with `std::fs::write`
- **Atomic file writes:** Follow the `tempfile::NamedTempFile` + persist pattern already used in `crates/assay-core/src/history/mod.rs`
- **Types:** All input types are in `assay-types` — no new type crates needed

## Architecture Patterns

### Input data shape

The formatter consumes `GateRunSummary` (from `assay-types::gate_run`). Key fields:

```
GateRunSummary {
    spec_name: String,
    results: Vec<CriterionResult>,  // per-criterion results
    passed: usize,
    failed: usize,
    skipped: usize,
    total_duration_ms: u64,
    enforcement: EnforcementSummary {
        required_passed, required_failed,
        advisory_passed, advisory_failed
    },
}
```

Each `CriterionResult` contains:
```
CriterionResult {
    criterion_name: String,
    result: Option<GateResult>,  // None = skipped
    enforcement: Enforcement,    // Required | Advisory
}
```

`GateResult` has two distinct shapes depending on `kind`:
1. **Deterministic** (`Command`, `FileExists`, `AlwaysPass`): `stdout`, `stderr`, `exit_code`, `truncated`, `original_bytes`
2. **Agent** (`AgentReport`): `evidence`, `reasoning`, `confidence`, `evaluator_role`

Both share: `passed: bool`, `kind: GateKind`, `duration_ms: u64`, `timestamp: DateTime<Utc>`

### Where formatting logic lives

**Place in `assay-core`, not `assay-types`.** Rationale:
- `assay-types` is "shared serializable types, no business logic" (per CLAUDE.md)
- Formatting is business logic (rendering decisions, truncation strategy)
- Follow the pattern: `assay-core::gate` has evaluation logic, `assay-core::history` has persistence logic
- Create a new module: `assay-core::gate::evidence` (or `assay-core::gate::format`)

### Function signature pattern

Follow the codebase's functional style — free functions, not methods on types:

```rust
// Primary API: returns (pr_body_markdown, full_report_markdown)
pub fn format_gate_evidence(
    summary: &GateRunSummary,
    report_path: &Path,       // where full report will be written (for link text)
    char_limit: usize,        // 65_536 for GitHub
) -> FormattedEvidence

pub struct FormattedEvidence {
    pub pr_body: String,      // truncated to fit char_limit
    pub full_report: String,  // untruncated version for disk
    pub truncated: bool,      // whether pr_body was truncated
}
```

The `GateRunRecord` wrapper adds `run_id`, `timestamp`, `assay_version` — these can be included as footer metadata. Consider accepting `&GateRunRecord` instead of `&GateRunSummary` if footer metadata is desired.

### Report file persistence

Follow the `history::save` pattern: use `std::fs::write` (or atomic tempfile if crash safety matters). The report file is a nice-to-have artifact, not a critical state file, so simple `std::fs::write` is acceptable.

Report file location suggestion: `.assay/reports/<spec-name>/<run-id>.md` (parallel to `.assay/results/<spec-name>/<run-id>.json`).

## Don't Hand-Roll

1. **Atomic file writes** — use the existing `tempfile` crate pattern from `history/mod.rs` if crash safety is needed
2. **Run ID generation** — use `history::generate_run_id()` (already exists)
3. **Path validation** — use `history::validate_path_component()` for spec names in file paths
4. **Enforcement resolution** — `Enforcement` and `EnforcementSummary` are already on `CriterionResult` and `GateRunSummary`; don't re-derive them
5. **Character counting** — use `str::len()` (byte length) for the 65,536 limit since GitHub counts bytes, not Unicode characters

## Common Pitfalls

### P1: GitHub counts bytes, not characters (HIGH confidence)
GitHub's 65,536 limit is on the JSON payload body field, which counts UTF-8 bytes. Use `.len()` not `.chars().count()`. The constant should be `65_536_usize`.

### P2: `<details>` tag rendering quirks (HIGH confidence)
GitHub markdown requires a blank line after `<summary>` and before content for markdown rendering inside `<details>` blocks:
```markdown
<details>
<summary>Title</summary>

Content with **markdown** renders correctly.

</details>
```
Without the blank lines, markdown inside `<details>` renders as raw text.

### P3: Truncation must preserve valid markdown (HIGH confidence)
Cutting markdown mid-table or mid-`<details>` block produces broken rendering. The truncation strategy must cut at section boundaries (between criterion detail blocks), not at arbitrary character positions.

### P4: The existing `truncate_head_tail` is wrong for this use case (HIGH confidence)
The gate module's `truncate_head_tail()` (32 KiB budget, head/tail split) is designed for raw command output. PR body truncation needs semantic truncation — removing criterion detail sections from the bottom until the output fits, then appending a "truncated" notice with a link to the full report. Do NOT reuse `truncate_head_tail`.

### P5: Empty results edge case (MEDIUM confidence)
`GateRunSummary.results` can be an empty vec (all criteria skipped, or spec has no criteria). The formatter must handle this gracefully — produce a valid markdown body, not crash or produce broken output.

### P6: Long `reasoning`/`evidence` strings from agents (MEDIUM confidence)
Agent-evaluated criteria have free-text `reasoning` and `evidence` fields with no size cap. A single criterion's detail section could exceed the 65,536 limit on its own. The truncation strategy must handle this — either truncate individual sections or skip detail sections entirely.

### P7: GitHub rendering of tables with pipes in content (LOW confidence)
If `stdout`/`stderr` or `reasoning` text contains `|` characters, they break markdown table rendering. Use `<details>` blocks for verbose content, not table cells.

## Code Examples

### Markdown structure (recommended output format)

```markdown
## Gate Results: spec-name

**Result:** 3/4 passed | 1 failed | 0 skipped
**Duration:** 2.3s
**Enforcement:** 3 required passed, 0 required failed | 1 advisory failed

| Status | Criterion | Enforcement | Duration |
|--------|-----------|-------------|----------|
| :white_check_mark: | cargo-test | required | 1.5s |
| :white_check_mark: | readme-exists | required | 0.0s |
| **:x:** | **lint-check** | **advisory** | **0.8s** |
| :white_check_mark: | code-review | required | 0.1s |

<details>
<summary>:x: lint-check (advisory) - FAILED</summary>

**Command:** `cargo clippy`
**Exit code:** 1

**stderr:**
```
warning: unused variable
```

</details>

<details>
<summary>:white_check_mark: code-review (required) - PASSED</summary>

**Evidence:** Found auth module with JWT validation
**Reasoning:** JWT validation present and tests pass
**Confidence:** high

</details>
```

### Truncation strategy (semantic, not character-level)

```rust
fn truncate_to_limit(
    summary_section: &str,    // always preserved
    detail_sections: &[String], // removed last-to-first
    report_path: &Path,
    char_limit: usize,
) -> (String, bool) {
    // 1. Try full output (summary + all details)
    // 2. If over limit, remove detail sections from the end
    //    (keep failures, remove passes first)
    // 3. If still over limit, remove all detail sections
    // 4. If summary alone exceeds limit, truncate summary table rows
    // 5. Append truncation notice with link to full report
}
```

### Priority order for detail section removal during truncation

1. Remove collapsed (passing, deterministic) detail sections first — these have lowest value
2. Remove collapsed (passing, agent) detail sections next
3. Remove expanded (failing) detail sections last — these have highest value
4. If summary table alone exceeds limit, truncate table rows (extremely unlikely with typical specs)

### Formatting duration helper

```rust
fn format_duration(ms: u64) -> String {
    if ms < 1_000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", ms as f64 / 1_000.0)
    }
}
```

## Key Findings

### F1: No existing formatting/rendering module in the codebase
There is no precedent for markdown generation in assay-core. This is greenfield. The closest pattern is the string-building in `gate::format_command_error()` which uses simple `format!()` calls.

### F2: `GateRunRecord` vs `GateRunSummary` as input
`GateRunRecord` wraps `GateRunSummary` with `run_id`, `assay_version`, `timestamp`, `working_dir`, and `diff_truncation`. For a PR body, the run metadata is useful for the footer. Recommend accepting `&GateRunRecord` as input rather than just `&GateRunSummary`.

### F3: The 65,536 limit is well-documented
GitHub API returns `422 Unprocessable Entity` when the PR body exceeds 65,536 characters. This is the `body` field in the REST API `POST /repos/{owner}/{repo}/pulls` endpoint.

### F4: Existing truncation infrastructure
The codebase has `truncate_head_tail()` and `truncate_diff()` in `gate/mod.rs`. These are byte-budget, head+tail-split strategies for raw output. They are NOT suitable for semantic markdown truncation but demonstrate the project's approach to truncation: explicit budget, metadata about what was truncated, and clean UTF-8 boundary handling.

### F5: Phase 50 will consume this output
Phase 50 (Merge Propose) will call this formatting function and pass the result to `gh pr create --body`. The API boundary should return a ready-to-use string, not require further processing.

### F6: `CriterionOutcome` enum exists but is not on `GateResult`
The `evaluator.rs` module has `CriterionOutcome` (Pass/Fail/Skip/Warn) but `GateResult` uses a simple `bool passed` field. The formatter will need to derive status from `passed: bool` + `result: Option<GateResult>` (None = skipped).

---

*Phase: 48-gate-evidence-formatting*
*Research completed: 2026-03-16*
