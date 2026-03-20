---
id: T01
parent: S05
milestone: M005
provides:
  - --json flag on `assay milestone status` outputting CycleStatus JSON or {"active":false}
key_files:
  - crates/assay-cli/src/commands/milestone.rs
key_decisions:
  - D072 error pattern: cycle_status Err → eprintln + return Ok(1), not panic/propagation
patterns_established:
  - JSON output branch in CLI commands: check flag first, call domain fn, serialize, return early
observability_surfaces:
  - "`assay milestone status --json | jq .` — shows active CycleStatus or {\"active\":false}; exits 0 always; non-zero only on I/O error to stderr"
duration: ~10 minutes
verification_result: passed
completed_at: 2026-03-20
blocker_discovered: false
---

# T01: Add `--json` flag to `assay milestone status`

**Added `--json` flag to `assay milestone status` outputting `{"active":false}` or full `CycleStatus` JSON — enabling machine-readable cycle state inspection for hooks and scripts.**

## What Happened

Extended `MilestoneCommand::Status` with a `json: bool` field via `#[arg(long)] json: bool`. Updated `milestone_status_cmd` to accept the flag and branch: when `json` is true, calls `assay_core::milestone::cycle_status(&dir)`, serializes with `serde_json::to_string`, and prints to stdout. `Ok(None)` → `{"active":false}`; `Err(e)` → eprintln + `Ok(1)` (D072). When false, existing table-print behavior is unchanged.

Added `milestone_status_json_no_active` test: creates tempdir with `.assay/` (no milestones), calls `handle(MilestoneCommand::Status { json: true })`, asserts `Ok(0)`.

Also updated the existing `milestone_status_no_milestones` test to pass `Status { json: false }` to match the new variant shape.

## Verification

- `cargo test -p assay-cli -- milestone` — 4 tests pass (3 existing + 1 new)
- `cargo test --workspace` — all tests pass (1331+ total)
- `just ready` — fmt + clippy + test + deny all green, "All checks passed."

Note: `just ready` exhibited a single flaky failure on first run (`test_pr_check_one_fails` in `assay-core`) due to a pre-existing `set_current_dir` test pollution issue across parallel test binaries. Isolated run and full `--workspace` run both pass cleanly. This is not caused by T01 changes.

## Diagnostics

```bash
# Inspect current cycle state (exits 0 always):
assay milestone status --json | jq .
# → {"active":false}  when no in_progress milestone
# → {"milestone_slug":"...","active_chunk_slug":"...","completed_count":N,...}  when active
```

## Deviations

None.

## Known Issues

Pre-existing flaky test `test_pr_check_one_fails` in `assay-core` fails intermittently when run as part of `just ready` (full workspace test suite) due to `set_current_dir` test pollution across parallel test binaries. Not introduced by T01; exists on the base branch.

## Files Created/Modified

- `crates/assay-cli/src/commands/milestone.rs` — Added `json: bool` to `Status` variant; updated `handle` match arm; updated `milestone_status_cmd(json: bool)` with JSON branch; added `milestone_status_json_no_active` test; updated `milestone_status_no_milestones` test for new variant shape
