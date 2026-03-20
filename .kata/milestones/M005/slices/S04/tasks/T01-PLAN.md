---
estimated_steps: 6
estimated_files: 5
---

# T01: Extend Milestone type, update literals, write failing integration tests

**Slice:** S04 — Gate-Gated PR Workflow
**Milestone:** M005

## Description

Add `pr_number: Option<u64>` and `pr_url: Option<String>` fields to the `Milestone` type in `assay-types`, update every `Milestone { ... }` struct literal in the workspace (compile safety), regenerate the milestone schema snapshot, and write the full `crates/assay-core/tests/pr.rs` integration test suite in test-first order. After T01 the test file will fail to compile because `assay_core::pr` does not yet exist — that is the expected red state. All other workspace tests must continue to pass.

## Steps

1. Open `crates/assay-types/src/milestone.rs`. Add two fields after `pr_base` and before `created_at`:
   ```rust
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub pr_number: Option<u64>,

   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub pr_url: Option<String>,
   ```
   Update the two struct literals in the `#[cfg(test)]` block at the bottom of that file (`milestone_toml_roundtrip` and `milestone_minimal_toml_roundtrip`) to add `pr_number: None, pr_url: None`.

2. Run `INSTA_UPDATE=always cargo test -p assay-types` to regenerate the milestone schema snapshot (`crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap`). Verify the test passes.

3. Update `crates/assay-core/tests/milestone_io.rs`: add `pr_number: None, pr_url: None` to the `make_milestone` helper. Run `cargo test -p assay-core --features assay-types/orchestrate --test milestone_io` to confirm all 5 tests still pass.

4. Update `crates/assay-core/tests/cycle.rs`: add `pr_number: None, pr_url: None` to `make_milestone_with_status` and any inline `Milestone { ... }` literal. Run `cargo test -p assay-core --features assay-types/orchestrate --test cycle` to confirm all 10 tests still pass.

5. Create `crates/assay-core/tests/pr.rs` with 8 integration tests. The file must:
   - Import `assay_core::pr::{pr_check_milestone_gates, pr_create_if_gates_pass}` (will fail to compile until T02)
   - Import `use serial_test::serial;` for the PATH-mutating tests
   - Reuse the `make_assay_dir`, `create_passing_spec`, `create_failing_spec`, and `make_milestone_with_status`-equivalent helpers (define locally or import from cycle module — if not public, define them inline in pr.rs)
   - Define a `write_fake_gh(dir: &Path, exit_code: i32, stdout: &str)` helper that writes a shell script to `dir/gh` and makes it executable (using `std::os::unix::fs::PermissionsExt`)
   - Define a `with_mock_gh_path<F: FnOnce(&Path)>(dir: &Path, f: F)` helper that prepends `dir` to `PATH`, runs `f`, then restores original `PATH`

   Test names and scenarios:
   - `test_pr_check_all_pass`: milestone with 2 passing specs → `pr_check_milestone_gates` returns `Ok(vec![])`
   - `test_pr_check_one_fails`: milestone with 1 passing + 1 failing spec → `Ok(vec![ChunkGateFailure { chunk_slug: "fail-chunk", required_failed: N }])` where N > 0
   - `test_pr_check_missing_spec`: milestone chunk references a non-existent spec → `Err(AssayError::Io)`
   - `test_pr_create_already_created`: milestone with `pr_number: Some(42)` → `Err` with message containing "PR already created"
   - `test_pr_create_gates_fail`: gates fail → `Err` without invoking `gh`; milestone TOML not modified
   - `test_pr_create_gh_not_found` (`#[serial]`): `PATH` set to empty/temp dir → `Err` with message containing "gh CLI not found"
   - `test_pr_create_success_mock_gh` (`#[serial]`): all gates pass + mock `gh` exits 0 with `{"number":42,"url":"https://github.com/owner/repo/pull/42"}` → `Ok(PrCreateResult { pr_number: 42, pr_url: "..." })`; milestone TOML reloaded to confirm `pr_number = 42` written
   - `test_pr_create_verify_transitions_to_complete` (`#[serial]`): milestone with `Verify` status + all gates pass + mock `gh` → milestone status becomes `Complete` after PR creation

6. Run `cargo test --workspace` and confirm: all pre-existing tests pass; `tests/pr.rs` produces a compile error about missing `assay_core::pr` module. This compile error is expected and is the "red" state.

## Must-Haves

- [ ] `Milestone` has `pr_number: Option<u64>` and `pr_url: Option<String>` with `serde(default, skip_serializing_if = "Option::is_none")`; both fields absent from TOML when `None`
- [ ] All `Milestone { ... }` struct literals in `assay-types` and `assay-core` tests compile without error (all have `pr_number: None, pr_url: None`)
- [ ] Milestone schema snapshot regenerated (`schema_snapshots__milestone-schema.snap` updated with new fields)
- [ ] `crates/assay-core/tests/pr.rs` exists with 8 named tests covering: all-pass, one-fails, missing-spec, already-created, gates-fail-no-gh, gh-not-found, mock-gh-success, verify-to-complete
- [ ] `cargo test -p assay-types` passes; `cargo test -p assay-core --features assay-types/orchestrate --test milestone_io` passes; `cargo test -p assay-core --features assay-types/orchestrate --test cycle` passes
- [ ] `cargo test --workspace` produces compile error only for `tests/pr.rs` (expected red state); all other test suites are green

## Verification

- `INSTA_UPDATE=always cargo test -p assay-types` — passes; snapshot updated
- `cargo test -p assay-core --features assay-types/orchestrate --test milestone_io` — 5 passed
- `cargo test -p assay-core --features assay-types/orchestrate --test cycle` — 10 passed
- `cargo test --workspace` — all suites pass except `tests/pr.rs` fails to compile (expected; `assay_core::pr` doesn't exist yet)
- `grep pr_number crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` — field appears in snapshot

## Observability Impact

- Signals added/changed: `Milestone.pr_number` and `Milestone.pr_url` fields added to the persisted type; downstream reads of milestone TOML will see these fields when set
- How a future agent inspects this: `cat .assay/milestones/<slug>.toml` — shows `pr_number`/`pr_url` after successful PR creation; `milestone_get` MCP tool returns them in JSON
- Failure state exposed: None yet (implementation in T02)

## Inputs

- `crates/assay-types/src/milestone.rs` — existing `Milestone` type; `pr_base`/`pr_branch` fields are the insertion reference point
- `crates/assay-core/tests/milestone_io.rs` — `make_milestone` helper needs `pr_number: None, pr_url: None`
- `crates/assay-core/tests/cycle.rs` — `make_milestone_with_status` and inline literals need same additions
- S01-SUMMARY.md / S02-SUMMARY.md — `Milestone` struct literal cascade pattern established; `INSTA_UPDATE=always` for snapshot acceptance

## Expected Output

- `crates/assay-types/src/milestone.rs` — two new fields added; test literals updated
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` — regenerated with `pr_number`/`pr_url`
- `crates/assay-core/tests/milestone_io.rs` — `make_milestone` helper updated
- `crates/assay-core/tests/cycle.rs` — `make_milestone_with_status` and literals updated
- `crates/assay-core/tests/pr.rs` — new: 8 integration tests (red until T02)
