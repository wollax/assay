---
id: T01
parent: S03
milestone: M009
provides:
  - `commands/run/` directory module replacing monolithic 791-line `run.rs`
  - `run/mod.rs` (116 lines) — public API surface (RunArgs, execute, AnyProvider)
  - `run/phases.rs` — full container lifecycle (run_with_cancellation, Phases 1-9)
  - `run/dry_run.rs` — dry-run validation and execution plan printing
  - `run/helpers.rs` — gitignore guard and PR creation guard + their unit tests
key_files:
  - crates/smelt-cli/src/commands/run/mod.rs
  - crates/smelt-cli/src/commands/run/phases.rs
  - crates/smelt-cli/src/commands/run/dry_run.rs
  - crates/smelt-cli/src/commands/run/helpers.rs
key_decisions:
  - "D128: File-to-directory module conversion with re-exports preserves API compatibility"
  - "D129: Tests distributed to the module containing the code they test"
  - "should_create_pr not re-exported from mod.rs — only used within run/ module (phases.rs imports directly from helpers)"
patterns_established:
  - "Flat file → directory module conversion pattern: move to mod.rs, extract child modules, re-export pub items"
  - "Tests co-located with implementation in each child module"
observability_surfaces:
  - none — pure refactoring, no runtime changes
duration: 15min
verification_result: passed
completed_at: 2026-03-24T17:00:00Z
blocker_discovered: false
---

# T01: Decompose run.rs into directory module with phases, dry-run, and helpers

**Converted `commands/run.rs` (791 lines) into a 4-file directory module with `mod.rs` at 116 lines — well under the 300-line threshold.**

## What Happened

Moved `run.rs` to `run/mod.rs`, then extracted three child modules:

- **`phases.rs`** — `run_with_cancellation()` and `ExecOutcome` enum (the full Phase 1-9 container lifecycle)
- **`dry_run.rs`** — `execute_dry_run()`, `print_execution_plan()`, `truncate_spec()` + truncate tests
- **`helpers.rs`** — `ensure_gitignore_assay()`, `should_create_pr()` + their 5 unit tests

`mod.rs` retains `RunArgs`, `AnyProvider` enum (with RuntimeProvider impl), and the `execute()` entry point. All public items are re-exported or directly accessible through the same import paths.

Initially re-exported `should_create_pr` from `mod.rs` but the compiler flagged it as unused — it's only consumed within the `run` module by `phases.rs`, which imports directly from `super::helpers`. Removed the unnecessary re-export.

## Verification

| Check | Result |
|-------|--------|
| `cargo build --workspace` | ✓ Clean, 0 warnings |
| `cargo test --workspace` | ✓ 286 passed, 0 failed, 9 ignored |
| `wc -l run/mod.rs` | ✓ 116 lines (threshold: < 300) |
| `cargo doc --workspace --no-deps` | ✓ 0 warnings |
| gitignore tests (4) | ✓ All pass in `helpers::tests` |
| should_create_pr test | ✓ Passes in `helpers::tests` |
| truncate_spec tests (3) | ✓ All pass in `dry_run::tests` |

Slice-level checks (partial — T01 only covers run.rs):
- `run/mod.rs` < 300 lines: ✓ (116)
- `ssh/mod.rs` < 400 lines: pending T02
- `tests/mod.rs` < 500 lines: pending T03
- `cargo test --workspace` ≥ 286: ✓ (286)
- `cargo doc` 0 warnings: ✓

## Diagnostics

None — pure refactoring with no runtime behavior changes.

## Deviations

- Removed `pub(crate) use helpers::should_create_pr` re-export from mod.rs — plan assumed it was used outside the `run` module, but `phases.rs` imports directly from `super::helpers`. No external consumer exists.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run/mod.rs` — Public API surface: RunArgs, AnyProvider, execute() (116 lines)
- `crates/smelt-cli/src/commands/run/phases.rs` — Full container lifecycle: run_with_cancellation, ExecOutcome
- `crates/smelt-cli/src/commands/run/dry_run.rs` — Dry-run mode: execute_dry_run, print_execution_plan, truncate_spec + tests
- `crates/smelt-cli/src/commands/run/helpers.rs` — Helpers: ensure_gitignore_assay, should_create_pr + tests
