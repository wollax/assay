---
id: S02
milestone: M008
status: ready
---

# S02: Manifest delivery + remote smelt run execution — Context

## Goal

Given a `WorkerConfig` and a local manifest path, deliver the manifest to the remote worker via `scp` and execute `smelt run <path>` over SSH, capturing the exit code and mapping it to job success/failure (including exit code 2 → GatesFailed).

## Why this Slice

S02 builds directly on the SSH primitives proven in S01 (`SshClient` trait, `SubprocessSshClient`, fast-fail timeout). It delivers the two core operations that everything downstream needs: manifest delivery and remote execution. S03 (state sync) and S04 (dispatch routing) cannot be built until S02 proves these operations work reliably with real `smelt run` invocations.

## Scope

### In Scope

- `deliver_manifest(worker: &WorkerConfig, job_id: &JobId, manifest_path: &Path) -> Result<String>` — scps manifest to `/tmp/smelt-<job_id>.toml` on the worker; returns the remote path as a `String`
- `run_remote_job(worker: &WorkerConfig, remote_manifest_path: &str) -> Result<i32>` — SSHes `smelt run <path>` on the worker; captures and returns the exit code
- Exit code mapping: `0` → success, `2` → GatesFailed (distinct from other non-zero failures), any other non-zero → failure; mapping is applied in the dispatch layer that calls `run_remote_job`, not inside `run_remote_job` itself (which just returns the raw exit code)
- Integration test gated by `SMELT_SSH_TEST=1`: delivers a real manifest TOML to localhost via scp, executes `smelt run --dry-run <remote_path>` over SSH, asserts exit code 0 and no error — proves the full delivery+exec path without requiring Docker on the test host
- Temp path convention: `/tmp/smelt-<job_id>.toml` on the worker — no cleanup needed (OS reclaims on reboot)
- Credentials: workers provide their own environment variables; the dispatcher does NOT forward credentials over SSH — operator sets `GITHUB_TOKEN`, API keys, etc. on each worker host directly

### Out of Scope

- State sync back (`.smelt/runs/<job>/`) — S03
- Dispatch routing integration — S04
- Credential forwarding from dispatcher to worker — not implemented in M008; workers are responsible for their own credentials
- Manifest cleanup on the remote after job completion — not in M008; `/tmp` cleanup is OS-managed
- Streaming `smelt run` output back to the dispatcher in real time — not in M008; only exit code is captured
- Remote `smelt run` invocation with real Docker (integration test uses `--dry-run` only)

## Constraints

- D111: subprocess `ssh`/`scp` — no Rust SSH library crate; consistent with git CLI pattern
- D112: `key_env` on `WorkerConfig` names the env var holding the SSH private key path; dispatcher reads it and passes `-i $KEY_PATH` to `scp`/`ssh` subprocesses
- Exit code 2 must be preserved distinctly through `run_remote_job`'s return value — the dispatch layer maps it to `GatesFailed` (matching local `smelt run` semantics from M002/S04)
- Worker credentials are entirely the operator's responsibility — no secrets flow from dispatcher to worker over SSH
- Integration test must skip gracefully when `SMELT_SSH_TEST=1` is not set; `cargo test --workspace` must stay green without it (consistent with `SMELT_K8S_TEST=1` pattern, D089)

## Integration Points

### Consumes

- `SshClient` trait + `SubprocessSshClient` from S01 — used to exec `smelt run` on the remote
- `WorkerConfig { host, user, key_env, port }` from S01 — source of SSH connection parameters
- `JobId` from `crates/smelt-cli/src/serve/types.rs` — used to construct the remote manifest path `/tmp/smelt-<job_id>.toml`

### Produces

- `deliver_manifest(worker, job_id, manifest_path) -> Result<String>` — returns the remote path; consumed by S03 (to know where the manifest landed) and S04 (dispatch layer chains deliver → run_remote_job)
- `run_remote_job(worker, remote_manifest_path) -> Result<i32>` — returns raw exit code (0, 2, or other); consumed by S04 dispatch layer which maps to job success/GatesFailed/failure
- Localhost SSH integration test (`SMELT_SSH_TEST=1`) — proves delivery + `--dry-run` exec chain works end-to-end

## Open Questions

- **`scp` destination directory existence**: `/tmp` always exists on Linux/macOS remote hosts, so no `mkdir` is needed before `scp`. This assumption should be verified in the integration test. If a non-`/tmp` path is ever needed, the caller would need to `mkdir -p` first.
- **SSH exec environment for `smelt run`**: non-interactive SSH sessions may not source `~/.bashrc` or `~/.profile`, so `smelt` must be on the non-interactive PATH. The integration test with `--dry-run` will surface this immediately if `smelt` is not found. Operator documentation should note this.
- **Exit code propagation through subprocess**: `ssh` exits with the remote command's exit code when using `ssh -o BatchMode=yes`; this should be verified in the integration test by checking that `smelt run --dry-run` (exit 0) and a deliberately bad manifest (exit non-zero) both propagate correctly through the subprocess layer.
