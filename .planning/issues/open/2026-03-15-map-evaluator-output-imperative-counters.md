# `map_evaluator_output` uses imperative counters instead of functional fold

**Area:** crates/assay-core/src/evaluator.rs:221-280
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

`map_evaluator_output` accumulates five mutable counters (`passed`, `failed`, `warned`, `skipped`, `required_failed`) in an imperative loop. CLAUDE.md prefers functional and declarative patterns. These counters can be derived from the results via a single `fold` or by using iterator combinators, eliminating the mutable state.

## Suggested Fix

Use `Iterator::fold` over the criterion results to accumulate a counter struct, or derive each count with filtered `count()` calls if clarity is preferred over a single pass:

```rust
let passed = results.iter().filter(|r| r.outcome == Pass).count();
// etc.
```

## Category

style
