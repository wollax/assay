# Smelt — Project Context

## What Smelt Is

Smelt is a job runner for AI coding sessions. It reads a job manifest, provisions Docker containers with the right image and credentials, mounts the repo, delegates orchestration to Assay inside the container, monitors progress, collects the result branch, and tears everything down. The output is a branch ready for human review.

A user who needs ephemeral containerized environments runs `smelt run` instead of `assay orchestrate` directly — same orchestration logic, with infrastructure provisioning wrapped around it.

## Architecture

- **Role:** Pure infrastructure layer — Smelt provisions environments, Assay owns orchestration (D001)
- **Assay integration:** Shell out to `assay` CLI; no crate dependency (D002)
- **Runtime abstraction:** Pluggable `RuntimeProvider` trait — Docker first, Compose/K8s via same trait (D004)
- **Repo delivery:** Bind-mount host repo into container at `/workspace` (D013)
- **Credential injection:** Environment variable passthrough (D014)
- **Manifest authorship:** Assay generates manifests, Smelt consumes (D010)

## Workspace Structure

```
crates/
  smelt-core/   — manifest types, RuntimeProvider trait, DockerProvider, AssayInvoker,
                  ResultCollector, JobMonitor, GitOps, SmeltConfig, SmeltError
  smelt-cli/    — smelt binary: `run` and `status` subcommands
examples/
  job-manifest.toml   — valid example manifest
  bad-manifest.toml   — invalid manifest for testing
```

## Current State

**M001 complete.** Smelt runs the full Docker-first infrastructure pipeline:

- `smelt run manifest.toml` provisions a container, bind-mounts the host repo, executes Assay via a translated TOML manifest, collects the result branch, and tears down
- `smelt run --dry-run` validates the manifest and prints the execution plan without touching Docker
- `smelt status` shows live job progress (phase, container ID, sessions, elapsed time)
- Container lifecycle is robust: timeout enforcement, Ctrl+C handling, and idempotent teardown
- 20 Docker integration tests verify the full pipeline including multi-session manifests, failure-path orphan safety, timeout, and cancellation

## Known Issues

- `run_without_dry_run_attempts_docker` in `crates/smelt-cli/tests/dry_run.rs` is a pre-existing failing test — the test logic incorrectly asserts Docker unavailability when Docker is present. Should be fixed before M002.
- AssayInvoker contract validated against real `assay` binary (M002-S01/S02 — D043 supersedes D029).
- Integration tests install `git` via `apk add` — require Alpine CDN network access; will fail in air-gapped CI.
- `.assay/` directory may be written to the bind-mounted host repo during live runs; no `.gitignore` entry exists yet.

## Milestones

| Milestone | Title | Status |
|-----------|-------|--------|
| M001 | Docker-First Infrastructure MVP | ✅ Complete (2026-03-17) |
| M002 | Real Assay Integration | 🔄 In Progress (S01 ✅, S02 ✅, S03-S04 pending) |

## Technology Decisions

| Decision | Choice | Notes |
|----------|--------|-------|
| Docker client | bollard | bollard 0.20 query params at `bollard::query_parameters::*` |
| Async traits | RPITIT (not async_trait) | Rust 2024 edition; makes trait not object-safe — use generics not dyn |
| Manifest parsing | deny_unknown_fields on all structs | Strict schema enforcement |
| Container keep-alive | sleep 3600 CMD, work via exec | Container stays running between exec calls |
| Cancellation | Generic future (not CancellationToken) | oneshot in tests, ctrl_c() in prod |
| State file | .smelt/run-state.toml TOML | Single-job model; concurrent jobs would clobber |
| Manifest delivery | Base64-encode + exec base64 -d | Avoids heredoc quoting issues |
| Result collection | Host-side via GitOps | Bind-mount means commits already on host |
