---
id: T03
parent: S05
milestone: M001
provides:
  - eprintln replaced with tracing::warn in cleanup()
  - detect_linked_worktree renamed from detect_main_worktree
  - WorktreeConfig::as_path() helper method
  - inventory::submit! for WorktreeInfo and WorktreeStatus
  - Schema snapshot for WorktreeInfo
  - Tests for corrupt metadata, git exclude, and prune warning propagation
  - ASSAY_WORKTREE_DIR documented in resolve_worktree_dir
  - MCP worktree_list surfaces warnings; TODO comments for deferred items
key_files:
  - crates/assay-core/src/worktree.rs
  - crates/assay-types/src/worktree.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - to_string_lossy kept with doc comments rather than switching to OsStr-aware git2 bindings
  - WorktreeConfig.base_dir stays as String (schema-breaking to change); as_path() added as helper
  - Field duplication between WorktreeInfo and WorktreeStatus documented but not unified (D005)
  - worktree_cleanup_all deferred to M002 per D005 (MCP additive-only)
patterns_established:
  - tracing::warn with structured fields for git operation warnings
observability_surfaces:
  - cleanup() branch deletion failure now uses tracing::warn (structured logging) instead of eprintln
  - WorktreeListResult.warnings surfaced in MCP worktree_list JSON response
  - corrupt metadata returns None with tracing::warn (already existed, now tested)
duration: 15min
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T03: Worktree tech debt resolution

**Resolved all 15 tracked worktree tech debt items: replaced eprintln with structured logging, renamed detect_main_worktree, added WorktreeConfig::as_path(), schema registry entries, 3 new tests, env var docs, MCP warning surfacing, and TODO comments for deferred items.**

## What Happened

Addressed each of the 15 tech debt items systematically:

1. **eprintln → tracing::warn**: Replaced the sole `eprintln!` in `cleanup()` with `tracing::warn!(branch = %branch_name, ...)` for structured logging capture.
2. **to_string_lossy documentation**: Added doc comments at both call sites in `create()` and `cleanup()` explaining the UTF-8 assumption.
3. **Renamed detect_main_worktree → detect_linked_worktree**: Updated function name, doc comment, and all test references. No external callers found outside the module.
4. **WorktreeConfig::as_path()**: Added `pub fn as_path(&self) -> &Path` helper with a comment explaining why `base_dir` stays as `String`.
5. **Schema registry entries**: Added `inventory::submit!` for `WorktreeInfo` and `WorktreeStatus`. Added `worktree_info_schema_snapshot` test. The `worktree_status_schema_snapshot` test already existed. Generated and accepted the new `WorktreeInfo` snapshot.
6. **Missing tests**: Added `test_read_metadata_corrupt_json_returns_none`, `test_write_metadata_adds_git_exclude_entry`, and `test_list_prune_warning_propagation`.
7. **ASSAY_WORKTREE_DIR docs**: Added `# Environment Variables` section to `resolve_worktree_dir` doc comment.
8. **MCP warnings/deferred items**: `worktree_list` already surfaced warnings through `WorktreeListResponse`. Added `TODO(M002)` comment for `worktree_cleanup_all`, and field duplication awareness comment on `WorktreeListResponse`.

## Verification

- `just ready` — all checks pass (fmt, lint, test, deny)
- `rg "eprintln" crates/assay-core/src/worktree.rs` — zero matches
- `rg "detect_main_worktree" crates/` — zero matches
- `cargo insta test -p assay-types --accept` — worktree-info-schema snapshot created and accepted, no pending snapshots
- `cargo test -p assay-core -- worktree` — 35 tests pass including 3 new ones
- `cargo test -p assay-mcp` — 27 MCP tests pass

### Slice-level verification (final task):
- ✅ `just ready` passes
- ✅ `cargo test -p assay-core -- worktree` — all worktree tests pass
- ✅ `cargo insta test -p assay-types` — all snapshots accepted (WorktreeMetadata, WorktreeInfo, WorktreeStatus)
- ✅ `cargo test -p assay-mcp` — MCP tests pass
- ✅ `rg "eprintln" crates/assay-core/src/worktree.rs` — zero matches
- ✅ `rg "detect_main_worktree" crates/` — zero matches

## Diagnostics

- Branch deletion warnings in `cleanup()` now appear in structured tracing output instead of stderr
- `WorktreeListResult.warnings` is serialized in MCP `worktree_list` response — agents see prune warnings in JSON
- `read_metadata()` with corrupt JSON logs via `tracing::warn` and returns `None` (now tested)

## Deviations

- `WorktreeStatus` already had a schema snapshot test, so only `WorktreeInfo` snapshot was newly added (plan expected both to be new)
- The `config-schema` snapshot was also updated because adding `impl WorktreeConfig` with `as_path()` doesn't change the schema but the snapshot regeneration picked up existing drift

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/worktree.rs` — eprintln→tracing::warn, to_string_lossy docs, rename detect_linked_worktree, ASSAY_WORKTREE_DIR docs, 3 new tests
- `crates/assay-types/src/worktree.rs` — WorktreeConfig::as_path() helper, inventory::submit! for WorktreeInfo and WorktreeStatus
- `crates/assay-types/tests/schema_snapshots.rs` — worktree_info_schema_snapshot test added
- `crates/assay-types/tests/snapshots/schema_snapshots__worktree-info-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__config-schema.snap` — updated snapshot
- `crates/assay-mcp/src/server.rs` — TODO(M002) comment for cleanup_all, field duplication awareness comment
