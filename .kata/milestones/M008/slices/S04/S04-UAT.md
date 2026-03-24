# S04: Dispatch routing + round-robin + TUI/API worker field — UAT

**Milestone:** M008
**Written:** 2026-03-24

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: Automated tests prove dispatch logic with MockSshClient; UAT must exercise real SSH connections to remote hosts to validate the full pipeline end-to-end.

## Preconditions

- Two machines reachable via SSH from the dispatcher host (can be the same machine with two `[[workers]]` entries pointing to localhost on different ports, or two distinct hosts)
- SSH key-based authentication configured for both workers
- `smelt` binary installed on both worker machines (available on PATH)
- A valid job manifest (e.g. `examples/manifest.toml`) with a quick-running job
- `server.toml` with two `[[workers]]` entries configured:
  ```toml
  [[workers]]
  host = "worker-a.local"
  user = "deploy"
  key_env = "WORKER_A_SSH_KEY"
  port = 22

  [[workers]]
  host = "worker-b.local"
  user = "deploy"
  key_env = "WORKER_B_SSH_KEY"
  port = 22
  ```
- Environment variables set: `WORKER_A_SSH_KEY=/path/to/key_a`, `WORKER_B_SSH_KEY=/path/to/key_b`

## Smoke Test

1. Start `smelt serve --config server.toml`
2. Submit one job: `curl -X POST http://localhost:3000/api/v1/jobs -H 'Content-Type: application/toml' --data-binary @examples/manifest.toml`
3. **Expected:** Job dispatched to one of the workers; TUI shows the worker hostname in the Worker column; `GET /api/v1/jobs` returns `worker_host` with the worker's hostname.

## Test Cases

### 1. Round-robin distribution

1. Start `smelt serve --config server.toml` with 2 workers configured
2. Submit 4 jobs rapidly via `curl -X POST ...` (4 separate requests)
3. Wait for all jobs to reach terminal status
4. Run `curl http://localhost:3000/api/v1/jobs | jq '.[].worker_host'`
5. **Expected:** Jobs alternate between worker-a and worker-b (round-robin pattern). Each worker gets approximately 2 jobs.

### 2. Worker host visible in TUI

1. Start `smelt serve --config server.toml` (TUI enabled, default)
2. Submit 2 jobs
3. Observe the TUI table
4. **Expected:** A "Worker" column is visible showing the hostname of the worker each job was dispatched to. Locally-dispatched jobs (if any) show "-".

### 3. Worker host in API response

1. Submit a job and wait for completion
2. Run `curl http://localhost:3000/api/v1/jobs | python3 -m json.tool`
3. **Expected:** Each job object contains a `worker_host` field — a string with the worker hostname for SSH-dispatched jobs, or `null` for locally-dispatched jobs.

### 4. State sync back — smelt status on dispatcher

1. Submit a job and wait for it to complete on a remote worker
2. Run `smelt status <job-name>` on the dispatcher machine
3. **Expected:** Status shows correct phase (Complete or GatesFailed), exit code, and elapsed time — state was synced back from the worker via scp.

## Edge Cases

### Offline worker failover

1. Stop SSH on worker-a (or configure a non-existent host)
2. Submit 4 jobs
3. **Expected:** All 4 jobs dispatch to worker-b. Logs show `probe failed for worker worker-a` warnings. TUI shows all jobs with worker-b hostname.

### All workers offline — re-queue

1. Configure both workers with non-existent hosts (or stop SSH on both)
2. Submit 1 job
3. Observe TUI and logs for ~10 seconds
4. **Expected:** Job status shows "Queued" (re-queued after all-workers-offline). Logs show "all workers offline — re-queueing job" messages. Job is not marked Failed.

### No workers configured — local fallback

1. Remove all `[[workers]]` entries from `server.toml`
2. Start `smelt serve --config server.toml`
3. Submit a job
4. **Expected:** Job runs locally (same behavior as pre-M008). `worker_host` is `null` in API response. TUI shows "-" in Worker column.

### Queue state persistence with worker_host

1. Submit and complete 2 jobs with workers configured
2. Stop `smelt serve` (Ctrl+C)
3. Inspect `.smelt-queue-state.toml` in queue_dir
4. **Expected:** Each `[[jobs]]` entry contains `worker_host = "worker-a"` or `worker_host = "worker-b"`.

## Failure Signals

- Job stuck in "Dispatching" status for > 10 seconds — SSH connection hanging (timeout not working)
- `worker_host` is `null` for all jobs despite workers being configured — dispatch routing not entering SSH path
- `smelt status <job>` shows "No state found" after remote completion — scp state sync failed
- TUI missing Worker column — tui.rs regression
- All jobs going to the same worker — round-robin index not advancing

## Requirements Proved By This UAT

- R027 (SSH worker pools / remote dispatch) — Full end-to-end proof: real SSH connections, manifest delivery, remote `smelt run` execution, state sync back, round-robin distribution, offline failover, worker_host visibility in API and TUI

## Not Proven By This UAT

- Performance under high concurrency (many workers, many simultaneous jobs)
- SSH key rotation or credential expiry handling
- Network partition recovery (worker becomes unreachable mid-execution)
- Cross-platform worker support (all testing assumes Linux/macOS workers)

## Notes for Tester

- For a quick local test, you can use `localhost` as both workers (same machine, same port) — round-robin will still alternate the `worker_host` value but both point to the same host.
- SSH connection timeout is controlled by `ssh_timeout_secs` in `server.toml` (default 5). If workers are slow to respond, increase this.
- Check `.smelt/serve.log` for detailed tracing output including SSH dispatch events.
- The `#[allow(dead_code)]` on `retry_backoff_secs` in config.rs is expected — that field is parsed but not yet wired into retry logic.
