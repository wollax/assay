---
id: T01
parent: S04
milestone: M001
provides:
  - ClaudeConfig struct with five fields (claude_md, mcp_json, settings_json, hooks_json, model)
  - generate_config() pure function translating HarnessProfile → Claude Code artifacts
  - 4 snapshot tests locking output format (11 snapshot files)
key_files:
  - crates/assay-harness/src/claude.rs
  - crates/assay-harness/src/snapshots/
key_decisions: []
patterns_established:
  - Hook event mapping via match on HookEvent enum → Claude Code string keys
  - BTreeMap for deterministic JSON key ordering in generated hooks
  - Empty-matcher groups (matches all tools) as the default hook format
observability_surfaces:
  - Snapshot files in src/snapshots/ serve as human-readable reference for generated formats
  - insta snapshot mismatches produce inline diffs showing expected vs actual
duration: 15m
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T01: Implement ClaudeConfig and generate_config() with snapshot tests

**Implemented pure `generate_config()` translating `HarnessProfile` into Claude Code's four configuration artifacts, locked by 11 insta snapshots across 4 test cases.**

## What Happened

Added `insta` and `tempfile` as dev-dependencies to `assay-harness`. Defined `ClaudeConfig` struct with `claude_md`, `mcp_json`, `settings_json`, `hooks_json`, and `model` fields. Implemented `generate_config()` as a plain function (not trait method, satisfying R009) that:

- Calls `build_prompt()` for CLAUDE.md content
- Translates settings to `{ "permissions": { "allow": [...], "deny": [] }, "model": "...", "hooks": { ... } }` format
- Maps hook events: `PreTool` → `"PreToolUse"`, `PostTool` → `"PostToolUse"`, `Stop` → `"Stop"`
- Groups hooks by event into Claude's matcher-group format with empty-string matcher (matches all tools)
- Defaults hook timeout to 30s when `timeout_secs` is `None`
- Generates `{ "mcpServers": {} }` structural wrapper for MCP
- Produces standalone hooks JSON separately for flexibility

Four snapshot tests cover: realistic full profile, minimal empty profile, hooks-without-model profile, and MCP structural wrapper verification.

## Verification

- `cargo test -p assay-harness` — 21 tests pass (17 existing + 4 new)
- `cargo clippy -p assay-harness` — no warnings
- `just ready` — all checks pass (fmt, clippy, test, deny)
- 11 snapshot files created in `crates/assay-harness/src/snapshots/`
- Minimal profile test exercises the empty/default path producing valid minimal output

## Diagnostics

- Read snapshot files in `crates/assay-harness/src/snapshots/` to see exact generated format for each artifact
- Run `cargo insta test -p assay-harness` to check for snapshot drift
- Snapshot mismatches produce inline diffs via insta

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-harness/Cargo.toml` — added insta + tempfile dev-dependencies
- `crates/assay-harness/src/claude.rs` — ClaudeConfig struct, generate_config(), build_hooks(), hook_event_key(), 4 snapshot tests
- `crates/assay-harness/src/snapshots/` — 11 insta snapshot files locking generated output
