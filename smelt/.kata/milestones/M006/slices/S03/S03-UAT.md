# S03: Ratatui TUI + Server Config + Graceful Shutdown — UAT

**Milestone:** M006
**Written:** 2026-03-23

## UAT Type

- UAT mode: mixed (live-runtime + human-experience)
- Why this mode is sufficient: The TUI rendering and Ctrl+C teardown require an interactive terminal and real Docker containers to verify. Automated tests cover config parsing, HTTP API, and TUI render-no-panic; the remaining gaps (live TUI display, Ctrl+C orphan check) require human observation.

## Preconditions

- Docker daemon running and accessible (`docker ps` succeeds)
- `smelt` binary built (`cargo build -p smelt-cli`)
- Working directory is the smelt repo root
- `examples/server.toml` is present (it is — shipped with the slice)
- A valid `job-manifest.toml` exists or can be created with `smelt init`

## Smoke Test

```bash
# Start serve in no-TUI mode; verify HTTP API responds; stop server
./target/debug/smelt serve --config examples/server.toml --no-tui &
SERVER_PID=$!
sleep 2
curl -s http://127.0.0.1:8765/api/v1/jobs | jq   # expect []
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null
echo "Exit: $?"   # expect 0 or 1 (signal)
```

If this returns `[]` and the process exits, the slice is basically working.

## Test Cases

### 1. ServerConfig loads and validates examples/server.toml

1. Run `cat examples/server.toml` — verify all fields are present with inline `#` comments
2. Run `./target/debug/smelt serve --config examples/server.toml --no-tui &`
3. **Expected:** Server starts with no error output; `smelt serve started on 127.0.0.1:8765` logged to stderr

### 2. HTTP API responds correctly with no jobs

1. Start server as above
2. Run `curl -s http://127.0.0.1:8765/api/v1/jobs | jq`
3. **Expected:** `[]` (empty array)
4. Kill server with `kill $SERVER_PID`
5. **Expected:** Process exits cleanly (no zombie process, no orphaned Docker containers)

### 3. TUI renders in alternate screen

1. Run `./target/debug/smelt serve --config examples/server.toml` (without `--no-tui`)
2. **Expected:** Terminal switches to alternate screen; Ratatui table header visible (ID | Manifest | Status | Attempt | Elapsed); no garbage characters or corrupted display
3. **Expected:** No tracing output appears on the TUI screen (it goes to `.smelt/serve.log`)

### 4. Submit a job via HTTP POST and observe TUI

1. Start server with TUI active (no `--no-tui`)
2. From another terminal: `curl -s -X POST http://127.0.0.1:8765/api/v1/jobs --data-binary @job-manifest.toml -H 'Content-Type: text/plain' | jq`
3. **Expected:** Response contains `{ "job_id": "job-1" }` (or similar)
4. **Expected:** TUI table shows a new row with status transitioning from Queued → Running
5. Run `curl -s http://127.0.0.1:8765/api/v1/jobs | jq` from a third terminal
6. **Expected:** Job appears in the array with current status

### 5. `q` key triggers graceful shutdown

1. Start server with TUI active
2. Press `q` in the TUI terminal
3. **Expected:** TUI exits cleanly; alternate screen restored; normal terminal output visible
4. **Expected:** Any running job tasks cancel and tear down; no orphaned Docker containers (`docker ps --filter label=smelt.job` returns empty)

### 6. Ctrl+C triggers graceful shutdown

1. Start server with TUI active
2. Press Ctrl+C in the TUI terminal
3. **Expected:** All running job tasks receive cancellation signal; containers stopped and removed; process exits 0 or 130 (SIGINT)
4. **Expected:** No orphaned containers: `docker ps --filter label=smelt.job` returns empty
5. **Expected:** `.smelt/serve.log` contains "Ctrl+C received — cancelling all jobs"

### 7. Tracing redirects to .smelt/serve.log in TUI mode

1. Start server with TUI active: `./target/debug/smelt serve --config examples/server.toml`
2. In another terminal: `tail -f .smelt/serve.log`
3. Submit a job via HTTP POST
4. **Expected:** Log lines (dispatch events, phase transitions) appear in `.smelt/serve.log`; no tracing output visible on TUI screen
5. Press `q` to stop server
6. **Expected:** Final log line mentions TUI or shutdown

### 8. --no-tui routes tracing to stderr

1. Start: `./target/debug/smelt serve --config examples/server.toml --no-tui 2>&1 | tee /tmp/serve-stderr.log`
2. **Expected:** `smelt serve started on 127.0.0.1:8765` visible in terminal (stderr)
3. **Expected:** No `.smelt/serve.log` file created (or it is empty)

## Edge Cases

### Invalid config rejects before starting

1. Create `/tmp/bad-config.toml` with `max_concurrent = 0`
2. Run `./target/debug/smelt serve --config /tmp/bad-config.toml --no-tui`
3. **Expected:** Immediate error message containing "max_concurrent"; process exits non-zero; no port bound; no server started

### Port already in use

1. Start a listener on 8765: `nc -l 8765 &`
2. Run `./target/debug/smelt serve --config examples/server.toml --no-tui`
3. **Expected:** Immediate error message about port bind failure; process exits non-zero

### Queue dir created automatically

1. Edit a copy of `examples/server.toml` with `queue_dir = "/tmp/smelt-test-queue-uat"`
2. Ensure `/tmp/smelt-test-queue-uat` does not exist
3. Run `./target/debug/smelt serve --config /tmp/uat-server.toml --no-tui &`
4. **Expected:** Server starts successfully; `/tmp/smelt-test-queue-uat` directory created

## Failure Signals

- Garbage/corrupted terminal after TUI exits → `ratatui::restore()` not called correctly; run `reset` to fix terminal
- Orphaned Docker containers after Ctrl+C → `docker ps --filter label=smelt.job` returns containers; teardown path broken
- `smelt serve` exits immediately after startup → config validation error or port bind failure; check stderr
- TUI table blank (no header) → render() panicking silently; check `.smelt/serve.log` for error
- Tracing output appearing in TUI alternate screen → tracing subscriber init branching broken in main.rs
- `.smelt/serve.log` not created in TUI mode → tracing-appender init not running; check main.rs conditional

## Requirements Proved By This UAT

- R023 — `smelt serve` parallel dispatch daemon: live end-to-end proof of startup, job submission, dispatch, and clean Ctrl+C shutdown with no orphaned containers
- R024 — HTTP API for job submission and status: POST /api/v1/jobs, GET /api/v1/jobs verified with real responses
- R025 — Live Ratatui TUI: human-visible confirmation that TUI renders job state, updates in real time, and exits cleanly on `q` or Ctrl+C

## Not Proven By This UAT

- Concurrent dispatch cap enforcement with exactly `max_concurrent=2` and 3 submitted jobs (S01 and S02 integration tests cover this; not re-verified here)
- Auto-retry backoff timing (retry_backoff_secs is in config but not wired into backoff sleep yet — known limitation)
- Directory watch pickup of `.toml` files (covered by S02 integration tests; not re-verified here)
- `smelt status <job>` reading per-job state written by `smelt serve` (S01 integration test covers this)
- `smelt run manifest.toml` regression (covered by `cargo test --workspace`; not re-verified manually here)
- Multi-machine or remote dispatch (deferred to R027)
- Persistent queue across restarts (deferred to R028)

## Notes for Tester

- The `examples/server.toml` ships with `queue_dir = "./queue"` relative to where `smelt serve` is invoked — it will be created automatically; adjust the path if you want jobs to land elsewhere.
- If the terminal gets stuck in raw mode after a crash, run `reset` to restore it.
- The TUI is alternate-screen: the normal shell prompt is preserved in the background and returns on exit.
- Job manifests submitted via HTTP POST must be valid TOML matching the `JobManifest` schema — use `smelt init` to generate a skeleton, then edit for your target runtime.
- `retry_backoff_secs` is configurable in `server.toml` but the actual sleep is not yet wired — retries will fire immediately. Don't rely on this for timing tests.
