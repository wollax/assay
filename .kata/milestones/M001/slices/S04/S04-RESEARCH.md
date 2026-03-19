# S04: Claude Code Adapter — Research

**Date:** 2026-03-16

## Summary

S04 generates four Claude Code configuration artifacts from a `HarnessProfile`: CLAUDE.md content, `.mcp.json`, settings overrides, and `hooks.json`. All formats are well-understood from the existing plugin at `plugins/claude-code/` which already ships working examples of each artifact. The adapter is a pure translation layer — no process launching, no I/O beyond file writes.

The real-world Claude Code config formats are confirmed by inspecting: `~/.claude/settings.json` (user-level hooks + permissions), `.claude/settings.json` (project-level), `plugins/claude-code/hooks/hooks.json` (hook contract format), `plugins/claude-code/.mcp.json` (MCP server config), and `plugins/claude-code/CLAUDE.md` (prompt content). Claude CLI flags (`--settings`, `--mcp-config`, `--system-prompt`, `--allowed-tools`, `--permission-mode`) provide additional config injection points beyond files.

The key design choice is whether to generate standalone config files that Claude reads from a worktree, or to generate CLI arguments for `claude --print`. The recommendation is **both**: a `ClaudeConfig` struct holds the generated content, `write_config()` writes files to a worktree directory, and a separate `build_cli_args()` builds the CLI argument list. S07 (pipeline) decides which path to use based on invocation mode.

## Recommendation

Implement a `ClaudeConfig` struct in `assay-harness/src/claude.rs` that holds generated content for all four artifacts. Provide three public functions:

1. `generate_config(profile: &HarnessProfile) -> ClaudeConfig` — pure translation from profile to Claude Code formats
2. `write_config(config: &ClaudeConfig, dir: &Path) -> Result<()>` — writes files to disk (CLAUDE.md, .mcp.json, settings.json, hooks/hooks.json)
3. `build_cli_args(config: &ClaudeConfig) -> Vec<String>` — builds CLI arguments for `claude --print` mode

Use snapshot tests (`insta`) for generated content verification. File-write tests use `tempdir`.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Prompt assembly | `assay-harness::prompt::build_prompt()` | Already implemented in S03, returns plain string for CLAUDE.md |
| Settings merge | `assay-harness::settings::merge_settings()` | Already implemented in S03, produces merged `SettingsOverride` |
| Snapshot testing | `insta` crate (already workspace dep) | Used by assay-types for schema snapshots, proven pattern |
| JSON serialization | `serde_json` (already in harness deps) | Generates .mcp.json, settings.json, hooks.json |
| Temp dirs for tests | `tempfile` crate (already workspace dep) | Used elsewhere for file-write test isolation |

## Existing Code and Patterns

- `plugins/claude-code/hooks/hooks.json` — **authoritative hooks format**: `{ "hooks": { "PostToolUse": [{ "matcher": "...", "hooks": [{ "type": "command", "command": "...", "timeout": N }] }], "Stop": [{ "hooks": [...] }] } }`. Event names are PascalCase: `PreToolUse`, `PostToolUse`, `Stop`, `PreCompact`, `SessionStart`.
- `plugins/claude-code/.mcp.json` — **authoritative MCP config**: `{ "mcpServers": { "name": { "type": "stdio", "command": "...", "args": [...] } } }`. Standard MCP JSON format.
- `plugins/claude-code/CLAUDE.md` — **reference prompt content**: free-form markdown, no special structure required. Claude reads it as context.
- `~/.claude/settings.json` — **user-level settings format**: `{ "permissions": { "allow": [...], "deny": [] }, "hooks": { "PreToolUse": [...], "PostToolUse": [...] }, "env": {...} }`. Hooks live inline in settings at user level, but in a separate `hooks.json` file at plugin level.
- `crates/assay-core/src/evaluator.rs` — **subprocess pattern**: builds `Command::new("claude")` with `-p`, `--output-format json`, `--json-schema`, `--system-prompt`, `--tools ""`, `--max-turns "1"`, `--model`, `--no-session-persistence`. Reuse this pattern for S07's launch; S04 only generates the config.
- `crates/assay-types/src/harness.rs` — **input types**: `HarnessProfile`, `PromptLayer`, `SettingsOverride`, `HookContract`, `HookEvent` (PreTool, PostTool, Stop).
- `crates/assay-harness/src/prompt.rs` — **build_prompt()**: returns assembled prompt string. S04 writes this as CLAUDE.md content.
- `crates/assay-harness/src/settings.rs` — **merge_settings()**: returns merged `SettingsOverride`. S04 translates to Claude's settings format.

## Constraints

- **Zero new workspace deps** — must use only `serde_json`, `serde`, `assay-types`, `assay-core` (already in Cargo.toml). Add `insta` and `tempfile` as dev-dependencies only.
- **deny_unknown_fields on all persisted types** — `ClaudeConfig` is not persisted (ephemeral generation output), so this doesn't apply to it, but any new types added to `assay-types` must have it.
- **HookEvent mapping is lossy** — our `HookEvent` has `PreTool`, `PostTool`, `Stop`. Claude Code supports `PreToolUse`, `PostToolUse`, `Stop`, `PreCompact`, `SessionStart`. The adapter maps our three; `PreCompact` and `SessionStart` are out of scope for M001.
- **No trait objects** (D001) — `generate_config` is a plain function, not a trait method. Callback-based control inversion (R009) applies to the orchestration layer in S07, not to config generation.
- **Settings have two delivery mechanisms** — file-based (`.claude/settings.json` in worktree) and CLI-based (`--settings <json>`, `--allowed-tools`, `--permission-mode`, `--model`). S04 should support both.
- **Hooks have two locations** — in `settings.json` directly (user/project level) or in a separate `hooks/hooks.json` (plugin level). For generated worktree config, putting hooks in `.claude/settings.json` is simpler — one less file.
- **Prompt has two delivery mechanisms** — `CLAUDE.md` file at worktree root or `--system-prompt`/`--append-system-prompt` CLI flags. File-based is better for multi-section prompts.

## Common Pitfalls

- **Hook event name mismatch** — Our `HookEvent::PreTool` must map to Claude's `"PreToolUse"` (not `"PreTool"`). Similarly `PostTool` → `"PostToolUse"`. `Stop` maps directly. Get this wrong and hooks silently don't fire.
- **Hook JSON nesting** — Claude hooks have two layers: event name → array of matcher groups → each with a `hooks` array of commands. Missing the outer `"hooks"` wrapper key is a common mistake. Format: `{ "hooks": { "EventName": [{ "matcher": "...", "hooks": [{ "type": "command", "command": "...", "timeout": N }] }] } }`.
- **MCP config requires "type" field** — `.mcp.json` entries need `"type": "stdio"` explicitly. Omitting it may cause Claude to fail silently.
- **Snapshot test brittleness** — JSON snapshots can break on field ordering changes. Use `serde_json::to_string_pretty` with deterministic key ordering (serde_json preserves insertion order by default, which is stable for structs).
- **Path separators in generated configs** — Commands in hooks and MCP configs must use forward slashes or be platform-aware. Since Assay targets local dev on macOS/Linux, this is low risk but worth noting.

## Open Risks

- **Claude Code hooks format stability** — The hooks format is not documented in a formal spec. It's reverse-engineered from working plugins and settings. A Claude Code update could change the format. Mitigation: snapshot tests catch format drift when manually verified against a real Claude installation.
- **Settings field completeness** — Our `SettingsOverride` has `model`, `permissions`, `tools`, `max_turns`. Claude's settings support many more fields (`env`, `attribution`, `statusLine`, `enabledPlugins`, etc.). The adapter only generates what we model — this is intentional but means some Claude features aren't configurable through Assay.
- **`--permission-mode` interaction** — When running `claude --print` with `--dangerously-skip-permissions` (likely needed for automated runs), the permissions in settings may be ignored. S07 needs to test this; S04 just generates the right settings regardless.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | — | Core language, no skill needed |
| Claude Code | — | Adapter target; no matching agent skill found in `<available_skills>` |
| serde/serde_json | — | Standard Rust serialization, no skill needed |
| insta | — | Snapshot testing crate, well-known, no skill needed |

No skill discovery via `npx skills find` was needed — this slice is pure Rust code generation with no external framework dependencies.

## Sources

- Claude Code hooks format confirmed from `plugins/claude-code/hooks/hooks.json` (project source)
- Claude Code settings format confirmed from `~/.claude/settings.json` (local installation)
- Claude Code MCP config format confirmed from `plugins/claude-code/.mcp.json` and `.mcp.json` (project source)
- Claude Code CLI flags confirmed from `claude --help` (local installation, current version)
- Evaluator subprocess pattern from `crates/assay-core/src/evaluator.rs` (project source)
- Hook event names confirmed: `PreToolUse`, `PostToolUse`, `Stop`, `PreCompact`, `SessionStart` (from working settings.json and hooks.json)
