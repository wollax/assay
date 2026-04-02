---
estimated_steps: 8
estimated_files: 1
---

# T02: Write integration tests for the full compose lifecycle

**Slice:** S03 — ComposeProvider Lifecycle
**Milestone:** M004

## Description

Three integration tests in `crates/smelt-cli/tests/compose_lifecycle.rs` that exercise the full `ComposeProvider` lifecycle against a real Docker daemon using `docker compose`. All tests skip gracefully when Docker or `docker compose` is unavailable (same pattern as `docker_lifecycle.rs`). These tests are the primary proof for R020 and retire the `docker compose ps --format json` stability risk identified in the M004 roadmap.

Test coverage:
1. **provision + exec + teardown** — alpine:3 agent, empty services, echo hello, verify output and exit code, teardown, confirm no containers remain
2. **healthcheck wait with real Postgres** — postgres:16-alpine sidecar with `pg_isready` healthcheck, prove provision only returns after postgres is ready, exec a connectivity check from the agent
3. **teardown after exec error** — provision, exec a failing command, teardown, confirm containers removed (validates idempotent teardown path)

## Steps

1. Create `crates/smelt-cli/tests/compose_lifecycle.rs` with imports:
   ```rust
   use std::collections::HashMap;
   use indexmap::IndexMap;
   use smelt_core::compose::ComposeProvider;
   use smelt_core::manifest::{
       ComposeService, CredentialConfig, Environment, JobManifest, JobMeta,
       MergeConfig, SessionDef,
   };
   use smelt_core::provider::RuntimeProvider;
   ```

2. Add the skip helper `fn compose_provider_or_skip() -> Option<ComposeProvider>`:
   - Try `std::process::Command::new("docker").args(["compose", "version"]).output()` — if it fails, `eprintln!("Skipping: docker compose not available")` and return `None`
   - Try `ComposeProvider::new()` — if it fails, `eprintln!("Skipping: Docker daemon not available: {e}")` and return `None`
   - Return `Some(provider)` on success

3. Add `fn compose_manifest(name: &str, services: Vec<ComposeService>) -> JobManifest` helper — builds a manifest with `runtime = "compose"`, `image = "alpine:3"`, `job.repo = env!("CARGO_MANIFEST_DIR")`, and the given services. Use the same credential/session/merge stub pattern as `test_manifest_with_repo()` in `docker_lifecycle.rs`.

4. Add `fn pre_clean_containers(job_name: &str)` helper — runs `docker ps -q --filter label=smelt.job=<job_name>` and, if any IDs are returned, runs `docker rm -f <ids>`. Tolerates empty output silently. Called at the top of each test to prevent orphan containers from prior failed runs (D041 pattern, D042 variant: job-specific label value).

5. Write `#[tokio::test] async fn test_compose_provision_exec_teardown()`:
   - Call `pre_clean_containers("compose-test-basic")`.
   - Get provider via `compose_provider_or_skip()` or return.
   - Build manifest with `name = "compose-test-basic"`, no services.
   - `let container = provider.provision(&manifest).await.unwrap()`.
   - `let handle = provider.exec(&container, &["echo".into(), "hello".into()]).await.unwrap()`.
   - Assert `handle.exit_code == 0`.
   - Assert `handle.stdout.trim() == "hello"`.
   - `provider.teardown(&container).await.unwrap()`.
   - Verify no containers remain: `docker ps -q --filter label=smelt.job=compose-test-basic` returns empty output.

6. Write `#[tokio::test] async fn test_compose_healthcheck_wait_postgres()`:
   - Call `pre_clean_containers("compose-test-postgres")`.
   - Get provider or return.
   - Build a `ComposeService` for `postgres:16-alpine` with extra fields parsed from a TOML string (use `toml::from_str::<toml::Value>` or `IndexMap` directly):
     ```
     name = "postgres", image = "postgres:16-alpine"
     extra: healthcheck.test = ["CMD", "pg_isready", "-U", "postgres"]
            healthcheck.interval = "2s"
            healthcheck.retries = 10
            environment.POSTGRES_PASSWORD = "test"
     ```
     Note: for the test, build `extra` as an `IndexMap<String, toml::Value>` directly in Rust code — no TOML parsing needed.
   - Build manifest with name `"compose-test-postgres"`, services = `[postgres_service]`, image `"alpine:3"`.
   - `let container = provider.provision(&manifest).await.expect("provision must succeed after postgres is healthy")`.
   - Assert provision completed (did not time out): the fact that `provision` returns without error proves healthcheck wait worked.
   - Exec a connectivity check: `provider.exec(&container, &["sh".into(), "-c".into(), "nc -z postgres 5432 && echo ok".into()]).await`. Assert exit code 0 and stdout contains "ok" (confirms network reachability to postgres service).
   - Call `provider.teardown(&container).await.unwrap()`.
   - Verify no containers remain: `docker ps -q --filter label=smelt.job=compose-test-postgres` returns empty.

7. Write `#[tokio::test] async fn test_compose_teardown_after_exec_error()`:
   - Call `pre_clean_containers("compose-test-teardown-err")`.
   - Get provider or return.
   - Build manifest with name `"compose-test-teardown-err"`, no services.
   - `let container = provider.provision(&manifest).await.unwrap()`.
   - `let handle = provider.exec(&container, &["sh".into(), "-c".into(), "exit 1".into()]).await.unwrap()`.
   - Assert `handle.exit_code == 1`.
   - `provider.teardown(&container).await.unwrap()` — must not panic or return error.
   - Verify no containers remain: `docker ps -q --filter label=smelt.job=compose-test-teardown-err` returns empty.

8. Verify: `cargo test -p smelt-cli --test compose_lifecycle 2>&1` — exits 0 (either skipping or all three passing); `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` — 0 FAILED lines.

## Must-Haves

- [ ] `compose_lifecycle.rs` exists in `crates/smelt-cli/tests/`
- [ ] `compose_provider_or_skip()` checks both `docker compose version` and `ComposeProvider::new()`, returns `None` with `eprintln!` message when either fails
- [ ] `pre_clean_containers()` is called at the start of each test (D041/D042)
- [ ] `test_compose_provision_exec_teardown`: asserts `exit_code == 0`, `stdout.trim() == "hello"`, and confirms no containers remain after teardown
- [ ] `test_compose_healthcheck_wait_postgres`: uses `postgres:16-alpine` with a `pg_isready` healthcheck; asserts provision returns without error AND exec connectivity check returns exit 0
- [ ] `test_compose_teardown_after_exec_error`: asserts exec returns `exit_code == 1` AND teardown succeeds (no error) AND no containers remain
- [ ] `cargo test -p smelt-cli --test compose_lifecycle 2>&1` exits 0 (tests skip or pass)
- [ ] `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` — 0 FAILED lines

## Verification

- `cargo test -p smelt-cli --test compose_lifecycle 2>&1` — exits 0; either prints skip message or test results with "ok" for all three
- `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` — 0 FAILED lines across all crates
- When Docker is present: `cargo test -p smelt-cli --test compose_lifecycle -- --nocapture 2>&1 | grep -E "(PASSED|ok|FAILED|skipping)"` — three "ok" lines

## Observability Impact

- Signals added/changed: `pre_clean_containers()` helper provides a `docker ps` diagnostics surface; container absence check after teardown (`docker ps -q --filter`) is a machine-verifiable cleanup signal
- How a future agent inspects this: `docker ps --filter label=smelt.job=<name>` — confirms container lifecycle; `cargo test -p smelt-cli --test compose_lifecycle -- --nocapture` reveals full compose up/down output
- Failure state exposed: test failure messages include container IDs and exact assertions; skip messages include the reason (daemon unavailable vs. compose unavailable)

## Inputs

- `crates/smelt-core/src/compose.rs` — `ComposeProvider` full impl from T01 (must be complete before these tests can pass)
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `docker_provider_or_skip()` pattern and struct literal pattern to follow exactly
- S03-RESEARCH.md — Integration test plan with three specific test cases and their skip-check pattern

## Expected Output

- `crates/smelt-cli/tests/compose_lifecycle.rs` — new file with skip helper, compose manifest helper, pre-clean helper, and 3 integration tests
