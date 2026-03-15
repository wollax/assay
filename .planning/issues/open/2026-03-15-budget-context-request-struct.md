# budget_context should take a BudgetRequest struct

**Area:** crates/assay-core/src/context/budgeting.rs
**Severity:** Low
**Source:** PR #125 review (type-reviewer)

## Description

`budget_context` takes 5 positional parameters where 4 are `&str`. A `BudgetRequest` struct would prevent transposition bugs and be easier to extend.

## Suggested Fix

```rust
pub struct BudgetRequest<'a> {
    pub system_prompt: &'a str,
    pub spec_body: &'a str,
    pub criteria_text: &'a str,
    pub diff: &'a str,
    pub model_window: u64,
}
```

Consider when `gate_evaluate` (Phase 44) becomes the caller — if it's the only caller and construction is obvious, the struct may be premature.
