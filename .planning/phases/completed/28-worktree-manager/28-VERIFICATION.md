# Phase 28 Verification: Worktree Manager

**Status: passed**

**Date:** 2026-03-09
**Verifier:** Claude (Kata phase verifier)

---

## Must-Have Checklist

### 1. `assay worktree create <spec>` creates an isolated git worktree

**Status: PASS**

- CLI command exists at `crates/assay-cli/src/commands/worktree.rs` (lines 9-35) with `name`, `--base`, `--worktree-dir`, and `--json` arguments.
- Core function `create()` at `crates/assay-core/src/worktree.rs` (lines 133-176):
  - Validates spec exists via `crate::spec::load_spec_entry`.
  - Creates worktree at `<worktree_base>/<spec_slug>`.
  - Branch named `assay/<spec_slug>` (line 144).
  - Uses configurable base branch with auto-detection fallback.
- Wired into CLI dispatch in `main.rs` (line 176).
- Integration test `test_create_list_status_cleanup` confirms branch is `assay/auth-flow`.

### 2. `assay worktree list` shows all active worktrees with spec, branch, and status

**Status: PASS**

- CLI command at `crates/assay-cli/src/commands/worktree.rs` (lines 37-51).
- Core function `list()` at `crates/assay-core/src/worktree.rs` (lines 182-205):
  - Prunes stale entries first.
  - Parses `git worktree list --porcelain`.
  - Filters to branches starting with `assay/`.
  - Returns `Vec<WorktreeInfo>` with `spec_slug`, `path`, `branch`.
- CLI renders a formatted table with Spec, Branch, Path columns or JSON output.

### 3. `assay worktree status <spec>` reports branch, dirty state, ahead/behind

**Status: PASS**

- CLI command at `crates/assay-cli/src/commands/worktree.rs` (lines 53-69).
- Core function `status()` at `crates/assay-core/src/worktree.rs` (lines 208-249):
  - Reports `branch`, `head` (abbreviated SHA), `dirty` (boolean), `ahead`, `behind`.
  - Uses `git status --porcelain` for dirty detection.
  - Uses `git rev-list --left-right --count HEAD...@{upstream}` for ahead/behind, defaulting to 0/0 if no upstream.
- CLI displays all fields in human-readable or JSON format.

### 4. `assay worktree cleanup <spec>` removes worktree and prunes refs

**Status: PASS**

- CLI command at `crates/assay-cli/src/commands/worktree.rs` (lines 71-99) with `--force`, `--all`, `--json`.
- Core function `cleanup()` at `crates/assay-core/src/worktree.rs` (lines 255-290):
  - Checks dirty state; returns `WorktreeDirty` if dirty and not forced.
  - Runs `git worktree remove [--force] <path>`.
  - Deletes the `assay/<spec_slug>` branch via `git branch -D`.
- CLI also supports `--all` mode for batch cleanup.
- Integration test `test_cleanup_dirty_without_force_returns_worktree_dirty` confirms dirty guard.

### 5. MCP tools `worktree_create`, `worktree_status`, `worktree_cleanup` callable by agents

**Status: PASS**

- All four MCP tools registered in `crates/assay-mcp/src/server.rs`:
  - `worktree_create` (line 921) — returns `WorktreeInfo` as JSON.
  - `worktree_list` (line 958) — returns `Vec<WorktreeInfo>` as JSON.
  - `worktree_status` (line 978) — returns `WorktreeStatus` as JSON.
  - `worktree_cleanup` (line 1009) — defaults to `force=true` for non-interactive agent use.
- Parameter structs with `JsonSchema` derives for schema generation.
- Domain errors returned as `CallToolResult` with `isError: true`.

---

## Additional Verification

### Types (`crates/assay-types/src/worktree.rs`)

- `WorktreeConfig` — `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]` with `base_dir` field.
- `WorktreeInfo` — `spec_slug`, `path`, `branch`, `base_branch` (optional).
- `WorktreeStatus` — `spec_slug`, `path`, `branch`, `head`, `dirty`, `ahead`, `behind`.
- All types registered in `schema_registry` (WorktreeConfig) and re-exported from `assay_types::lib`.
- `Config` struct includes `worktree: Option<WorktreeConfig>`.

### Error Variants (`crates/assay-core/src/error.rs`)

- `WorktreeGit` — git command failed to spawn.
- `WorktreeGitFailed` — git command exited non-zero.
- `WorktreeExists` — worktree already exists for spec.
- `WorktreeNotFound` — no worktree for spec.
- `WorktreeDirty` — uncommitted changes, cleanup refused.

### Configurable Paths (`resolve_worktree_dir`)

- Precedence: CLI override > `ASSAY_WORKTREE_DIR` env var > config `worktree.base_dir` > default `../<project>-worktrees/`.
- Relative paths resolved against project root.
- 5 unit tests verify all precedence levels.

### Helper Function (`detect_main_worktree`)

- Detects if CWD is a linked worktree by checking if `.git` is a file.
- Navigates gitdir path to find main repo root.
- Integration test confirms detection from linked worktree and returns `None` from main worktree.

### Tests

- **18 tests passed** (`cargo test -p assay-core worktree`):
  - 4 parse unit tests (normal, empty, bare, detached).
  - 5 resolve_worktree_dir unit tests (default, config, env, cli, relative).
  - 9 integration tests (create/list/status/cleanup lifecycle, nonexistent spec, duplicate, dirty guard, directory-based spec, status not found, cleanup not found, detect_main_worktree from linked, detect from main).

### CLI Help

- `assay worktree --help` outputs correctly with all four subcommands: `create`, `list`, `status`, `cleanup`.

---

## Summary

All 5 must-haves are fully implemented and verified against the actual codebase. The implementation includes comprehensive error handling, configurable paths with a clear precedence chain, both human-readable and JSON output modes, and 18 passing tests covering unit and integration scenarios.
