# 36-01 Summary: Base-Branch-Relative Ahead/Behind

## Outcome

**Complete** — All tasks executed successfully. `just ready` passes.

## What Changed

### Task 1: Type changes and metadata persistence
**Commit:** `5c7d61a`

- Added `WorktreeMetadata` struct to `assay-types` with `base_branch` and `spec_slug` fields, registered in schema registry
- Changed `WorktreeStatus.ahead` and `.behind` from `usize` to `Option<usize>`
- Added `base_branch: Option<String>` and `warnings: Vec<String>` fields to `WorktreeStatus`
- Added `write_metadata()` and `read_metadata()` helpers to `assay-core::worktree`
- `create()` now writes `.assay/worktree.json` after `git worktree add`, and adds the file to `$GIT_COMMON_DIR/info/exclude` so it doesn't pollute `git status`
- `list()` now reads metadata to populate `base_branch` on each `WorktreeInfo`
- `status()` computes ahead/behind against `origin/<base>` (remote-tracking) with fallback to `refs/heads/<base>` (local). Missing base ref returns `None` counts with a warning string
- Updated CLI worktree status display to handle `Option<usize>` and show base branch / warnings
- Updated `schema_roundtrip.rs` test to match new field types

### Task 2: MCP fetch parameter and schema snapshots
**Commit:** `edf482c`

- Added `fetch: Option<bool>` to `WorktreeStatusParams`
- `worktree_status` handler runs `git fetch origin <base>` when `fetch=true` before calling `status()`
- Added `read_metadata_public()` as public API for MCP handler use
- Updated tool description to reflect base-branch-relative ahead/behind
- Added schema snapshot tests for `WorktreeStatus` and `WorktreeMetadata`
- All schema snapshots accepted and passing

## Deviations

1. **Git exclude for metadata file** — The plan did not address that writing `.assay/worktree.json` inside a worktree makes it dirty (untracked file). Solved by adding the file to `$GIT_COMMON_DIR/info/exclude` (the shared git exclude, which works for linked worktrees). Used `git rev-parse --git-common-dir` since per-worktree excludes (`$GIT_DIR/info/exclude`) do not apply to linked worktrees.

2. **Public metadata reader** — Added `read_metadata_public()` as a thin public wrapper around the private `read_metadata()` to support the MCP handler's fetch logic. The plan referenced `assay_core::worktree::status()` as the only public API, but the fetch handler needs to read metadata before calling status.

## Verification

- `just ready` passes (fmt-check, lint, test, deny)
- 18 worktree unit/integration tests pass including new assertions for `base_branch`, `ahead`, `behind`, and `warnings`
- 25 schema snapshot tests pass (including 2 new: worktree-status, worktree-metadata)
- 34 schema roundtrip tests pass
- 20 MCP handler tests pass

## Duration

~15 minutes
