---
estimated_steps: 7
estimated_files: 5
---

# T02: Implement smelt watch polling command

**Slice:** S03 — PR Status Tracking
**Milestone:** M003

## Description

`smelt watch <job-name>` is the blocking poll command for R004. It reads RunState from `.smelt/run-state.toml`, reconstructs the GitHub client using `forge_token_env` (the env var name stored by Phase 9), then polls `poll_pr_status()` every 30s (configurable). On each poll it updates RunState and prints a one-liner. Exits 0 on `Merged`, exits 1 on `Closed`, loops on `Open`. Transient API errors print a warning but do not abort.

The key to testability is extracting a `run_watch()` inner function that accepts a `ForgeClient` impl — unit tests inject a mock client with a pre-programmed state sequence, no network required.

## Steps

1. **Create `crates/smelt-cli/src/commands/watch.rs`:** define `WatchArgs`:
   ```rust
   #[derive(Debug, Args)]
   pub struct WatchArgs {
       /// Job name to watch (must match job.name in the manifest used for smelt run).
       pub job_name: String,
       /// Polling interval in seconds.
       #[arg(long, default_value_t = 30)]
       pub interval_secs: u64,
   }
   ```
   And `pub async fn execute(args: &WatchArgs) -> Result<i32>`.

2. **Implement `execute()`:** 
   - Read RunState from `.smelt/run-state.toml` (current dir); if missing, print "No state file — has `smelt run` been called?" and return `Ok(1)`
   - Check `state.pr_url`: if None, print "No PR was created for this job (pr_url is not set in state). Did `smelt run` complete Phase 9?" and return `Ok(1)`
   - Check `state.forge_token_env`: if None, print "No forge context in state (forge_token_env missing). State was written before S03." and return `Ok(1)`
   - Read token from env: `std::env::var(&token_env)` — if missing, print "env var `<TOKEN_ENV>` not set — required for PR polling" and return `Ok(1)`
   - Parse repo from `state.forge_repo` (unwrap or return error)
   - Parse PR number from `state.pr_number` (unwrap or return error)
   - Construct `GitHubForge::new(token)?`
   - Call `run_watch(&state_dir, forge, pr_number, &repo, Duration::from_secs(args.interval_secs)).await`

3. **Extract inner `run_watch` function:**
   ```rust
   pub(crate) async fn run_watch<F: ForgeClient>(
       state_dir: &Path,
       forge: F,
       pr_number: u64,
       repo: &str,
       interval: Duration,
   ) -> Result<i32>
   ```
   Loop body:
   - Call `forge.poll_pr_status(repo, pr_number).await`
   - On `Err(e)`: print `"[WARN] poll failed: {e:#} — retrying in {interval}s"` to stderr; sleep; continue
   - On `Ok(status)`: update RunState fields (`pr_status`, `ci_status`, `review_count`) by reading and re-writing RunState; print poll line; match on `status.state`: `Merged` → print "PR merged." and return `Ok(0)`, `Closed` → print "PR closed without merging." and return `Ok(1)`, `Open` → sleep interval and continue
   
   Poll line format: `eprintln!("[{time}] PR #{pr_number} — state: {state} | CI: {ci} | reviews: {n}")` where `{time}` is `HH:MM:SS` local time.

4. **Implement `MockForge` for tests** (inside `#[cfg(test)]` mod in `watch.rs`):
   ```rust
   struct MockForge {
       responses: std::sync::Mutex<std::collections::VecDeque<PrStatus>>,
       default: PrStatus,
   }
   impl ForgeClient for MockForge {
       async fn create_pr(...) { unimplemented!() }
       async fn poll_pr_status(&self, _repo: &str, _number: u64) -> Result<PrStatus> {
           let mut q = self.responses.lock().unwrap();
           Ok(q.pop_front().unwrap_or_else(|| self.default.clone()))
       }
   }
   ```
   Helper: `fn open_status() -> PrStatus`, `fn merged_status() -> PrStatus`, `fn closed_status() -> PrStatus`.

5. **Write unit tests in `watch.rs`:**
   - `test_watch_exits_0_on_merged`: MockForge with sequence [Open, Open, Merged]; use a tempdir as state_dir; write initial RunState with pr_url/pr_number/forge_repo; call `run_watch(..., interval: 0s)`; assert returns `Ok(0)`
   - `test_watch_exits_1_on_closed`: MockForge with [Open, Closed]; assert returns `Ok(1)`
   - `test_watch_immediate_merged`: MockForge with [Merged]; assert returns `Ok(0)` immediately
   - `test_watch_updates_run_state_each_poll`: MockForge with [Open, Merged]; after run_watch, read RunState from tempdir; assert `pr_status == Some(PrState::Merged)` and `ci_status`/`review_count` are set

6. **Wire into mod.rs and main.rs:**
   - Add `pub mod watch;` to `commands/mod.rs`
   - In `main.rs`, add `Watch(commands::watch::WatchArgs)` to `Commands` enum
   - Add `Commands::Watch(ref args) => commands::watch::execute(args).await` to the match in `main()`

7. **Verify the full suite:** `cargo build --bin smelt` (must compile); `cargo run --bin smelt -- watch --help` (must print usage with job_name and --interval-secs); `cargo test -p smelt-cli --lib` (4 watch tests pass); `cargo test --workspace` (no regressions).

## Must-Haves

- [ ] `smelt watch <job-name>` appears in `smelt --help` output
- [ ] `smelt watch --help` shows `job_name` positional arg and `--interval-secs` flag
- [ ] `run_watch()` exits 0 when poll returns `PrState::Merged`
- [ ] `run_watch()` exits 1 when poll returns `PrState::Closed`
- [ ] `run_watch()` updates `pr_status`, `ci_status`, `review_count` in RunState on each successful poll
- [ ] `execute()` exits 1 with a clear message when `pr_url` is None in RunState
- [ ] `execute()` exits 1 with a clear message when token env var is unset
- [ ] All 4 unit tests pass; `cargo test --workspace` passes

## Verification

- `cargo build --bin smelt 2>&1` — exit 0, no errors
- `cargo run --bin smelt -- watch --help 2>&1` — shows `<job-name>` and `--interval-secs`
- `cargo test -p smelt-cli --lib -q 2>&1` — includes 4 watch tests, all pass
- `cargo test --workspace -q 2>&1` — 0 failed
- Inspect `watch.rs`: `run_watch` function signature accepts generic `F: ForgeClient`; `MockForge` exists under `#[cfg(test)]`

## Observability Impact

- Signals added/changed: `smelt watch` prints one stderr line per poll: `[HH:MM:SS] PR #N — state: X | CI: Y | reviews: N`; terminal messages "PR merged." or "PR closed without merging." are the exit signals
- How a future agent inspects this: read `.smelt/run-state.toml` after a `smelt watch` session — `pr_status`, `ci_status`, `review_count` reflect the last poll; `pr_status = "merged"` is the authoritative signal that watch exited 0
- Failure state exposed: any watch error (missing pr_url, missing token, API failure) is printed to stderr before exit; exit code 1 distinguishes "closed without merge" from "error" (both exit 1 — a future improvement could differentiate with exit 2 for errors)

## Inputs

- `crates/smelt-core/src/forge.rs` — `ForgeClient` trait, `GitHubForge`, `PrState`, `CiStatus`, `PrStatus` (from S01)
- `crates/smelt-core/src/monitor.rs` — `RunState` with all new fields from T01 (`pr_status`, `ci_status`, `review_count`, `forge_repo`, `forge_token_env`)
- `crates/smelt-cli/src/commands/run.rs` — pattern for reading RunState (for reference)
- `crates/smelt-cli/src/commands/status.rs` — pattern for `StatusArgs` + `execute()` structure
- T01 output: RunState fields `forge_repo`, `forge_token_env`, `pr_status`, `ci_status`, `review_count` all available

## Expected Output

- `crates/smelt-cli/src/commands/watch.rs` — full watch implementation: `WatchArgs`, `execute()`, `run_watch()`, `MockForge`, 4 unit tests
- `crates/smelt-cli/src/commands/mod.rs` — `pub mod watch;` added
- `crates/smelt-cli/src/main.rs` — `Watch` variant in `Commands` + match arm
