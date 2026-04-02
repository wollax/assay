# Phase 07 Plan 03: CLI Integration — AI Conflict Resolution UX Summary

**Status:** Tasks 1-2 complete; Task 3 (manual verification) is a checkpoint awaiting human testing.

**One-liner:** AiInteractiveConflictHandler wired into CLI with colored diff display, Accept/Edit/Reject UX, retry-with-feedback, --no-ai flag, and fallback to InteractiveConflictHandler

## What Was Done

### Task 1: AiInteractiveConflictHandler with diff display and fallback chain

- Added `similar.workspace = true` to `crates/smelt-cli/Cargo.toml`
- Added `--no-ai` flag to `MergeCommands::Run` via `#[arg(long)]`
- Created `AiInteractiveConflictHandler<G, P>` struct in `merge.rs`:
  - Holds `AiConflictHandler`, `Arc<P>` provider, `AiConfig`, verbose flag
  - `ConflictHandler` impl: saves original conflicted content, attempts AI resolution, shows colored unified diff per file, prompts Accept/Edit/Reject
  - Accept: returns `Resolved(ResolutionMethod::AiAssisted)`
  - Edit: prompts user to edit files then press Enter, returns `Resolved(ResolutionMethod::AiEdited)`
  - Reject: prompts for feedback, retries via `build_retry_prompt` + `provider.complete()` up to `max_retries`, then falls back to `InteractiveConflictHandler`
  - API failure: prints error, restores original conflicted files, delegates to `InteractiveConflictHandler`
- Created `format_colored_diff()` using `similar::TextDiff` — red for removals, green for additions, cyan for hunk headers
- Created helper functions: `prompt_accept_edit_reject`, `prompt_feedback`, `prompt_continue_after_edit` (all use `spawn_blocking` + `dialoguer`)
- Created `MergeConflictHandler` enum dispatcher (AiInteractive(Box<...>) | Interactive) implementing `ConflictHandler` — avoids RPITIT-no-dyn limitation
- Created `build_conflict_handler()` factory function: checks `no_ai`, TTY, `AiConfig.enabled`, `GenAiProvider::new()` — falls back to Interactive on any failure
- Updated `execute_merge_run` signature to accept `no_ai: bool`; computes effective target branch for handler construction; passes `git.clone()` to handler
- Updated `main.rs` match arm to destructure and pass `no_ai`
- Added resolution method annotations in progress output (e.g., "Merged 'sess' (AI resolved)")

### Task 2: Integration tests for AI resolution chain

- Added 3 CLI integration tests in `crates/smelt-cli/tests/cli_merge.rs`:
  - `test_merge_no_ai_flag_clean_merge`: `--no-ai` with clean merge succeeds
  - `test_merge_ai_disabled_config_conflict_exits_error`: `[ai] enabled = false` in config, conflict exits with error (non-TTY path)
  - `test_merge_no_ai_conflict_exits_error`: `--no-ai` with conflict exits with error (non-TTY path)
- Added 4 unit tests in `crates/smelt-core/src/merge/mod.rs`:
  - `test_format_commit_message_ai_assisted`: verifies `[resolved: ai-assisted]` suffix
  - `test_format_commit_message_ai_edited`: verifies `[resolved: ai-edited]` suffix
  - `test_format_commit_message_clean_no_suffix`: verifies clean has no `[resolved:` suffix
  - `test_conflict_action_resolved_carries_method`: verifies `Resolved(AiAssisted)` and `Resolved(AiEdited)` pattern matching
- Added 2 unit tests in `merge.rs` module:
  - `test_format_colored_diff_produces_output`: verifies diff output contains filename
  - `test_format_colored_diff_no_changes`: verifies no output for identical content

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] large_enum_variant clippy warning**

- **Found during:** Task 1 (clippy check)
- **Issue:** `MergeConflictHandler` enum had large size difference between `AiInteractive` (~304 bytes) and `Interactive` (~1 byte)
- **Fix:** Boxed the `AiInteractive` variant: `AiInteractive(Box<AiInteractiveConflictHandler<GitCli, GenAiProvider>>)`
- **Files modified:** `crates/smelt-cli/src/commands/merge.rs`

**2. [Rule 1 - Bug] Dead code warning for target_branch field**

- **Found during:** Task 1 (cargo check)
- **Issue:** `target_branch` field on `AiInteractiveConflictHandler` was unused — the retry logic doesn't need it (it's already captured inside `ai_handler`)
- **Fix:** Removed the field
- **Files modified:** `crates/smelt-cli/src/commands/merge.rs`

## Test Results

- 186 tests pass (146 smelt-core + 10 cli_merge + 16 cli_session + 6 cli_worktree + 8 cli_init)
- `cargo clippy --workspace -- -D warnings` clean

## Decisions Made

- Retry logic calls `build_resolution_prompt` + `build_retry_prompt` + `provider.complete()` directly (bypasses `AiConflictHandler::handle_conflict`) because retry needs user feedback in the prompt
- `MergeConflictHandler` enum uses `Box<AiInteractiveConflictHandler>` to satisfy clippy's `large_enum_variant` lint
- `build_conflict_handler` factory defaults to AI-enabled when no `.smelt/config.toml` exists (matches `AiConfig::default()`)
- Non-TTY mode always falls back to `InteractiveConflictHandler` (which propagates MergeConflict error — CI safety)

## Files

### Modified

- `crates/smelt-cli/Cargo.toml` — added `similar.workspace = true`
- `crates/smelt-cli/src/commands/merge.rs` — AiInteractiveConflictHandler, MergeConflictHandler enum, build_conflict_handler, --no-ai flag, format_colored_diff, prompt helpers
- `crates/smelt-cli/src/main.rs` — pass no_ai through to execute_merge_run
- `crates/smelt-cli/tests/cli_merge.rs` — 3 new integration tests
- `crates/smelt-core/src/merge/mod.rs` — 4 new unit tests

## Commits

- `922c2a8`: feat(07-03): wire AI conflict resolution into CLI with interactive UX
- `8b3e27f`: test(07-03): add integration tests for AI resolution chain

## Duration

~10 minutes

## Checkpoint

Task 3 requires manual verification with a real API key. See checkpoint details in the plan executor's return message.
