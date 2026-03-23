---
id: T01
parent: S04
milestone: M004
provides:
  - "── Compose Services ──` section in `print_execution_plan()` — lists each service as `  name  image` with 16-char name padding"
  - "examples/job-manifest-compose.toml — canonical compose manifest with Postgres 16-alpine service, healthcheck, and minimal session"
  - "dry_run_compose_manifest_shows_services_section test — asserts section present with postgres and postgres:16-alpine"
  - "dry_run_docker_manifest_no_services_section test — asserts section absent for docker manifests"
  - "Section positioned after ── Merge ── and before ── Forge ── in the dry-run output"
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/dry_run.rs
  - examples/job-manifest-compose.toml
key_decisions:
  - "Omitted service count from section header (`── Compose Services ──` not `── Compose Services (1) ──`) — consistent with other section headers like `── Merge ──` and `── Credentials ──`; task plan had an internal inconsistency between step 1 (with count) and step 3 test predicate (without count)"
patterns_established:
  - "Conditional section in print_execution_plan(): `if !manifest.services.is_empty()` guard — same pattern applicable for any optional manifest section"
observability_surfaces:
  - "`smelt run <manifest> --dry-run` stdout — `── Compose Services ──` section lists all services by name and image when runtime=compose"
duration: 10min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Add compose services display and example manifest

**Extended `print_execution_plan()` with `── Compose Services ──` section and shipped `examples/job-manifest-compose.toml` with Postgres 16 service and healthcheck.**

## What Happened

Added a conditional `── Compose Services ──` section to `print_execution_plan()` in `run.rs`, positioned after `── Merge ──` and before `── Forge ──`. The section only renders when `manifest.services` is non-empty, using 16-char left-justified name padding for alignment.

Created `examples/job-manifest-compose.toml` as the canonical compose runtime example: `runtime = "compose"`, `image = "alpine:3"`, one `[[services]]` entry for `postgres:16-alpine` with environment and healthcheck TOML blocks, and a minimal `db-check` session.

Added two integration tests to `dry_run.rs`:
- `dry_run_compose_manifest_shows_services_section` — asserts exit 0 + stdout contains `── Compose Services ──`, `postgres`, and `postgres:16-alpine`
- `dry_run_docker_manifest_no_services_section` — asserts exit 0 + stdout does NOT contain `── Compose Services ──`

## Verification

- `cargo test -p smelt-cli --test dry_run` → `test result: ok. 15 passed; 0 failed` (both new tests + all 13 existing)
- `cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run` → exits 0; stdout shows `── Compose Services ──` with `postgres  postgres:16-alpine`
- `cargo run --bin smelt -- run examples/job-manifest.toml --dry-run` → exits 0; stdout has no `── Compose Services ──` section
- `cargo test -p smelt-core --lib` → `test result: ok. 138 passed; 0 failed` (no regressions)

## Diagnostics

`smelt run <manifest> --dry-run` stdout is the primary inspection surface. The `── Compose Services ──` section will be absent if `manifest.services` is empty (docker manifests) and present with name/image rows for compose manifests. The `dry_run_compose_manifest_shows_services_section` test failure pinpoints issues to `print_execution_plan()` in `run.rs`.

## Deviations

Task plan step 1 used `── Compose Services ({}) ──` (with count) while step 3's test predicate checked for `── Compose Services ──` (without count) — an internal inconsistency in the plan. Resolved by omitting the count to match the Must-Haves description and the test predicate, and for consistency with other section headers in the output.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — Added `── Compose Services ──` conditional section to `print_execution_plan()`
- `examples/job-manifest-compose.toml` — New canonical compose manifest with Postgres 16-alpine service and healthcheck
- `crates/smelt-cli/tests/dry_run.rs` — Added `dry_run_compose_manifest_shows_services_section` and `dry_run_docker_manifest_no_services_section` tests
