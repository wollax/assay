---
id: S05
parent: M001
milestone: M001
provides:
  - session_id field on WorktreeMetadata with deny_unknown_fields
  - create() accepts session_id parameter and persists it
  - detect_orphans() cross-references worktrees against work sessions
  - WorktreeCollision error variant with collision prevention in create()
  - detect_linked_worktree renamed from detect_main_worktree
  - WorktreeConfig::as_path() helper
  - inventory::submit! for WorktreeInfo and WorktreeStatus with schema snapshots
  - eprintln replaced with tracing::warn in cleanup()
  - 3 new tests (corrupt metadata, git exclude, prune warning propagation)
  - ASSAY_WORKTREE_DIR env var documented
  - MCP worktree_list surfaces warnings
requires:
  - slice: S01
    provides: GateEvalContext persistence, work_session module with list_sessions/load_session
affects:
  - S07
key_files:
  - crates/assay-types/src/worktree.rs
  - crates/assay-core/src/worktree.rs
  - crates/assay-core/src/error.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-cli/src/commands/worktree.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__worktree-info-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__worktree-metadata-schema.snap
key_decisions:
  - session_id is metadata-only — not added to WorktreeInfo since session linkage is a persistence concern, not a list/status concern
  - Collision check derives assay_dir from specs_dir parent to avoid changing create() signature further
  - to_string_lossy kept with doc comments (git CLI requires String args; switching to OsStr-aware git2 not worth the dep)
  - WorktreeConfig.base_dir stays as String (schema-breaking to change); as_path() added as ergonomic helper
  - Field duplication between WorktreeInfo and WorktreeStatus documented but not unified (deferred per D005)
  - worktree_cleanup_all deferred to M002 per D005 (MCP additive-only)
patterns_established:
  - detect_orphans uses list() + read_metadata + load_session composition pattern for cross-referencing lifecycle state
  - tracing::warn with structured fields for git operation warnings (replaces eprintln)
  - Collision check runs before filesystem/branch checks for clearer error messages
observability_surfaces:
  - session_id field visible in .assay/worktree.json metadata files
  - detect_orphans() returns Vec<WorktreeInfo> listing all orphaned worktrees with paths
  - WorktreeCollision error includes spec_slug and existing_path for actionable diagnosis
  - cleanup() branch deletion warnings in structured tracing output
  - WorktreeListResult.warnings surfaced in MCP worktree_list JSON response
  - corrupt metadata logged via tracing::warn and returns None gracefully
drill_down_paths:
  - .kata/milestones/M001/slices/S05/tasks/T01-SUMMARY.md
  - .kata/milestones/M001/slices/S05/tasks/T02-SUMMARY.md
  - .kata/milestones/M001/slices/S05/tasks/T03-SUMMARY.md
duration: 32min
verification_result: passed
completed_at: 2026-03-16
---

# S05: Worktree Enhancements & Tech Debt

**Worktrees now have session linkage, orphan detection, collision prevention, and all 15 tracked tech debt items resolved with structured observability.**

## What Happened

**T01 — Session linkage:** Added `session_id: Option<String>` to `WorktreeMetadata` with `deny_unknown_fields`, threaded it through `create()` and all callers (MCP, CLI, tests). Added round-trip test covering metadata with/without session_id and legacy JSON deserialization. Schema snapshot updated.

**T02 — Orphan detection & collision prevention:** Implemented `detect_orphans(project_root, assay_dir)` that cross-references `list()` entries against work sessions — worktrees are orphaned when session_id is None, session doesn't exist, or session is terminal. Added `WorktreeCollision` error variant. Added collision check at top of `create()` that rejects when the same spec already has an active worktree. 7 new tests cover orphan classification (4) and collision scenarios (3).

**T03 — Tech debt resolution:** Addressed all 15 items: replaced `eprintln!` with `tracing::warn!` in cleanup, renamed `detect_main_worktree` → `detect_linked_worktree`, added `WorktreeConfig::as_path()`, registered `WorktreeInfo`/`WorktreeStatus` in schema registry with snapshots, added tests for corrupt metadata/git exclude/prune warning propagation, documented `ASSAY_WORKTREE_DIR`, surfaced MCP warnings, and added TODO comments for deferred items.

## Verification

- ✅ `just ready` — all checks pass (fmt, lint, test, deny)
- ✅ `cargo test -p assay-core -- worktree` — 35 tests pass (10 new across T01–T03)
- ✅ `cargo insta test -p assay-types` — all snapshots accepted (WorktreeMetadata, WorktreeInfo, WorktreeStatus)
- ✅ `cargo test -p assay-mcp` — 27 MCP tests pass
- ✅ `rg "eprintln" crates/assay-core/src/worktree.rs` — zero matches
- ✅ `rg "detect_main_worktree" crates/` — zero matches

## Requirements Advanced

- R010 (Worktree orphan detection) — `detect_orphans()` implemented and tested with 4 classification scenarios
- R011 (Worktree collision prevention) — collision check in `create()` with `WorktreeCollision` error, tested with 3 scenarios
- R012 (WorktreeMetadata session linkage) — `session_id: Option<String>` added with serde defaults, deny_unknown_fields, schema snapshot
- R013 (Worktree tech debt resolution) — all 15 items addressed (fixed or explicitly deferred with rationale)

## Requirements Validated

- R010 — orphan detection verified by unit tests covering no-session, active-session, terminal-session, and missing-session cases
- R011 — collision prevention verified by unit tests covering active-collision-rejected, terminal-allowed, and no-existing-worktree cases
- R012 — session linkage verified by round-trip test with/without session_id and legacy JSON backward compatibility
- R013 — all 15 items verified: zero eprintln matches, zero detect_main_worktree matches, schema snapshots accepted, 3 new edge-case tests pass, just ready passes

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T01 also updated `crates/assay-cli/src/commands/worktree.rs` (not listed in task plan but required for compilation)
- T01 decided not to add `session_id` to `WorktreeInfo` — evaluated and determined it's metadata-only
- T03 found `WorktreeStatus` already had a schema snapshot test, so only `WorktreeInfo` was newly added

## Known Limitations

- `worktree_cleanup_all` MCP tool deferred to M002 per D005 (additive-only MCP convention)
- Field duplication between `WorktreeInfo` and `WorktreeStatus` documented but not unified — deferred until API stabilizes
- `to_string_lossy` retained at git CLI call sites with doc comments explaining UTF-8 assumption

## Follow-ups

- none

## Files Created/Modified

- `crates/assay-types/src/worktree.rs` — session_id field, deny_unknown_fields, WorktreeConfig::as_path(), inventory entries
- `crates/assay-core/src/worktree.rs` — create() session_id param, detect_orphans(), collision check, detect_linked_worktree rename, eprintln→tracing::warn, ASSAY_WORKTREE_DIR docs, 10 new tests
- `crates/assay-core/src/error.rs` — WorktreeCollision error variant
- `crates/assay-mcp/src/server.rs` — worktree_create passes None for session_id, TODO comments for deferred items
- `crates/assay-cli/src/commands/worktree.rs` — updated create() call with session_id: None
- `crates/assay-types/tests/schema_snapshots.rs` — worktree_info_schema_snapshot test
- `crates/assay-types/tests/snapshots/schema_snapshots__worktree-info-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__worktree-metadata-schema.snap` — updated with session_id
- `crates/assay-types/tests/snapshots/schema_snapshots__config-schema.snap` — regenerated
- `crates/assay-types/tests/schema_roundtrip.rs` — updated WorktreeMetadata construction

## Forward Intelligence

### What the next slice should know
- `create()` now takes `session_id: Option<&str>` — S07 pipeline must pass the actual session ID when creating worktrees for manifest execution
- `detect_orphans()` requires both `project_root` and `assay_dir` — the pipeline can use it for pre-flight cleanup
- Collision check derives `assay_dir` from `specs_dir.parent()` inside `create()` — no additional parameter needed from callers

### What's fragile
- Collision check depends on `specs_dir.parent()` being the assay directory — if the directory layout changes, the derivation breaks silently

### Authoritative diagnostics
- `cargo test -p assay-core -- worktree` — 35 tests covering all worktree operations including new lifecycle features
- `.assay/worktree.json` in any worktree — inspect session_id linkage directly

### What assumptions changed
- Plan assumed both WorktreeInfo and WorktreeStatus needed new schema snapshot tests — WorktreeStatus already had one
