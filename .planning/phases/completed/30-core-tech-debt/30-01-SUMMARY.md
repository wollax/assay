# Phase 30 Plan 01: Validation & Evaluation Deduplication Summary

Extracted shared helpers to eliminate ~160 lines of near-identical code across spec validation and gate evaluation paths.

## Tasks

| # | Task | Commit | Status |
|---|------|--------|--------|
| 1 | Extract shared validation helper (CORE-02 + CORE-09) | `977b1ec` | Done |
| 2 | Extract shared evaluation helper (CORE-03) | `f0b963f` | Done |

## Changes

### validate_criteria() — spec/mod.rs
- Extracted the per-criterion validation loop into `validate_criteria(criteria, gate, errors)`.
- `validate()` and `validate_gates_spec()` both delegate to this helper after their own name/empty-criteria checks.
- Net change: -53 lines (79 added, 132 removed).

### evaluate_criteria() — gate/mod.rs
- Extracted the evaluation loop into `evaluate_criteria(spec_name, criteria, working_dir, cli_timeout, config_timeout)`.
- `evaluate_all()` and `evaluate_all_gates()` are now thin wrappers that map criteria to `(Criterion, Enforcement)` pairs and delegate.
- Net change: -53 lines (53 added, 106 removed).

### CORE-09 (scan warning emission)
- Verified as already implemented. `scan()` collects parse errors in `ScanResult.errors`; all CLI callers (`gate.rs`, `spec.rs`, `init.rs`) iterate `.errors` and display warnings. No changes needed.

## Deviations

- Pre-existing build failures in `guard/` module and `assay-mcp` crate (from uncommitted 30-02/30-03 work) prevented running `just ready`. Verified via `cargo test/clippy/fmt` on the four relevant crates instead — all 495 tests pass, no clippy warnings, formatting clean.

## Metrics

- **Tests:** 495 passed, 3 ignored
- **Lines saved:** ~106 net lines removed
- **Duration:** ~5 minutes
