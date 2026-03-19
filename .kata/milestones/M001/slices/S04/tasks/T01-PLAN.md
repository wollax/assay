---
estimated_steps: 5
estimated_files: 3
---

# T01: Implement ClaudeConfig and generate_config() with snapshot tests

**Slice:** S04 — Claude Code Adapter
**Milestone:** M001

## Description

Implement the core translation layer that converts a `HarnessProfile` into Claude Code's four configuration artifacts: CLAUDE.md content, .mcp.json content, settings JSON (with embedded hooks), and optionally standalone hooks JSON. The `generate_config()` function is pure — no I/O, no side effects. Snapshot tests lock the exact generated output to catch format drift.

## Steps

1. Add `insta` and `tempfile` as dev-dependencies to `crates/assay-harness/Cargo.toml` (workspace refs).
2. Define `ClaudeConfig` struct in `claude.rs` with fields: `claude_md: String`, `mcp_json: String`, `settings_json: String`, `hooks_json: String` (the last for standalone use if needed). Add a `model: Option<String>` field for CLI arg generation in T02.
3. Implement `generate_config(profile: &HarnessProfile) -> ClaudeConfig`:
   - Call `crate::prompt::build_prompt(&profile.prompt_layers)` for `claude_md`
   - Translate `profile.settings` to Claude's settings JSON format: `{ "permissions": { "allow": [...], "deny": [] }, "model": "...", "hooks": { ... } }`. Map `permissions` vec to `allow` array, keep `deny` empty. Include `model` only if `Some`.
   - Translate `profile.hooks` to Claude's hooks format embedded in settings: group by event, map `HookEvent::PreTool` → `"PreToolUse"`, `PostTool` → `"PostToolUse"`, `Stop` → `"Stop"`. Each group becomes `"EventName": [{ "matcher": "", "hooks": [{ "type": "command", "command": "...", "timeout": N }] }]`. Use empty string matcher (matches all tools). Default timeout to 30 if `timeout_secs` is None.
   - Build `mcp_json` as `{ "mcpServers": {} }` — empty by default. The profile doesn't carry MCP server definitions (those come from the worktree context in S07). Generate the structural wrapper so `write_config` always writes a valid file.
   - Store `hooks_json` separately as well (the standalone `{ "hooks": { ... } }` format) for flexibility.
4. Write snapshot tests covering:
   - Realistic profile with prompt layers, settings (model, permissions, tools, max_turns), and hooks (PreTool, PostTool, Stop)
   - Minimal profile with empty prompt layers, default settings, no hooks
   - Profile with hooks but no model override
   - Profile with MCP (just structural — verifies empty mcpServers wrapper)
5. Run `cargo test -p assay-harness` and review/accept snapshots with `cargo insta review` or inline acceptance.

## Must-Haves

- [ ] `ClaudeConfig` struct defined with `claude_md`, `mcp_json`, `settings_json`, `hooks_json`, `model` fields
- [ ] `generate_config()` is a plain function (not trait method) — satisfies R009
- [ ] Hook event mapping: `PreTool` → `"PreToolUse"`, `PostTool` → `"PostToolUse"`, `Stop` → `"Stop"`
- [ ] Generated settings JSON matches Claude Code's format: `{ "permissions": { "allow": [...], "deny": [] }, "hooks": { ... } }`
- [ ] At least 4 snapshot tests locked via `insta`
- [ ] `cargo test -p assay-harness` passes

## Verification

- `cargo test -p assay-harness` — all tests pass (existing 17 + new snapshot tests)
- Snapshot files exist in `crates/assay-harness/src/snapshots/` with expected content
- `cargo clippy -p assay-harness` — no warnings

## Observability Impact

- Signals added/changed: None — pure function, no runtime state
- How a future agent inspects this: Read snapshot files in `src/snapshots/` to see exact generated format; run `cargo insta test -p assay-harness` to check for drift
- Failure state exposed: Snapshot mismatches produce inline diffs showing expected vs actual

## Inputs

- `crates/assay-types/src/harness.rs` — `HarnessProfile`, `HookContract`, `HookEvent`, `SettingsOverride`, `PromptLayer` types
- `crates/assay-harness/src/prompt.rs` — `build_prompt()` function
- `plugins/claude-code/hooks/hooks.json` — reference hooks format
- `plugins/claude-code/.mcp.json` — reference MCP config format
- S03 forward intelligence: `build_prompt()` returns plain string, hooks are type-validated

## Expected Output

- `crates/assay-harness/Cargo.toml` — updated with insta + tempfile dev-deps
- `crates/assay-harness/src/claude.rs` — `ClaudeConfig` struct + `generate_config()` + snapshot tests
- `crates/assay-harness/src/snapshots/` — insta snapshot files locking generated output
