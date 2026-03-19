---
id: S04
parent: M001
milestone: M001
provides:
  - "ClaudeConfig struct with five fields (claude_md, mcp_json, settings_json, hooks_json, model)"
  - "generate_config() — pure function translating HarnessProfile → Claude Code configuration artifacts"
  - "write_config() — writes ClaudeConfig files to a worktree directory (CLAUDE.md, .mcp.json, .claude/settings.json)"
  - "build_cli_args() — generates Vec<String> CLI arguments for claude --print invocation"
requires:
  - slice: S02
    provides: "assay-harness crate structure, HarnessProfile type in assay-types"
  - slice: S03
    provides: "build_prompt() function, merge_settings() function, HookContract/HookEvent types"
affects:
  - S07
key_files:
  - crates/assay-harness/src/claude.rs
  - crates/assay-harness/src/snapshots/
  - crates/assay-harness/Cargo.toml
key_decisions: []
patterns_established:
  - "Hook event mapping via match on HookEvent enum → Claude Code string keys (PreTool→PreToolUse, PostTool→PostToolUse, Stop→Stop)"
  - "BTreeMap for deterministic JSON key ordering in generated hooks"
  - "Empty-matcher groups (matches all tools) as the default hook format"
  - "write_config skips CLAUDE.md when claude_md is empty — conditional file writes based on content presence"
  - "build_cli_args uses relative paths (.mcp.json, .claude/settings.json) — assumes CWD is worktree root"
observability_surfaces:
  - "Snapshot files in src/snapshots/ serve as human-readable reference for generated Claude Code formats"
  - "insta snapshot mismatches produce inline diffs showing expected vs actual"
  - "write_config returns std::io::Error with OS-level path context on failure"
drill_down_paths:
  - ".kata/milestones/M001/slices/S04/tasks/T01-SUMMARY.md"
  - ".kata/milestones/M001/slices/S04/tasks/T02-SUMMARY.md"
duration: 23m
verification_result: passed
completed_at: 2026-03-16
---

# S04: Claude Code Adapter

**Pure adapter translating HarnessProfile into valid Claude Code configuration artifacts (CLAUDE.md, .mcp.json, settings.json, hooks), with CLI arg builder and file writer — locked by 12 insta snapshots and 6 file/args tests.**

## What Happened

Built the Claude Code adapter as three plain functions (not trait methods, satisfying R009) in the `assay-harness::claude` module:

1. **`generate_config()`** (T01) — Pure translation from `HarnessProfile` to `ClaudeConfig`. Calls `build_prompt()` for CLAUDE.md content. Translates settings into Claude's `{ "permissions": { "allow": [...], "deny": [] }, "model": "...", "hooks": { ... } }` format. Maps hook events (`PreTool` → `"PreToolUse"`, `PostTool` → `"PostToolUse"`, `Stop` → `"Stop"`) and groups them by event into Claude's matcher-group format with empty-string matcher. Generates `{ "mcpServers": {} }` structural wrapper for MCP servers with `"type": "stdio"` entries. Four snapshot tests cover realistic, minimal, hooks-only, and MCP-only scenarios (11 snapshot files).

2. **`write_config()`** (T02) — Writes CLAUDE.md (skipped when empty), .mcp.json, and .claude/settings.json to a target directory, creating the .claude/ subdirectory. Three tempfile-based tests verify file existence, directory creation, and conditional skip behavior.

3. **`build_cli_args()`** (T02) — Returns `Vec<String>` with `--print`, `--output-format json`, `--mcp-config`, `--settings`, and conditionally `--model` and `--system-prompt`. Three tests including one snapshot lock the flag ordering.

Added `insta` and `tempfile` as dev-dependencies to assay-harness.

## Verification

- `cargo test -p assay-harness` — 27/27 tests pass (17 pre-existing + 4 snapshot + 6 file/args)
- `just ready` — fmt, clippy, test, deny all pass
- 12 snapshot files in `crates/assay-harness/src/snapshots/` lock generated output
- Minimal profile test exercises the empty/default path producing valid minimal output
- File-write tests confirm directory creation and conditional CLAUDE.md skip

## Requirements Advanced

- R008 (Claude Code adapter) — fully implemented: generate_config, write_config, build_cli_args all functional with snapshot-locked output
- R009 (Callback-based control inversion) — satisfied: all three functions are plain functions, not trait methods
- R007 (Hook contract definitions) — hook contracts now translated to Claude Code's hooks.json format

## Requirements Validated

- R008 — Claude Code adapter generates valid CLAUDE.md, .mcp.json, settings.json, and hooks.json from HarnessProfile, locked by 12 snapshots and verified by file-write tests
- R009 — All adapter functions are plain functions (not trait methods), verified by code inspection and compilation

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T02 plan mentioned `--permission-mode` and `--allowed-tools` CLI flags, but these are not standard Claude Code CLI flags. Used `--system-prompt`, `--model`, `--mcp-config`, and `--settings` which match Claude Code's actual CLI interface.

## Known Limitations

- Claude Code adapter is pure file/arg generation — no runtime invocation or process management (deferred to S07)
- Hook timeout defaults to 30s when `timeout_secs` is `None` — may need configurability later
- `build_cli_args()` uses relative paths, assuming CWD is worktree root — S07 must set CWD correctly when launching

## Follow-ups

- S07 must set CWD to worktree root when calling `build_cli_args()` for correct relative path resolution
- S07 must handle Claude Code process lifecycle (launch, timeout, exit codes)

## Files Created/Modified

- `crates/assay-harness/Cargo.toml` — added insta + tempfile dev-dependencies
- `crates/assay-harness/src/claude.rs` — ClaudeConfig struct, generate_config(), write_config(), build_cli_args(), 10 tests
- `crates/assay-harness/src/snapshots/` — 12 insta snapshot files locking generated output

## Forward Intelligence

### What the next slice should know
- S07 consumes `generate_config()`, `write_config()`, and `build_cli_args()` from `assay_harness::claude`. The API is: build a `HarnessProfile`, call `generate_config()` to get a `ClaudeConfig`, call `write_config()` to write files to the worktree, call `build_cli_args()` to get the CLI invocation flags.
- `build_cli_args()` returns relative paths for `--mcp-config` and `--settings` — the caller must set CWD to the worktree root.

### What's fragile
- Snapshot tests are locked to Claude Code's current config format — if Claude Code changes its settings.json schema, hooks format, or CLI flags, snapshots will need updating.
- The `--print` and `--output-format json` flags in `build_cli_args()` assume Claude Code's print mode interface — verify this is still current before S07 integration testing.

### Authoritative diagnostics
- Run `cargo insta test -p assay-harness` to check for snapshot drift — any format change will produce an inline diff.
- Read snapshot files in `crates/assay-harness/src/snapshots/` to see exact expected output for each artifact type.

### What assumptions changed
- Plan assumed `--permission-mode` and `--allowed-tools` were valid Claude Code CLI flags — they aren't. Actual flags are `--system-prompt`, `--model`, `--mcp-config`, `--settings`.
