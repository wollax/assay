---
id: T04
parent: S02
milestone: M005
provides:
  - "`MilestoneCommand::Status` variant: prints InProgress milestone progress tables with `[x]`/`[ ]` chunk markers"
  - "`MilestoneCommand::Advance { milestone: Option<String> }` variant: calls `cycle_advance` and exits 0 on success or 1 on error"
  - "`milestone_status_cmd()`: scans milestones, filters to InProgress, prints chunk completion state sorted by order"
  - "`milestone_advance_cmd(slug)`: wraps `cycle_advance` with CLI-contract error handling (eprintln + exit 1, no panic/propagation)"
  - "2 new CLI tests: `milestone_status_no_milestones` and `milestone_advance_no_active_milestone`; total CLI milestone tests: 3"
key_files:
  - crates/assay-cli/src/commands/milestone.rs
key_decisions:
  - "cycle_advance takes 4 params (no config_timeout) — task plan showed 5; corrected at compile time per T02 carry-forward context"
  - "milestone_advance_cmd returns Ok(1) rather than propagating Err — CLI contract: exit code 1 signals cycle/gate failure, not unhandled exception"
patterns_established:
  - "CLI error contract for cycle commands: Err from core → eprintln!(\"Error: {e}\") + return Ok(1); success → structured print + return Ok(0)"
observability_surfaces:
  - "`assay milestone status` stdout: MILESTONE: <slug> (<phase>) followed by [x]/<[ ]> <chunk_slug> (complete|active) per chunk"
  - "`assay milestone advance` stdout on success: Advanced: <slug> (N/M chunks complete, phase: <Phase>)"
  - "`assay milestone advance` stderr on failure: Error: <AssayError::Io message> with operation label, path, and cause"
duration: 10min
verification_result: passed
completed_at: 2026-03-19T00:00:00Z
blocker_discovered: false
---

# T04: Add `assay milestone status` and `assay milestone advance` CLI subcommands

**`Status` and `Advance` subcommands added to `MilestoneCommand`; 3 CLI milestone tests pass; `just ready` green.**

## What Happened

Extended `crates/assay-cli/src/commands/milestone.rs` with two new subcommands completing the CLI surface for the S02 cycle state machine.

`milestone_status_cmd` scans for InProgress milestones and prints a progress table. Each milestone's chunks are sorted by `order` ascending; chunks whose slug appears in `completed_chunks` render as `[x]`; the rest as `[ ]`. Empty case prints "No active milestones." and exits 0.

`milestone_advance_cmd` calls `assay_core::milestone::cycle_advance` with `assay_dir`, `specs_dir` (`root/.assay/specs`), `working_dir` (project root), and the optional milestone slug. On success it prints the structured summary (slug, completed/total counts, phase) and exits 0. On error it writes to stderr and returns exit code 1 — not propagating the error — as the CLI contract for cycle/gate failures.

One compile error appeared: the task plan showed 5 args to `cycle_advance` (including a `None` for config_timeout), but the actual function signature takes 4 (no timeout param). Corrected immediately per the T02 carry-forward that noted this discrepancy.

## Verification

```
cargo test -p assay-cli -- milestone
# 3 passed: milestone_list_subcommand_no_milestones, milestone_status_no_milestones, milestone_advance_no_active_milestone

cargo test --workspace
# All tests green

just ready
# fmt-check: cargo fmt applied to fix pre-existing formatting in assay-mcp/src/server.rs
# lint, test, deny: all passed
# "All checks passed."
```

## Diagnostics

- `assay milestone status` — human-readable cycle state; shows phase and per-chunk completion at a glance
- `assay milestone advance` — structured success output includes slug, chunk counts, and phase; stderr error text includes operation label and path from `AssayError::Io`
- Exit code 1 from `assay milestone advance` is the stable signal for gate/precondition failure (distinguishable from panic/unhandled error)

## Deviations

`cycle_advance` takes 4 parameters, not 5 as shown in the task plan (the task plan included a `None` for config_timeout that doesn't exist). Fixed at compile time.

`cargo fmt` was required to fix a pre-existing formatting issue in `assay-mcp/src/server.rs` before `just ready` could pass.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/commands/milestone.rs` — added `Status` and `Advance` variants to `MilestoneCommand`; implemented `milestone_status_cmd` and `milestone_advance_cmd`; added 2 new tests (3 total)
