---
phase: 45-tech-debt-cleanup
plan: 04
status: complete
commits:
  - 79f3df5 feat(45-02): Task 2 — field type fixes, validation, doc fixes (validate.rs additions)
  - b6b000f docs(45-02): complete assay-types v0.4.0 sweep plan (session.rs, gates_spec.rs, issue moves)
issues_resolved: 17
---

# 45-04 Summary: Spec Validation Sweep

## What Was Done

Hardened the spec validation subsystem used by the `spec_validate` MCP tool. Changes span
`assay-types/src/validation.rs`, `assay-core/src/spec/validate.rs`,
`assay-core/src/gate/session.rs`, and `assay-types/src/gates_spec.rs`.

Note: The changes in this plan were committed as part of the concurrent 45-02 executor run.
All must_haves are satisfied at HEAD.

## Task 1: Validation Type Fixes (assay-types)

**Derives added:**
- `Diagnostic`: added `Hash` derive (all fields — `Severity`, `String` — are Hash-compatible)
- `CycleDiagnostic`: added `Debug` derive
- `ValidationResult`: added `Default` derive (`bool` defaults false, `String` defaults "", `Vec` defaults empty)
- `DiagnosticSummary`: added `Default` derive (all `usize` fields default to 0)

**Naming fix:**
- `DiagnosticSummary.info` renamed to `DiagnosticSummary.infos` for plural consistency with `errors` and `warnings`

**Refactor — `build_summary` → `DiagnosticSummary::from_diagnostics`:**
- Free function `build_summary` replaced by associated function `DiagnosticSummary::from_diagnostics()`
- Rewritten using functional `filter().count()` combinators instead of imperative loop
- All call sites updated: `validate.rs` (3 sites) and `server.rs` (3 sites)
- Old `build_summary` function removed (no callers remain)

**Issues closed:** cycle-diagnostic-missing-debug-derive, diagnostic-derive-hash,
validation-result-diagnostic-summary-derive-default, diagnostic-summary-info-naming,
build-summary-as-diagnostic-summary-method, build-summary-imperative-loop-vs-functional

## Task 2: Spec Validation Logic and Tests (assay-core)

**New validation rule — duplicate `depends` entries:**
- Added `validate_depends()` function emitting `Severity::Warning` for duplicates
- Integrated into `validate_spec()` via `diagnostics.extend(validate_depends(depends))`

**New validation rule — empty/whitespace `depends` entries:**
- `validate_depends()` emits `Severity::Error` for empty/whitespace-only entries
- Causes `valid: false` for affected specs

**Doc comments for `depends` slug-keyed behavior:**
- `assay-types/src/lib.rs` `Spec.depends`: clarified as slug-keyed with example
- `assay-types/src/gates_spec.rs` `GatesSpec.depends`: same clarification

**FeatureSpec skip documented:**
- Added doc comment to `validate_spec()` explaining `FeatureSpec` is intentionally not handled
  (it uses a separate path via `validate_feature_spec()`, never goes through `SpecEntry`)

**DFS invariant violation:**
- Replaced silent `continue` with an emitted `Severity::Warning` diagnostic describing the violation
- Diagnostic includes the invariant-violating node name for debuggability

**`finalize_as_timed_out` duplication extracted:**
- Extracted `count_results()` helper that tallies pass/fail/skip counts using `.fold()`
- Both `build_finalized_record()` and `finalize_as_timed_out()` now call `count_results()`
- Eliminated ~25 lines of duplicated imperative counting code from `finalize_as_timed_out`

**Tests added (12 new tests in validate.rs, 27 total):**
- `test_validate_agent_prompts_criteria_index_gt_0`: verifies location is `criteria[1].prompt` not `[0]`
- `test_validate_depends_clean`: no diagnostics for valid list
- `test_validate_depends_duplicate_entry`: duplicate produces `Warning` at correct index
- `test_validate_depends_empty_entry`: empty entry produces `Error`
- `test_validate_depends_whitespace_only_entry`: whitespace-only produces `Error`
- `test_validate_spec_depends_duplicate_warns`: duplicate depends doesn't block validity
- `test_validate_spec_depends_empty_is_error`: empty depends blocks validity
- `test_diagnostic_summary_info_count`: `infos` field is counted correctly
- `test_detect_cycles_diamond_dag_no_false_positive`: A→B, A→C, B→D, C→D has no false cycle
- `test_detect_cycles_empty_deps_no_diagnostics`: specs with no deps produce no diagnostics
- `test_detect_cycles_multiple_unknown_deps`: two unknown deps produce two warnings
- `test_detect_cycles_unknown_dep_at_nonzero_index`: unknown dep at index 1 reports `depends[1]`

**Issues closed:** depends-allows-duplicate-entries, empty-whitespace-depends-not-validated,
depends-field-slug-keyed-undocumented, finalize-as-timed-out-duplication,
validate-spec-feature-spec-ignored-undocumented, dfs-invariant-violation-silent-continue,
detect-cycles-diamond-dag-untested, detect-cycles-empty-and-multi-unknown-dep-untested,
detect-cycles-unknown-dep-index-only-tested-at-0, agent-report-criteria-index-gt-0-untested,
build-summary-info-count-untested

## Verification

`just ready` passes: fmt-check, clippy (-D warnings), all tests (539 assay-core, 159 assay-types),
cargo-deny.
