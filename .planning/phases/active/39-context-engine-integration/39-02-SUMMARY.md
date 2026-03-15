---
phase: 39-context-engine-integration
plan: 02
subsystem: context
tags: [cupel, budgeting, integration, passthrough, pipeline]
dependencies: [39-01]
tech-stack: [rust, cupel]
key-files:
  - crates/assay-core/src/context/budgeting.rs
  - crates/assay-core/src/context/mod.rs
  - crates/assay-core/src/error.rs
  - crates/assay-core/Cargo.toml
decisions:
  - "Pipeline method is .run() not .execute() (corrected from research notes)"
  - "Pipeline returns Vec<ContextItem> not Vec<ScoredItem> (corrected from research notes)"
  - "ContextKind/ContextSource use ::new() with string constants, not typed enum constructors"
  - "ContextBudget variant added to AssayError for cupel error mapping"
  - "tokens module visibility kept as pub(crate) -- budgeting accesses via super::tokens"
metrics:
  tasks_completed: 2
  tasks_total: 2
  tests_added: 7
  files_created: 1
  files_modified: 3
  duration_minutes: ~5
---

# Phase 39 Plan 02: Context Budgeting Integration Summary

## What was done

### Task 1: Wire cupel dependency
Added `cupel.workspace = true` to `crates/assay-core/Cargo.toml`. Cargo.lock updated with cupel 1.0.0.

**Commit:** `3d12086` — `feat(39-02): wire cupel dependency into assay-core`

### Task 2: Implement budget_context
Created `crates/assay-core/src/context/budgeting.rs` with the `budget_context()` public function that:

1. **Passthrough path** — When total estimated tokens fit within the target budget (model_window - output_reserve - 5% safety), returns content directly without constructing the cupel pipeline. This optimizes the common case of small diffs.

2. **Pipeline path** — When content exceeds budget, builds cupel ContextItems with:
   - System prompt: pinned (always included)
   - Criteria: pinned (always included)
   - Spec body: priority 80, Document/Rag
   - Diff: priority 50, custom "Diff" kind, Tool source

   Uses PriorityScorer + GreedySlice + ChronologicalPlacer with Truncate overflow and deduplication disabled.

3. **Error handling** — Added `ContextBudget` variant to `AssayError` for mapping `cupel::CupelError`.

4. **Module wiring** — `pub mod budgeting` and `pub use budgeting::budget_context` in context/mod.rs.

**Tests (7):**
- `passthrough_when_content_fits` — all 4 items returned in order
- `passthrough_skips_empty_diff` — 3 items when diff is empty
- `passthrough_skips_empty_spec_body` — 3 items when spec body is empty
- `truncates_large_diff` — 1MB diff results in smaller output
- `pinned_items_always_included` — system prompt and criteria survive truncation
- `empty_everything_returns_empty` — empty vec for all-empty inputs
- `budget_calculation_correctness` — verifies arithmetic: 200k window yields target of 186,109

**Commit:** `044e722` — `feat(39-02): implement budget_context with passthrough and pipeline paths`

## Deviations from plan

1. **API corrections** — Research notes had `pipeline.execute()` and `Vec<ScoredItem>` return type. Actual cupel API uses `pipeline.run()` returning `Vec<ContextItem>`. Verified by reading source before coding.

2. **Pre-existing fmt issues** — `just fmt-check` reports diffs in tokens.rs, server.rs (pre-existing, not from this plan). Not fixed to avoid scope creep. `just ready` passes ("All checks passed").

## Verification

- `cargo test -p assay-core -- context::budgeting` — 7/7 pass
- `cargo clippy -p assay-core -- -D warnings` — clean
- `just ready` — all checks passed
