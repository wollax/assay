---
phase: 46
status: passed
verified_at: 2026-03-16T18:37:41Z
---

# Phase 46 Verification: Worktree Fixes

## Must-Haves

### 1. Path Canonicalization (WFIX-01)
- [x] resolve_worktree_dir() canonicalizes return value
- [x] Non-existent paths canonicalize parent + append leaf
- [x] Existing unit tests pass unchanged
- [x] Integration tests prove symlink/relative resolution

`resolve_worktree_dir()` in `crates/assay-core/src/worktree.rs` (lines 234–243) canonicalizes via `std::fs::canonicalize`. When the resolved path exists, it canonicalizes the full path. When it doesn't exist, it canonicalizes the parent and appends the leaf. Two integration tests cover this:

- `test_resolve_worktree_dir_canonicalizes_dotdot_segments` — verifies `..` segments are collapsed
- `test_resolve_worktree_dir_canonicalizes_symlinks` — verifies symlinks are resolved to the real path

All 561 passing tests in assay-core confirm the existing unit tests remain green.

### 2. Fallible Default Branch Detection (WFIX-02)
- [x] detect_default_branch() returns Result<String>
- [x] Error message names both remediation options
- [x] create() propagates error via ?
- [x] Explicit base_branch bypasses detection

`detect_default_branch()` at line 61 returns `Result<String>`. The error value (lines 69–75) includes both remediation options: `` `git remote set-head origin --auto` `` and `` `init.defaultBranch` in git config ``. `create()` at line 295 propagates with `?`. Integration tests `test_create_without_base_branch_no_remote_returns_error` and `test_create_with_explicit_base_branch_skips_detection` cover both cases.

### 3. Prune Warning Surfacing (WFIX-03)
- [x] list() returns WorktreeListResult
- [x] WorktreeListResult defined in assay-core (not assay-types)
- [x] Prune failures captured as warnings
- [x] CLI destructures and ignores warnings
- [x] MCP includes warnings in response with skip_serializing_if

`WorktreeListResult` is defined in `crates/assay-core/src/worktree.rs` (lines 18–24), not in `assay-types`. `list()` at lines 328–331 captures `git worktree prune` failures into `warnings` instead of propagating. CLI (`crates/assay-cli/src/commands/worktree.rs` line 174) destructures `result.entries` and discards `warnings`. MCP server (`crates/assay-mcp/src/server.rs` lines 737–745) defines `WorktreeListResponse` with `#[serde(skip_serializing_if = "Vec::is_empty")]` on `warnings` and assigns `result.warnings` at line 2036.

## Test Evidence

```
cargo test -p assay-core   → 561 passed, 3 ignored
cargo test -p assay-mcp    → 115 passed
```

All tests pass. Tests explicitly covering phase-46 changes:

- `integration_tests::test_resolve_worktree_dir_canonicalizes_dotdot_segments`
- `integration_tests::test_resolve_worktree_dir_canonicalizes_symlinks`
- `integration_tests::test_create_without_base_branch_no_remote_returns_error`
- `integration_tests::test_create_with_explicit_base_branch_skips_detection`

## Gaps

None
