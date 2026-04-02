---
id: T02
parent: S03
milestone: M003
provides:
  - "WatchArgs struct with job_name positional arg and --interval-secs flag (default 30)"
  - "execute() public async fn — reads RunState, validates pr_url/forge_token_env/token/repo/pr_number, constructs GitHubForge, delegates to run_watch"
  - "run_watch<F: ForgeClient>() pub(crate) inner fn — testable polling loop, updates RunState on each poll, exits 0 on Merged / 1 on Closed / loops on Open"
  - "MockForge under #[cfg(test)] — VecDeque-backed, implements ForgeClient, enables unit tests without network"
  - "4 unit tests: test_watch_exits_0_on_merged, test_watch_exits_1_on_closed, test_watch_immediate_merged, test_watch_updates_run_state_each_poll"
  - "`smelt watch` wired into Commands enum and match arm in main.rs"
key_files:
  - "crates/smelt-cli/src/commands/watch.rs"
  - "crates/smelt-cli/src/commands/mod.rs"
  - "crates/smelt-cli/src/main.rs"
  - "crates/smelt-cli/Cargo.toml"
key_decisions:
  - "Added toml as regular dep to smelt-cli (previously dev-only) so persist_run_state helper can serialize RunState without a JobMonitor; JobMonitor.state_dir is private so constructing one for write-back would require API changes to smelt-core"
  - "run_watch takes generic F: ForgeClient (not dyn) — avoids object-safety issues with RPITIT trait and works cleanly with MockForge in tests"
  - "persist_run_state is best-effort (errors ignored) — a watch poll failure to update state is observability degradation, not a fatal error"
  - "Duration::ZERO used in all unit tests — tokio::time::sleep(Duration::ZERO) yields but returns immediately, keeping tests fast without mocking the clock"
patterns_established:
  - "Inner run_watch<F: ForgeClient> pattern — generic over forge client, enables unit tests with MockForge and production use with GitHubForge without trait objects"
  - "MockForge with VecDeque<PrStatus> + default fallback — pop from queue, fall back to default when empty; reusable pattern for forge-related command tests"
observability_surfaces:
  - "stderr poll line: [HH:MM:SS] PR #N — state: X | CI: Y | reviews: N — one line per interval"
  - "Exit signals: 'PR merged.' (exit 0) or 'PR closed without merging.' (exit 1) printed to stderr"
  - "cat .smelt/run-state.toml — pr_status, ci_status, review_count updated after each successful poll; pr_status = 'merged' is the authoritative exit-0 signal"
  - "[WARN] poll failed: ... — transient API errors printed to stderr without aborting"
duration: 25min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T02: Implement smelt watch polling command

**`smelt watch <job-name>` blocks and polls a PR via MockForge-testable run_watch<F>, updating RunState each iteration; exits 0 on Merged, 1 on Closed.**

## What Happened

Created `crates/smelt-cli/src/commands/watch.rs` with `WatchArgs`, a public `execute()` function, and a `pub(crate) run_watch<F: ForgeClient>()` inner function. The inner function is generic over `F: ForgeClient` (not `dyn`) to avoid RPITIT object-safety issues and to cleanly accommodate `MockForge` in tests.

`execute()` performs all the guard checks required by the task plan: missing state file, missing `pr_url`, missing `forge_token_env`, unset token env var, missing `forge_repo`, missing `pr_number`. Each check prints a clear human-readable error to stderr and returns `Ok(1)`.

`run_watch` loops: polls the forge client, on success updates RunState via `persist_run_state` (best-effort, errors are swallowed so a failed write doesn't abort the polling session), prints a `[HH:MM:SS] PR #N — state: X | CI: Y | reviews: N` stderr line, then matches on `PrState`: Merged → `Ok(0)`, Closed → `Ok(1)`, Open → sleep interval and loop. On transient poll errors it prints a `[WARN]` line and retries.

`MockForge` under `#[cfg(test)]` uses a `Mutex<VecDeque<PrStatus>>` and a `default` fallback — pops from the queue on each `poll_pr_status` call, falls back to `default` when empty. All 4 unit tests use `Duration::ZERO` so `tokio::time::sleep` yields without blocking.

Added `toml` to regular dependencies in `smelt-cli/Cargo.toml` (it was dev-only) so `persist_run_state` can use `toml::to_string_pretty` in non-test code.

Wired `Watch(commands::watch::WatchArgs)` into the `Commands` enum in `main.rs` and added the corresponding match arm.

## Verification

- `cargo build --bin smelt` — clean, 0 errors, 0 warnings after import cleanup
- `cargo run --bin smelt -- watch --help` — shows `<JOB_NAME>` positional and `--interval-secs` flag  
- `cargo run --bin smelt -- --help` — shows `watch` subcommand
- `cargo test -p smelt-cli --lib -q` — 15 tests pass (includes all 4 watch tests)
- `cargo test --workspace -q` — 0 failed across all crates
- `cargo test -p smelt-cli --test status_pr -q` — 5 tests pass (slice-level regression check)
- Confirmed `run_watch` signature is `pub(crate) async fn run_watch<F: ForgeClient>` and `MockForge` exists under `#[cfg(test)]`

## Diagnostics

After a `smelt watch` session, inspect via:
- `cat .smelt/run-state.toml` — `pr_status`, `ci_status`, `review_count` reflect the last successful poll
- `pr_status = "merged"` is the authoritative signal that watch exited 0
- Stderr output captures the full poll history (each `[HH:MM:SS]` line) and the terminal `PR merged.`/`PR closed without merging.` line

## Deviations

None from the task plan. All guard checks, poll line format, and state update behavior match the spec exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/watch.rs` — full watch implementation: WatchArgs, execute(), run_watch(), persist_run_state(), local_time_hms(), MockForge, 4 unit tests
- `crates/smelt-cli/src/commands/mod.rs` — added `pub mod watch;`
- `crates/smelt-cli/src/main.rs` — added `Watch(commands::watch::WatchArgs)` variant and match arm
- `crates/smelt-cli/Cargo.toml` — added `toml.workspace = true` to regular deps
