---
estimated_steps: 5
estimated_files: 3
---

# T01: Add compose services display and example manifest

**Slice:** S04 — CLI Integration + Dry-Run
**Milestone:** M004

## Description

Extend `print_execution_plan()` with a `── Compose Services ──` section that appears after `── Environment ──` when the manifest has `[[services]]` entries. Create `examples/job-manifest-compose.toml` as the canonical compose example. Add two integration tests in `dry_run.rs` that assert the section is present for compose manifests and absent for docker manifests.

This task proves the dry-run UX requirement from the M004 success criteria without needing Docker: `smelt run --dry-run` with a compose manifest shows `── Compose Services ──` and exits 0.

## Steps

1. Open `crates/smelt-cli/src/commands/run.rs`. In `print_execution_plan()`, locate the block that prints `── Environment ──`. Immediately after that block's closing `println!()`, add the compose services section:
   ```rust
   // ── Compose Services ──
   if !manifest.services.is_empty() {
       println!("── Compose Services ({}) ──", manifest.services.len());
       for svc in &manifest.services {
           println!("  {:<16} {}", svc.name, svc.image);
       }
       println!();
   }
   ```

2. Create `examples/job-manifest-compose.toml`. Use `examples/job-manifest.toml` as a structural template. Key differences: `runtime = "compose"`, image set to `"alpine:3"` (agent image; same as job-manifest.toml), add one `[[services]]` entry:
   ```toml
   [[services]]
   name = "postgres"
   image = "postgres:16-alpine"

   [services.environment]
   POSTGRES_PASSWORD = "smelt"

   [services.healthcheck]
   test = ["CMD-SHELL", "pg_isready -U postgres"]
   interval = "5s"
   retries = 5
   ```
   Keep `job.repo = "."` with a comment noting it should be an absolute path for real (non-dry-run) execution. Sessions should reference postgres connectivity or a simple shell command — keep it minimal.

3. Open `crates/smelt-cli/tests/dry_run.rs`. Add test `dry_run_compose_manifest_shows_services_section` after the existing forge section tests:
   ```rust
   #[test]
   fn dry_run_compose_manifest_shows_services_section() {
       smelt()
           .args(["run", "examples/job-manifest-compose.toml", "--dry-run"])
           .assert()
           .success()
           .stdout(
               predicate::str::contains("── Compose Services ──")
                   .and(predicate::str::contains("postgres"))
                   .and(predicate::str::contains("postgres:16-alpine")),
           );
   }
   ```

4. Add test `dry_run_docker_manifest_no_services_section` asserting the docker manifest does NOT print the services section:
   ```rust
   #[test]
   fn dry_run_docker_manifest_no_services_section() {
       smelt()
           .args(["run", "examples/job-manifest.toml", "--dry-run"])
           .assert()
           .success()
           .stdout(predicate::str::contains("── Compose Services ──").not());
   }
   ```

5. Run `cargo test -p smelt-cli --test dry_run` to confirm both new tests pass and all existing dry-run tests are unaffected. Also run `cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run` manually to verify the output format.

## Must-Haves

- [ ] `print_execution_plan()` emits `── Compose Services ──` section when `manifest.services` is non-empty; the section lists each service as `  name  image` with at least 16-char name padding
- [ ] `── Compose Services ──` section is absent for manifests with empty `manifest.services` (i.e., the docker manifest)
- [ ] `examples/job-manifest-compose.toml` exists, parses without error, and passes `JobManifest::validate()` (proven by the dry-run exit 0)
- [ ] `examples/job-manifest-compose.toml` contains a `postgres:16-alpine` service with healthcheck TOML fields
- [ ] `dry_run_compose_manifest_shows_services_section` test passes: stdout contains `── Compose Services ──`, `postgres`, and `postgres:16-alpine`
- [ ] `dry_run_docker_manifest_no_services_section` test passes: stdout does NOT contain `── Compose Services ──`
- [ ] All previously-passing dry-run tests still pass (zero regressions)

## Verification

- `cargo test -p smelt-cli --test dry_run 2>&1 | grep -E "(test result|FAILED)"` → `test result: ok. N passed; 0 failed`
- `cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run` → exits 0; stdout contains `── Compose Services ──` followed by `postgres` and `postgres:16-alpine`
- `cargo run --bin smelt -- run examples/job-manifest.toml --dry-run` → exits 0; stdout does NOT contain `── Compose Services ──`
- `cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"` → no regressions in smelt-core (no changes there, but confirm)

## Observability Impact

- Signals added/changed: `print_execution_plan()` now includes compose services in the dry-run output — this is the primary user-visible observability surface for the compose runtime path at dry-run time
- How a future agent inspects this: `smelt run <manifest> --dry-run` stdout — the `── Compose Services ──` section lists all services by name and image, giving an immediate overview of what will be provisioned
- Failure state exposed: if the section is missing for a compose manifest, `dry_run_compose_manifest_shows_services_section` test failure pinpoints the issue to `print_execution_plan()` in `run.rs`

## Inputs

- `crates/smelt-cli/src/commands/run.rs` — `print_execution_plan()` function to extend; `manifest.services` field access pattern
- `crates/smelt-core/src/manifest.rs` — `ComposeService` struct: `name: String`, `image: String` (from S01)
- `examples/job-manifest.toml` — structural template for the compose variant
- `crates/smelt-cli/tests/dry_run.rs` — existing test pattern with `smelt()` helper and `assert_cmd` predicates

## Expected Output

- `crates/smelt-cli/src/commands/run.rs` — `print_execution_plan()` extended with `── Compose Services ──` section (6–8 lines of new code)
- `examples/job-manifest-compose.toml` — new file: working compose manifest with Postgres 16 service, healthcheck, and minimal sessions
- `crates/smelt-cli/tests/dry_run.rs` — 2 new tests: `dry_run_compose_manifest_shows_services_section` and `dry_run_docker_manifest_no_services_section`
