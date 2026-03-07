# Plan 22-05 Summary: CLI Integration

## Status: COMPLETE

## What Was Done

### Task 1 + 2: Prune Subcommand and Handler

Added `Prune` variant to `ContextCommand` enum with all CLI flags:
- `session_id` (required positional)
- `--tier gentle|standard|aggressive` (default: standard)
- `--strategy <name>` (conflicts with --tier)
- `--execute` (default is dry-run)
- `--restore` (list backups, conflicts with tier/strategy/execute)
- `--json` (output PruneReport as JSON)
- `--plain` (no color, no Unicode)

Implemented `handle_context_prune` handler that:
- Resolves session path using `find_session_dir` + `resolve_session`
- In restore mode: lists backups via `backup::list_backups`, outputs as numbered list or JSON
- In prune mode: parses tier/strategy, calls `prune_session`, formats output via `format_dry_run_report` or JSON serialization
- Invalid tier/strategy names produce clear error messages with valid options listed

## Verification

- `just ready` passes (fmt-check + lint + test + deny)
- 275 tests pass
- Clippy clean, fmt clean

## Files Modified

- `crates/assay-cli/src/main.rs` — added Prune variant, match arm, and handler function

## Commits

- `b90052f` feat(22-05): wire pruning engine into CLI as `assay context prune`
