---
phase: 21-team-state-checkpointing
plan: 02
status: complete
started: 2026-03-06
completed: 2026-03-06
commits:
  - c16d6e8 "feat(21-02): add checkpoint save|show|list CLI subcommands"
  - 57cd404 "style(21-02): auto-format checkpoint modules and CLI"
files_modified:
  - crates/assay-cli/src/main.rs
  - crates/assay-core/src/checkpoint/config.rs (fmt only)
  - crates/assay-core/src/checkpoint/extractor.rs (fmt only)
  - crates/assay-core/src/checkpoint/mod.rs (fmt only)
  - crates/assay-core/src/checkpoint/persistence.rs (fmt only)
---

## Summary

Added `assay checkpoint save|show|list` CLI subcommands exposing the checkpoint core logic from plan 01.

## Implementation Details

### CheckpointCommand enum
Three subcommands following existing CLI patterns (ContextCommand as closest parallel):
- **Save**: `--trigger` (default "manual"), `--session` (optional), `--json`
- **Show**: `--json`
- **List**: `--limit` (default 10)

### Handler functions
- `handle_checkpoint_save`: Calls `extract_team_state` + `save_checkpoint`, prints summary or JSON
- `handle_checkpoint_show`: Reads `latest.md` for markdown display, or parses + serializes for `--json`
- `handle_checkpoint_list`: Prints a formatted table with Timestamp, Trigger, Agents, Tasks columns

### Error handling
All three handlers check for `.assay/` directory existence upfront with a clear "Run `assay init` first" message. Missing checkpoints produce "Run `assay checkpoint save` to create one." Underlying `AssayError` variants propagate through `anyhow`.

## Deviations

### Formatting fixes (auto-fix)
Plan 01 left formatting issues in checkpoint core modules. Running `just fmt` fixed these alongside the new CLI code. Committed separately to keep concerns distinct.

## Verification

- `assay checkpoint --help` shows all three subcommands with examples
- `assay checkpoint save --help` shows `--trigger`, `--session`, `--json` flags
- `assay checkpoint show --help` shows `--json` flag
- `assay checkpoint list --help` shows `--limit` flag
- `just ready` passes (fmt-check + clippy + 349 tests + cargo-deny)
