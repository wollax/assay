---
estimated_steps: 7
estimated_files: 5
---

# T03: Add `smelt list` command and `.assay/` gitignore guard

**Slice:** S04 — Infrastructure Hardening
**Milestone:** M003

## Description

Two independent net-new additions with minimal coupling to existing code:

**`smelt list`** (R006): enumerates all per-job runs in `.smelt/runs/`, printing a one-line summary per job. It is strictly forward-looking — reads only `state.toml` files created by T01's new path convention, not the legacy flat file.

**`.assay/` gitignore guard** (R008): a small host-side helper in `run.rs` that ensures `.assay/` appears in the repo's `.gitignore` before any container work begins. Without this, every `smelt run` leaves ephemeral Assay state showing in `git status`.

Both deliverables are testable without Docker or a live GitHub token.

## Steps

1. **Create `crates/smelt-cli/src/commands/list.rs`.** Define `#[derive(Debug, Args)] pub struct ListArgs { #[arg(long, default_value = ".")] pub dir: PathBuf }`. Define `pub async fn execute(args: &ListArgs) -> anyhow::Result<i32>`.

2. **Implement `smelt list` logic.** In `execute()`:
   - Compute `runs_dir = args.dir.join(".smelt").join("runs")`.
   - If `runs_dir` doesn't exist, print `No past runs.` to stdout and return `Ok(0)`.
   - Call `std::fs::read_dir(&runs_dir)?`.
   - For each entry, compute `entry_path = entry.path()` and `state_path = entry_path.join("state.toml")`.
   - Skip entries where `state_path` doesn't exist (e.g. empty directories).
   - Call `JobMonitor::read(&entry_path)`. On `Err(e)`, print a warning to stderr: `[WARN] skipping {}: {}` and continue.
   - On `Ok(state)`, format a one-line row: print `  {job_name:<20}  {phase:<15}  {elapsed_secs}s  {pr_url}` where `pr_url` is the value or `-`. Use a simple table layout with fixed column widths. Print a header line before the first row.
   - Return `Ok(0)`.

3. **Register `list` in `mod.rs` and `main.rs`.** Add `pub mod list;` to `commands/mod.rs`. Add `/// List past runs List(commands::list::ListArgs),` to `Commands` in `main.rs`. Add handler `Commands::List(ref args) => commands::list::execute(args).await,` to the match arm.

4. **Add `ensure_gitignore_assay()` helper in `run.rs`.** Write `fn ensure_gitignore_assay(repo_path: &std::path::Path) -> anyhow::Result<()>` that:
   - Resolves `gitignore_path = repo_path.join(".gitignore")`.
   - If the file exists: read content; if it contains `.assay/`, return `Ok(())` immediately (idempotent).
   - If the file exists but lacks `.assay/`: check if content ends with `\n`; if yes, append `.assay/\n`; if no, append `\n.assay/\n`. Use `std::fs::OpenOptions::new().append(true).open()` or read + write the full content.
   - If the file does not exist: create it with content `.assay/\n`.
   - Return `Ok(())`.
   Call `ensure_gitignore_assay(&repo_path)?;` in `execute_run()` (specifically in `run_with_cancellation()`) after Phase 3 runtime type check and before Phase 5 container provision. The `repo_path` variable can be resolved using `manifest::resolve_repo_path(&manifest.job.repo)` — the same call already present in Phase 7's collect block; factor it out or call it again (it's cheap and idempotent).

5. **Add unit tests for `smelt list` in `list.rs` `#[cfg(test)]`.** Four tests using `TempDir`:
   - `test_list_missing_runs_dir`: call `execute()` with a dir that has no `.smelt/runs`; assert `Ok(0)` (no error, "No past runs." printed).
   - `test_list_with_state_files`: create `.smelt/runs/job-a/state.toml` and `.smelt/runs/job-b/state.toml` using `toml::to_string_pretty()` on manually-constructed `RunState` values; call `execute()`; assert `Ok(0)`.
   - `test_list_skips_corrupt_state`: create `.smelt/runs/bad/state.toml` with invalid TOML content; call `execute()`; assert `Ok(0)` (skips, doesn't abort).
   - `test_list_skips_entry_without_state_toml`: create `.smelt/runs/empty-dir/` with no `state.toml`; call `execute()`; assert `Ok(0)` (no panic, no error).

6. **Add unit tests for `ensure_gitignore_assay()` in `run.rs` `#[cfg(test)]`.** Three tests using `TempDir`:
   - `test_ensure_gitignore_creates`: call with a dir that has no `.gitignore`; assert `.gitignore` is created and contains `.assay/`.
   - `test_ensure_gitignore_appends`: create `.gitignore` with `target/\n` (ending with newline); call; assert file now contains `target/` and `.assay/`.
   - `test_ensure_gitignore_trailing_newline`: create `.gitignore` with `target/` (no trailing newline); call; assert file contains `.assay/` and the two entries are on separate lines (i.e. no `target/.assay/` mash).
   - `test_ensure_gitignore_idempotent`: create `.gitignore` already containing `.assay/`; call twice; assert `.assay/` appears exactly once.

7. **Compile and test.** Run `cargo test -p smelt-cli`. Confirm all tests pass.

## Must-Haves

- [ ] `smelt list` prints a header and one row per job found in `.smelt/runs/`
- [ ] `smelt list` prints "No past runs." and exits 0 when `.smelt/runs/` doesn't exist
- [ ] `smelt list` warns to stderr and skips jobs whose `state.toml` is corrupt or missing
- [ ] `ensure_gitignore_assay()` creates `.gitignore` with `.assay/` when no `.gitignore` exists
- [ ] `ensure_gitignore_assay()` appends `.assay/` to an existing `.gitignore` without mangling adjacent lines
- [ ] `ensure_gitignore_assay()` is idempotent: calling it twice does not duplicate `.assay/`
- [ ] `ensure_gitignore_assay()` is called from `run_with_cancellation()` after Phase 3 runtime check
- [ ] `cargo test -p smelt-cli` — all 7 new tests (4 list + 3+ gitignore) pass

## Verification

- `cargo test -p smelt-cli` passes with no failures
- `test_list_skips_corrupt_state`: verify the corrupt entry is skipped (no panic, exit 0) and a `[WARN]` line goes to stderr
- `test_ensure_gitignore_trailing_newline`: read resulting `.gitignore` content and assert it does NOT contain `target/.assay/` (the no-newline corruption case)
- `test_ensure_gitignore_idempotent`: count occurrences of `.assay/` in the resulting file and assert exactly 1

## Observability Impact

- Signals added/changed: `smelt list` provides the first aggregate view of all runs from a directory — a future agent can call it to determine which jobs have completed, their phases, and PR URLs without reading individual state files
- How a future agent inspects this: `smelt list` is the correct tool for "what runs exist in this repo?"; `smelt status <job-name>` is for "what is the detailed state of one specific job?"
- Failure state exposed: corrupt `state.toml` files warn to stderr with the path — a future agent can locate and remove or re-run the specific failing job

## Inputs

- T01 must be complete: `JobMonitor::read(&state_dir)` reads `state.toml`; per-job state directories exist
- T02 should be complete (registration pattern for `smelt list` in `main.rs` follows `init`)
- `crates/smelt-core/src/manifest.rs` — `resolve_repo_path()` needed for `ensure_gitignore_assay()` in `run.rs`
- Research: trailing newline handling for gitignore append (pitfalls section); `smelt list` does not read legacy flat file (constraints section)

## Expected Output

- `crates/smelt-cli/src/commands/list.rs` — new file: `ListArgs`, `execute()`, 4 unit tests
- `crates/smelt-cli/src/commands/mod.rs` — `pub mod list;` added
- `crates/smelt-cli/src/main.rs` — `List` variant and match arm added
- `crates/smelt-cli/src/commands/run.rs` — `ensure_gitignore_assay()` helper + call site after Phase 3 + 4 unit tests
