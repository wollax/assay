# S04: Infrastructure Hardening — Research

**Date:** 2026-03-21

## Summary

S04 is four independent sub-deliverables with no external dependencies: `smelt init`, per-job state isolation, `smelt list`, and the `.assay/` gitignore guard. All work is in `smelt-cli` (new commands + one mutation to `run.rs`) and `smelt-core/src/monitor.rs` (filename change). The codebase is clean (124 tests passing), compiles without warnings, and all S01–S03 types are in place.

The dominant complexity is the `JobMonitor` state-path migration. Every caller (`run.rs`, `watch.rs`, `status.rs`) needs updating to use `.smelt/runs/<job-name>/state.toml` instead of the flat `.smelt/run-state.toml`. The backward-compat requirement adds nuance: `smelt status` must still work against old flat state files. Plan this migration first — it touches the most existing code and the tests are the safety net.

The other three deliverables (`smelt init`, `smelt list`, gitignore guard) are net-new code with minimal coupling to the existing system.

## Recommendation

**Do the state-path migration first (T01), then the three independent features in any order (T02–T04).**

The state migration is the only piece that modifies existing tested code. Doing it first means the new commands (`init`, `list`) and the gitignore guard can be written against the already-updated `JobMonitor` API without needing to think about the legacy path.

For the state filename change: rename `run-state.toml` → `state.toml` inside `JobMonitor`. Add a `read_legacy()` helper that reads the old flat location. The backward-compat logic lives in a single function called by `smelt status` and `smelt watch` — not scattered.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Manifest validation for generated `smelt init` output | `JobManifest::validate()` already implements all rules | Re-run validation on the generated string in the unit test to guarantee it passes `--dry-run` |
| Directory iteration for `smelt list` | `std::fs::read_dir()` + filter | No external dep needed; already used in tests |
| Gitignore file manipulation | `std::fs::read_to_string` + `contains` + `std::fs::write` / append | Trivial host-side operation; no library needed |
| State serialization | `toml::to_string_pretty` + `serde` (already in use) | `RunState` already derives `Serialize`/`Deserialize` |

## Existing Code and Patterns

- `crates/smelt-core/src/monitor.rs` — `JobMonitor` writes `{state_dir}/run-state.toml`. `write()` and `read()` both hardcode the filename. Change both to `state.toml`, add `read_legacy(state_dir)` → reads `{state_dir}/run-state.toml` for backward compat. The `cleanup()` method also needs updating.
- `crates/smelt-cli/src/commands/run.rs` — `state_dir` is computed as `manifest.parent()/.smelt`. Change to `manifest.parent()/.smelt/runs/<manifest.job.name>` after manifest is loaded. The `JobMonitor::new()` call follows immediately. One-line path change.
- `crates/smelt-cli/src/commands/status.rs` — `StatusArgs` currently has only `--dir`. Add an optional `job_name: Option<String>` positional argument. When given, read from `.smelt/runs/<job_name>/state.toml`; when absent, fall back to `.smelt/run-state.toml` (legacy path). `format_pr_section()` is already correct.
- `crates/smelt-cli/src/commands/watch.rs` — `execute()` hardcodes `PathBuf::from(".").join(".smelt")` as state_dir and then reads `run-state.toml`. Already has `job_name: String` in `WatchArgs`. Change `execute()` to compute `.smelt/runs/<job_name>` before calling `run_watch()`. `run_watch()` inner signature is already correct.
- `crates/smelt-cli/src/main.rs` — Register `Init` and `List` in the `Commands` enum. Pattern is identical to `Status` and `Watch` — two lines each.
- `crates/smelt-cli/src/commands/mod.rs` — Add `pub mod init; pub mod list;`.
- `examples/job-manifest.toml` — The canonical reference for what a valid manifest looks like. `smelt init` should produce structurally identical content with placeholder values and inline comments.
- `examples/.smelt/run-state.toml` — Example of the old flat state file. The backward-compat path must be able to read this.

## Constraints

- **`run-state.toml` → `state.toml` is a breaking change for the state file name.** Any existing `.smelt/run-state.toml` file at the flat location won't be found by the updated `JobMonitor::read()`. The `read_legacy()` fallback in `smelt status` and `smelt watch` closes this gap.
- **`smelt status` backward compat**: The `StatusArgs.job_name` must be `Option<String>` (not required), so existing invocations `smelt status` without a job name continue to work by reading the legacy flat file.
- **`smelt init` idempotency guard**: `job-manifest.toml` must fail immediately with a clear error if the file already exists. Check with `Path::exists()` before any write.
- **`smelt init` output must pass validation**: The generated TOML must pass `JobManifest::validate()`. Required non-empty fields: `job.name`, `job.repo`, `job.base_ref`, `environment.runtime` (= `"docker"`), `environment.image`, `credentials.provider`, `credentials.model`, at least one `[[session]]` with non-empty `name`/`spec`/`harness` and `timeout > 0`, `merge.strategy`, `merge.target`. Use realistic placeholder strings, not empty ones.
- **Gitignore guard operates on the host repo**: Uses `manifest.job.repo` → `resolve_repo_path()`. Place the guard after Phase 3 (runtime check) and before Phase 4 (Docker connect) — purely host-side I/O, no container needed. The guard must be idempotent: check `contains(".assay/")` before appending.
- **`smelt list` reads only `.smelt/runs/`**: Does not read the legacy flat file. Legacy state is only surfaced by `smelt status` (no-arg path). `smelt list` is forward-looking.
- **No new workspace dependencies**: Everything needed (`std::fs`, `toml`, `serde`, `clap`) is already in scope.
- **Editions**: Workspace uses Rust 2024 (`edition = "2024"`); `if let` chains and RPITIT are stable.

## Common Pitfalls

- **`watch.rs` ignores `job_name` arg for state path**: `execute()` ignores `args.job_name` when building the state_dir — it just hardcodes `.smelt`. Fix: `let state_dir = PathBuf::from(".").join(".smelt").join("runs").join(&args.job_name)`. The `run_watch()` inner function doesn't need changes (it already takes `state_dir`).
- **State dir creation**: `JobMonitor::write()` already calls `fs::create_dir_all(&self.state_dir)`. When state_dir becomes `.smelt/runs/<job-name>`, this nested creation will work correctly — no extra `mkdir` needed.
- **`smelt list` with empty runs dir**: `read_dir(".smelt/runs/")` returns an error if the directory doesn't exist. Guard with `Path::exists()` and print "No past runs." (exit 0), not an error.
- **`smelt list` partial/corrupt state files**: A `state.toml` that fails to deserialize should be skipped with a warning rather than aborting the entire listing. Use `match JobMonitor::read(...)` not `unwrap()`.
- **Gitignore newline handling**: If `.gitignore` exists but doesn't end in `\n`, appending `.assay/\n` would smash it to the previous line. Always check the trailing newline: if `content.is_empty() || content.ends_with('\n')` then append `.assay/\n`; else append `\n.assay/\n`.
- **`smelt init` TOML comments**: `toml` crate's `to_string_pretty()` drops comments. The skeleton must be written as a raw string literal with embedded comments, not serialized from a struct.
- **Test coverage for state path resolution**: Add unit tests that verify the mapping `job_name → .smelt/runs/<name>/state.toml` using `TempDir`. Don't assume the path construction is trivial — a missing `create_dir_all` call or wrong join order will cause silent write failures.
- **`monitor.rs` `cleanup()` hardcodes `run-state.toml`**: Must be updated to `state.toml` alongside `write()` and `read()`. Easy to miss.

## Open Risks

- **`smelt status` without job-name on a new-style run**: After this slice, `smelt run` writes to `.smelt/runs/<name>/state.toml` but `smelt status` (no args) reads `.smelt/run-state.toml`. If the user runs `smelt run` then `smelt status` without a job name, they'll get "No running job." This is expected behavior post-migration — document it in the command's help text: "Pass job-name to read per-job state; omit to read legacy flat state for backward compat."
- **`smelt watch` without existing state file at new path**: If `smelt watch <job-name>` is called before `smelt run` completes Phase 9, the per-job directory may not exist yet. The existing "No state file" error message covers this but the path shown in the message will change — make sure error context is clear.
- **`examples/.smelt/run-state.toml`**: This legacy example file will no longer be written by `smelt run` after S04. It should be kept as a static example (document as legacy) rather than deleted, since it doubles as backward-compat test fixture.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust CLI (clap, tokio) | — | none found; codebase patterns are sufficient |

## Sources

- Codebase read: `monitor.rs`, `run.rs`, `status.rs`, `watch.rs`, `manifest.rs`, `main.rs` — existing patterns confirmed
- `examples/job-manifest.toml` — reference for `smelt init` skeleton content
- Decisions register D034, D044, D045 — state file pattern, `.assay/` idempotency constraints
