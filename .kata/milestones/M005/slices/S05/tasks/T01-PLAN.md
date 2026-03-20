---
estimated_steps: 4
estimated_files: 1
---

# T01: Add `--json` flag to `assay milestone status`

**Slice:** S05 — Claude Code Plugin Upgrade
**Milestone:** M005

## Description

Add a `--json` flag to the `assay milestone status` CLI subcommand. When passed, the command calls `assay_core::milestone::cycle_status` and serializes the result to stdout as JSON — outputting `{"active":false}` when no milestone is in_progress (matching the MCP `cycle_status` sentinel exactly), or the full `CycleStatus` JSON when a milestone is active. This flag is required by `cycle-stop-check.sh` (T03) to detect the active chunk in bash without fragile TOML parsing.

No new domain logic is needed — `cycle_status()` from S02 is already the authoritative function.

## Steps

1. Add `#[arg(long)] json: bool` field to `MilestoneCommand::Status` variant in `crates/assay-cli/src/commands/milestone.rs`. Update the `handle` match arm to pass `json` to `milestone_status_cmd`.
2. Update `milestone_status_cmd` to accept a `json: bool` parameter. When `json` is true: call `assay_core::milestone::cycle_status(&dir)`, serialize with `serde_json::to_string`, print to stdout, and return `Ok(0)`. On `Ok(None)`, output `{"active":false}`. On `Err(e)`, eprintln and return `Ok(1)` (D072 pattern). When `json` is false, existing table-print behavior is unchanged.
3. Add unit test `milestone_status_json_no_active` in the `#[cfg(test)]` block: create a tempdir, create `.assay/` (no milestones dir), `set_current_dir`, call `handle(MilestoneCommand::Status { json: true })`, assert `result.is_ok()` and exit code 0. (Output capture is not needed — the test proves it compiles and exits cleanly; the `{"active":false}` output behavior is verified by the existing `cycle_status` integration tests in S02.)
4. Run `just ready` to confirm fmt + clippy + test + deny all pass.

## Must-Haves

- [ ] `MilestoneCommand::Status` has `json: bool` field
- [ ] `milestone_status_cmd(json: bool)` — when true: outputs `{"active":false}` or CycleStatus JSON to stdout; exits 0
- [ ] `milestone_status_cmd` — when false: existing table-print behavior unchanged
- [ ] Test `milestone_status_json_no_active` passes
- [ ] `just ready` exits 0

## Verification

- `cargo test -p assay-cli -- milestone_status_json` — 1 test passes
- `cargo test -p assay-cli -- milestone` — all 3 existing + 1 new = 4 tests pass
- `just ready` — "All checks passed."

## Observability Impact

- Signals added/changed: `assay milestone status --json` stdout is now a stable machine-readable inspection surface for hooks and scripts
- How a future agent inspects this: `assay milestone status --json | jq .` — shows current cycle state or `{"active":false}`
- Failure state exposed: non-zero exit + eprintln when `cycle_status` returns Err (I/O failures on milestone directory)

## Inputs

- `crates/assay-cli/src/commands/milestone.rs` — existing `MilestoneCommand`, `handle()`, `milestone_status_cmd()` to extend
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status()` and `CycleStatus` (already re-exported from `assay_core::milestone`)
- S02-SUMMARY.md — D072 pattern: domain errors exit 1 via eprintln + `return Ok(1)`, not panic/propagation

## Expected Output

- `crates/assay-cli/src/commands/milestone.rs` — `Status { json: bool }` variant; `milestone_status_cmd(json: bool)` with JSON branch; `milestone_status_json_no_active` test
