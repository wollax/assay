# Missing test: empty system prompt ordering

**Area:** crates/assay-core/src/context/budgeting.rs (tests)
**Severity:** Low
**Source:** PR #125 review (test-reviewer)

## Description

No test covers `budget_context("", "spec", "criteria", "diff", 200_000)` — the case where system prompt is empty. The passthrough path should return `[spec, criteria, diff]`.
