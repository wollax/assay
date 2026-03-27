---
id: T02
parent: S01
milestone: M011
provides:
  - git/cli/ directory module replacing flat git/cli.rs
  - git/cli/tests/ with 5 domain-specific test submodules (basic, worktree, branch, commit, merge)
  - shared setup_test_repo() fixture in tests/mod.rs
key_files:
  - crates/smelt-core/src/git/cli/mod.rs
  - crates/smelt-core/src/git/cli/tests/mod.rs
  - crates/smelt-core/src/git/cli/tests/basic.rs
  - crates/smelt-core/src/git/cli/tests/worktree.rs
  - crates/smelt-core/src/git/cli/tests/branch.rs
  - crates/smelt-core/src/git/cli/tests/commit.rs
  - crates/smelt-core/src/git/cli/tests/merge.rs
key_decisions: []
patterns_established:
  - "Same file-to-directory module pattern from T01 (D128) applied to git/cli"
  - "Test submodule grouping by operation family: basic, worktree, branch, commit, merge (D129)"
observability_surfaces:
  - "`cargo test -p smelt-core --lib git::cli` lists all 29 tests in domain submodule paths"
duration: 10min
verification_result: passed
completed_at: 2026-03-24T00:00:00Z
blocker_discovered: false
---

# T02: Decompose git/cli.rs into directory module

**Converted 1365-line git/cli.rs into 7-file directory module with 5 domain-specific test submodules, all under 500 lines**

## What Happened

Moved `git/cli.rs` to `git/cli/mod.rs` via `git mv`, which Rust's module system resolves transparently — `git/mod.rs` needed zero changes since `mod cli;` resolves both `cli.rs` and `cli/mod.rs`. The `#[cfg(test)] mod tests` block was replaced with `mod tests;` pointing to a new `tests/` directory.

Created `tests/mod.rs` with the shared `setup_test_repo()` fixture (pub(super) visibility) and 5 submodule declarations. Distributed all 29 async tests across: `basic.rs` (5 tests: repo_root, current_branch, head_short, is_inside_work_tree, rev_parse), `worktree.rs` (4 tests), `branch.rs` (5 tests), `commit.rs` (10 tests: add/commit, rev_list_count, diff_numstat, diff_name_only, log_subjects), `merge.rs` (5 tests: merge_squash_clean/conflict, reset_hard, unmerged_files, fetch_ref).

## Verification

- `test ! -f crates/smelt-core/src/git/cli.rs` — flat file gone ✓
- `ls crates/smelt-core/src/git/cli/mod.rs crates/smelt-core/src/git/cli/tests/mod.rs` — exist ✓
- All files under 500 lines: mod.rs=332, basic=53, worktree=119, branch=200, commit=310, merge=319, tests/mod=49 ✓
- `git/mod.rs` unchanged (zero diff) ✓
- `cargo test -p smelt-core --lib git::cli` — 29 passed, 0 failed ✓
- `cargo test --workspace` — 290 passed, 0 failed ✓
- `cargo clippy --workspace` — clean ✓
- `cargo doc --workspace --no-deps` — clean ✓
- `cargo build --workspace` — clean ✓

### Slice-level checks
- All files in manifest/ and git/cli/ under 500L ✓
- `cargo test --workspace` — 290+ pass ✓
- `cargo clippy --workspace` — clean ✓
- `cargo doc --workspace --no-deps` — clean ✓

## Diagnostics

`cargo test -p smelt-core --lib git::cli` lists all 29 cli tests in their new submodule paths for future inspection.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/git/cli/mod.rs` — GitCli struct + full GitOps impl (~332L, moved from cli.rs)
- `crates/smelt-core/src/git/cli/tests/mod.rs` — shared setup_test_repo() + 5 submodule declarations
- `crates/smelt-core/src/git/cli/tests/basic.rs` — basic operation tests (repo_root, current_branch, head_short, is_inside_work_tree, rev_parse)
- `crates/smelt-core/src/git/cli/tests/worktree.rs` — worktree add/remove/dirty/add_existing tests
- `crates/smelt-core/src/git/cli/tests/branch.rs` — branch exists/delete/is_merged/merge_base/create tests
- `crates/smelt-core/src/git/cli/tests/commit.rs` — add/commit, rev_list_count, diff_numstat, diff_name_only, log_subjects tests
- `crates/smelt-core/src/git/cli/tests/merge.rs` — merge_squash clean/conflict, reset_hard, unmerged_files, fetch_ref tests
