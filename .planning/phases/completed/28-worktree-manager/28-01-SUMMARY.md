# Phase 28 Plan 01: Worktree Foundation Summary

Git worktree lifecycle types, error variants, and core module with 6 public functions and full integration test coverage.

## Tasks Completed

### Task 1: Worktree types and error variants
- **Commit:** `1f61790`
- Created `assay-types/src/worktree.rs` with `WorktreeConfig`, `WorktreeInfo`, `WorktreeStatus` (all derive Serialize, Deserialize, JsonSchema, etc.)
- Added `pub mod worktree` and re-exports to `assay-types/src/lib.rs`
- Added optional `worktree: Option<WorktreeConfig>` field to `Config` struct
- Added 5 error variants to `AssayError`: `WorktreeGit`, `WorktreeGitFailed`, `WorktreeExists`, `WorktreeNotFound`, `WorktreeDirty`
- Updated schema snapshot and roundtrip test for new Config field

### Task 2: Core worktree module with git CLI integration
- **Commit:** `075287f`
- Created `assay-core/src/worktree.rs` with 6 public functions:
  - `resolve_worktree_dir` — 4-level config precedence (CLI > env > config > default)
  - `create` — validates spec exists, creates worktree with `assay/{slug}` branch
  - `list` — prunes stale entries, parses porcelain output, filters to assay branches
  - `status` — reports branch, HEAD, dirty state, ahead/behind counts
  - `cleanup` — dirty check with force override, removes worktree and branch
  - `detect_main_worktree` — detects linked worktree context via `.git` file parsing
- 7 unit tests: porcelain parsing (normal, empty, bare, detached) + config resolution (default, config, env override, CLI override, relative paths)
- 9 integration tests with real git repos: full lifecycle, SpecNotFound, WorktreeExists, WorktreeDirty, directory-based spec, nonexistent status/cleanup, detect_main_worktree
- Added `serial_test` dev-dependency for env var test isolation

## Deviations

- **Config field propagation:** The new `worktree` field on `Config` required updating struct constructors in `assay-core/src/config/mod.rs` (1 test), `assay-mcp/src/server.rs` (3 tests), and `assay-types/tests/schema_roundtrip.rs` (1 test). Schema snapshot also updated. These are mechanical fixes, not architectural changes.
- **Spec format in tests:** Initial test spec content used incorrect field names (`gate`, `command` instead of `cmd`, missing `description`). Fixed to match the actual `Criterion` struct schema.
- **Rust 2024 edition:** `std::env::set_var` and `std::env::remove_var` are unsafe in edition 2024. Wrapped in `unsafe` blocks with SAFETY comments.
- **Pre-existing issue:** `just ready` fails on `check-plugin-version` (plugin.json version 0.1.0 != workspace 0.2.0). This is pre-existing and unrelated. All other checks (fmt, lint, test, deny) pass.

## Verification

- `cargo check -p assay-types` — pass
- `cargo check -p assay-core` — pass
- `cargo test -p assay-core worktree` — 18/18 pass
- `just fmt-check && just lint && just test && just deny` — all pass
