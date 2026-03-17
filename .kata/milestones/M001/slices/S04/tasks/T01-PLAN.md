---
estimated_steps: 5
estimated_files: 3
---

# T01: Create ResultCollector with unit tests

**Slice:** S04 — Result Collection & Branch Output
**Milestone:** M001

## Description

Create the `ResultCollector` struct in a new `collector.rs` module. It operates on the host-side git repo (no Docker dependency) and is generic over `<G: GitOps>` for testability. After Assay completes, the collector reads the git state (HEAD position, commits since base_ref), creates or force-updates the target branch, and returns a structured result. Unit tests exercise all paths using real temp git repos.

## Steps

1. Create `crates/smelt-core/src/collector.rs` with:
   - `BranchCollectResult` struct: `branch: String`, `commit_count: usize`, `files_changed: Vec<String>`, `subjects: Vec<String>`, `no_changes: bool`
   - `ResultCollector<G: GitOps>` struct holding a `G` instance and repo path
   - `ResultCollector::new(git: G, repo_path: PathBuf)` constructor
   - `ResultCollector::collect(&self, base_ref: &str, target_branch: &str) -> Result<BranchCollectResult>` that:
     a. Calls `rev_parse("HEAD")` to get current HEAD
     b. Calls `rev_parse(base_ref)` to get base commit
     c. If HEAD == base, returns `BranchCollectResult { no_changes: true, .. }`
     d. Calls `worktree_is_dirty(&repo_path)` — if dirty, log warning via `tracing::warn`
     e. Calls `rev_list_count("HEAD", base_ref)` for commit count
     f. Calls `diff_name_only(base_ref, "HEAD")` for changed files
     g. Calls `log_subjects(&format!("{base_ref}..HEAD"))` for commit subjects
     h. If `branch_exists(target_branch)`, log warning with old position via `rev_parse`, then delete and recreate
     i. Calls `branch_create(target_branch, "HEAD")` to create target branch
     j. Returns populated `BranchCollectResult`

2. Register `pub mod collector` in `lib.rs` and add re-exports for `ResultCollector` and `BranchCollectResult`

3. Add unit tests in `collector.rs` using the `setup_test_repo()` pattern (copied/adapted from `git/cli.rs` tests) — create temp git repos with known commit histories:
   - `test_collect_basic` — base repo with 1 extra commit after base_ref → target branch created, commit_count=1, files list matches
   - `test_collect_no_changes` — HEAD == base_ref → `no_changes: true`, no branch created
   - `test_collect_target_already_exists` — target branch pre-exists at different commit → force-updated to new HEAD
   - `test_collect_multiple_commits` — 3 commits after base → commit_count=3, subjects collected
   - `test_collect_dirty_worktree` — uncommitted changes present → still collects, result is valid (warn only)

4. Verify all tests pass: `cargo test -p smelt-core -- collector::tests`

5. Run full workspace test to confirm no regressions: `cargo test --workspace`

## Must-Haves

- [ ] `BranchCollectResult` struct with branch, commit_count, files_changed, subjects, no_changes fields
- [ ] `ResultCollector` generic over `<G: GitOps>` — no Docker dependency
- [ ] `collect()` creates target branch pointing at HEAD when new commits exist
- [ ] `collect()` returns `no_changes: true` when HEAD == base_ref (no branch created)
- [ ] `collect()` handles pre-existing target branch by force-updating
- [ ] `tracing::warn` for dirty worktree and target branch overwrite
- [ ] 5 unit tests covering happy path, no changes, target exists, multiple commits, dirty worktree

## Verification

- `cargo test -p smelt-core -- collector::tests` — 5/5 pass
- `cargo test --workspace` — zero regressions

## Observability Impact

- Signals added/changed: `tracing::warn` for "no new commits", "dirty working tree detected", "target branch already exists at {old_hash}, updating to {new_hash}"; `tracing::info` for "HEAD at {hash}", "base_ref at {hash}", "{N} commits to collect", "target branch '{name}' created at {hash}"
- How a future agent inspects this: `SMELT_LOG=info cargo test -p smelt-core -- collector` shows collector decision points; `git log --oneline <target>` verifies branch content
- Failure state exposed: `SmeltError::GitExecution` for git command failures; `SmeltError::Provider` with operation "collect" if rev_parse or branch_create fails

## Inputs

- `crates/smelt-core/src/git/mod.rs` — `GitOps` trait (RPITIT, not object-safe — must use generics)
- `crates/smelt-core/src/git/cli.rs` — `GitCli` impl and `setup_test_repo()` test helper pattern
- `crates/smelt-core/src/error.rs` — `SmeltError` variants for error returns
- S04-RESEARCH: Constraints on RPITIT, bind-mount semantics, single-session simplification

## Expected Output

- `crates/smelt-core/src/collector.rs` — new file with `BranchCollectResult`, `ResultCollector<G: GitOps>`, `collect()`, and 5 unit tests
- `crates/smelt-core/src/lib.rs` — updated with `pub mod collector` and re-exports
