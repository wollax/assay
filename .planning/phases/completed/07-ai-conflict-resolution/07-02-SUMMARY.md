# Phase 07 Plan 02: AiConflictHandler — Core AI Resolution Engine Summary

**One-liner:** AiConflictHandler implements ConflictHandler with per-file LLM resolution via 3-way index stages; ResolutionMethod extended with AiAssisted/AiEdited; format_commit_message accepts ResolutionMethod

## What Was Done

### Task 1: GitOps::show_index_stage + ResolutionMethod variants + format_commit_message update

- Added `show_index_stage(work_dir, stage, file)` to `GitOps` trait — extracts file content at git index stages (:1: base, :2: ours, :3: theirs)
- Implemented in `GitCli` using existing `run_in()` helper with `git show :N:file`
- Added `ResolutionMethod::AiAssisted` and `ResolutionMethod::AiEdited` variants — serialize to `"ai-assisted"` and `"ai-edited"` via existing `rename_all = "kebab-case"`
- Changed `ConflictAction::Resolved` from unit variant to `Resolved(ResolutionMethod)` — carries resolution method through the merge loop
- Updated `format_commit_message` signature: `manually_resolved: bool` replaced with `resolution: ResolutionMethod`, supports all five resolution suffixes
- Updated both call sites in `merge_sessions()`: clean merge passes `ResolutionMethod::Clean`, conflict resolution extracts `method` from `Resolved(method)`
- Updated `sessions_resolved` filter from `== Manual` to `!= Clean && != Skipped` to include AI-resolved sessions
- Updated `MergeReport.sessions_resolved` doc comment from "manually resolved" to "resolved (manual or AI)"
- Updated `InteractiveConflictHandler` in smelt-cli to return `Resolved(ResolutionMethod::Manual)`
- Updated test `ResolveConflictHandler` to return `Resolved(ResolutionMethod::Manual)`

### Task 2: AiConflictHandler with per-file resolution

- Created `crates/smelt-core/src/merge/ai_handler.rs` with:
  - `AiConflictHandler<G: GitOps, P: AiProvider + 'static>` struct with `git`, `provider: Arc<P>`, `config: AiConfig`, `target_branch: String`
  - Constructor: `new(git, provider, config, target_branch)`
  - `ConflictHandler` implementation that:
    1. Selects model from config or `default_model_for_provider()`
    2. Gets commit subjects via `log_subjects` (non-fatal on failure)
    3. Per file: extracts 3-way context via `show_index_stage`, builds prompt, calls `provider.complete()`, writes resolved content via `tokio::fs::write`
    4. Returns `Resolved(ResolutionMethod::AiAssisted)` on success
    5. Returns `SmeltError::AiResolution` on any per-file failure
  - `default_model_for_provider()` helper: anthropic -> claude-sonnet-4, openai -> gpt-4o, ollama -> llama3.1, gemini/google -> gemini-2.0-flash, unknown -> claude-sonnet-4
- Registered `pub mod ai_handler` in merge/mod.rs with `pub use AiConflictHandler`
- Added `AiConflictHandler` to lib.rs re-exports
- 7 unit tests for `default_model_for_provider` covering all provider strings and fallbacks

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Send + Sync bounds on ConflictHandler impl**

- **Found during:** Task 2 (compilation)
- **Issue:** `ConflictHandler` requires `Send + Sync`, so the `impl` block for `AiConflictHandler` needed `G: GitOps + Send + Sync` bounds
- **Fix:** Added `Send + Sync` bounds to the impl block
- **Files modified:** `crates/smelt-core/src/merge/ai_handler.rs`

## Test Results

- 142 tests pass (all existing + 7 new ai_handler tests)
- `cargo clippy -p smelt-core -- -D warnings` clean
- `cargo clippy -p smelt-cli -- -D warnings` clean

## Decisions Made

- `provider` field is `Arc<P>` (not owned) to allow sharing with CLI-layer retry wrapper in Plan 03
- `task_description` is always `None` in the prompt — accepted v0.1.0 limitation; can be threaded via constructor or trait extension later
- AI handler is a single-attempt resolver — retry/feedback loop handled at CLI layer (Plan 03)
- `show_index_stage` errors on stages 2/3 are propagated as `SmeltError::AiResolution`; stage 1 (base) falls back to empty string for new files

## Files

### Created

- `crates/smelt-core/src/merge/ai_handler.rs` — AiConflictHandler implementing ConflictHandler

### Modified

- `crates/smelt-core/src/git/mod.rs` — show_index_stage on GitOps trait
- `crates/smelt-core/src/git/cli.rs` — show_index_stage impl on GitCli
- `crates/smelt-core/src/merge/types.rs` — AiAssisted/AiEdited variants, ConflictAction::Resolved(ResolutionMethod)
- `crates/smelt-core/src/merge/mod.rs` — format_commit_message signature, merge loop updates, ai_handler module, re-exports
- `crates/smelt-core/src/lib.rs` — AiConflictHandler re-export
- `crates/smelt-cli/src/commands/merge.rs` — InteractiveConflictHandler returns Resolved(Manual)

## Commits

- `6cec6c4`: feat(07-02): add show_index_stage, ResolutionMethod AI variants, and update format_commit_message
- `39ae711`: feat(07-02): add AiConflictHandler with per-file LLM resolution

## Duration

~5 minutes

## Next Phase Readiness

Plan 03 (CLI UX integration) can proceed. All core pieces are in place:
- `AiConflictHandler` implements `ConflictHandler` for single-attempt AI resolution
- `show_index_stage` extracts 3-way context from git index
- `ResolutionMethod::AiAssisted` and `AiEdited` for commit messages and reports
- `format_commit_message` supports AI resolution suffixes
- `Arc<P>` provider design enables sharing between AI handler and CLI retry wrapper
