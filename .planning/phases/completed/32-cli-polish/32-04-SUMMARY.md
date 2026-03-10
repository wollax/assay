# Plan 32-04 Summary: Enforcement Dedup

**Status:** Complete
**Duration:** ~2.5 minutes
**Files modified:** `crates/assay-cli/src/commands/gate.rs`

## Tasks Completed (3/3)

1. **StreamConfig::new() constructor** - Added constructor to deduplicate identical struct-literal construction in `handle_gate_run` and `handle_gate_run_all`. Both sites now call `StreamConfig::new(cli_timeout, config_timeout, verbose, color)`.

2. **gate_exit_code() helper** - Extracted shared function that delegates to `StreamCounters::gate_blocked()` (from Plan 02) to determine exit code. Both handlers now call `gate_exit_code(&counters)` instead of inline `if counters.gate_blocked() { 1 } else { 0 }`.

3. **History table color branch collapse** - Replaced nested if/else for pass/fail status string with a single color-code lookup followed by one color/plain branch. The `status_width` computation was already collapsed in a prior plan.

## Commits

- `7b97ad6`: refactor(32-04): deduplicate StreamConfig construction via new() constructor
- `470cfae`: refactor(32-04): extract gate_exit_code() helper to deduplicate enforcement check
- `b86adca`: refactor(32-04): collapse history table color branch into single format path

## Verification

`just ready` passed (fmt-check, lint, test, deny).

## Notes

- `StreamCounters` already had `#[derive(Default)]` and both sites already used `StreamCounters::default()` from Plan 02, so no changes needed there.
- The `gate_blocked()` and `tally()` methods from Plan 02 were already in use at both call sites.
