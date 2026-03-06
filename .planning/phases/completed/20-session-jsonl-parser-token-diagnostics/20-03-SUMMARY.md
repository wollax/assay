# Plan 20-03 Summary: CLI Context Commands

## Completed

### Task 1: Add Context subcommand to CLI
- Added `Context` variant to `Command` enum with nested `ContextCommand` subcommand
- `ContextCommand` defines `Diagnose` and `List` sub-subcommands
- `Diagnose` accepts optional `session_id`, `--json`, and `--plain` flags
- `List` accepts `--limit` (default 20), `--all`, `--tokens`, `--json`, and `--plain` flags
- Wired dispatch in `run()` — Context commands do not require an .assay directory

### Task 2: Implement diagnose and list display functions
- `handle_context_diagnose()`: resolves session via `find_session_dir()` + `resolve_session()`, calls `diagnose()`, renders dashboard with Overview and Bloat Breakdown sections. Color-coded health status (green/yellow/red for healthy/warning/critical thresholds at 60%/85%).
- `handle_context_list()`: calls `list_sessions()`, renders table with Session ID, Size, Entries, Modified columns. Optional Tokens column with `--tokens`. Cyan-colored session IDs when color enabled.
- Both commands support `--json` (serializes via serde_json) and `--plain` (disables ANSI color and Unicode).
- Added utility functions: `format_size()`, `format_number()`, `format_relative_time()`, `colorize()`.

## Verification

- `cargo check -p assay-cli` passes
- `cargo build -p assay-cli` succeeds
- `assay context diagnose --help` shows all expected flags and examples
- `assay context list --help` shows all expected flags and examples
- Full test suite passes (322 tests, 0 failures)

## Files Modified

- `crates/assay-cli/src/main.rs` (+423 lines)

## Notes

- Discovery uses project slug mapping (`path_to_project_slug`). The slug format has a known inconsistency with Claude Code's actual directory naming (leading dash). This is a pre-existing issue in the discovery module from Plan 20-02, not introduced by this plan. Sessions will resolve correctly once the slug mapping is fixed.
