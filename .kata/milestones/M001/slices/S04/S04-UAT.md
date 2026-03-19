# S04: Claude Code Adapter — UAT

**Milestone:** M001
**Written:** 2026-03-16

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S04 is pure config generation with no runtime side effects — all outputs are deterministic strings and files. Snapshot tests lock the exact output format, and file-write tests verify the directory layout. No live Claude Code process is invoked (that's S07).

## Preconditions

- Rust toolchain installed
- Working directory is the assay project root
- `just ready` passes (confirms clean build state)

## Smoke Test

Run `cargo test -p assay-harness -- claude` — all 10 claude-specific tests pass, confirming generate/write/args functions work.

## Test Cases

### 1. Realistic profile generates valid Claude Code artifacts

1. Run `cargo test -p assay-harness -- realistic_profile --nocapture`
2. **Expected:** Test passes. Snapshot files in `src/snapshots/` show CLAUDE.md with prompt content, settings.json with permissions and hooks, mcp.json with server entries including `"type": "stdio"`, and standalone hooks.json.

### 2. Minimal profile produces valid but empty output

1. Run `cargo test -p assay-harness -- minimal_profile --nocapture`
2. **Expected:** Test passes. Generated CLAUDE.md is empty string, settings.json has empty permissions and no hooks, mcp.json has empty mcpServers object.

### 3. write_config creates correct directory layout

1. Run `cargo test -p assay-harness -- write_config --nocapture`
2. **Expected:** Tests pass. Tempdir contains CLAUDE.md at root, .mcp.json at root, .claude/settings.json in subdirectory. When claude_md is empty, CLAUDE.md is not written.

### 4. build_cli_args returns correct flags

1. Run `cargo test -p assay-harness -- cli_args --nocapture`
2. **Expected:** Tests pass. Full config produces `--print`, `--output-format json`, `--model`, `--system-prompt`, `--mcp-config .mcp.json`, `--settings .claude/settings.json`. No-model config omits `--model`. Empty-claude_md config omits `--system-prompt`.

## Edge Cases

### Empty hooks but non-empty settings

1. Run `cargo test -p assay-harness -- minimal_profile`
2. **Expected:** Settings JSON contains `"hooks": {}` (empty object), not missing key.

### Hook event mapping correctness

1. Run `cargo test -p assay-harness -- hooks_no_model`
2. **Expected:** PreTool maps to "PreToolUse", PostTool maps to "PostToolUse", Stop maps to "Stop" in generated hooks JSON.

## Failure Signals

- Any `cargo insta test -p assay-harness` failure means generated format has drifted from locked snapshots
- `write_config` test failure means file layout or conditional write logic is broken
- Clippy warnings in claude.rs may indicate unused fields or dead code paths

## Requirements Proved By This UAT

- R008 (Claude Code adapter) — generate_config produces valid CLAUDE.md, .mcp.json, settings.json, and hooks.json from HarnessProfile; write_config creates correct file layout; build_cli_args returns correct CLI flags. All locked by snapshots and file-write tests.
- R009 (Callback-based control inversion) — all three public functions are plain functions, not trait methods. Verified by code inspection and successful compilation without any trait definitions.
- R007 (Hook contract definitions) — hook events translate correctly to Claude Code's hooks.json format (PreTool→PreToolUse, PostTool→PostToolUse, Stop→Stop).

## Not Proven By This UAT

- Runtime Claude Code invocation — adapter generates config but does not launch or communicate with Claude Code (deferred to S07)
- Claude Code actually accepting the generated config files — format correctness is locked by snapshots against documented format, but real acceptance testing requires S07
- Process lifecycle management (timeout, crash, exit codes) — deferred to S07

## Notes for Tester

- All tests are automated — no manual steps required beyond running `cargo test -p assay-harness`
- Snapshot files in `crates/assay-harness/src/snapshots/` are the authoritative reference for expected output formats
- To review snapshots interactively: `cargo insta review -p assay-harness`
