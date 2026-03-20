---
id: T01
parent: S04
milestone: M005
provides:
  - "`Milestone` type extended with `pr_number: Option<u64>` and `pr_url: Option<String>` fields (serde default + skip_serializing_if)"
  - "All `Milestone { ... }` struct literals across workspace updated (milestone.rs, wizard.rs test, milestone_io.rs, cycle.rs) — compile safety maintained"
  - "Milestone schema snapshot regenerated: `schema_snapshots__milestone-schema.snap` now includes `pr_number` and `pr_url`"
  - "`crates/assay-core/tests/pr.rs` created with 8 integration tests: all-pass, one-fails, missing-spec, already-created, gates-fail, gh-not-found, mock-gh-success, verify-to-complete"
  - "Red state confirmed: `tests/pr.rs` fails to compile only on `assay_core::pr` import; all other suites green"
requires: []
affects: [T02]
key_files:
  - crates/assay-types/src/milestone.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap
  - crates/assay-core/tests/milestone_io.rs
  - crates/assay-core/tests/cycle.rs
  - crates/assay-core/tests/pr.rs
  - crates/assay-core/src/wizard.rs
  - crates/assay-core/tests/wizard.rs
key_decisions:
  - "D079 confirmed: tests/pr.rs written test-first against assay_core::pr which doesn't exist yet; compile error is the expected red state"
  - "`with_mock_gh_path` helper returns generic `R` so callers can return values through it; uses `unsafe { set_var }` with `#[serial]` guard"
  - "Found `wizard.rs` (both src and tests) also had `Milestone { ... }` literals needing `pr_number`/`pr_url` — fixed as part of cascade"
patterns_established:
  - "Milestone struct cascade: every new optional field requires searching all Milestone { ... } literals workspace-wide with `grep -rn` before running tests"
drill_down_paths:
  - .kata/milestones/M005/slices/S04/tasks/T01-PLAN.md
duration: 25min
verification_result: pass
completed_at: 2026-03-20T00:00:00Z
---

# T01: Extend Milestone type, update literals, write failing integration tests

**`Milestone` extended with `pr_number`/`pr_url` fields; schema snapshot regenerated; 8 integration tests in `tests/pr.rs` written test-first (red until T02)**

## What Happened

Added `pr_number: Option<u64>` and `pr_url: Option<String>` to `assay-types/src/milestone.rs` after `pr_base`, with the standard `#[serde(default, skip_serializing_if = "Option::is_none")]` annotations. The cascade of `Milestone { ... }` struct literals was wider than the plan indicated — `crates/assay-core/src/wizard.rs` (two construction sites) and `crates/assay-core/tests/wizard.rs` both needed `pr_number: None, pr_url: None` added in addition to the explicitly listed files.

Ran `INSTA_UPDATE=always cargo test -p assay-types --features assay-types/orchestrate` to regenerate the milestone schema snapshot; the test passed and `grep pr_number` confirmed the field appears in the snapshot.

Created `crates/assay-core/tests/pr.rs` with all 8 required tests. The helper `with_mock_gh_path` was written as a generic `fn with_mock_gh_path<R, F: FnOnce(&Path) -> R>` to allow returning values through it (needed for `test_pr_create_success_mock_gh`). `std::env::set_var` requires `unsafe {}` in the current Rust edition; all callers are `#[serial]` so the safety invariant holds. Inline `write_fake_gh` helper creates a shell script that echoes the provided JSON and exits with the given code.

## Deviations

- `crates/assay-core/src/wizard.rs` and `crates/assay-core/tests/wizard.rs` were not in the plan's listed files but also had `Milestone { ... }` literals that needed updating — fixed during compilation feedback.

## Files Created/Modified

- `crates/assay-types/src/milestone.rs` — added `pr_number`, `pr_url` fields; updated 2 test literals
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` — regenerated with new fields
- `crates/assay-core/tests/milestone_io.rs` — `make_milestone` helper updated
- `crates/assay-core/tests/cycle.rs` — `make_milestone_with_status` + 2 inline literals updated
- `crates/assay-core/tests/wizard.rs` — `make_milestone` helper updated
- `crates/assay-core/src/wizard.rs` — 2 `Milestone { ... }` construction sites updated
- `crates/assay-core/tests/pr.rs` — new: 8 integration tests (red until T02)
