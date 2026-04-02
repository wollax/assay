# S04: Infrastructure Hardening

**Goal:** Ship four independent quality-of-life deliverables: per-job state isolation (`.smelt/runs/<job-name>/state.toml`), `smelt init`, `smelt list`, and the `.assay/` gitignore guard — with backward-compatible `smelt status` throughout.
**Demo:** `smelt init` writes a manifest that passes `--dry-run`; `smelt run` writes to `.smelt/runs/<name>/state.toml`; `smelt status <job-name>` reads from the per-job path while `smelt status` (no args) still reads the legacy flat file; `smelt list` enumerates all past runs; a freshly-run job finds `.assay/` in `.gitignore`.

## Must-Haves

- `JobMonitor::write()` and `JobMonitor::read()` use `state.toml` (not `run-state.toml`)
- `JobMonitor::read_legacy(base_dir)` reads `.smelt/run-state.toml` for backward compat
- `smelt run manifest.toml` writes state to `.smelt/runs/<job.name>/state.toml`
- `smelt watch <job-name>` reads from `.smelt/runs/<job-name>/state.toml` (not the flat path)
- `smelt status <job-name>` reads from `.smelt/runs/<job-name>/state.toml`; `smelt status` (no args) falls back to `.smelt/run-state.toml`
- `smelt init` creates `./job-manifest.toml` with placeholder content that passes `JobManifest::validate()`; exits 1 with a clear error if the file already exists
- `smelt list` reads `.smelt/runs/`, prints job name / phase / started_at / PR URL for each run; handles missing dir (prints "No past runs.") and corrupt state files (warns, skips)
- `.assay/` gitignore guard: during `smelt run`, after Phase 3 and before Phase 4, appends `.assay/` to `.gitignore` if not already present; creates `.gitignore` if absent; operation is idempotent

## Proof Level

- This slice proves: operational
- Real runtime required: no (all verification via `cargo test`; gitignore guard exercised by unit test with `TempDir`)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-core` — all tests pass; `test_write_and_read_roundtrip` reads `state.toml`, not `run-state.toml`
- `cargo test -p smelt-cli` — all 124+ pre-existing tests pass; new tests for init, list, state path resolution, and gitignore guard also pass
- `smelt init` in a temp directory → file `job-manifest.toml` created → `smelt run job-manifest.toml --dry-run` exits 0
- `smelt init` again in same dir → exits 1 with "already exists" message
- Unit test proves: `JobMonitor::read_legacy()` reads `.smelt/run-state.toml`; `JobMonitor::read()` reads `state.toml`; state path for a job named "my-job" resolves to `.smelt/runs/my-job/state.toml`
- Unit tests prove gitignore guard creates/appends/is-idempotent

## Observability / Diagnostics

- Runtime signals: `smelt watch <job-name>` will now fail loudly ("No state file") if called before `smelt run` writes the per-job directory — the error message must include the expected path to aid diagnosis
- Inspection surfaces: `smelt status <job-name>` reads from the per-job path; `smelt list` shows all runs in `.smelt/runs/`
- Failure visibility: `smelt list` warns to stderr and skips corrupt `state.toml` files rather than aborting
- Redaction constraints: none — no secrets are written to state files; `forge_token_env` stores env var name only

## Integration Closure

- Upstream surfaces consumed: `JobMonitor`, `RunState` (smelt-core), all three existing CLI commands
- New wiring introduced in this slice: `smelt init` and `smelt list` registered in `main.rs`; gitignore guard called from `run.rs` Phase 3.5; `status.rs` gains optional positional `job_name` arg; `watch.rs` `execute()` uses per-job path
- What remains before the milestone is truly usable end-to-end: S05 (library API polish) and S06 (integration proof with real Docker + GitHub token)

## Tasks

- [x] **T01: Migrate JobMonitor state path from flat file to per-job directories** `est:45m`
  - Why: R006 — concurrent runs must not clobber each other's state; this is the prerequisite for T02/T03 and the only task that modifies existing tested code
  - Files: `crates/smelt-core/src/monitor.rs`, `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/src/commands/watch.rs`, `crates/smelt-cli/src/commands/status.rs`
  - Do: In `monitor.rs`, rename `run-state.toml` → `state.toml` in `write()`, `read()`, `cleanup()`; add `pub fn read_legacy(base_dir: &Path) -> Result<RunState>` that reads `{base_dir}/run-state.toml`. In `run.rs`, change state_dir computation from `.smelt` to `.smelt/runs/<manifest.job.name>` (one-line change after manifest load). In `watch.rs`, fix `execute()` to compute `state_dir = ".smelt/runs/<args.job_name>"`; update `persist_run_state()` to write `state.toml` not `run-state.toml`. In `status.rs`, add optional positional `job_name: Option<String>` to `StatusArgs` (clap trailing_var_arg not needed — just `#[arg]`); in `execute()`, when `job_name` is Some, build path as `.smelt/runs/<job_name>` and call `JobMonitor::read()`; when None, call `JobMonitor::read_legacy(&state_dir_base)`; update help text to note the backward-compat behavior. Update `status.rs` inline test helper `write_state()` to write to the per-job path. Update `monitor.rs` unit test `test_run_state_toml_serialization` to assert on `state.toml`, not `run-state.toml`. Add unit tests: `test_state_path_resolution` (asserts `.smelt/runs/my-job` path construction), `test_read_legacy_reads_flat_file`, `test_cleanup_uses_state_toml`.
  - Verify: `cargo test -p smelt-core && cargo test -p smelt-cli` both pass with 0 failures
  - Done when: All 124+ pre-existing tests pass; `JobMonitor::read()` opens `state.toml`; `JobMonitor::read_legacy()` opens `run-state.toml`; `watch.rs` `execute()` uses the per-job path from `args.job_name`

- [x] **T02: Add `smelt init` command** `est:30m`
  - Why: R007 — new users need a valid starting manifest without memorizing the schema
  - Files: `crates/smelt-cli/src/commands/init.rs` (new), `crates/smelt-cli/src/commands/mod.rs`, `crates/smelt-cli/src/main.rs`
  - Do: Write `init.rs` with `InitArgs` (no fields) and `execute()`: check `Path::new("job-manifest.toml").exists()` first, return `Ok(1)` with a clear error if so; write a raw string literal skeleton with inline `#`-comments (see constraint: `toml::to_string_pretty` strips comments, so the skeleton must be a `const` or `&str` literal, not serialized); register `Init(commands::init::InitArgs)` in `main.rs` Commands enum and match arm; add `pub mod init;` to `mod.rs`. Unit tests in `init.rs`: happy path (file doesn't exist → creates it → `JobManifest::load()` + `JobManifest::validate()` succeed on the created file), idempotency guard (file exists → returns 1 + error message), validate skeleton passes validation using `TempDir`.
  - Verify: `cargo test -p smelt-cli` passes; `smelt init` creates a file that passes `smelt run job-manifest.toml --dry-run`
  - Done when: `smelt init` creates `job-manifest.toml` that passes `JobManifest::validate()`; second invocation exits 1 with "already exists" message; `smelt --help` shows `init` subcommand

- [x] **T03: Add `smelt list` command and `.assay/` gitignore guard** `est:30m`
  - Why: R006 (list past runs from per-job dirs) + R008 (protect host repo from accidental `.assay/` commits)
  - Files: `crates/smelt-cli/src/commands/list.rs` (new), `crates/smelt-cli/src/commands/mod.rs`, `crates/smelt-cli/src/main.rs`, `crates/smelt-cli/src/commands/run.rs`
  - Do: Write `list.rs` with `ListArgs { #[arg(long, default_value=".")] dir: PathBuf }` and `execute()`: check if `.smelt/runs` exists (if not, print "No past runs." and return Ok(0)); call `std::fs::read_dir(".smelt/runs")`; for each entry, try to read `state.toml` with `JobMonitor::read()`; on error, print warning to stderr and skip; on success, print "  <job_name>  <phase>  <elapsed>  <pr_url or ->". Register `List(commands::list::ListArgs)` in `main.rs` and `mod.rs`. In `run.rs`, add `fn ensure_gitignore_assay(repo_path: &Path) -> anyhow::Result<()>`: resolves `<repo_path>/.gitignore`; if it exists, reads content and returns early if it contains `.assay/`; if it exists but lacks `.assay/`, appends `\n.assay/\n` or `.assay/\n` depending on whether content ends with `\n`; if it doesn't exist, creates it with content `.assay/\n`; call `ensure_gitignore_assay()` in `execute_run()` after Phase 3 (runtime check) and before Phase 5 (provision container). Unit tests: `test_list_empty_runs_dir`, `test_list_with_state_files`, `test_list_skips_corrupt_state`, `test_ensure_gitignore_creates`, `test_ensure_gitignore_appends`, `test_ensure_gitignore_idempotent`, `test_ensure_gitignore_trailing_newline`.
  - Verify: `cargo test -p smelt-cli` passes with all new tests green
  - Done when: `smelt list` runs on empty and populated `.smelt/runs/`; gitignore guard is called from `execute_run()`; 7 unit tests pass for guard + list

## Files Likely Touched

- `crates/smelt-core/src/monitor.rs`
- `crates/smelt-cli/src/commands/run.rs`
- `crates/smelt-cli/src/commands/watch.rs`
- `crates/smelt-cli/src/commands/status.rs`
- `crates/smelt-cli/src/commands/mod.rs`
- `crates/smelt-cli/src/main.rs`
- `crates/smelt-cli/src/commands/init.rs` (new)
- `crates/smelt-cli/src/commands/list.rs` (new)
