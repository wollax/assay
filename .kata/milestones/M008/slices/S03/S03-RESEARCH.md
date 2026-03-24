# S03: State sync back via scp — Research

**Date:** 2026-03-24

## Summary

S03 adds the reverse-direction scp capability: after `run_remote_job()` completes on a worker, the dispatcher pulls `.smelt/runs/<job_name>/` back to its local filesystem so `smelt status <job>` and the TUI/API can display correct phase, exit code, and elapsed time for remotely-executed jobs.

The implementation is straightforward — it mirrors S02's `scp_to()` with a new `scp_from()` trait method and a `sync_state_back()` free function. The main subtlety is the remote state path: `smelt run /tmp/smelt-<job_id>.toml` computes `state_dir` as `manifest.parent()/.smelt/runs/<job_name>/` (line 174-180 of `run.rs`), so the remote path is `/tmp/.smelt/runs/<job_name>/state.toml`. This means we need the manifest's `job.name` (from the TOML content), not the queue's `JobId` (like `job-1`), to locate the remote state directory.

The S03-CONTEXT.md proposes a `StateBackend` trait abstraction. After analysis, this adds unnecessary indirection for M008 — the only backend is `scp`, and S04 calls a single free function. A simpler approach: add `scp_from()` to `SshClient`, implement `sync_state_back<C: SshClient>()` as a free function (matching `deliver_manifest` and `run_remote_job` patterns), and defer the trait abstraction to a future milestone that actually adds a second backend. This follows YAGNI and keeps the code consistent with S01/S02's established pattern.

## Recommendation

**Skip the `StateBackend` trait; implement `sync_state_back()` as a free function generic over `C: SshClient`.**

1. Add `scp_from(&self, worker, timeout, remote_src, local_dest) -> Result<()>` to `SshClient` trait — symmetric with `scp_to()`
2. Implement `SubprocessSshClient::scp_from()` using `build_scp_args()` with `-r` flag (recursive) and swapped src/dest order
3. Add `MockSshClient::with_scp_from_result()` for test double (new queue alongside existing `scp_results`)
4. Implement `sync_state_back<C: SshClient>(client, worker, timeout, job_name, local_dest_dir) -> Result<()>` — pulls `/tmp/.smelt/runs/<job_name>/` from worker to `local_dest_dir/.smelt/runs/<job_name>/`
5. On failure: return `Err` — caller (S04 dispatch) decides whether to mark Failed or log warn
6. Unit tests via `MockSshClient`; gated integration test with `SMELT_SSH_TEST=1`

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| SCP argument construction | `SubprocessSshClient::build_scp_args()` | Already handles BatchMode, StrictHostKeyChecking, ConnectTimeout, port (-P), key_env — just add `-r` for recursive |
| Mock SSH for tests | `MockSshClient` in `ssh.rs` tests module | Builder pattern with `VecDeque` queues; add `scp_from_results` queue alongside existing `scp_results` |
| State file reading/verification | `JobMonitor::read(state_dir)` | Returns `RunState` with phase, exit_code, job_name — use in integration test to verify sync worked |
| Remote path computation | Derive from `run.rs` line 174-180 pattern | `manifest.parent()/.smelt/runs/<job_name>/` — for `/tmp/smelt-<job_id>.toml` that's `/tmp/.smelt/runs/<job_name>/` |

## Existing Code and Patterns

- `crates/smelt-cli/src/serve/ssh.rs` — Core SSH module. `SshClient` trait with `exec()`, `probe()`, `scp_to()`; `SubprocessSshClient` impl; `build_ssh_args()` and `build_scp_args()` helpers; `deliver_manifest()` and `run_remote_job()` free functions. S03 extends this with `scp_from()` and `sync_state_back()`.
- `crates/smelt-cli/src/serve/ssh.rs::MockSshClient` — Test double with `exec_results`, `probe_results`, `scp_results` queues (all `Arc<Mutex<VecDeque>>`). S03 adds a `scp_from_results` queue. Builder methods: `with_exec_result()`, `with_scp_result()`, `with_probe_result()`.
- `crates/smelt-core/src/monitor.rs::JobMonitor::read(state_dir)` — Reads `state_dir/state.toml` into `RunState`. Used in integration test to verify synced state is valid.
- `crates/smelt-cli/src/commands/run.rs` lines 174-180 — State dir computation: `manifest.parent()/.smelt/runs/<job_name>/`. Critical for determining the remote path to pull from.
- `crates/smelt-cli/src/serve/dispatch.rs::run_job_task()` — Current local dispatch; S04 will add the SSH path here, calling `sync_state_back()` after `run_remote_job()`.
- `crates/smelt-cli/src/serve/types.rs::QueuedJob` — Has `manifest_path` but no `job_name` field. S04 may need to parse the manifest to extract `job.name` for the state path, or S03 can accept `job_name: &str` as a parameter.

## Constraints

- **Remote state path is derived from manifest location, not job_id.** When `smelt run /tmp/smelt-<job_id>.toml` runs on a worker, it writes state to `/tmp/.smelt/runs/<job_name>/state.toml` where `<job_name>` comes from the `[job] name` field in the manifest TOML. The free function must accept `job_name` as a parameter — the caller (S04) must parse it from the manifest or pass it through.
- **`scp -r` is required for recursive directory copy.** The state directory contains `state.toml` and potentially other files. The `-r` flag must be added to the scp args.
- **D111 (subprocess scp)** — no Rust SSH library; shell out to system `scp`.
- **D112 (key_env)** — resolved key path only in DEBUG logs.
- **D121 (generics)** — `SshClient` is not object-safe; use `<C: SshClient>` at callsites.
- **`build_scp_args()` uses uppercase `-P` for port** (S02 forward intelligence) — must not change this when adding `-r`.
- **`scp_from()` reverses the src/dest order** vs `scp_to()` — `scp -r user@host:/remote/dir /local/dir` (remote first, then local).
- **Fire-and-forget semantics** — sync failure should not retry; caller logs warning and marks job appropriately.

## Common Pitfalls

- **Wrong remote path** — Using `job_id` (e.g. `job-1`) instead of `job_name` (from manifest's `[job] name`) for the remote state directory. The `run.rs` state_dir computation uses `manifest.job.name`, not any external ID. Always use the manifest's job name.
- **Forgetting `-r` flag on scp** — Without `-r`, `scp` cannot copy directories. The state is a directory (`/tmp/.smelt/runs/<job_name>/`) not a single file.
- **Port flag case mismatch** — `scp` uses uppercase `-P` for port (per S02 finding). If someone refactors `build_scp_args()`, the case difference with `build_ssh_args()` (`-p`) must be preserved.
- **`scp_from` arg order** — `scp_to` is `scp [flags] local remote`; `scp_from` is `scp [flags] remote local`. The reversed order is easy to get wrong.
- **Local destination directory must exist** — `scp -r` creates the leaf directory but not intermediate parents. Ensure `std::fs::create_dir_all()` on the local destination before scp.
- **MockSshClient `scp_from` vs `scp_to` queue confusion** — These are separate operations with separate result queues. Using the wrong builder method produces "no results configured" panics.

## Open Risks

- **Job name extraction** — `QueuedJob` has `manifest_path` but no `job_name` field. S04 dispatch will need to read and parse the manifest TOML to get `job.name` for the state sync path. This is a minor I/O cost but should be done once at dispatch time, not at sync time.
- **Partial state sync** — If `scp -r` is interrupted mid-transfer (e.g. network drop), the local state directory may contain partial files. `JobMonitor::read()` will fail to deserialize, which is acceptable (job shows as sync-failed). No mitigation needed for M008.
- **Worker state cleanup** — `/tmp/.smelt/runs/<job_name>/` is left on the worker after sync. This is acceptable for M008 (OS cleans `/tmp`), but long-running workers may accumulate state dirs. Noted for future cleanup.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| SSH/SCP | N/A | none found — subprocess ssh/scp is standard; no skill needed |
| Rust async/tokio | N/A | already well-established in codebase; no skill needed |

## Sources

- `crates/smelt-cli/src/commands/run.rs` lines 174-180 — state_dir computation from manifest parent
- `crates/smelt-cli/src/serve/ssh.rs` — SshClient trait, build_scp_args(), MockSshClient
- `crates/smelt-core/src/monitor.rs` — JobMonitor::read() API for state verification
- `.kata/milestones/M008/slices/S02/S02-SUMMARY.md` — Forward intelligence on scp_from need and MockSshClient usage
- `.kata/milestones/M008/slices/S03/S03-CONTEXT.md` — Original scope (StateBackend trait proposed but deemed premature)
