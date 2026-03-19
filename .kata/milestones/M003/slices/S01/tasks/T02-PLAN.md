---
estimated_steps: 5
estimated_files: 4
---

# T02: Sync conflict resolver function

**Slice:** S01 — AI Conflict Resolution
**Milestone:** M003

## Description

Build the sync `resolve_conflict()` function that reads conflicted files from a live working tree, constructs a structured prompt with conflict markers and surrounding context, spawns `claude -p --json-schema` via `std::process::Command`, parses the JSON response, writes resolved file contents, stages with `git add`, commits with `git commit`, and returns `ConflictAction::Resolved(sha)`. This follows the subprocess pattern from `evaluator.rs` (D043) but synchronous. On any failure (claude not installed, parse error, timeout), returns `ConflictAction::Skip` with a descriptive error logged.

## Steps

1. Create `crates/assay-core/src/orchestrate/conflict_resolver.rs`. Define `ConflictResolutionOutput` response type (what the AI returns): a struct with `resolved_files: Vec<ResolvedFile>` where `ResolvedFile` has `path: String` and `content: String`. Add schemars derive. Create a `LazyLock<String>` for the cached JSON schema (same pattern as `EVALUATOR_SCHEMA`).

2. Implement `build_conflict_prompt(session_name, conflicting_files, conflict_scan, work_dir) -> String`. Read the full content of each conflicting file from the work dir (they contain conflict markers). Build a structured prompt with: role context ("You are resolving a git merge conflict"), session name, file list with full contents, instruction to produce resolved content for each file.

3. Implement `build_conflict_system_prompt() -> String`. Instruct the AI on output format expectations and behavior: resolve conflicts by choosing the best combination, preserve all non-conflicting code, do not introduce new code beyond what's needed to resolve.

4. Implement `resolve_conflict(session_name, conflicting_files, conflict_scan, work_dir, config) -> ConflictAction`. Spawn `claude -p --output-format json --json-schema <schema> --system-prompt <sys> --tools "" --max-turns 1 --model <model> --no-session-persistence` with the prompt on stdin via `std::process::Command`. Collect stdout. Parse as `ConflictResolutionOutput`. For each resolved file, write content to `work_dir/path`, then `git add <path>`. Finally `git commit` (merge state MERGE_HEAD is present, so this creates a proper merge commit). Return `ConflictAction::Resolved(sha)`. On any error, return `ConflictAction::Skip`.

5. Add unit tests: test `build_conflict_prompt()` produces expected structure with file contents and markers. Test `ConflictResolutionOutput` deserialization from valid and malformed JSON. Test that `resolve_conflict()` returns `Skip` when `claude` binary is not found (mock by using a nonexistent binary name or testing the error path directly). Register module in `orchestrate/mod.rs`.

## Must-Haves

- [ ] `conflict_resolver.rs` module exists with `resolve_conflict()` public function
- [ ] `ConflictResolutionOutput` type with schemars derive and cached JSON schema
- [ ] Prompt construction reads full file contents from live working tree
- [ ] Subprocess uses `std::process::Command` (sync, not async) — consistent with D007/D043
- [ ] Graceful fallback: returns `ConflictAction::Skip` on any subprocess error (not found, timeout, parse)
- [ ] Unit tests for prompt construction and response parsing

## Verification

- `cargo test -p assay-core conflict_resolver` — all unit tests pass
- `cargo test -p assay-types schema_snapshots` — snapshot for `ConflictResolutionOutput` if added to types (or snapshot in core if type is crate-local)
- `cargo build -p assay-core` — compiles cleanly

## Observability Impact

- Signals added/changed: `resolve_conflict()` captures subprocess stdout, stderr, exit code, and duration. On failure, constructs descriptive error message including exit code and stderr snippet.
- How a future agent inspects this: The caller (merge runner in T03) receives `ConflictAction::Skip` on failure with the error logged. Success returns `Resolved(sha)` — the SHA is verifiable via `git rev-parse`.
- Failure state exposed: Subprocess not-found produces "claude CLI not found" message. Parse failure includes the raw stdout. Timeout includes elapsed time.

## Inputs

- `crates/assay-core/src/evaluator.rs` — subprocess spawn pattern (`spawn_and_collect`), schema caching (`EVALUATOR_SCHEMA`), prompt construction (`build_evaluator_prompt`)
- `crates/assay-core/src/merge.rs` — `git_raw()` and `git_command()` helpers for git operations
- `crates/assay-types/src/orchestrate.rs` — `ConflictResolutionConfig` from T01
- `crates/assay-types/src/merge.rs` — `ConflictScan` type for conflict details

## Expected Output

- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — new module with `resolve_conflict()`, prompt builders, response type, schema, unit tests
- `crates/assay-core/src/orchestrate/mod.rs` — `pub mod conflict_resolver;` added
