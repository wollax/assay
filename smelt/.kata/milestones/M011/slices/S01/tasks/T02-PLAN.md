---
estimated_steps: 5
estimated_files: 7
---

# T02: Decompose git/cli.rs into directory module

**Slice:** S01 — Decompose manifest.rs and git/cli.rs
**Milestone:** M011

## Description

Convert `git/cli.rs` (1365L) from a flat file into a directory module: `git/cli/mod.rs` retains the full `GitCli` struct + `GitOps` trait impl (~330L), and `git/cli/tests/` distributes the 29 async tests across 5 domain-specific submodules. The `git/mod.rs` declaration `mod cli;` resolves either `cli.rs` or `cli/mod.rs` transparently — no parent changes needed.

## Steps

1. Create `crates/smelt-core/src/git/cli/` directory. Move `git/cli.rs` to `git/cli/mod.rs` (via `git mv`).
2. Verify `cargo build --workspace` succeeds immediately — `git/mod.rs` uses `mod cli;` which resolves the directory module automatically.
3. Create `git/cli/tests/mod.rs` with shared `pub(super) fn setup_test_repo()` fixture and `mod` declarations for 5 submodules: `basic`, `worktree`, `branch`, `commit`, `merge`.
4. Distribute 29 tests into submodules: (a) `tests/basic.rs` — test_repo_root, test_current_branch, test_head_short, test_is_inside_work_tree, test_rev_parse (~5 tests). (b) `tests/worktree.rs` — test_worktree_add_and_list, test_worktree_remove, test_worktree_is_dirty, test_worktree_add_existing (~4 tests). (c) `tests/branch.rs` — test_branch_exists, test_branch_delete, test_branch_is_merged, test_branch_create, test_merge_base (~5 tests). (d) `tests/commit.rs` — test_add_and_commit, test_commit_returns_valid_hash, test_rev_list_count, test_add_specific_paths, test_add_and_commit_in_worktree, test_diff_numstat, test_diff_name_only, test_diff_name_only_empty, test_log_subjects, test_log_subjects_empty_range (~10 tests). (e) `tests/merge.rs` — test_merge_squash_clean, test_merge_squash_conflict, test_reset_hard, test_unmerged_files_empty_when_clean, test_fetch_ref_creates_local_branch (~5 tests). Each submodule uses `use super::setup_test_repo;` and `use crate::git::cli::GitCli;` etc.
5. Verify `cargo test -p smelt-core --lib git::cli` runs all 29 tests, then `cargo test --workspace` for full 290+ suite.

## Must-Haves

- [ ] `git/cli.rs` flat file no longer exists; replaced by `git/cli/mod.rs`
- [ ] `git/cli/tests/mod.rs` contains shared `setup_test_repo()` accessible to all submodules
- [ ] 29 git/cli tests distributed across 5 test submodules by operation family
- [ ] All files in `git/cli/` and `git/cli/tests/` are under 500 lines
- [ ] `git/mod.rs` unchanged (or minimal change) — `mod cli;` still works
- [ ] All existing import paths preserved (`smelt_core::git::GitCli`, `crate::git::cli::GitCli`)
- [ ] `cargo test --workspace` passes with 290+ tests, 0 failures

## Verification

- `test ! -f crates/smelt-core/src/git/cli.rs` — flat file gone
- `ls crates/smelt-core/src/git/cli/mod.rs crates/smelt-core/src/git/cli/tests/mod.rs` — exist
- `wc -l crates/smelt-core/src/git/cli/*.rs crates/smelt-core/src/git/cli/tests/*.rs` — all under 500L
- `cargo test --workspace` — 290+ pass, 0 failures
- `cargo build --workspace` — clean

## Observability Impact

- Signals added/changed: None — pure structural refactor
- How a future agent inspects this: `cargo test -p smelt-core --lib git::cli` lists all cli tests in their new submodule paths
- Failure state exposed: Compiler errors immediately surface missing imports or broken paths

## Inputs

- `crates/smelt-core/src/git/cli.rs` — the 1365L file being decomposed
- T01 completed — manifest decomposition done, workspace in clean state
- D128, D129 patterns from M009

## Expected Output

- `crates/smelt-core/src/git/cli/mod.rs` — `GitCli` struct + full `GitOps` impl (~330L)
- `crates/smelt-core/src/git/cli/tests/mod.rs` — shared `setup_test_repo()` + submodule declarations (~70L)
- `crates/smelt-core/src/git/cli/tests/basic.rs` — basic operation tests (~120L)
- `crates/smelt-core/src/git/cli/tests/worktree.rs` — worktree tests (~180L)
- `crates/smelt-core/src/git/cli/tests/branch.rs` — branch tests (~200L)
- `crates/smelt-core/src/git/cli/tests/commit.rs` — commit/diff tests (~350L)
- `crates/smelt-core/src/git/cli/tests/merge.rs` — merge/conflict/fetch tests (~250L)
