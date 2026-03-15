# Phase 45: Tech Debt Cleanup — Research

**Researched:** 2026-03-15
**Confidence:** HIGH — all findings come from direct file inspection of the codebase and issue files.

---

## Overview

The backlog contains **270 open issues** (`.planning/issues/open/`). This research surveys all of them, categorises by crate and type, identifies v0.4.0-interacting issues (highest priority), and recommends grouping into independently-shippable plans.

---

## Issue Inventory by Date / Source

| Date range | Count | Source / era |
|---|---|---|
| 2026-03-01 | 11 | Early brainstorm / phase 3-8 era |
| 2026-03-02 | 9 | Phase 7-8 PR reviews |
| 2026-03-04 | 28 | Phase 9-12 PR reviews |
| 2026-03-05 | 22 | Phase 13-14 PR reviews |
| 2026-03-06 | 3 | Misc |
| 2026-03-07 | 18 | Guard daemon (phase ~25) |
| 2026-03-09 | 27 | Worktree (phase 35-36) + truncation |
| 2026-03-10 | 24 | Gate/MCP/CLI (phase 35-38) |
| 2026-03-11 | 24 | Spec validation (phase 37) |
| 2026-03-13 | 6 | Context engine, spec_get (phase 39-41) |
| 2026-03-15 | **68** | **v0.4.0 phases 40-44 (sessions, evaluator, recovery)** |

---

## Staleness / Won't-Fix Candidates

### HIGH confidence won't-fix (superseded by v0.4.0 architecture)

These reference phases, types, or APIs that were replaced or significantly restructured:

| Issue | Reason |
|---|---|
| `2026-03-01-phase3-output-detail-enum` | `OutputDetail` enum was not implemented; gate evaluation uses subprocess model now. Architecture has moved on. |
| `2026-03-01-phase3-wire-format-types` | Phase 3 wire-format design discussion. MCP response types were designed differently in phases 40-44. |
| `2026-03-01-phase7-streaming-capture` | Streaming capture design note for phase 7; truncation was implemented differently (truncate_head_tail). |
| `2026-03-01-phase8-progressive-disclosure` | Progressive disclosure / `gate_evidence` concept from phase 8 brainstorm. Not implemented; superseded. |
| `2026-03-01-comment-cleanup-phase3` | References "phase 3" residue. Likely resolved or superseded. |
| `2026-03-01-ci-plugin-schema-validation` | CI plugin JSON schema; the plugin system wasn't implemented in v0.4.0 scope. |
| `2026-03-01-type-invariant-enforcement` | Very early design note; not actionable in isolation. |
| `2026-03-01-error-ergonomics` | General error ergonomics brainstorm from phase 3 era. |
| `2026-03-01-cli-spec-cleanup` | Phase 3 CLI cleanup note; may be resolved. |
| `2026-03-01-spec-type-refinements` | Phase 3 spec type refinements; some may be covered, some deferred. |
| `2026-03-01-tui-use-try-init` | TUI-specific (assay-tui). Out of scope for v0.4.0 cleanup sweep. |

**Recommendation:** Move 2026-03-01 issues (except `tui-use-try-init`) to `closed/` as won't-fix with note "superseded by v0.4.0 architecture." `tui-use-try-init` can be closed as won't-fix (TUI not in scope).

### MEDIUM confidence won't-fix (guard daemon issues)

The guard daemon (`crates/assay-core/src/guard/`) is a separate subsystem not touched by phases 35-44. Its 18 issues (2026-03-07) are self-contained improvements but **do not interact with v0.4.0 changes**. They are lower priority for this sweep but are actionable.

**Recommendation:** Deprioritise all `2026-03-07-guard-*` issues for this phase. They form a coherent sub-sweep of their own. Skip or defer to a dedicated guard-cleanup phase.

### NOT stale — early issues that still apply

Several 2026-03-04 and 2026-03-05 issues describe persistent problems:

- `gate-finalize-untyped-response` — `gate_finalize` still lacks a typed response struct (Severity: Important)
- `gate-history-entry-missing-passed-counts` — `GateHistoryEntry` still missing `required_passed`/`advisory_passed`
- `gate-history-silent-entry-skip` — Still applies to `gate_history` handler
- `gate-history-unused-config-load` — `gate_history` loads config unnecessarily
- `spec-get-silent-feature-spec-error` — `spec_get` silently swallows feature spec errors
- `session-timeout-dead-wd-capture` — Dead `wd_string` capture in timeout task

---

## v0.4.0-Interacting Issues (Primary Priority)

These 68 issues were filed 2026-03-15 and directly touch code written in phases 40-44.

### Crate: assay-types (work_session.rs, evaluator.rs, lib.rs)

**Types / derives (trivial, batch together):**
- `criterion-outcome-missing-hash-derive` — `CriterionOutcome` missing `Hash`
- `diff-truncation-missing-hash-derive` — `DiffTruncation` missing `Hash`
- `evaluator-result-types-missing-hash-derive` — `EvaluatorCriterionResult`, `EvaluatorSummary` missing `Hash`
- `phase-transition-hash-derive` — `PhaseTransition` missing `Hash`
- `recovery-summary-missing-derives` — `RecoverySummary` missing `Clone`, `PartialEq`
- `sessions-config-default-derive` — `SessionsConfig` should derive `Default`
- `diff-truncation-usize-vs-u64` — `DiffTruncation` byte fields should be `u64`

**Naming / consistency:**
- `stale-threshold-secs-naming` — `stale_threshold` → `stale_threshold_secs` (breaking config change)
- `evaluator-criterion-result-name-doc-overstates` — doc fix on `name` field
- `gate-runs-id-format-doc` — doc the ID format for `gate_runs: Vec<String>`

**Design:**
- `session-phase-non-exhaustive` — `SessionPhase` should be `#[non_exhaustive]`
- `trigger-string-to-enum` — `PhaseTransition.trigger: String` → `TransitionTrigger` enum (medium complexity)
- `session-id-newtype` — `WorkSession.id: String` → `SessionId` newtype (medium complexity, multi-file)
- `gate-evaluate-response-missing-serde-default` — `GateEvaluateResponse.diff_truncation` missing `serde(default)`

**Validation:**
- `stale-threshold-accepts-zero` — validate non-zero at config-load time

### Crate: assay-core (work_session.rs, evaluator.rs, context/budgeting.rs, checkpoint/)

**Evaluator fixes:**
- `budget-priority-magic-numbers` — Name constants 80/50 in `budget_context`
- `evaluator-error-missing-io-variant` — Add `Io` variant to `EvaluatorError` (Medium severity)
- `run-evaluator-last-error-unreachable` — Replace misleading fallback with `unreachable!()`
- `evaluator-schema-lazy-lock-caching` — Cache schema with `LazyLock`
- `map-evaluator-output-duration-param` — Accept `Duration` not raw `duration_ms: u64`
- `map-evaluator-output-imperative-counters` — Refactor to functional fold
- `build-evaluator-prompt-empty-diff-untested` — Add test
- `map-evaluator-output-empty-criteria-no-count-assertions` — Strengthen test
- `map-evaluator-output-warn-required-untested` — Add test
- `map-pass-outcome-kind-role-test-incomplete` — Extend test
- `schema-generation-test-key-structure-not-asserted` — Strengthen test
- `budget-test-empty-system-prompt` — Add test
- `extract-diff-files-rename-test` — Add rename diff test

**Session / recovery fixes:**
- `load-session-validate-path` — Add path-traversal guard to `load_session`
- `save-session-json-error-context` — Fix error path to reference file, not dir
- `sessions-config-doc-phase-ref` — Doc fix: `agent_running` case ref
- `previous-phase-capture-fragile` — Refactor closure in `session_update`
- `convenience-fn-error-paths-untested` — Test error paths of `record_gate_result` / `complete_session`
- `list-sessions-non-json-filter-test` — Add non-JSON file filter test
- `full-lifecycle-transition-fields` — Add per-transition field assertions in `full_lifecycle` test
- `session-phase-deserialization-tests` — Add `SessionPhase` round-trip tests
- `phase-transition-notes-some-test` — Test `notes: Some(...)` path
- `recover-skips-non-running-skipped-assert` — Tighten test assertions
- `recovery-scan-cap-untested` — Test 100-session cap
- `recovery-summary-truncated-field` — Add `truncated: bool` to `RecoverySummary`
- `load-recovery-threshold-untested` — Add tests for `load_recovery_threshold`
- `checkpoint-timestamp-silent-write` — Propagate or warn on `.last-checkpoint-ts` write error

**Context budgeting (phase 44):**
- `budget-context-request-struct` — Wrap 5 params in `BudgetRequest<'a>` struct
- `budget-context-return-type` — Return `BudgetedContext` struct instead of `Vec<String>`
- `budget-from-cupel-error` — Consider `From<CupelError>` impl

### Crate: assay-mcp (server.rs)

**Session MCP tools (phase 41):**
- `session-create-agent-command-example-stale` — Update stale doc example
- `session-create-agent-model-example-stale` — Update stale doc example
- `session-response-warnings-always-empty` — Investigate/fix warnings field
- `session-list-entry-missing-worktree-path` — Add `worktree_path` to list entry
- `session-list-ordering-claim-delegated` — Fix/clarify ordering doc
- `session-update-gate-run-dedup-linear` — Dedup gate_run IDs (O(n) is fine)
- `session-create-worktree-path-absolute-validation` — Validate path is absolute
- `session-create-spec-validation-overhead` — Document/remove spec validation in session_create

**Evaluator MCP tools (phase 43-44):**
- `evaluate-criterion-result-freeform-enum-strings` — Fix `outcome`/`enforcement` re-serialization in `EvaluateCriterionResult`
- `spawn-blocking-clone-naming-convention` — Use idiomatic shadowing in `spawn_blocking` clones
- `server-module-doc-tool-count` — Remove hardcoded tool count from module doc
- `zero-timeout-not-validated` — Validate non-zero timeout in MCP server (Medium severity)
- `load-recovery-threshold-untested` — Covered above (assay-mcp tests)
- `tool-count-in-docs-fragile` — Remove fragile tool count
- `gate-report-session-id-ambiguous` — Clarify session_id semantics in gate_report
- `gate-report-warnings-comment-noise` — Clean up comment noise

**Gate history (phase 35-38 era, still relevant):**
- `gate-finalize-untyped-response` — Create `GateFinalizeResponse` struct (Important)
- `gate-history-entry-missing-passed-counts` — Add `required_passed`/`advisory_passed` (Important)
- `gate-history-silent-entry-skip` — Log instead of silently drop
- `gate-history-unused-config-load` — Remove unnecessary config load
- `history-save-failure-not-surfaced` — Surface history save error to caller
- `gate-history-total-runs-doc-unclear` — Doc fix
- `gate-runs-id-format-doc` — Cross-reference format (see assay-types above)
- `get-info-session-vs-gate-workflow-clarity` — Clarify session vs gate workflow in docs

**Spec MCP (phase 41, 2026-03-13):**
- `resolved-block-clone-to-move` — Avoid clone in `spec_get` resolved block
- `spec-get-resolve-directory-format-test` — Add directory-format test
- `spec-get-resolve-duplicate-description` — Remove duplicate description

---

## Non-v0.4.0 Issues with High Actionability

These are from phases 35-38 (worktree, truncation, gate/MCP) and are still relevant.

### Truncation (assay-core gate/)

All filed 2026-03-09, area: assay-core `gate/mod.rs`:
- `truncation-result-missing-debug` — Add `#[derive(Debug)]` to `TruncationResult` (trivial)
- `truncate-over-budget-test-assertions` — Strengthen test
- `truncate-marker-newline-test` — Add newline format test
- `truncate-multiline-input-test` — Add multi-line test
- `truncate-omitted-debug-assert` — Add `debug_assert!` invariant
- `truncate-head-tail-unnecessary-alloc` — Accept `String` by value / return `Cow<str>`

### Worktree (assay-types, assay-core, assay-cli, assay-mcp)

Filed 2026-03-09:
- `worktree-missing-deny-unknown-fields` — Add `#[serde(deny_unknown_fields)]` to `WorktreeInfo`, `WorktreeStatus`
- `worktree-usize-serialization` — Change `ahead`/`behind` from `usize` to `u32`
- `worktree-types-not-in-schema-registry` — Register `WorktreeInfo`, `WorktreeStatus` with inventory
- `worktree-types-field-duplication` — Compose `WorktreeStatus` via `#[serde(flatten)]` (medium)
- `worktree-prune-failure-silent` — Log warning on prune failure (assay-core)
- `worktree-status-unwrap-or-false` — Distinguish error types in cleanup (assay-cli)
- `worktree-list-ignores-worktree-dir-flag` — Remove or document ignored flag (assay-cli)
- `worktree-list-mcp-no-project-check` — Add `load_config` call (assay-mcp)
- `worktree-cleanup-all-path` — Use canonical git path always (assay-cli)
- `worktree-env-var-undocumented` — Doc the env var (assay-core)
- `worktree-detect-main-error-handling` — Improve error handling
- `worktree-detect-default-branch-fallback` — Handle fallback
- `worktree-dirty-error-cli-advice` — Better error message
- `worktree-cli-error-chain-lost` — Preserve error chain
- `worktree-config-base-dir-type` — Type fix
- `worktree-mcp-cleanup-no-all` — MCP worktree_cleanup missing --all support
- `worktree-test-empty-base-dir` — Add test
- `worktree-test-force-clean` — Add test
- `worktree-test-parse-malformed` — Add test
- `worktree-to-string-lossy-non-utf8` — Fix non-UTF8 path handling
- `worktree-prune-failure-silent` — Already listed

### Gate / CLI (assay-cli, assay-core)

Filed 2026-03-10:
- `command-error-kind-derive-hash` — Add `Hash` to `CommandErrorKind`
- `enriched-error-display-rename` — Rename to `format_enriched_error`
- `first-nonempty-line-whitespace` — Trim whitespace in `first_nonempty_line` (Medium)
- `is-executable-filter-repeated` — Extract helper method
- `levenshtein-collect-chars-upfront` — Performance fix
- `spec-entry-criteria-extraction-dup` — Consolidate duplicate match blocks
- `evidence-truncation-magic-number` — Named constant + consistent strategy
- `format-relative-time-dedup` — Extract shared inner function
- `column-gap-invisible-value` — Add comment
- `classify-exit-code-boundary-tests` — Add boundary tests
- `format-command-error-test-weak-assert` — Strengthen test
- `format-toml-error-multiline-weak-assert` — Strengthen test
- `gate-exit-code-doc-cross-ref` — Add doc cross-reference
- `stream-config-new-missing-doc` — Add doc comment
- `stream-counters-failed-doc-unclear` — Clarify field doc
- `config-text-utils-wrong-module` — Move utils to `crate::fmt` (medium refactor)
- `deterministic-results-variable-name` — Rename variable
- `gate-history-params-missing-name-test` — Add test
- `format-spec-not-found-boundary-test` — Add boundary test
- `format-spec-not-found-vec-simplify` — Simplify vec usage
- `format-criteria-type-static-str` — Return `&'static str`
- `mcp-tests-no-specs-disjunction` — Pin test assertions
- `multiline-stdout-fallback-test` — Add test
- `timeout-test-assertion-escape-hatch` — Strengthen test
- `test-unwrap-expect-messages` — Add `.expect()` messages
- `failure-reason-test-helper` — Extract test helper
- `levenshtein-transposition-test` — Add test case

### Spec Validation (assay-types, assay-core/spec/)

Filed 2026-03-11:
- `cycle-diagnostic-missing-debug-derive` — `#[derive(Debug)]`
- `diagnostic-derive-hash` / `diagnostic-missing-hash-derive` — Add `Hash` to `Diagnostic` (duplicate issues)
- `validation-result-diagnostic-summary-derive-default` — Derive `Default`
- `diagnostic-summary-info-naming` — Naming inconsistency fix
- `depends-allows-duplicate-entries` — Validate for duplicates
- `depends-field-slug-keyed-undocumented` — Add doc
- `empty-whitespace-depends-not-validated` — Validate whitespace entries
- `finalize-as-timed-out-duplication` — Extract shared helper
- `persisted-field-fragile-derivation` — Fix fragile derivation from `warnings.is_empty()`
- `build-summary-as-diagnostic-summary-method` — Refactor to method
- `build-summary-imperative-loop-vs-functional` — Refactor to functional
- `validate-spec-feature-spec-ignored-undocumented` — Document `FeatureSpec` skip
- `dfs-invariant-violation-silent-continue` — Emit warning on invariant violation
- Various test gap issues (detect_cycles, validate_spec, agent_prompts, check_commands)

### Context Engine (assay-core/context/, assay-types)

Filed 2026-03-13:
- `collect-turn-tokens-none-message-test` — Add test for `message: None`
- `growth-rate-roundtrip-test` — Add `GrowthRate` serde round-trip test
- `growth-rate-threshold-visibility` — Expose `MIN_TURNS_FOR_GROWTH_RATE` as pub const

---

## Issue Categories Summary

| Category | Count | Typical effort |
|---|---|---|
| Missing derives (`Hash`, `Clone`, `PartialEq`, `Default`, `Debug`) | ~20 | Trivial (1–2 lines each) |
| Doc fixes / naming | ~25 | Trivial |
| Test gaps (missing test, weak assertions) | ~60 | Low (add 5-15 line test) |
| Error handling / correctness | ~25 | Low-Medium |
| Refactors / code quality | ~30 | Low-Medium |
| Design / type changes | ~15 | Medium (multi-file) |
| Won't-fix / superseded | ~11 | No code change |
| Guard daemon (defer) | 18 | Own sweep |

---

## Recommended Plan Grouping

### Plan A: assay-types v0.4.0 sweep
**Target:** `assay-types/src/work_session.rs`, `evaluator.rs`, `lib.rs`, `worktree.rs`

Issues:
- All missing derives on v0.4.0 types (`CriterionOutcome`, `DiffTruncation`, `EvaluatorCriterionResult`, `EvaluatorSummary`, `PhaseTransition`, `RecoverySummary`, `SessionsConfig`)
- `session-phase-non-exhaustive`
- `gate-evaluate-response-missing-serde-default`
- `stale-threshold-secs-naming` (breaking — rename field)
- `stale-threshold-accepts-zero` (add validation)
- `worktree-missing-deny-unknown-fields`
- `worktree-usize-serialization` (u32)
- `worktree-types-not-in-schema-registry`
- `gate-runs-id-format-doc`
- `evaluator-criterion-result-name-doc-overstates`
- `sessions-config-doc-phase-ref`

**Crate impact:** assay-types only. Compile-time safe.

### Plan B: assay-core evaluator sweep
**Target:** `assay-core/src/evaluator.rs`, `context/budgeting.rs`

Issues:
- `evaluator-error-missing-io-variant`
- `run-evaluator-last-error-unreachable`
- `budget-priority-magic-numbers`
- `evaluator-schema-lazy-lock-caching`
- `map-evaluator-output-duration-param`
- `map-evaluator-output-imperative-counters`
- All evaluator test gaps (5 issues)
- `budget-context-request-struct` + `budget-context-return-type` (if gate_evaluate has one caller)
- `budget-from-cupel-error`
- `extract-diff-files-rename-test`
- `schema-generation-test-key-structure-not-asserted`

**Crate impact:** assay-core, may require assay-mcp test updates for changed call sites.

### Plan C: assay-core session/recovery sweep
**Target:** `assay-core/src/work_session.rs`, `checkpoint/`

Issues:
- `load-session-validate-path`
- `save-session-json-error-context`
- `checkpoint-timestamp-silent-write`
- `previous-phase-capture-fragile` (technically assay-mcp but touches session logic)
- `convenience-fn-error-paths-untested`
- `list-sessions-non-json-filter-test`
- `full-lifecycle-transition-fields`
- `session-phase-deserialization-tests`
- `phase-transition-notes-some-test`
- `recover-skips-non-running-skipped-assert`
- `recovery-scan-cap-untested`
- `recovery-summary-truncated-field` (adds field to type in assay-core)
- `recovery-summary-missing-derives` (covered in Plan A if moved to assay-types, but `RecoverySummary` lives in assay-core)

**Crate impact:** assay-core primary, assay-mcp (server.rs references RecoverySummary).

### Plan D: assay-mcp session tools + gate_finalize sweep
**Target:** `assay-mcp/src/server.rs` — session handlers, gate_history, gate_finalize

Issues:
- `gate-finalize-untyped-response` (Important — create `GateFinalizeResponse`)
- `gate-history-entry-missing-passed-counts` (Important)
- `gate-history-silent-entry-skip`
- `gate-history-unused-config-load`
- `history-save-failure-not-surfaced`
- `session-timeout-dead-wd-capture`
- `session-response-warnings-always-empty`
- `session-create-agent-command-example-stale`
- `session-create-agent-model-example-stale`
- `evaluate-criterion-result-freeform-enum-strings`
- `spawn-blocking-clone-naming-convention`
- `server-module-doc-tool-count`
- `zero-timeout-not-validated`
- `gate-report-session-id-ambiguous`
- `gate-history-total-runs-doc-unclear`
- `get-info-session-vs-gate-workflow-clarity`
- `load-recovery-threshold-untested`
- `gate-report-warnings-comment-noise`
- `first-nonempty-line-whitespace` (medium correctness fix)
- `mcp-tests-no-specs-disjunction`
- `multiline-stdout-fallback-test`
- `failure-reason-test-helper`
- `spec-get-silent-feature-spec-error` (Important, 2026-03-05)
- `resolved-block-clone-to-move`
- `spec-get-resolve-directory-format-test`
- `spec-get-resolve-duplicate-description`
- `worktree-list-mcp-no-project-check`

**Crate impact:** assay-mcp only. Largest single file (server.rs 209KB).

### Plan E: assay-core truncation + gate sweep
**Target:** `assay-core/src/gate/mod.rs`, `assay-cli/src/commands/gate.rs`, `assay-core/src/spec/mod.rs`

Issues:
- `truncation-result-missing-debug`
- `truncate-over-budget-test-assertions`
- `truncate-marker-newline-test`
- `truncate-multiline-input-test`
- `truncate-omitted-debug-assert`
- `truncate-head-tail-unnecessary-alloc`
- `command-error-kind-derive-hash`
- `enriched-error-display-rename`
- `is-executable-filter-repeated`
- `levenshtein-collect-chars-upfront`
- `classify-exit-code-boundary-tests`
- `gate-exit-code-doc-cross-ref`
- `levenshtein-transposition-test`
- `spec-entry-criteria-extraction-dup` (also touches gate.rs)
- `evidence-truncation-magic-number`

**Crate impact:** assay-core and assay-cli.

### Plan F: assay-cli cosmetic / CLI sweep
**Target:** `assay-cli/src/commands/mod.rs`, `gate.rs`, `spec.rs`, `worktree.rs`

Issues:
- `column-gap-invisible-value`
- `format-relative-time-dedup`
- `format-spec-not-found-boundary-test`
- `format-spec-not-found-vec-simplify`
- `format-criteria-type-static-str`
- `stream-config-new-missing-doc`
- `stream-counters-failed-doc-unclear`
- `deterministic-results-variable-name`
- `gate-history-params-missing-name-test`
- `test-unwrap-expect-messages`
- `format-command-error-test-weak-assert`
- `format-toml-error-multiline-weak-assert`
- `timeout-test-assertion-escape-hatch`
- `worktree-list-ignores-worktree-dir-flag`
- `worktree-cleanup-all-path`
- `worktree-status-unwrap-or-false`
- `worktree-dirty-error-cli-advice`
- `worktree-cli-error-chain-lost`

**Crate impact:** assay-cli only.

### Plan G: Spec validation sweep (assay-types validation + assay-core/spec/)

Issues:
- `cycle-diagnostic-missing-debug-derive`
- `diagnostic-derive-hash` (duplicate with `diagnostic-missing-hash-derive` — pick one, close other)
- `validation-result-diagnostic-summary-derive-default`
- `diagnostic-summary-info-naming`
- `depends-allows-duplicate-entries`
- `depends-field-slug-keyed-undocumented`
- `empty-whitespace-depends-not-validated`
- `finalize-as-timed-out-duplication` (assay-core/gate/session.rs)
- `persisted-field-fragile-derivation` (assay-mcp)
- `build-summary-as-diagnostic-summary-method`
- `build-summary-imperative-loop-vs-functional`
- `validate-spec-feature-spec-ignored-undocumented`
- `dfs-invariant-violation-silent-continue`
- Test gaps: `detect-cycles-*`, `validate-spec-*`, `agent-report-criteria-index-gt-0-untested`, `build-summary-info-count-untested`

**Crate impact:** assay-types, assay-core/spec/, assay-mcp (persisted field).

### Plan H: Won't-fix triage (no code changes)
Move to `closed/` with notes:
- All 11 `2026-03-01-*` issues (superseded)
- All 18 `2026-03-07-guard-*` issues (defer to dedicated guard sweep — not won't-fix, just out of scope)
- Duplicate issues: `diagnostic-derive-hash` vs `diagnostic-missing-hash-derive` (close duplicate)

**Note on guard issues:** Recommend moving to `closed/won't-fix` for THIS phase with "deferred to guard cleanup phase" note, not permanently closing.

---

## Architecture Patterns for Fixes

### Missing derives
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Foo { ... }
```
Just add to existing derive list. No logic changes.

### `#[non_exhaustive]`
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionPhase { ... }
```
Downstream match sites in the same crate are unaffected (only external crates). All match sites are within the workspace, so run `just build` to find any that need `_ => unreachable!()` or similar.

### `serde(default)` on optional fields
```rust
#[serde(default)]
pub diff_truncation: Option<DiffTruncation>,
```
Safe to add — backward compatible.

### Validation in config types
```rust
impl SessionsConfig {
    pub fn validate(&self) -> Result<(), AssayError> {
        if self.stale_threshold_secs == 0 {
            return Err(AssayError::config("stale_threshold_secs must be > 0"));
        }
        Ok(())
    }
}
```
Call at server startup where config is loaded.

### `LazyLock` for static schema caching
```rust
static EVALUATOR_SCHEMA: LazyLock<String> = LazyLock::new(|| {
    schemars::schema_for!(EvaluatorOutput)
        .to_string()
});
```
`LazyLock` is stable since Rust 1.80. Verify Rust edition in `Cargo.toml`.

### `unreachable!()` for documented invariants
```rust
_ => unreachable!("run_evaluator exhausted retries with no last_error set"),
```

### Functional fold replacing imperative counters
```rust
let (required_passed, required_failed, advisory_passed, advisory_failed) = results
    .iter()
    .fold((0usize, 0, 0, 0), |acc, r| match r { ... });
```

---

## Common Pitfalls

1. **`stale_threshold_secs` rename is a breaking config change.** Any existing `.assay/config.toml` files using `stale_threshold` will silently use the default after rename. Add `#[serde(alias = "stale_threshold")]` for backward compat, or document as breaking.

2. **`#[non_exhaustive]` on `SessionPhase`.** All `match` on `SessionPhase` within the workspace must be found and updated. Run `just build` — the compiler will catch them.

3. **`session-id-newtype` is a medium-complexity change.** `WorkSession.id` is referenced across `assay-types`, `assay-core`, and `assay-mcp`. If implementing, do it in its own plan step and search all usages before starting.

4. **`trigger-string-to-enum` is medium complexity.** `PhaseTransition.trigger` is serialized on-disk. Adding a `TransitionTrigger` enum requires serde compat (untagged or string rename). The `gate_run:<id>` variant needs special handling.

5. **`budget-context-return-type` (return `BudgetedContext`).** Phase 44 is the only caller today. Check `budget_context` call sites before wrapping — if there's exactly one caller in `evaluator.rs`, the struct is straightforward. If `budget-context-request-struct` is done simultaneously, do both in the same commit.

6. **`worktree-types-field-duplication` (`#[serde(flatten)]`).** This is a breaking serde change — the JSON shape changes. Confirm no external consumers depend on the current shape before applying.

7. **Duplicate issues.** `diagnostic-derive-hash` and `diagnostic-missing-hash-derive` describe the same fix. Close one, fix via the other.

8. **Guard issues (2026-03-07).** These reference `crates/assay-core/src/guard/`. The guard daemon code is in the workspace. The issues are not stale — the code exists. They are just low priority for this sweep.

---

## Complexity Assessment

| Tier | Description | Issues |
|---|---|---|
| Trivial (< 5 min) | Add derive, fix doc, add const | ~80 issues |
| Low (< 30 min) | Add test, small refactor, rename | ~100 issues |
| Medium (30–90 min) | Multi-file refactor, new struct, type change | ~25 issues |
| Skip (too complex) | `session-id-newtype`, `trigger-string-to-enum`, `worktree-types-field-duplication`, `budget-context-return-type` + `budget-context-request-struct` | 4-6 issues |

The minimum of 10 closed issues is easily achievable from trivial + low tier alone. Plans A+B alone cover 20+ issues.

---

## Verification Strategy

Per the phase requirements: **all resolved issues verified by `just ready`** (fmt-check + lint + test + deny). Each plan must pass `just ready` independently before moving to the next.

Order of plans by risk (lowest first): A → E → F → G → B → C → D

Plan A is purely additive (derives, doc, serde attrs) — lowest risk.
Plan D (assay-mcp server.rs) is highest risk — largest file, most test surface.
