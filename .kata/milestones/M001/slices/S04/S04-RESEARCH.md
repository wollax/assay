# S04: Result Collection & Branch Output — Research

**Date:** 2026-03-17

## Summary

S04 adds the result collection step to the `smelt run` pipeline: after Assay completes inside the container, Smelt extracts the git state from the bind-mounted repo and creates/updates the target branch specified in `merge.target`. Because the host repo is bind-mounted at `/workspace` (D013, D027), all Assay commits are already on disk in the host filesystem — no `docker cp` or tar extraction is needed. The core work is orchestrating git operations on the host repo to create the target branch from the container's working state.

The existing `git/` module (`GitCli` + `GitOps` trait) provides all necessary primitives: `branch_exists`, `branch_create`, `rev_parse`, `diff_name_only`, `log_subjects`. The `CollectResult` type in `provider.rs` already exists as a stub. The `DockerProvider::collect()` method returns a no-op today — this slice implements it meaningfully, though the real collection logic belongs in a new `collector.rs` module that operates on the host-side git repo (not inside the container).

The primary risk is correctly handling the git state after Assay has made commits inside the container. Since Assay writes directly to the bind-mounted repo, the host git index and refs are mutated in-place. The collector needs to verify the work, tag/branch it to `merge.target`, and report what was collected — all before teardown.

## Recommendation

Create a `ResultCollector` struct in `crates/smelt-core/src/collector.rs` that:

1. Takes `&dyn GitOps` + `&JobManifest` as inputs (no Docker dependency)
2. Verifies Assay left commits on the expected branch (the container's HEAD vs the original `base_ref`)
3. Creates or force-updates the target branch (`merge.target`) to point at the container repo's HEAD
4. Returns a `CollectResult` with commit count, file list, and branch name

The collector should operate on the **host repo path** (the resolved repo from `manifest.job.repo`), not through Docker exec. This is correct because the bind-mount means the host filesystem already has all the commits. This keeps the collector testable without Docker.

Insert the collection step in `execute_run()` between "Assay complete" and teardown.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Branch ops, rev-parse, diff | `git/mod.rs` + `git/cli.rs` (`GitOps` trait) | 25+ methods already implemented and tested — `branch_exists`, `branch_create`, `rev_parse`, `diff_name_only`, `log_subjects`, `rev_list_count` |
| Error types | `error.rs` `SmeltError` | `GitExecution`, `Manifest`, `Provider` variants cover all needed cases |
| Repo path resolution | `manifest::resolve_repo_path()` | Already canonicalizes and validates local paths |

## Existing Code and Patterns

- `crates/smelt-core/src/git/cli.rs` — `GitCli` implements `GitOps` with `run()` and `run_in()` for repo-root and worktree-scoped git commands. All methods are async, return `Result<T>`. Tests use `setup_test_repo()` to create temp git repos with initial commits. **Reuse directly** for collector.
- `crates/smelt-core/src/git/mod.rs` — `GitOps` trait defines the seam. `preflight()` discovers git binary + repo root. Collector should accept `&dyn GitOps` for testability. Note: trait uses RPITIT (D019), not `async_trait`.
- `crates/smelt-core/src/provider.rs` — `CollectResult` exists with `exit_code`, `stdout`, `stderr`, `artifacts` fields. This is oriented around command output. For S04 the collector should return richer info (branch name, commit count, files changed). Consider either extending `CollectResult` or defining a separate `BranchCollectResult` for the git layer.
- `crates/smelt-cli/src/commands/run.rs` — `execute_run()` is the orchestration hub. Collection inserts between the assay exec result check and teardown. Follow the existing async-block pattern (D026) — collection runs inside the block, teardown runs unconditionally after.
- `crates/smelt-core/src/docker.rs` — `DockerProvider::collect()` is a stub returning empty `CollectResult`. The trait method takes `&ContainerId` + `&JobManifest`. Since bind-mount means host-side collection, the Docker collect impl may just delegate to the host-side collector or remain minimal.
- `crates/smelt-core/src/manifest.rs` — `MergeConfig.target` is the target branch name. `MergeConfig.strategy` is "sequential"/"octopus" but S04 only needs to handle the simple case of pointing target at the result. Multi-session merge ordering is an S06 concern.

## Constraints

- **Bind-mount means host-side mutation** — Assay's git commits are already in the host repo's `.git` directory. The collector doesn't need to extract files from the container; it just needs to read the git state. This is per D013 and confirmed by S03's bind-mount tests.
- **GitOps trait uses RPITIT** (D019) — Cannot use `&dyn GitOps` directly since RPITIT methods aren't object-safe. Collector must either be generic (`impl GitOps`) or use `GitCli` directly. The existing codebase uses concrete `GitCli` in `lib.rs` re-exports. **Go generic with `<G: GitOps>`** for testability.
- **Target branch may or may not exist** — First run creates it; subsequent runs update it. Use `branch_exists` + `branch_create` or force-update.
- **Single-session simplification** — S04 only needs to handle the case where Assay's output is already on the repo. Multi-session merge (where sessions produce separate branches that need merging) is an S06 concern. For S04, assume Assay leaves its work on the repo's current branch/HEAD, and the collector points `merge.target` at that state.
- **No remote push** — The roadmap says "result branch on the host repository." Push to remote is not in scope.

## Common Pitfalls

- **Detached HEAD after container exec** — If Assay checks out a different branch or detaches HEAD inside the container, the collector needs to handle that. Use `rev_parse("HEAD")` on the host repo (at the resolved repo path) to find where the work landed, rather than assuming a specific branch name.
- **Dirty working tree** — If Assay left unstaged/uncommitted changes, the collector should detect and report this. Use `worktree_is_dirty()` on the repo path. Decide policy: error, warn, or auto-commit.
- **Target branch already exists at a different point** — If `merge.target` already exists (from a previous run), force-updating it loses the old state. Log a warning with the old and new commit hashes.
- **No new commits from Assay** — If Assay exited 0 but made no commits (HEAD == base_ref), the collector should report "no changes" rather than creating an empty target branch.

## Open Risks

- **Assay's git behavior is unspecified** — We don't know if Assay commits to the current branch, creates new branches, or leaves changes uncommitted. The collector needs to be defensive. Mitigation: check HEAD position relative to `base_ref` and report what was found.
- **Concurrent repo access** — If the user has the repo open in an editor or another git operation is running, the bind-mount writes from the container could conflict. This is inherent to D013 (bind-mount) and not solvable in S04 — document it as a known limitation.
- **RPITIT object safety** — The `GitOps` trait can't be used as `dyn GitOps`. This means the collector must be generic. If this becomes unwieldy, a future decision could add a boxed wrapper, but for now generics work fine.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | — | No specialized skill needed; standard Rust patterns |
| Docker/bollard | — | Already well-established in S02; no skill needed |
| Git operations | — | Covered by existing `git/` module; no external skill |

No relevant professional agent skills found in `<available_skills>` or needed for this slice. The work is pure Rust + git operations using existing crate infrastructure.

## Sources

- Existing codebase analysis (primary source — all code read directly)
- D013: Bind-mount host repo into container (from DECISIONS.md)
- D015: Keep git/cli.rs and git/mod.rs from v0.1.0 (from DECISIONS.md)
- D027: Fixed /workspace mount point (from DECISIONS.md)
- S03-SUMMARY: Forward intelligence on repo mount path and Assay completion signal
