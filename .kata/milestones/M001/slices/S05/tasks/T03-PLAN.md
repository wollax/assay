---
estimated_steps: 8
estimated_files: 5
---

# T03: Worktree tech debt resolution

**Slice:** S05 — Worktree Enhancements & Tech Debt
**Milestone:** M001

## Description

Resolve all 15 tracked worktree tech debt items. Most are mechanical fixes (rename, attribute additions, test gaps). Items that require schema-breaking changes or violate D005 (MCP additive-only) are addressed with helpers or deferred with documented rationale.

## Steps

1. **eprintln! → tracing::warn!** (item #3): Replace `eprintln!("Warning: failed to delete branch...")` in `cleanup()` with `tracing::warn!(branch = %branch_name, "failed to delete branch: {e}")`.
2. **to_string_lossy documentation** (item #2): Add doc comments at each `to_string_lossy` call site in `create()` and `cleanup()` documenting the UTF-8 assumption: "Git CLI requires UTF-8 string args; non-UTF-8 paths are not supported." Keep lossy conversion (changing to OsStr-aware passing would require git2 or custom process building — not worth it).
3. **Rename detect_main_worktree → detect_linked_worktree** (item #4): Rename function and update all callers across crates. Update doc comment to clarify: "Returns the main repo root if `cwd` is inside a linked worktree."
4. **WorktreeConfig.base_dir helper** (item #5): Add `pub fn as_path(&self) -> &Path` method to `WorktreeConfig` in assay-types. Do NOT change the field type (schema-breaking). Add a comment explaining why it stays as String.
5. **Schema registry + snapshots for WorktreeInfo and WorktreeStatus** (items #6, #7, #8): Add `inventory::submit!` entries for both types. Add snapshot test cases in `schema_snapshots.rs`. Run `cargo insta test --accept` to generate snapshots.
6. **Missing tests** (items #10, #11, #12): (a) Add test for `read_metadata` with corrupt JSON — write invalid JSON to worktree.json, call `read_metadata()`, assert returns `None`. (b) Add test for `write_metadata` git exclude behavior — create a worktree-like dir structure, call `write_metadata()`, assert `.git/info/exclude` contains `.assay/worktree.json`. (c) Add test for `list()` prune failure — this is harder to trigger in a unit test; add a test that verifies warnings are returned in the `WorktreeListResult` when prune fails (mock by setting up invalid worktree state).
7. **ASSAY_WORKTREE_DIR documentation** (item #13): Add doc comment to `resolve_worktree_dir()` listing the env var in the precedence chain. Add a `# Environment Variables` section comment.
8. **MCP warning surfacing and remaining items** (items #9, #14, #15): Surface `WorktreeListResult.warnings` in MCP `worktree_list` response by appending them to the JSON output. For MCP `cleanup --all` (item #14), add a `// TODO(M002): worktree_cleanup_all tool — deferred per D005` comment. For field duplication (item #9), add awareness comment. For prune failure surfacing in MCP (item #15), include warnings in the serialized response.

## Must-Haves

- [ ] No `eprintln!` in `crates/assay-core/src/worktree.rs`
- [ ] `detect_main_worktree` renamed to `detect_linked_worktree` everywhere
- [ ] `WorktreeConfig::as_path()` helper exists
- [ ] `WorktreeInfo` and `WorktreeStatus` have `inventory::submit!` + schema snapshots
- [ ] Test for `read_metadata` with corrupt JSON
- [ ] Test for `write_metadata` git exclude behavior
- [ ] Test for `list()` prune warning propagation
- [ ] `ASSAY_WORKTREE_DIR` documented in `resolve_worktree_dir` doc comment
- [ ] MCP `worktree_list` surfaces warnings
- [ ] `just ready` passes

## Verification

- `just ready` — all checks pass
- `rg "eprintln" crates/assay-core/src/worktree.rs` — zero matches
- `rg "detect_main_worktree" crates/` — zero matches
- `cargo insta test -p assay-types` — no pending snapshots (new snapshots for WorktreeInfo, WorktreeStatus)
- `cargo test -p assay-core -- worktree` — all tests pass including 3 new tests
- `cargo test -p assay-mcp` — MCP tests pass

## Observability Impact

- Signals added/changed: Branch deletion failure in `cleanup()` now uses `tracing::warn!` instead of `eprintln!` — captured by structured logging
- How a future agent inspects this: `WorktreeListResult.warnings` now surfaced in MCP `worktree_list` JSON response — agents see prune warnings
- Failure state exposed: corrupt metadata returns `None` (already did, now tested); prune warnings propagated to MCP callers

## Inputs

- `crates/assay-core/src/worktree.rs` — full module with T01/T02 changes applied
- `crates/assay-types/src/worktree.rs` — type definitions with T01 `session_id` change
- `crates/assay-mcp/src/server.rs` — MCP handlers with T01 `create()` call site update
- S05 research: 15 tech debt items enumerated with specific line numbers and recommendations

## Expected Output

- `crates/assay-core/src/worktree.rs` — 15 tech debt fixes applied, 3 new tests, function renamed
- `crates/assay-types/src/worktree.rs` — `WorktreeConfig::as_path()` helper, registry entries for `WorktreeInfo`/`WorktreeStatus`
- `crates/assay-types/tests/schema_snapshots.rs` — 2 new snapshot test cases
- `crates/assay-types/tests/snapshots/` — 2 new snapshot files for `WorktreeInfo` and `WorktreeStatus`
- `crates/assay-mcp/src/server.rs` — warnings surfaced in `worktree_list` response
