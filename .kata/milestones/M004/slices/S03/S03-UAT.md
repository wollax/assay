# S03: ComposeProvider Lifecycle — UAT

**Milestone:** M004
**Written:** 2026-03-22

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: All acceptance criteria are machine-verifiable via `cargo test`. The integration tests exercise real Docker, real `docker compose` subprocesses, and real healthcheck polling — no human interaction needed to prove the lifecycle works.

## Preconditions

- Docker daemon running locally with `docker compose` v2 available (`docker compose version` exits 0)
- `cargo` and Rust toolchain installed
- No orphan containers from prior failed test runs (the tests clean up automatically via `pre_clean_containers()`)

## Smoke Test

```bash
cargo test -p smelt-cli --test compose_lifecycle
# → test result: ok. 3 passed; 0 failed
```

## Test Cases

### 1. Provision + Exec + Teardown (no sidecars)

```bash
cargo test -p smelt-cli --test compose_lifecycle test_compose_provision_exec_teardown -- --nocapture
```

1. `ComposeProvider::new()` connects to Docker daemon
2. `provision()` runs `docker compose up -d` with `alpine:3` agent (no sidecars)
3. `exec(&container, &["echo", "hello"])` runs inside agent container
4. **Expected:** exit code 0, stdout == "hello"
5. `teardown()` runs `docker compose down --remove-orphans`
6. **Expected:** `docker ps --filter label=smelt.job=compose-test-basic` returns empty — no containers remain

### 2. Healthcheck Wait with Real Postgres

```bash
cargo test -p smelt-cli --test compose_lifecycle test_compose_healthcheck_wait_postgres -- --nocapture
```

1. Provisions `postgres:16-alpine` sidecar with `pg_isready` healthcheck (`interval=2s, retries=10`)
2. `provision()` polls `docker compose ps --format json` until postgres reports `Health == "healthy"`
3. **Expected:** provision returns without timeout (proves healthcheck polling works against `docker compose ps` NDJSON output)
4. `exec(&container, &["sh", "-c", "nc -z postgres 5432 && echo ok"])` from the agent container
5. **Expected:** exit code 0, stdout contains "ok" — confirms the agent can reach postgres by name on the shared default project network

### 3. Teardown After Exec Error

```bash
cargo test -p smelt-cli --test compose_lifecycle test_compose_teardown_after_exec_error -- --nocapture
```

1. Provisions agent with no sidecars
2. `exec(&container, &["sh", "-c", "exit 1"])` — deliberate failure
3. **Expected:** exit code 1 returned (not an error/panic)
4. `teardown()` called — **Expected:** returns `Ok(())` (fault-tolerant teardown per D023/D038)
5. **Expected:** `docker ps --filter label=smelt.job=compose-test-teardown-err` returns empty

## Edge Cases

### Docker Unavailable (skip guard)

```bash
# With Docker stopped or docker compose not installed:
cargo test -p smelt-cli --test compose_lifecycle
```

**Expected:** all 3 tests print a skip message and exit 0 — no panics, no failures.

### Full Workspace Regression Check

```bash
cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
```

**Expected:** all suites report `test result: ok`, zero `FAILED` lines.

## Failure Signals

- Any test reporting `FAILED` — indicates a regression in the compose lifecycle
- `"timed out waiting for services to become healthy after 120s"` in provision error — Postgres healthcheck not passing within 2 minutes; check `docker compose ps` output for service state
- `"service postgres became unhealthy"` — Postgres reported unhealthy state; check container logs with `docker logs <container>`
- `nc: bad address 'postgres'` in exec stderr — DNS resolution failed; check that D082 (default project network, no custom network) is in effect in the generated YAML
- `docker ps --filter` returns non-empty after teardown — `compose down` failed; check `tracing::warn!` output for the teardown error

## Requirements Proved By This UAT

- R020 (Docker Compose runtime for multi-service environments) — the three integration tests collectively prove: provision generates a valid compose file and starts containers; healthcheck polling waits for service readiness before returning; exec works inside the agent container; inter-service DNS works on the shared default network; teardown removes all containers cleanly; teardown is fault-tolerant after exec errors.

## Not Proven By This UAT

- End-to-end `smelt run examples/job-manifest-compose.toml` — CLI dispatch in `run.rs` is not yet wired (S04 scope). The `ComposeProvider` implementation is proven; the CLI entrypoint is not.
- `--dry-run` with compose services — S04 scope.
- Ctrl+C signal handling for compose teardown — not exercised by these tests; operational proof deferred to S04/manual verification.
- Multi-platform Docker behaviour — tested on macOS with Docker Desktop; Linux CI behaviour not separately verified.

## Notes for Tester

- Tests take ~15-30 seconds each when Docker is present (postgres image pull on first run can add 30-60s).
- The Postgres test requires `nc` (netcat) in `alpine:3` — it's included by default.
- If tests fail due to leftover containers from a prior run, `docker ps --filter label=smelt.job` lists them; `docker rm -f <ids>` cleans them up. The tests also auto-clean at the start via `pre_clean_containers()`.
- The `--nocapture` flag shows stderr progress lines (`Waiting for postgres to be healthy...`) and tracing output for observability verification.
