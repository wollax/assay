---
estimated_steps: 6
estimated_files: 6
---

# T01: Decompose run.rs into directory module with phases, dry-run, and helpers

**Slice:** S03 — Large file decomposition
**Milestone:** M009

## Description

Convert `commands/run.rs` (791 lines) from a flat file into a `commands/run/` directory module. The main module keeps the public API surface (`RunArgs`, `execute()`, `AnyProvider` enum) while phases, dry-run logic, and helper functions move to focused child modules. All existing imports from other crates (`crate::commands::run::*`) must continue working via re-exports.

## Steps

1. Create `crates/smelt-cli/src/commands/run/` directory. Move `run.rs` to `run/mod.rs`.
2. Extract `run_with_cancellation()` and the `ExecOutcome` enum to `run/phases.rs`. This is the bulk of the file (~350 lines covering Phases 1-9). Add `pub(crate)` visibility and re-export from `mod.rs`.
3. Extract `execute_dry_run()`, `print_execution_plan()`, and `truncate_spec()` to `run/dry_run.rs`. Add appropriate visibility and re-export from `mod.rs`.
4. Extract `ensure_gitignore_assay()` and `should_create_pr()` to `run/helpers.rs`. Re-export `should_create_pr` (it's `pub(crate)`) from `mod.rs`.
5. Move the `#[cfg(test)] mod tests` block — distribute tests to whichever child module they test (gitignore tests → helpers, should_create_pr tests → helpers, truncate_spec tests → dry_run).
6. Verify: `cargo test --workspace`, `cargo doc --workspace --no-deps`, and `wc -l run/mod.rs` < 300.

## Must-Haves

- [ ] `run/mod.rs` exists and is < 300 lines
- [ ] `run/phases.rs` exists with `run_with_cancellation` and `ExecOutcome`
- [ ] `run/dry_run.rs` exists with `execute_dry_run`, `print_execution_plan`, `truncate_spec`
- [ ] `run/helpers.rs` exists with `ensure_gitignore_assay`, `should_create_pr`
- [ ] All existing unit tests pass (gitignore tests, should_create_pr tests, truncate_spec tests)
- [ ] `cargo build --workspace` compiles with no new warnings
- [ ] `cargo doc --workspace --no-deps` exits 0 with zero warnings

## Verification

- `cargo test -p smelt-cli` — all tests pass, 0 failures
- `cargo test --workspace` — 286+ pass, 0 failures
- `wc -l crates/smelt-cli/src/commands/run/mod.rs` — under 300
- `cargo doc --workspace --no-deps 2>&1 | grep -c warning` — 0

## Observability Impact

- Signals added/changed: None — pure refactoring
- How a future agent inspects this: `cargo test`, `cargo build`, `wc -l`
- Failure state exposed: Compiler errors on broken imports/visibility

## Inputs

- `crates/smelt-cli/src/commands/run.rs` — the 791-line file to decompose
- `crates/smelt-cli/src/commands/mod.rs` — declares `pub mod run`
- S01 summary — `deny(missing_docs)` is enforced; all new `pub` items need doc comments

## Expected Output

- `crates/smelt-cli/src/commands/run/mod.rs` — public API surface (< 300 lines)
- `crates/smelt-cli/src/commands/run/phases.rs` — execution phases
- `crates/smelt-cli/src/commands/run/dry_run.rs` — dry-run logic and formatting
- `crates/smelt-cli/src/commands/run/helpers.rs` — gitignore and PR guard helpers + their tests
