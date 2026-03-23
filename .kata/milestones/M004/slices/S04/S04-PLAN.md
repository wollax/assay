# S04: CLI Integration + Dry-Run

**Goal:** Wire `ComposeProvider` into `smelt run` dispatch, extend `print_execution_plan()` with a `── Compose Services ──` section, and ship `examples/job-manifest-compose.toml` — completing the M004 milestone so `smelt run manifest.toml` with `runtime = "compose"` works end-to-end and `--dry-run` shows compose services.

**Demo:** `smelt run examples/job-manifest-compose.toml --dry-run` exits 0 and prints `── Compose Services ──` with service names and images. `smelt run examples/job-manifest.toml --dry-run` is unchanged. When Docker is available, `smelt run examples/job-manifest-compose.toml` provisions the compose stack, runs Assay in the agent, and tears down.

## Must-Haves

- `smelt run manifest.toml` with `runtime = "compose"` dispatches to `ComposeProvider` (not the docker-only guard)
- `smelt run manifest.toml` with `runtime = "docker"` still dispatches to `DockerProvider` — zero regressions
- `smelt run examples/job-manifest-compose.toml --dry-run` exits 0 and prints `── Compose Services ──` listing each service name and image
- `examples/job-manifest-compose.toml` exists with Postgres 16 service, healthcheck, and a valid smelt-agent image
- `cargo test --workspace` green — all existing tests pass, new dry-run tests pass

## Proof Level

- This slice proves: final-assembly (end-to-end CLI dispatch wiring)
- Real runtime required: yes — for the live `smelt run` compose path (Docker + docker-compose required); no for dry-run tests
- Human/UAT required: no — all acceptance criteria are machine-verifiable

## Verification

```bash
# 1. All workspace tests pass (no regressions + new tests)
cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"

# 2. Compose dry-run integration tests specifically
cargo test -p smelt-cli --test dry_run 2>&1 | grep -E "(test result|FAILED|compose)"

# 3. Spot-check compose dry-run output manually
cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run

# Expected stdout contains:
#   ── Compose Services ──
#   postgres    postgres:16-alpine
#   ═══ End Plan ═══

# 4. Docker path unaffected (dry-run)
cargo run --bin smelt -- run examples/job-manifest.toml --dry-run
# Expected: no "── Compose Services ──" section; exits 0
```

## Observability / Diagnostics

- Runtime signals: existing `tracing::info!` / `eprintln!` in `run_with_cancellation` unchanged; `AnyProvider` delegates transparently so observability is identical to each inner provider
- Inspection surfaces: `-- dry-run` output shows `── Compose Services ──` section for compose manifests; `── Environment ──` still shows `Runtime: compose`
- Failure visibility: `AnyProvider::_` arm returns explicit error message; `ComposeProvider::new()` failures surface as "failed to connect to Docker daemon" (same as DockerProvider); Phase 3 `_` arm provides defence-in-depth error message for any runtime that passes validation but isn't matched
- Redaction constraints: no new secrets introduced; credentials are already handled by existing `resolve_credentials()` path

## Integration Closure

- Upstream surfaces consumed:
  - `smelt_core::ComposeProvider` (from S03) — failable constructor, full `RuntimeProvider` impl
  - `smelt_core::provider::RuntimeProvider` — 5 methods: `provision`, `exec`, `exec_streaming`, `collect`, `teardown`
  - `JobManifest.services: Vec<ComposeService>` (from S01) — iterated in `print_execution_plan()`
  - `generate_compose_file()` (from S02) — consumed inside `ComposeProvider::provision()`, not in `run.rs`
- New wiring introduced in this slice:
  - `enum AnyProvider { Docker(DockerProvider), Compose(ComposeProvider) }` in `run.rs`
  - `RuntimeProvider` impl on `AnyProvider` delegating all 5 methods via `async fn`
  - Phase 3 match replacing the docker-only guard — constructs the right `AnyProvider` variant
  - `── Compose Services ──` section in `print_execution_plan()`
  - `examples/job-manifest-compose.toml` as the canonical end-to-end example
- What remains before the milestone is truly usable end-to-end: nothing — this slice completes M004

## Tasks

- [x] **T01: Add compose services display and example manifest** `est:30m`
  - Why: closes the dry-run UX gap — `print_execution_plan()` must show `── Compose Services ──` for compose manifests; the example manifest enables both dry-run tests and live-run verification
  - Files: `crates/smelt-cli/src/commands/run.rs`, `examples/job-manifest-compose.toml`, `crates/smelt-cli/tests/dry_run.rs`
  - Do:
    1. In `print_execution_plan()`, add a `── Compose Services ──` block immediately after the `── Environment ──` section: emit it only when `!manifest.services.is_empty()`; print `Services (N) ──` header then each service as `  {name:<16} {image}`
    2. Create `examples/job-manifest-compose.toml` — copy `examples/job-manifest.toml` as template; change `runtime = "compose"`; add `[[services]]` entry for `postgres:16-alpine` with `name = "postgres"`, `image = "postgres:16-alpine"`, `environment = { POSTGRES_PASSWORD = "smelt" }`, and a healthcheck (`test = ["CMD-SHELL", "pg_isready -U postgres"]`, `interval = "5s"`, `retries = 5`); use `"."` for `job.repo` with a comment that it should be an absolute path for real runs
    3. In `dry_run.rs`, add test `dry_run_compose_manifest_shows_services_section`: run `smelt run examples/job-manifest-compose.toml --dry-run`, assert stdout contains `── Compose Services ──`, `postgres`, and `postgres:16-alpine`
    4. Add test `dry_run_docker_manifest_no_services_section`: run `smelt run examples/job-manifest.toml --dry-run`, assert stdout does NOT contain `── Compose Services ──`
    5. Run `cargo test -p smelt-cli --test dry_run` — confirm new tests pass and existing tests are unaffected
  - Verify: `cargo test -p smelt-cli --test dry_run 2>&1 | grep -E "(test result|FAILED)"` shows all passing; manual `cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run` shows `── Compose Services ──`
  - Done when: both new dry-run tests pass and `── Compose Services ──` with `postgres / postgres:16-alpine` appears in stdout for the compose manifest; `── Compose Services ──` absent for the docker manifest

- [x] **T02: Wire `AnyProvider` dispatch in `run_with_cancellation`** `est:30m`
  - Why: replaces the docker-only guard (Phase 3) with a `ComposeProvider`-aware dispatch — the final assembly step that makes `smelt run manifest.toml` with `runtime = "compose"` work end-to-end
  - Files: `crates/smelt-cli/src/commands/run.rs`
  - Do:
    1. At the top of `run_with_cancellation`, define a local enum and impl:
       ```rust
       enum AnyProvider {
           Docker(smelt_core::docker::DockerProvider),
           Compose(smelt_core::ComposeProvider),
       }
       ```
       Implement `RuntimeProvider` for `AnyProvider` by delegating all 5 methods (`provision`, `exec`, `exec_streaming`, `collect`, `teardown`) to the inner variant via `async fn` match arms; for `exec_streaming`, `output_cb: F` where `F: FnMut(&str) + Send + 'static` is forwarded by move into the matching arm
    2. Replace Phase 3 (the `if manifest.environment.runtime != "docker"` block) with a match that constructs `AnyProvider`:
       ```rust
       let provider = match manifest.environment.runtime.as_str() {
           "docker" => AnyProvider::Docker(DockerProvider::new()
               .with_context(|| "failed to connect to Docker daemon")?),
           "compose" => AnyProvider::Compose(smelt_core::ComposeProvider::new()
               .with_context(|| "failed to connect to Docker daemon")?),
           other => {
               eprintln!("Error: unsupported runtime `{other}`. Supported: docker, compose.");
               return Ok(1);
           }
       };
       ```
    3. Remove the now-separate Phase 4 `DockerProvider::new()` line (it was constructing `provider` after Phase 3 — the match above replaces both Phase 3 guard and Phase 4 construction)
    4. Replace all references to `provider` (which was `DockerProvider`) with `provider` (now `AnyProvider`) — the method signatures are identical via the trait; ensure `provider` is typed as implementing `RuntimeProvider` or accessed via the impl methods directly
    5. Run `cargo build -p smelt-cli` to confirm the enum impl compiles without errors; fix any RPITIT-related compiler hints
    6. Run `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` — confirm zero regressions; all existing dry-run and docker lifecycle tests pass
    7. If Docker is available locally, run `cargo test -p smelt-cli --test compose_lifecycle` to confirm S03 tests still pass with the build changes
  - Verify: `cargo test --workspace` all green; `cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run` exits 0; `cargo run --bin smelt -- run examples/job-manifest.toml` still attempts Docker (does not print "unsupported runtime" error)
  - Done when: `cargo test --workspace` passes with 0 failures; `run_without_dry_run_attempts_docker` test passes; compose manifest dry-run exits 0; `manifest.environment.runtime = "compose"` no longer triggers the "unsupported runtime" error path

## Files Likely Touched

- `crates/smelt-cli/src/commands/run.rs` — `AnyProvider` enum + `RuntimeProvider` impl, Phase 3/4 replacement, `print_execution_plan()` compose services section
- `crates/smelt-cli/tests/dry_run.rs` — 2 new integration tests for compose dry-run
- `examples/job-manifest-compose.toml` — new example manifest
