# Phase 46 Context: Worktree Fixes

## Canonical Path Resolution (WFIX-01)

**Decision:** Canonicalize paths in `resolve_worktree_dir()` — one fix point, all downstream consumers (CLI, MCP, future code) get canonical paths automatically.

**Key finding:** The CLI `--all` cleanup already uses canonical paths from `git worktree list --porcelain` output (core/worktree.rs:438). The actual mismatch bug is in `resolve_worktree_dir()` (core/worktree.rs:189-213) where raw strings from config/env/default are returned without canonicalization.

**Non-existent paths:** Canonicalize the existing parent directory, then append the non-existent leaf segment. Example: `canonicalize("../")` then join `"foo-worktrees"`. No side effects, no directory creation.

**Scope:** Only modify `resolve_worktree_dir()`. Do not change how `list()` or `cleanup()` handle paths — they already use git's canonical output.

## Warning Surfacing Mechanism (WFIX-03)

**Decision:** `list()` returns a named wrapper struct instead of `Vec<WorktreeInfo>`:
```rust
pub struct WorktreeListResult {
    pub entries: Vec<WorktreeInfo>,
    pub warnings: Vec<String>,
}
```

**Prune failure handling:** All prune failures are warnings (not errors, not parsed for severity). Prune is best-effort cleanup before list — if it fails, list still works. Capture the error message from the failed `git_command` Result and push to warnings.

**Current code:** `let _ = git_command(&["worktree", "prune"], project_root);` at core/worktree.rs:296 — completely swallows Result.

**Propagation:** Warnings flow through the wrapper struct to MCP response `warnings` field and CLI stderr. No need to add warnings to individual `WorktreeInfo` entries.

## Default Branch Detection Error (WFIX-02)

**Decision:** `detect_default_branch()` returns `Result<String>` instead of infallible `String`. On failure, `worktree_create` errors with an actionable message — no silent fallback to `"main"`.

**Error message content:** Include both the git command and the config key:
> "Could not detect default branch. Run `git remote set-head origin --auto` or set `init.defaultBranch` in git config, or pass base_branch explicitly."

**Single error variant:** Do not differentiate "no remote" from "remote HEAD not configured" — the fix is the same regardless.

**Bypass:** When `base_branch` is explicitly provided to `worktree_create`, detection is skipped entirely (this already works — core/worktree.rs:262-264). The error only fires when auto-detection is needed.

## Backward Compatibility

**Decision:** Not breaking — only internal consumers (assay-cli, assay-mcp) within the workspace. Pre-1.0, no external semver contract. Update all callsites in the same commits.

**WFIX-02 is the most disruptive** to user experience: repos that previously fell back to `main` will now error. This is correct behavior — the previous silent fallback was a bug that could create worktrees based on the wrong branch.

## Commit Strategy

One commit per WFIX — three separate, self-contained commits. Each is independently testable and bisectable.

## Deferred Ideas

None identified during discussion.
