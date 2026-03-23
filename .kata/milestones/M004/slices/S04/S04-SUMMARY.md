---
id: S04
parent: M004
milestone: M004
provides:
  - "── Compose Services ──` section in `print_execution_plan()` — renders when `manifest.services` is non-empty; 16-char left-justified name padding"
  - "examples/job-manifest-compose.toml — canonical compose manifest with Postgres 16-alpine service, healthcheck, and minimal session"
  - "enum AnyProvider { Docker(DockerProvider), Compose(ComposeProvider) } — module-private dispatch enum in run.rs with full RuntimeProvider impl"
  - "Phase 3 docker-only guard removed — replaced by match dispatch on manifest.environment.runtime.as_str()"
  - "runtime = \"compose\" routes to ComposeProvider; runtime = \"docker\" routes to DockerProvider; unknown runtimes return Ok(1) with explicit error"
  - "2 new dry_run integration tests: compose shows section; docker does not"
requires:
  - slice: S01
    provides: "ComposeService type, JobManifest.services: Vec<ComposeService>, runtime validation"
  - slice: S02
    provides: "generate_compose_file(), serde_yaml type fidelity"
  - slice: S03
    provides: "ComposeProvider: RuntimeProvider impl — provision, exec, exec_streaming, collect, teardown"
affects: []
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/dry_run.rs
  - examples/job-manifest-compose.toml
key_decisions:
  - "D084 — AnyProvider enum dispatch pattern in run.rs using async fn RPITIT delegation (not Box::pin); local to binary crate"
  - "Omit service count from section header — `── Compose Services ──` not `── Compose Services (N) ──`; consistent with other section headers"
patterns_established:
  - "Conditional section in print_execution_plan(): `if !manifest.services.is_empty()` guard — extensible for any optional manifest section"
  - "AnyProvider dispatch: local enum + RuntimeProvider impl with async fn match delegation — reusable if additional runtimes added"
observability_surfaces:
  - "`smelt run <manifest> --dry-run` stdout — `── Compose Services ──` section lists all services when runtime=compose; absent when runtime=docker"
  - "`grep -n \"AnyProvider\\|Phase 3\\|Phase 4\" crates/smelt-cli/src/commands/run.rs` — locates enum definition and dispatch block"
  - "`eprintln!(\"Error: unsupported runtime. Supported: docker, compose.\")` — defence-in-depth for unknown runtimes (D077)"
drill_down_paths:
  - .kata/milestones/M004/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M004/slices/S04/tasks/T02-SUMMARY.md
duration: 20min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# S04: CLI Integration + Dry-Run

**Wired `ComposeProvider` into `smelt run` dispatch via `AnyProvider` enum and extended `--dry-run` output with `── Compose Services ──` section — M004 is complete end-to-end.**

## What Happened

**T01** added a conditional `── Compose Services ──` section to `print_execution_plan()` in `run.rs`, positioned after `── Merge ──` and before `── Forge ──`. The section only renders when `manifest.services` is non-empty. Also created `examples/job-manifest-compose.toml` — the canonical compose runtime example with `runtime = "compose"`, `alpine:3` agent image, and one `[[services]]` entry for `postgres:16-alpine` with environment vars and healthcheck. Two new integration tests were added to `dry_run.rs`: one asserting the section appears for compose manifests, one asserting it is absent for docker manifests.

**T02** replaced the Phase 3 docker-only guard (`if manifest.environment.runtime != "docker"`) with a module-private `enum AnyProvider { Docker(DockerProvider), Compose(ComposeProvider) }` implementing `RuntimeProvider` via Rust 2024 `async fn` in trait impls (RPITIT). All 5 methods delegate via exhaustive match. A single Phase 3+4 block now constructs the appropriate variant or returns `Ok(1)` with an explicit error message for unknown runtimes.

## Verification

- `cargo test --workspace` → 9 suites, 220 tests, 0 FAILED — zero regressions
- `cargo test -p smelt-cli --test dry_run` → 15 passed, 0 failed (both new tests + all 13 existing)
- `cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run` → exits 0; stdout shows `── Compose Services ──` with `postgres  postgres:16-alpine`
- `cargo run --bin smelt -- run examples/job-manifest.toml --dry-run` → exits 0; no `── Compose Services ──` section in stdout

## Requirements Advanced

- R020 — CLI dispatch now routes `runtime = "compose"` to ComposeProvider; `--dry-run` surfaces compose services; end-to-end `smelt run` compose path is complete

## Requirements Validated

- R020 — Fully validated: S01 (manifest + validation), S02 (compose file generation), S03 (ComposeProvider lifecycle with real Docker), S04 (CLI dispatch + dry-run UX) together prove the complete Docker Compose runtime capability

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

Task plan step 1 used `── Compose Services ({}) ──` (with count) while step 3's test predicate checked for `── Compose Services ──` (without count) — an internal inconsistency in the plan. Resolved by omitting the count to match the Must-Haves description and the test predicate, and for consistency with other section headers.

## Known Limitations

- `smelt run examples/job-manifest-compose.toml` (live, non-dry-run) requires Docker and docker-compose installed; `ANTHROPIC_API_KEY` must be set for Assay to execute. These are runtime prerequisites, not implementation gaps.

## Follow-ups

- None — M004 is complete. R021 (multi-machine) and R022 (budget tracking) remain deferred per roadmap.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — Added `── Compose Services ──` section to `print_execution_plan()`; added `AnyProvider` enum + `RuntimeProvider` impl; replaced Phase 3/4 with match dispatch
- `crates/smelt-cli/tests/dry_run.rs` — Added `dry_run_compose_manifest_shows_services_section` and `dry_run_docker_manifest_no_services_section` tests
- `examples/job-manifest-compose.toml` — New canonical compose manifest with Postgres 16-alpine service and healthcheck

## Forward Intelligence

### What the next slice should know
- M004 is fully complete. All R020 acceptance criteria are proven. The next milestone can build on `ComposeProvider` as a stable, tested `RuntimeProvider` impl.
- `AnyProvider` in `run.rs` is the extension point for any future runtimes — add a variant, add an arm in `generate_compose_file()`'s caller, and implement `RuntimeProvider`.

### What's fragile
- `docker compose ps --format json` NDJSON parsing was made robust in S03 but depends on Docker Compose v2+ output format. Older compose v1 (`docker-compose` binary) is not supported and would produce different output.

### Authoritative diagnostics
- `smelt run <manifest> --dry-run` stdout — fastest end-to-end verification; shows runtime, services, and plan without Docker
- `cargo test -p smelt-cli --test dry_run` — regression check for all CLI dry-run behavior including compose
- `cargo test -p smelt-core --lib` — unit + integration tests for ComposeProvider and generate_compose_file

### What assumptions changed
- RPITIT (`async fn` in trait impl) compiled cleanly for `AnyProvider` without needing `Box::pin` fallback — Rust 2024 edition handles lifetime inference for all 5 methods including the generic `exec_streaming<F>`.
