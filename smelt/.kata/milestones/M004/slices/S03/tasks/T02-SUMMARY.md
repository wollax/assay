---
id: T02
parent: S03
milestone: M004
provides:
  - crates/smelt-cli/tests/compose_lifecycle.rs — 3 integration tests covering full compose lifecycle
  - compose_provider_or_skip() — skip guard checking docker compose version + ComposeProvider::new()
  - compose_manifest() — test manifest builder for compose runtime
  - pre_clean_containers() — orphan-cleanup helper (D041/D042 pattern)
  - assert_no_containers_for_job() — post-teardown verification via docker ps --filter label
  - Bug fix: smelt-agent service now includes command: [sleep, "3600"] in compose YAML (container kept alive for bollard exec)
  - Bug fix: removed custom named network from generate_compose_file; rely on Docker Compose default project network for inter-service DNS
  - Updated 6 snapshot tests in smelt-core to match new compose YAML shape
affects: [S03]
key_files:
  - crates/smelt-cli/tests/compose_lifecycle.rs
  - crates/smelt-core/src/compose.rs
key_decisions:
  - "D075: smelt-agent uses Docker Compose default project network (no explicit networks: key). Custom named network isolated the agent from user services — default network gives all services shared DNS resolution."
  - "smelt-agent service gets command: [sleep, '3600'] in generated compose YAML — alpine:3 exits immediately without it; docker compose ps only shows running containers so agent container ID was never captured"
patterns_established:
  - "Compose integration test pattern: compose_provider_or_skip() + pre_clean_containers() + assert_no_containers_for_job() — matches docker_lifecycle.rs style"
  - "Agent keep-alive pattern: compose YAML always sets command: [sleep, 3600] on smelt-agent service"
drill_down_paths:
  - .kata/milestones/M004/slices/S03/tasks/T02-PLAN.md
duration: ~45min
verification_result: pass
completed_at: 2026-03-22T16:00:00Z
blocker_discovered: false
---

# T02: Write integration tests for the full compose lifecycle

**3 integration tests pass against real Docker; 2 T01 bugs fixed (keep-alive command + default network)**

## What Happened

Wrote `crates/smelt-cli/tests/compose_lifecycle.rs` with three integration tests:

1. **test_compose_provision_exec_teardown** — provisions alpine:3 agent, no sidecars, runs `echo hello`, asserts exit 0 and `stdout.trim() == "hello"`, tears down, confirms no containers with `smelt.job=compose-test-basic` label remain.

2. **test_compose_healthcheck_wait_postgres** — provisions a `postgres:16-alpine` sidecar with `pg_isready` healthcheck (interval=2s, retries=10). Proves provision only returns after postgres is healthy (fact that it returns without error). Execs `nc -z postgres 5432 && echo ok` from the agent, asserts exit 0 and stdout contains "ok" — confirms compose-network DNS and TCP reachability.

3. **test_compose_teardown_after_exec_error** — provisions, runs `exit 1`, asserts exit_code == 1, calls teardown (must not error), confirms no containers remain.

Running the tests revealed two bugs in the T01 `generate_compose_file` implementation:

**Bug 1: missing keep-alive command.** `alpine:3` exits immediately without an explicit command. `docker compose ps --format json` only shows running containers; after the container exits it disappears from ps output, so the provision loop can never capture the agent container ID. Fix: add `command: [sleep, "3600"]` to the smelt-agent service block, consistent with `DockerProvider` which uses `sleep 3600` explicitly.

**Bug 2: custom network isolated agent from user services.** The generated compose YAML placed smelt-agent on a custom named network (`smelt-<project_name>`) while user services (postgres, etc.) had no explicit network and were placed on Docker Compose's automatic default network. These two networks are isolated — DNS name resolution failed (`nc: bad address 'postgres'`). Fix: remove the custom `networks:` key from smelt-agent and top-level `networks:` section entirely; rely on Docker Compose's automatic default project network, which all services share.

Both fixes required updating 6 snapshot tests in `smelt-core/src/compose.rs`. All 138 unit tests + 3 integration tests pass; workspace total 0 FAILED.

## Deviations

- Fixed two bugs in T01's `generate_compose_file` function (`command:` and network topology) — not in scope for T02 but required to make tests pass. Both are correctness bugs, not optional enhancements.
- Added `indexmap` to `smelt-cli` dev-dependencies (needed to construct `ComposeService.extra`).

## Files Created/Modified

- `crates/smelt-cli/tests/compose_lifecycle.rs` — new integration test file (3 tests + helpers)
- `crates/smelt-core/src/compose.rs` — added `command: [sleep, 3600]` to agent service; removed custom network; updated 6 snapshot tests
- `crates/smelt-cli/Cargo.toml` — added `indexmap.workspace = true` to `[dev-dependencies]`
