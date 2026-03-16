# Phase 47: Merge Check — Context

## Phase Scope

**Goal:** Standalone conflict detection via `git merge-tree --write-tree` with zero side effects
**Requirement:** MERGE-01 — `merge_check` MCP tool
**Boundary:** This phase builds the git-level conflict detection tool only. Spec-aware merge operations are Phase 50.

## Decisions

### 1. Conflict Detail Granularity

- **Rich conflict entries:** Each conflict includes file path, conflict type enum (ModifyModify, AddAdd, RenameDelete, etc.), and content (conflict markers / diff hunks)
- **Rename detection enabled:** Use git's rename detection so rename/rename and rename/delete conflicts are properly categorized
- **Clean merge file listing:** When merge is clean, include list of merged files (not just `clean: true`)
- **Configurable cap:** Accept optional `max_conflicts` parameter (default 20). Return first N conflicts with `"and X more"` truncation indicator when exceeded

### 2. Ref Resolution & Validation

- **Accept any valid git ref:** Branches, tags, raw SHAs, relative refs (HEAD~3), etc.
- **Upfront validation:** Resolve both `base` and `head` before running merge-tree. If either fails, report both errors (don't stop at the first)
- **Always resolve to SHA:** Response always includes `base_sha` and `head_sha` as resolved commit SHAs, regardless of input format
- **Both refs required:** No defaults — caller must specify both `base` and `head` explicitly

### 3. MergeCheck Response Shape

Required fields beyond the basics:
- `fast_forward: bool` — whether head is a direct descendant of base (no merge commit needed)
- `merge_base_sha: String` — common ancestor SHA for divergence context
- `ahead: u32, behind: u32` — commit counts on each side since divergence
- For clean merges: file list includes path + change type (Added, Modified, Deleted)
- For conflicts: file list includes path + conflict type + content (markers/hunks)

### 4. Worktree Integration

- **Pure git operation:** No spec_id parameter, no Assay-specific awareness. merge_check operates on git refs only
- **Rationale:** Phase 50 (merge_propose) handles spec-to-branch resolution. The agent already knows the branch from worktree creation. Adding spec lookup muddies the abstraction
- **Repo context:** Uses project root git context (consistent with other Assay tools). Worktree vs main repo is irrelevant since they share the object store
- **No branch validation:** Accept any branch/ref — no Assay-specific checks on whether a branch belongs to a managed worktree

## Deferred Ideas

None surfaced during discussion.

## Open Questions for Research

- Exact `git merge-tree --write-tree` output format and parsing strategy (stdout structure, conflict markers format)
- Git version requirement validation (2.38+ needed) — error handling for older git
- Whether `git merge-tree` exposes rename detection flags or if it needs separate configuration
- Performance characteristics with large repos / many conflicts
