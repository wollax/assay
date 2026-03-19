---
id: T02
parent: S04
milestone: M001
provides:
  - "write_config() — writes ClaudeConfig files to a directory (CLAUDE.md, .mcp.json, .claude/settings.json)"
  - "build_cli_args() — generates Vec<String> CLI arguments for claude --print invocation"
key_files:
  - crates/assay-harness/src/claude.rs
  - crates/assay-harness/src/snapshots/assay_harness__claude__tests__cli_args_full.snap
key_decisions: []
patterns_established:
  - "write_config skips CLAUDE.md when claude_md is empty — conditional file writes based on content presence"
  - "build_cli_args uses relative paths (.mcp.json, .claude/settings.json) — assumes CWD is worktree root"
observability_surfaces:
  - "write_config returns std::io::Error with OS-level path context on failure"
  - "CLI args snapshot locked at cli_args_full.snap — drift detectable via cargo insta test"
duration: 8m
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T02: Implement write_config() and build_cli_args() with file-write tests

**Added `write_config()` file writer and `build_cli_args()` CLI arg builder with 6 new tests (3 file-write, 3 CLI args) plus 1 new snapshot.**

## What Happened

Implemented two public functions completing the Claude Code adapter API:

1. `write_config(config: &ClaudeConfig, dir: &Path) -> io::Result<()>` — writes CLAUDE.md (skipped when empty), .mcp.json, and .claude/settings.json to the given directory, creating the .claude/ subdirectory via `create_dir_all`.

2. `build_cli_args(config: &ClaudeConfig) -> Vec<String>` — always includes `--print`, `--output-format json`, `--mcp-config .mcp.json`, `--settings .claude/settings.json`. Conditionally adds `--model` and `--system-prompt` based on config content.

Added 6 tests: 3 tempfile-based tests for write_config (full write, directory creation, empty claude_md skip) and 3 tests for build_cli_args (full snapshot, no-model, empty-claude_md).

## Verification

- `cargo insta test -p assay-harness --accept` — 27/27 tests pass, 1 new snapshot accepted
- `cargo test -p assay-harness` — 27 passed, 0 failed
- `just ready` — fmt, clippy, test, deny all green
- Snapshot file `cli_args_full.snap` locked with expected flag ordering

### Slice-level verification status:
- ✅ `cargo test -p assay-harness` — all 27 tests pass (T01 snapshots + T02 file/args tests)
- ✅ `just ready` — fmt, clippy, test, deny all pass
- ✅ Snapshot files lock generated output (11 from T01 + 1 from T02)
- ✅ Minimal/empty profile test exercises valid-but-minimal output path

## Diagnostics

- Run `cargo test -p assay-harness -- claude --nocapture` to see test output
- Read snapshot files in `crates/assay-harness/src/snapshots/` for locked formats
- Run `cargo insta test -p assay-harness` to check for snapshot drift

## Deviations

- Task plan mentioned `--permission-mode` and `--allowed-tools` flags in build_cli_args, but these are not standard Claude Code CLI flags. Used `--system-prompt`, `--model`, `--mcp-config`, and `--settings` which match Claude Code's actual CLI interface. The step-level description (authoritative) was followed correctly.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-harness/src/claude.rs` — added `write_config()`, `build_cli_args()`, and 6 new tests
- `crates/assay-harness/src/snapshots/assay_harness__claude__tests__cli_args_full.snap` — new snapshot for CLI args
