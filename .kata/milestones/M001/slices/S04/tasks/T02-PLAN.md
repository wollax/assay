---
estimated_steps: 5
estimated_files: 2
---

# T02: Implement write_config() and build_cli_args() with file-write tests

**Slice:** S04 — Claude Code Adapter
**Milestone:** M001

## Description

Complete the adapter's public API with two functions: `write_config()` writes Claude Code config files to a directory (for worktree-based config), and `build_cli_args()` generates CLI arguments for `claude --print` mode. Both are consumed by S07's pipeline. File-write tests use `tempfile` for isolation. A final `just ready` pass confirms the full workspace is clean.

## Steps

1. Implement `write_config(config: &ClaudeConfig, dir: &Path) -> Result<(), std::io::Error>`:
   - Write `CLAUDE.md` at `dir/CLAUDE.md` (skip if `claude_md` is empty)
   - Write `.mcp.json` at `dir/.mcp.json`
   - Create `dir/.claude/` directory if it doesn't exist
   - Write `dir/.claude/settings.json` with `settings_json` content
   - All writes use `std::fs::write` with pretty-printed content
2. Implement `build_cli_args(config: &ClaudeConfig) -> Vec<String>`:
   - Always include `--print` and `--output-format json`
   - If `model` is `Some`, add `--model <model>`
   - Add `--system-prompt <claude_md>` if non-empty
   - Add `--mcp-config .mcp.json` (relative path — assumes CWD is worktree)
   - Add `--settings .claude/settings.json` (relative path)
   - Return the complete args vector
3. Add tempfile-based tests for `write_config()`:
   - Test: writes all files to tempdir, verify each file exists and content matches
   - Test: creates `.claude/` subdirectory automatically
   - Test: skips CLAUDE.md when `claude_md` is empty
4. Add tests for `build_cli_args()`:
   - Test: full config produces expected arg list (snapshot test)
   - Test: minimal config (no model) omits `--model` flag
   - Test: empty claude_md omits `--system-prompt` flag
5. Run `just ready` to verify full workspace passes fmt, clippy, test, deny.

## Must-Haves

- [ ] `write_config()` writes correct file layout: `CLAUDE.md`, `.mcp.json`, `.claude/settings.json`
- [ ] `write_config()` creates `.claude/` directory if missing
- [ ] `write_config()` returns `Result<(), std::io::Error>` with propagated I/O errors
- [ ] `build_cli_args()` returns correct flags including `--print`, `--output-format json`
- [ ] `build_cli_args()` conditionally includes `--model` and `--system-prompt`
- [ ] At least 3 file-write tests using tempdir
- [ ] At least 2 CLI args tests
- [ ] `just ready` passes

## Verification

- `cargo test -p assay-harness` — all tests pass (existing + T01 snapshots + T02 file/args tests)
- `just ready` — fmt, clippy, test, deny all green
- Manual check: snapshot files for CLI args output locked

## Observability Impact

- Signals added/changed: `write_config()` returns `io::Error` with OS-level context on failure — no custom error types needed for this pure file-write function
- How a future agent inspects this: Read test assertions to understand expected file layout; run `cargo test -p assay-harness -- claude --nocapture` to see details
- Failure state exposed: I/O errors propagated with path context via `?` operator

## Inputs

- `crates/assay-harness/src/claude.rs` — `ClaudeConfig` struct and `generate_config()` from T01
- T01 snapshot files — locked format references

## Expected Output

- `crates/assay-harness/src/claude.rs` — `write_config()` + `build_cli_args()` + all tests
- `crates/assay-harness/src/snapshots/` — additional snapshot files for CLI args
- `just ready` green across full workspace
