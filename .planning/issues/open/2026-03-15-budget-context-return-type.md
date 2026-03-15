# budget_context should return a typed BudgetedContext struct

**Area:** crates/assay-core/src/context/budgeting.rs
**Severity:** Low
**Source:** PR #125 review (type-reviewer)

## Description

`Vec<String>` return type encodes meaning via position. A typed struct would make the contract explicit and allow callers to detect truncation.

## Suggested Fix

```rust
pub struct BudgetedContext {
    pub system_prompt: Option<String>,
    pub spec_body: Option<String>,
    pub criteria_text: Option<String>,
    pub diff: Option<String>,
    pub was_truncated: bool,
}
```

Evaluate when gate_evaluate (Phase 44) is built — the consumer will clarify what structure is needed.
