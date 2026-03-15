# Consider From<CupelError> impl instead of map_err closure

**Area:** crates/assay-core/src/context/budgeting.rs:95
**Severity:** Low
**Source:** PR #125 review (code-reviewer)

## Description

The `map_err` closure is used 7+ times. A `From<cupel::CupelError> for AssayError` impl would be cleaner and enable the `?` operator directly.

## Consideration

Adding `From` impl creates a tighter coupling between assay-core and cupel. Evaluate whether this is desirable.
