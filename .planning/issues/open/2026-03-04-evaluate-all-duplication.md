---
area: refactoring
severity: low
source: phase-12 PR review
---

# Structural duplication between evaluate_all and evaluate_all_gates

Both functions in crates/assay-core/src/gate/mod.rs are ~80 lines of near-identical code. Extract a shared helper that accepts `&[Criterion]` and `spec_name: &str`.
