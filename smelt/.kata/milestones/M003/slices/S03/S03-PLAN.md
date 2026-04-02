# S03: PR Status Tracking

**Goal:** `smelt status` shows the PR section (URL, state, CI status, review count) when a PR exists, and `smelt watch <job-name>` polls until the PR is merged (exit 0) or closed without merging (exit 1).
**Demo:** After `smelt run` with `[forge]` creates a PR, `smelt status` prints a `── Pull Request ──` section showing the URL and current state; `smelt watch integration-test` blocks, printing a status line every 30s, then exits 0 when the PR is merged.

## Must-Haves

- `RunState` carries `pr_status`, `ci_status`, `review_count`, `forge_repo`, and `forge_token_env` — all backward-compat (`#[serde(default)]`); existing state files without these fields deserialize cleanly
- Phase 9 in `run.rs` persists `forge_repo` and `forge_token_env` into `RunState` alongside `pr_url`/`pr_number`
- `smelt status` prints a `── Pull Request ──` section containing URL, state, CI status, and review count when `pr_url` is set; section is absent when `pr_url` is None
- `smelt watch <job-name>` reads `RunState`, polls `GitHubForge::poll_pr_status()` every 30s, prints a one-liner each poll, and exits 0 on `Merged`, exits 1 on `Closed`
- `smelt watch` updates `RunState` (pr_status, ci_status, review_count) on each successful poll
- `smelt watch` errors clearly when `pr_url` is None (no PR was created for this job) or when the token env var is unset
- All 124+ workspace tests continue to pass

## Proof Level

- This slice proves: contract + integration (CLI arg parsing)
- Real runtime required: no (mock forge via test trait impl)
- Human/UAT required: no (live watch proof deferred to S06 UAT)

## Verification

- `cargo test -p smelt-cli --test status_pr` — all PR section display tests pass (file created in T01)
- `cargo test -p smelt-cli --lib` — watch command unit tests pass (test_watch_exits_0_on_merged, test_watch_exits_1_on_closed, test_watch_no_pr_url_errors, existing should_create_pr_guard still passes)
- `cargo test --workspace` — full workspace passes (no regressions)
- `cargo build --bin smelt` — binary builds; `smelt --help` shows `watch` subcommand
- `cargo run --bin smelt -- watch --help` — prints usage for `smelt watch <job-name>`

## Observability / Diagnostics

- Runtime signals: `smelt watch` prints `[HH:MM:SS] PR #N — state: Open | CI: Pending | reviews: 0` to stderr on each poll; final line is `PR merged — exiting 0` or `PR closed — exiting 1`
- Inspection surfaces: `cat .smelt/run-state.toml` — after each watch poll, `pr_status`, `ci_status`, `review_count` are updated; `smelt status` reads these cached values for display
- Failure visibility: `smelt watch` exits non-zero and prints a clear error when: (a) no state file found, (b) no `pr_url` in state, (c) `forge_token_env` env var not set, (d) GitHub API call fails
- Redaction constraints: `forge_token_env` (env var *name*) stored in RunState — never the token value itself

## Integration Closure

- Upstream surfaces consumed:
  - `smelt_core::forge::{ForgeClient, GitHubForge, PrState, CiStatus, PrStatus}` — from S01
  - `smelt_core::monitor::{RunState, JobMonitor}` — pr_url/pr_number from S02
  - `smelt_core::forge::ForgeConfig` — token_env/repo fields from S01/S02
- New wiring introduced in this slice:
  - `run.rs` Phase 9 now writes `forge_repo` and `forge_token_env` into RunState
  - `watch.rs` new command wired into `mod.rs` and `main.rs`
  - `status.rs` `print_status()` gains PR section output
- What remains before the milestone is truly usable end-to-end:
  - S04: per-job state isolation (`.smelt/runs/<job-name>/state.toml`), `smelt init`, `smelt list`, `.assay/` gitignore guard
  - S05: `smelt-core` library API hardening and docs
  - S06: live end-to-end proof with real Docker + real GitHub

## Tasks

- [x] **T01: Extend RunState with forge context fields and add smelt status PR section** `est:45m`
  - Why: RunState needs `forge_repo` and `forge_token_env` so `smelt watch` can reconstruct the GitHub client without the original manifest; `pr_status`/`ci_status`/`review_count` give status.rs cached values to display. The PR section in `smelt status` closes R003.
  - Files: `crates/smelt-core/src/monitor.rs`, `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/src/commands/status.rs`, `crates/smelt-cli/tests/status_pr.rs` (new)
  - Do:
    1. Add five backward-compat fields to `RunState` in `monitor.rs`: `pr_status: Option<PrState>`, `ci_status: Option<CiStatus>`, `review_count: Option<u32>`, `forge_repo: Option<String>`, `forge_token_env: Option<String>` — all with `#[serde(default)]`; add `use smelt_core::forge::{PrState, CiStatus}` imports
    2. In `run.rs` Phase 9 (after `create_pr` succeeds), persist `monitor.state.forge_repo = Some(forge_cfg.repo.clone())` and `monitor.state.forge_token_env = Some(forge_cfg.token_env.clone())`, then call `monitor.write()`
    3. In `status.rs`, refactor `print_status()` to call a new `format_pr_section(state: &RunState) -> Option<String>` helper that returns Some(section_text) when `state.pr_url.is_some()` and None otherwise; call `println!("{}", section)` in `print_status()` when Some
    4. Implement `format_pr_section`: builds multiline string with `── Pull Request ──`, URL line, state line (uses `pr_status` if set, else "unknown"), CI line (uses `ci_status` if set, else "unknown"), review count line
    5. Create `crates/smelt-cli/tests/status_pr.rs` with: `test_format_pr_section_absent_when_no_url` (None when pr_url is None), `test_format_pr_section_shows_url` (Some when pr_url is set), `test_format_pr_section_shows_state_ci_reviews` (state/CI/count rendered), `test_format_pr_section_shows_unknown_when_no_cached_status` (graceful None fields), `test_run_state_new_fields_backward_compat` (deserialize old TOML without new fields → all None)
    6. Update any test helper `RunState` literals that need the new fields (set all new fields to `None`)
    7. Fix `JobMonitor::new()` initializer to set all five new fields to `None`
  - Verify: `cargo test -p smelt-cli --test status_pr` passes; `cargo test -p smelt-core` passes; `cargo test --workspace` passes
  - Done when: `format_pr_section` is exported (pub(crate)) and all 5 tests in `status_pr.rs` pass; backward-compat test confirms old TOML without new fields deserializes cleanly; workspace tests pass

- [x] **T02: Implement smelt watch polling command** `est:45m`
  - Why: `smelt watch` is R004 — the blocking poll command needed by CI pipelines. It reads RunState, polls GitHub every 30s, updates state, and exits correctly on terminal PR states.
  - Files: `crates/smelt-cli/src/commands/watch.rs` (new), `crates/smelt-cli/src/commands/mod.rs`, `crates/smelt-cli/src/main.rs`
  - Do:
    1. Create `crates/smelt-cli/src/commands/watch.rs` with `WatchArgs { job_name: String, #[arg(long, default_value_t = 30)] interval_secs: u64 }` and `pub async fn execute(args: &WatchArgs) -> Result<i32>`
    2. Extract a testable inner function `pub(crate) async fn run_watch<F: ForgeClient>(state_dir: &Path, forge: F, interval: Duration) -> Result<i32>` that takes an already-constructed ForgeClient — this enables unit tests with a mock client
    3. In `execute()`: read RunState from `state_dir`; if `pr_url` is None, print error and return `Ok(1)`; read token from `forge_token_env` env var (if None/unset, print clear error and return `Ok(1)`); construct `GitHubForge::new(token)?`; call `run_watch()`
    4. In `run_watch()`: loop — call `forge.poll_pr_status(repo, number)`; on success, update RunState fields (`pr_status`, `ci_status`, `review_count`), call `monitor.write()`; print `[HH:MM:SS] PR #N — state: Open | CI: Pending | reviews: 0` to stderr; match on state: `Merged` → print "PR merged." and return `Ok(0)`, `Closed` → print "PR closed without merging." and return `Ok(1)`, `Open` → sleep interval; on API error, print warning and continue (don't abort on transient errors)
    5. Implement a `MockForge` struct (in watch.rs under `#[cfg(test)]`) that holds a `Vec<PrStatus>` and returns them in sequence, returning the last one indefinitely once exhausted — implements `ForgeClient` trait
    6. Write unit tests: `test_watch_exits_0_on_merged` (Open → Merged sequence, expect Ok(0)), `test_watch_exits_1_on_closed` (Open → Closed, expect Ok(1)), `test_watch_updates_run_state_each_poll` (verify RunState file updated after each poll), `test_watch_immediate_merged` (already Merged on first poll, expect Ok(0) immediately)
    7. Add `pub mod watch;` to `commands/mod.rs`; add `Watch(commands::watch::WatchArgs)` to `Commands` enum in `main.rs`; add `Commands::Watch(ref args) => commands::watch::execute(args).await` arm to the match
  - Verify: `cargo test -p smelt-cli --lib` passes (all watch tests pass); `cargo build --bin smelt` builds; `cargo run --bin smelt -- watch --help` shows usage; `cargo test --workspace` passes
  - Done when: `smelt watch --help` shows job_name and interval_secs args; all 4 watch unit tests pass; no workspace regressions

## Files Likely Touched

- `crates/smelt-core/src/monitor.rs` — 5 new RunState fields + initializer update
- `crates/smelt-cli/src/commands/run.rs` — Phase 9 persists forge_repo and forge_token_env
- `crates/smelt-cli/src/commands/status.rs` — format_pr_section + PR section in print_status
- `crates/smelt-cli/tests/status_pr.rs` — new test file (5 tests)
- `crates/smelt-cli/src/commands/watch.rs` — new watch command (full implementation + unit tests)
- `crates/smelt-cli/src/commands/mod.rs` — add watch module
- `crates/smelt-cli/src/main.rs` — wire Watch subcommand
