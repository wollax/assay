# S05: Worktree Enhancements & Tech Debt — UAT

**Milestone:** M001
**Written:** 2026-03-16

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All deliverables are internal library functions and type changes verified by unit/integration tests against real git worktrees. No user-facing UI or runtime service to exercise.

## Preconditions

- Git repository with `.assay/` directory initialized
- Rust toolchain installed (`cargo`, `rustc`)
- `just` command runner available

## Smoke Test

Run `just ready` — all fmt, lint, test, and deny checks pass. This confirms the entire workspace compiles and all 35 worktree tests pass including session linkage, orphan detection, and collision prevention.

## Test Cases

### 1. Session linkage round-trip

1. Run `cargo test -p assay-core -- test_metadata_session_id_round_trip`
2. **Expected:** Test passes — metadata with session_id serializes/deserializes correctly, legacy JSON without session_id deserializes to None, create() with session_id persists to disk.

### 2. Orphan detection accuracy

1. Run `cargo test -p assay-core -- test_detect_orphans`
2. **Expected:** 4 tests pass — worktrees classified correctly: no session_id → orphaned, active session → not orphaned, terminal session → orphaned, missing session → orphaned.

### 3. Collision prevention

1. Run `cargo test -p assay-core -- test_collision`
2. **Expected:** 3 tests pass — active session → WorktreeCollision error, terminal session → allowed, no existing worktree → succeeds.

### 4. Tech debt verification

1. Run `rg "eprintln" crates/assay-core/src/worktree.rs`
2. Run `rg "detect_main_worktree" crates/`
3. **Expected:** Both return zero matches.

### 5. Schema snapshots

1. Run `cargo insta test -p assay-types`
2. **Expected:** All snapshots accepted, no pending reviews. WorktreeMetadata includes session_id field, WorktreeInfo snapshot exists.

## Edge Cases

### Corrupt metadata handling

1. Run `cargo test -p assay-core -- test_read_metadata_corrupt_json`
2. **Expected:** Returns None gracefully (no panic), logs warning.

### Git exclude behavior

1. Run `cargo test -p assay-core -- test_write_metadata_adds_git_exclude`
2. **Expected:** `.assay/worktree.json` is added to `.git/info/exclude` in the worktree.

### Prune warning propagation

1. Run `cargo test -p assay-core -- test_list_prune_warning`
2. **Expected:** Prune warnings from git are captured and surfaced in `WorktreeListResult.warnings`.

## Failure Signals

- Any `cargo test -p assay-core -- worktree` failure indicates regression in worktree operations
- `cargo insta test -p assay-types` showing pending snapshots means schema drift
- `rg "eprintln" crates/assay-core/src/worktree.rs` returning matches means tech debt regression
- `rg "detect_main_worktree" crates/` returning matches means incomplete rename

## Requirements Proved By This UAT

- R010 — Orphan detection: `detect_orphans()` correctly classifies worktrees by cross-referencing session state (4 test scenarios)
- R011 — Collision prevention: `create()` rejects duplicate active worktrees with actionable `WorktreeCollision` error (3 test scenarios)
- R012 — Session linkage: `WorktreeMetadata.session_id` persists, round-trips, and handles legacy JSON (backward compatible)
- R013 — Tech debt: all 15 items addressed — zero eprintln, zero detect_main_worktree, schema registry complete, edge-case tests pass

## Not Proven By This UAT

- Runtime orphan detection in a real multi-session pipeline scenario (deferred to S07 E2E)
- MCP-level collision rejection through the MCP tool interface (MCP handler passes None for session_id; real session_id threading happens in S07)
- worktree_cleanup_all MCP tool (deferred to M002)

## Notes for Tester

All verification is automated via `just ready` and targeted `cargo test` commands. No manual testing required. The collision check in `create()` currently only fires when `session_id` is provided by the caller — MCP and CLI callers pass `None` until S07 wires in the pipeline with real session IDs.
