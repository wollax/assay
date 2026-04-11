---
phase: 65-resolution-core
plan: "02"
subsystem: composition-engine
tags: [resolution, gate-inheritance, criteria-library, cycle-detection, own-wins]
dependency_graph:
  requires: [ResolvedGate, ResolvedCriterion, CriterionSource, validate_slug, load_library, AssayError]
  provides: [resolve]
  affects: [assay-core]
tech_stack:
  added: []
  patterns: [tdd, closure-injection, reverse-dedup, single-level-inheritance]
key_files:
  created: []
  modified:
    - crates/assay-core/src/spec/compose.rs
decisions:
  - "Reverse-dedup algorithm (collect all, iterate in reverse, keep first-seen-by-name) chosen over IndexMap dependency for own-wins with preserved ordering"
  - "resolve() takes two closures (load_gate, load_library) instead of traits — consistent with zero-trait convention"
  - "Mutual cycle detection inspects only parent.extends == gate_slug (single-level decision prevents deeper chain detection)"
  - "Type annotation on closure parameter avoided — resolve() signature inferred from context once function exists"
metrics:
  duration_minutes: 4
  completed_date: "2026-04-11"
  tasks_completed: 2
  files_changed: 1
---

# Phase 65 Plan 02: Gate Composition resolve() Summary

**One-liner:** resolve() merges parent, library, and own criteria into ResolvedGate using closure injection, slug validation, mutual cycle detection, and reverse-dedup own-wins semantics.

## What Was Built

### resolve() Function (assay-core)

`crates/assay-core/src/spec/compose.rs` — `pub fn resolve()` added:

```rust
pub fn resolve(
    gate: &GatesSpec,
    gate_slug: &str,
    load_gate: impl Fn(&str) -> Result<GatesSpec>,
    load_library: impl Fn(&str) -> Result<CriteriaLibrary>,
) -> Result<ResolvedGate>
```

**Algorithm:**

1. **Slug validation** — validates `extends` slug and each `include` slug before any loading. Returns `InvalidSlug` on first failure.

2. **Cycle detection + parent loading** — if `extends` is set:
   - Self-extend check: `extends_slug == gate_slug` → `CycleDetected`
   - Load parent via `load_gate` closure
   - Mutual-extend check: `parent.extends == Some(gate_slug)` → `CycleDetected`
   - Extract parent's own criteria only (parent's `extends`/`include` ignored — single-level decision)

3. **Library criteria** — loads each `include` slug via `load_library` closure, appends criteria with `CriterionSource::Library { slug }`.

4. **Reverse-dedup merge** — collects all criteria in order (parent → libraries → own), then:
   - Iterates in reverse, tracking seen names in a `HashSet`
   - Keeps first-seen (which is last in forward order = own if present)
   - Reverses result to restore forward order
   - This gives own-wins semantics and later-library-wins within library group

**Output ordering:** Parent criteria first (non-overridden), then library criteria (non-overridden), then own criteria. Within each group, original order preserved.

## Test Coverage

| Suite | Tests | Location |
|-------|-------|----------|
| resolve() happy paths | 5 | `assay-core::spec::compose::tests` |
| resolve() own-wins merge | 3 | `assay-core::spec::compose::tests` |
| resolve() cycle detection | 2 | `assay-core::spec::compose::tests` |
| resolve() slug validation | 2 | `assay-core::spec::compose::tests` |
| resolve() error paths | 2 | `assay-core::spec::compose::tests` |
| resolve() edge cases | 4 | `assay-core::spec::compose::tests` |
| **New total** | **15** | |
| **Plan 01 tests** | **27** | (unchanged) |
| **Total in module** | **39** | |

## Commits

| Hash | Description |
|------|-------------|
| 8c058ec | feat(65-02): implement resolve() for gate composition with cycle detection |

Note: Tests and implementation were committed together in a single GREEN commit due to the pre-commit hook running clippy (which rejected the RED-only state as compilation errors). The RED state was verified by running `cargo test` manually before implementing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Pre-commit hook requires compilation-clean commits**
- **Found during:** RED commit attempt
- **Issue:** The pre-commit hook runs `cargo clippy` which fails when tests reference a nonexistent `resolve()` function. This prevented committing the RED test state.
- **Fix:** Implemented `resolve()` (GREEN) before committing. Both tests and implementation are in one commit. The RED state was verified by running `cargo test` manually first (confirming 18 "cannot find function" errors).
- **Files modified:** `crates/assay-core/src/spec/compose.rs`
- **Commit:** 8c058ec

**2. [Rule 3 - Formatting] Pre-commit hook required cargo fmt**
- **Found during:** First commit attempt of RED tests
- **Issue:** cargo fmt --check failed on test code line length (same pattern as Plan 01)
- **Fix:** Ran `cargo fmt --all`, re-staged
- **Files modified:** `crates/assay-core/src/spec/compose.rs`
- **Commit:** (resolved before final commit)

### Algorithm Choice

The plan suggested several approaches for the merge algorithm. Chose **reverse-dedup** (collect all → iterate reverse → keep first-seen-by-name → reverse result) over:
- `IndexMap` — would add a new dependency not in workspace
- `HashMap<String, usize>` + ordered Vec rebuild — more complex
- Two-pass with `HashMap` — requires extra allocation

The reverse-dedup approach uses only `std::collections::HashSet` and is O(n) with a single allocation.

## Self-Check: PASSED

- `crates/assay-core/src/spec/compose.rs` — FOUND
- Commit `8c058ec` — FOUND
- 39 tests pass in compose module
- `just ready` green (2368/2368 tests)
