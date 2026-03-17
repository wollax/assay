# S04: Claude Code Adapter

**Goal:** Adapter translates a `HarnessProfile` into valid Claude Code configuration artifacts (CLAUDE.md, .mcp.json, settings.json, hooks) and CLI arguments.
**Demo:** Snapshot tests lock the generated output for each artifact. File-write tests confirm `write_config()` produces the expected directory layout. `build_cli_args()` returns a correct `Vec<String>` for `claude --print` invocation.

## Must-Haves

- `ClaudeConfig` struct holding generated content for all four artifacts (claude_md, mcp_json, settings_json, hooks_json)
- `generate_config(profile: &HarnessProfile) -> ClaudeConfig` — pure translation, no I/O
- `write_config(config: &ClaudeConfig, dir: &Path) -> Result<()>` — writes CLAUDE.md, .mcp.json, .claude/settings.json, hooks optionally embedded in settings
- `build_cli_args(config: &ClaudeConfig) -> Vec<String>` — builds CLI argument list for `claude --print`
- `HookEvent::PreTool` maps to `"PreToolUse"`, `PostTool` → `"PostToolUse"`, `Stop` → `"Stop"`
- MCP entries include `"type": "stdio"` field
- All three public functions are plain functions (not trait methods) — satisfies R009
- Snapshot tests via `insta` lock generated JSON and markdown content
- File-write tests via `tempfile` verify directory layout and file content

## Proof Level

- This slice proves: contract (generated output matches expected Claude Code formats, locked by snapshots)
- Real runtime required: no (Claude Code is not invoked — that's S07)
- Human/UAT required: no

## Verification

- `cargo test -p assay-harness` — all tests pass including new snapshot and file-write tests
- `just ready` — fmt, clippy, test, deny all pass
- Snapshot files in `crates/assay-harness/src/snapshots/` lock the exact generated output
- At least one test exercises the error path (empty profile produces valid but minimal output)

## Observability / Diagnostics

- Runtime signals: none — pure functions with no runtime state
- Inspection surfaces: snapshot files serve as human-readable reference for generated formats
- Failure visibility: `generate_config` is infallible (returns struct); `write_config` returns `Result` with `std::io::Error` context; snapshot mismatches produce inline diffs via `insta`
- Redaction constraints: none — no secrets in generated configs

## Integration Closure

- Upstream surfaces consumed: `assay-types::HarnessProfile`, `HookContract`, `HookEvent`, `SettingsOverride`, `PromptLayer`; `assay-harness::prompt::build_prompt()`; `assay-harness::settings::merge_settings()` (available but not called — `generate_config` translates the already-merged profile)
- New wiring introduced in this slice: `claude` module populated with `ClaudeConfig`, `generate_config()`, `write_config()`, `build_cli_args()` — all public API
- What remains before the milestone is truly usable end-to-end: S05 (worktree enhancements), S06 (manifest parsing), S07 (pipeline wiring that calls `generate_config` + `write_config` on a real worktree and launches claude)

## Tasks

- [x] **T01: Implement ClaudeConfig and generate_config() with snapshot tests** `est:30m`
  - Why: Core translation layer — converts HarnessProfile into Claude Code's four artifact formats. Snapshot tests lock the output format so drift is detectable.
  - Files: `crates/assay-harness/src/claude.rs`, `crates/assay-harness/Cargo.toml`
  - Do: Add `insta` + `tempfile` as dev-deps. Define `ClaudeConfig` struct. Implement `generate_config()` translating prompt layers → CLAUDE.md string, settings → Claude settings JSON, hooks → Claude hooks JSON (embedded in settings), MCP servers → .mcp.json string. Map `HookEvent::PreTool` → `"PreToolUse"`, `PostTool` → `"PostToolUse"`, `Stop` → `"Stop"`. Add insta snapshot tests for: realistic profile, minimal profile, profile with no hooks, profile with no MCP.
  - Verify: `cargo test -p assay-harness` passes with all new snapshot tests
  - Done when: `generate_config()` produces correct Claude Code JSON for all four artifacts, locked by snapshot files

- [x] **T02: Implement write_config() and build_cli_args() with file-write tests** `est:25m`
  - Why: Completes the adapter's public API — file output for worktree-based config and CLI args for `claude --print` mode. Both are needed by S07's pipeline.
  - Files: `crates/assay-harness/src/claude.rs`
  - Do: Implement `write_config(config, dir)` that writes CLAUDE.md at root, .mcp.json at root, .claude/settings.json (creating .claude/ dir). Implement `build_cli_args(config)` returning Vec<String> with `--system-prompt`, `--model`, `--permission-mode`, `--allowed-tools`, `--mcp-config` flags as appropriate. Add tempfile-based tests verifying file existence, content, and directory creation. Add snapshot test for CLI args output. Verify `just ready` passes.
  - Verify: `cargo test -p assay-harness` and `just ready` both pass
  - Done when: `write_config()` creates correct file layout in tempdir, `build_cli_args()` returns expected flags, `just ready` green

## Files Likely Touched

- `crates/assay-harness/Cargo.toml` — add insta + tempfile dev-deps
- `crates/assay-harness/src/claude.rs` — main implementation
- `crates/assay-harness/src/snapshots/` — insta snapshot files (auto-generated)
