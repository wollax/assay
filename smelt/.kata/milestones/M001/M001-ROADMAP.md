# M001: Docker-First Infrastructure MVP

**Vision:** Smelt becomes a job runner for AI coding sessions: it reads a job manifest, provisions Docker containers with the right image and credentials, mounts the repo, delegates orchestration to Assay inside the container, monitors progress, collects the result branch, and tears everything down. The output is a branch ready for human review. A user who needs ephemeral containerized environments runs `smelt run` instead of `assay orchestrate` directly — same orchestration logic, with infrastructure provisioning wrapped around it.

## Success Criteria

- User can run `smelt run manifest.toml` and get a result branch on the host repo containing merged agent work, with all containers cleaned up
- A multi-session job with dependencies between sessions executes in the correct order inside Docker, with each session's output available to the next
- Container failures (OOM, timeout, crash) are detected, reported, and cleaned up — no orphaned containers or dangling volumes
- Credentials are resolved from the host environment and injected into containers without being written to disk inside the container
- The full deploy → execute → collect → teardown cycle completes without manual intervention for the happy path
- `smelt status` shows live job progress while containers are running
- `smelt run --dry-run` validates a manifest and prints the execution plan without touching Docker

## Key Risks / Unknowns

- **bollard Docker exec reliability** — The bollard crate's exec API is the primary interface for running commands inside containers. If exec attach/stream proves unreliable for long-running processes, the fallback is shelling out to `docker exec`. This risk is retired in S02.
- **Assay CLI contract stability** — Smelt shells out to `assay orchestrate` inside containers. If Assay's CLI flags or output format changes, Smelt's integration breaks. Mitigated by strict manifest schema and version pinning in the image. This risk is exercised in S03.

## Proof Strategy

- bollard exec reliability → retire in S02 by provisioning a real Docker container, executing a multi-step command sequence inside it, streaming output, and tearing down cleanly
- Assay CLI integration → retire in S03 by running `assay orchestrate` (or a mock standing in for it) inside a provisioned container with a mounted repo and collecting structured output

## Verification Classes

- Contract verification: Unit tests on manifest parsing/validation, RuntimeProvider trait contracts, credential resolution logic, result extraction
- Integration verification: Docker container lifecycle with bollard (real Docker daemon required), `assay orchestrate` invocation inside container, repo mount read/write fidelity, branch extraction across container boundary
- Operational verification: Timeout enforcement, container cleanup on failure/Ctrl+C, `smelt status` live output, graceful shutdown
- UAT / human verification: Full `smelt run` with a real multi-session manifest producing a reviewable branch

## Milestone Definition of Done

This milestone is complete only when all are true:

- `smelt run manifest.toml` provisions containers, executes sessions via Assay, collects the result branch, and tears down — demonstrated with a real Docker environment and a multi-session job
- Container lifecycle is robust: containers are always cleaned up on success, failure, timeout, and Ctrl+C
- Credentials are injected securely and used successfully by Assay inside the container
- `smelt run --dry-run` validates manifests and prints the execution plan without side effects
- `smelt status` shows meaningful live progress for a running job
- The final integration slice exercises the full pipeline through the real `smelt run` entrypoint, not just individual subsystems in isolation

## Requirement Coverage

> **Note:** No `.kata/REQUIREMENTS.md` exists. Operating in legacy compatibility mode. The `.planning/REQUIREMENTS.md` covers v0.1.0 (all complete, all being gutted). M001 defines a new capability set for the pivoted Smelt. New requirements should be formalized in a future `.kata/REQUIREMENTS.md`.

- Covers: Manifest parsing (new format), Docker provisioning, Assay delegation, result collection, credential management, job monitoring, teardown
- Partially covers: Multi-runtime support (Docker only in M001; Compose and K8s deferred)
- Leaves for later: Compose runtime (M002+), Kubernetes runtime (M003+), PR creation/forge integration, cost tracking, multi-machine coordination, web/mobile companion
- Orphan risks: None — v0.1.0 requirements are fully superseded by the pivot

## Slices

- [x] **S01: Scaffold, Manifest & Dry-Run CLI** `risk:low` `depends:[]`
  > After this: `smelt run manifest.toml --dry-run` parses a complete job manifest, validates all fields (sessions, environment, credentials, merge config), resolves credential sources from environment, and prints the execution plan — or rejects malformed input with clear errors. Verified by running `smelt run --dry-run` against valid and invalid manifests from the terminal.

- [x] **S02: Docker Container Provisioning & Teardown** `risk:high` `depends:[S01]`
  > After this: `smelt run manifest.toml` creates a Docker container from the specified image with configured resource limits and environment variables, executes a health-check command inside it, streams the output to the terminal, and removes the container on completion or failure. Verified by running `smelt run` against a real Docker daemon — the container appears in `docker ps` during execution and is gone after.

- [x] **S03: Repo Mount & Assay Execution** `risk:medium` `depends:[S02]`
  > After this: `smelt run manifest.toml` provisions a container, bind-mounts the host repo at the specified ref, executes `assay orchestrate` (or a mock standing in for it) inside the container with the job's session definitions, and streams orchestration output to the terminal. Verified by inspecting the mounted repo inside the container and confirming Assay's output appears in the terminal stream.

- [x] **S04: Result Collection & Branch Output** `risk:medium` `depends:[S03]`
  > After this: After `smelt run` completes, the target branch specified in the manifest exists on the host repository containing the merged work from all sessions. The results are extracted from the container's repo mount before teardown. Verified by checking out the target branch on the host and confirming it contains commits from the agent sessions.

- [x] **S05: Job Monitoring, Timeout & Graceful Shutdown** `risk:low` `depends:[S03]`
  > After this: `smelt status` shows live progress for a running job (active sessions, elapsed time, container health). Jobs exceeding their timeout are terminated and cleaned up. Ctrl+C during `smelt run` triggers graceful container teardown with no orphans. Verified by running a long job, checking `smelt status` in another terminal, sending Ctrl+C, and confirming containers are removed.

- [x] **S06: End-to-End Integration** `risk:low` `depends:[S04,S05]`
  > After this: `smelt run` with a multi-session manifest provisions containers, executes all sessions through Assay with dependency ordering, collects results into the target branch, tears down all containers, and handles failures gracefully — the full deploy → execute → collect → teardown cycle works through the real `smelt run` entrypoint with real Docker. Verified by running a complete multi-session job and confirming the output branch, container cleanup, and error handling in a real environment.

## Boundary Map

### S01 → S02

Produces:
- `manifest.rs` → `JobManifest`, `Environment`, `SessionDef`, `CredentialConfig`, `MergeConfig` (serde structs with validation)
- `provider.rs` → `RuntimeProvider` trait with async methods: `provision()`, `exec()`, `collect()`, `teardown()`
- `error.rs` → `SmeltError` enum with variants for manifest, provider, credential, config errors
- `config.rs` → `SmeltConfig` loader (reads `.smelt/config.toml`)
- `cli: smelt run` → Accepts manifest path, `--dry-run` flag; calls `RuntimeProvider` methods when not dry-run

Consumes:
- nothing (first slice)

### S02 → S03

Produces:
- `docker.rs` → `DockerProvider` implementing `RuntimeProvider`: creates containers with image/resources/env, execs commands, streams output, removes containers, handles failures
- Container lifecycle guarantees: containers are always cleaned up on success or failure
- `bollard` integration patterns: client init, image pull, container create/start/exec/stop/remove

Consumes from S01:
- `provider.rs` → `RuntimeProvider` trait (implemented by `DockerProvider`)
- `manifest.rs` → `Environment` (image, resources), `CredentialConfig` (env vars to inject)
- `error.rs` → `SmeltError::Provider` variant for Docker errors

### S03 → S04

Produces:
- Repo mount logic: bind-mounts host repo into container at a known path
- Assay invocation: constructs and executes `assay orchestrate` command inside container with session definitions
- Output streaming: real-time forwarding of Assay's stdout/stderr to host terminal
- `assay.rs` → `AssayInvoker` that translates `JobManifest` sessions/merge config into `assay orchestrate` CLI args

Consumes from S02:
- `docker.rs` → `DockerProvider::provision()` with volume mount support, `DockerProvider::exec()` for running assay

### S03 → S05

Produces:
- Exec handle with output stream → used by monitoring to track live progress
- Session-level status from Assay output parsing → used by `smelt status`

Consumes from S02:
- `docker.rs` → `DockerProvider::exec()` with stream attachment

### S04 → S06

Produces:
- `collector.rs` → `ResultCollector` that extracts branch state from container's repo mount, creates target branch on host repo
- Branch verification: confirms target branch exists and contains expected commits

Consumes from S03:
- Repo mount path (where to read results from inside container)
- Assay completion signal (when to start collection)

Consumes from S01:
- `manifest.rs` → `MergeConfig` (target branch name, merge strategy)
- `git/` module → branch creation, commit operations on host repo

### S05 → S06

Produces:
- `monitor.rs` → `JobMonitor` tracking container health, session progress, elapsed time
- `smelt status` CLI command → reads monitor state, prints live progress
- Timeout enforcement: kills containers exceeding session/job timeouts
- Signal handling: Ctrl+C triggers graceful `teardown()` on all active containers

Consumes from S03:
- Exec output stream (for progress parsing)
- Container IDs (for health checking)

Consumes from S02:
- `DockerProvider::teardown()` for cleanup on timeout/signal

### S06 (integration)

Produces:
- Integration test suite exercising full `smelt run` pipeline with real Docker
- Error recovery scenarios: session failure, timeout, Ctrl+C, Docker daemon errors
- Multi-session dependency ordering verification

Consumes from S04:
- `ResultCollector` (full result extraction pipeline)

Consumes from S05:
- `JobMonitor`, timeout enforcement, signal handling (full lifecycle management)
