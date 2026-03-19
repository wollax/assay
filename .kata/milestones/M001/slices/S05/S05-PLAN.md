# S05: Worktree Enhancements & Tech Debt

**Goal:** Worktrees have session linkage, orphan detection, collision prevention, and 15 tech debt issues resolved.
**Demo:** `create()` accepts `session_id` and rejects duplicate active worktrees for a spec; `detect_orphans()` returns worktrees with no active session; `just ready` passes with all tech debt fixed.

## Must-Haves

- `WorktreeMetadata` has `session_id: Option<String>` with `#[serde(default)]` and `deny_unknown_fields` (R012)
- `detect_orphans()` cross-references worktree metadata against `WorkSession` records (R010)
- `create()` rejects when spec already has an active worktree with in-progress session (R011)
- 15 tech debt items resolved: `deny_unknown_fields`, `to_string_lossy`, `eprintln!`, `detect_main_worktree` rename, `WorktreeConfig.base_dir` helper, schema snapshots/registry for `WorktreeInfo`/`WorktreeStatus`, missing tests for `read_metadata` corrupt JSON / `write_metadata` git exclude / `list()` prune failure, `ASSAY_WORKTREE_DIR` doc comment, MCP warning surfacing (R013)
- MCP `worktree_create` handler updated to pass `session_id: None` without breaking callers (D005)
- Schema snapshot updated for `WorktreeMetadata` with new field
- `just ready` passes

## Proof Level

- This slice proves: contract + integration (unit tests + git-worktree integration tests)
- Real runtime required: yes (git CLI integration tests create real worktrees)
- Human/UAT required: no

## Verification

- `just ready` — all checks pass (fmt, lint, test, deny)
- `cargo test -p assay-core -- worktree` — all worktree tests pass including new tests for session linkage, orphan detection, collision prevention, corrupt metadata, git exclude, and prune failure
- `cargo insta test -p assay-types` — schema snapshots accepted for `WorktreeMetadata`, `WorktreeInfo`, `WorktreeStatus`
- `cargo test -p assay-mcp` — MCP tests pass with updated `create()` call site
- `rg "eprintln" crates/assay-core/src/worktree.rs` — zero matches
- `rg "detect_main_worktree" crates/` — zero matches (renamed)

## Observability / Diagnostics

- Runtime signals: `tracing::warn!` on orphan detection failures, collision rejection includes spec_slug + existing worktree path
- Inspection surfaces: `detect_orphans()` returns `Vec<WorktreeInfo>` listing all orphaned worktrees; `WorktreeMetadata.session_id` inspectable in `.assay/worktree.json`
- Failure visibility: `WorktreeCollision` error variant with spec_slug and existing path; orphan entries include worktree path for cleanup targeting
- Redaction constraints: none (no secrets in worktree metadata)

## Integration Closure

- Upstream surfaces consumed: `work_session::list_sessions()` + `load_session()` from assay-core (proven in S01), `SessionPhase::is_terminal()` from assay-types
- New wiring introduced in this slice: `session_id` linkage between worktree metadata and work sessions; orphan detection cross-referencing worktrees against sessions; collision check in `create()`
- What remains before the milestone is truly usable end-to-end: S06 (RunManifest parsing), S07 (pipeline assembly wiring worktree create into orchestration flow)

## Tasks

- [x] **T01: Session linkage on WorktreeMetadata and create() signature** `est:30m`
  - Why: Foundation for R010/R011 — adds `session_id` to the persisted type and threads it through `create()` and all callers
  - Files: `crates/assay-types/src/worktree.rs`, `crates/assay-core/src/worktree.rs`, `crates/assay-mcp/src/server.rs`, `crates/assay-types/tests/snapshots/`
  - Do: Add `session_id: Option<String>` with `#[serde(default, skip_serializing_if)]` to `WorktreeMetadata`. Add `deny_unknown_fields`. Update schema snapshot. Add `session_id: Option<&str>` param to `create()`. Update all callers (MCP handler, tests). Add unit test verifying metadata round-trip with and without session_id.
  - Verify: `cargo test -p assay-core -- worktree` + `cargo insta test -p assay-types` + `cargo test -p assay-mcp`
  - Done when: `WorktreeMetadata` has `session_id`, `create()` accepts and persists it, schema snapshot updated, all tests pass

- [x] **T02: Orphan detection and collision prevention** `est:35m`
  - Why: Delivers R010 (orphan detection) and R011 (collision prevention) — the safety features that prevent worktree leaks and duplicate launches
  - Files: `crates/assay-core/src/worktree.rs`, `crates/assay-core/src/error.rs`
  - Do: Implement `detect_orphans(project_root, assay_dir)` that cross-references `list()` entries against `work_session::list_sessions()` + `load_session()`, returning entries where session_id is None or points to a terminal session. Add `WorktreeCollision` error variant. Add collision check at top of `create()` — scan existing worktrees for matching spec_slug with active session. Write tests: orphan detection with mixed active/terminal/missing sessions; collision rejection; collision allowed when no active session.
  - Verify: `cargo test -p assay-core -- worktree::tests::test_detect_orphans` + `cargo test -p assay-core -- worktree::tests::test_collision`
  - Done when: `detect_orphans()` returns correct orphan list, `create()` rejects collisions with `WorktreeCollision` error, all new tests pass

- [x] **T03: Worktree tech debt resolution** `est:40m`
  - Why: Resolves all 15 tracked tech debt items (R013) — cleans the foundation before S07 harness integration
  - Files: `crates/assay-core/src/worktree.rs`, `crates/assay-types/src/worktree.rs`, `crates/assay-mcp/src/server.rs`
  - Do: (1) `eprintln!` → `tracing::warn!` in cleanup. (2) `to_string_lossy` → document UTF-8 assumption with comment, keep lossy (git CLI requires string args). (3) Rename `detect_main_worktree` → `detect_linked_worktree` across codebase. (4) Add `WorktreeConfig::as_path()` helper returning `Path`. (5) Add `inventory::submit!` for `WorktreeInfo` and `WorktreeStatus` + schema snapshot tests. (6) Add test for `read_metadata` with corrupt JSON. (7) Add test for `write_metadata` git exclude behavior. (8) Add test for `list()` prune failure warning path. (9) Add doc comment for `ASSAY_WORKTREE_DIR` env var. (10) Surface `WorktreeListResult.warnings` in MCP `worktree_list` response. (11-15) Remaining items: field duplication awareness comment, MCP cleanup --all deferred (out of D005 scope — add TODO comment), prune failure surfacing in MCP.
  - Verify: `just ready` + `rg "eprintln" crates/assay-core/src/worktree.rs` returns 0 + `rg "detect_main_worktree" crates/` returns 0
  - Done when: All 15 tech debt items addressed (fixed or explicitly deferred with rationale), `just ready` passes, no `eprintln!` in worktree module, `detect_main_worktree` fully renamed

## Files Likely Touched

- `crates/assay-types/src/worktree.rs`
- `crates/assay-core/src/worktree.rs`
- `crates/assay-core/src/error.rs`
- `crates/assay-mcp/src/server.rs`
- `crates/assay-types/tests/schema_snapshots.rs`
- `crates/assay-types/tests/snapshots/` (schema snapshot files)
