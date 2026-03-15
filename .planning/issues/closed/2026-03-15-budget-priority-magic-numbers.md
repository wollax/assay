# Priority values 80/50 in budget_context are undocumented

**Area:** crates/assay-core/src/context/budgeting.rs:110,120
**Severity:** Low
**Source:** PR #125 review (type-reviewer, code-reviewer)

## Description

`.priority(80)` for spec body and `.priority(50)` for diff are inline literals. Named constants would document the intent and the relative scale.
