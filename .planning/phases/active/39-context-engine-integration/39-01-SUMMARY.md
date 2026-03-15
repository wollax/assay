---
phase: 39-context-engine-integration
plan: 01
subsystem: workspace
tags: [cupel, dependency-management, cleanup]
dependency-graph:
  requires: []
  provides: [cupel-workspace-dependency]
  affects: [39-02]
tech-stack:
  added: [cupel]
  patterns: [external-path-dependency]
key-files:
  created: []
  modified: [Cargo.toml, Cargo.lock]
  deleted: [crates/assay-cupel/]
decisions:
  - id: 39-01-D1
    decision: "cupel added as workspace path dependency pointing to ../cupel/crates/cupel"
    rationale: "External cupel repo is the stable v1.0.0 context engine; in-repo prototype was stale"
metrics:
  duration: ~2min
  completed: 2026-03-15
---

# Phase 39 Plan 01: Remove Stale assay-cupel and Add External cupel Dependency Summary

**One-liner:** Deleted stale in-repo assay-cupel prototype (68 files, 4185 lines) and added external cupel v1.0.0 as workspace path dependency.

## What Was Done

### Task 1: Remove stale assay-cupel and add external cupel dependency

- Deleted entire `crates/assay-cupel/` directory (68 files, 4185 lines removed)
- Replaced `assay-cupel = { path = "crates/assay-cupel" }` with `cupel = { path = "../cupel/crates/cupel" }` in root `Cargo.toml`
- Verified no other workspace crates depended on `assay-cupel`
- `cargo check` and `just build` both pass cleanly

## Decisions Made

| ID | Decision | Rationale |
|----|----------|-----------|
| 39-01-D1 | cupel added as workspace path dep to `../cupel/crates/cupel` | External repo is stable v1.0.0; in-repo copy was stale prototype |

## Deviations

None — plan executed exactly as written.

## Commits

| Hash | Message |
|------|---------|
| bc8f283 | feat(39-01): replace stale assay-cupel with external cupel dependency |

## Next Phase Readiness

Plan 39-02 can proceed immediately. The `cupel` workspace dependency is available for any crate to declare `cupel.workspace = true` in its own `Cargo.toml`.
