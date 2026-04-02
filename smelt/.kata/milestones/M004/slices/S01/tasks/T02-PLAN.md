---
estimated_steps: 6
estimated_files: 1
---

# T02: Extend validate() with compose rules and write full test suite

**Slice:** S01 — Manifest Extension
**Milestone:** M004

## Description

Extend `JobManifest::validate()` with three compose-specific rules, then write the complete test suite that proves the S01 boundary contract. This is all in `manifest.rs`. After this task, the contract that S02/S03/S04 depend on is fully proven: `ComposeService` parses correctly, serde flatten excludes `name`/`image` from `extra`, type fidelity holds for int/bool/array extra fields, and all validation invariants are enforced.

## Steps

1. Add runtime value validation in `validate()` (place after the existing `environment.image` check):
   ```rust
   const VALID_RUNTIMES: &[&str] = &["docker", "compose"];
   if !VALID_RUNTIMES.contains(&self.environment.runtime.as_str()) {
       errors.push(format!(
           "environment.runtime: must be one of {:?}, got `{}`",
           VALID_RUNTIMES, self.environment.runtime
       ));
   }
   ```

2. Add services-without-compose validation immediately after the runtime check:
   ```rust
   if self.environment.runtime != "compose" && !self.services.is_empty() {
       errors.push(format!(
           "services: `[[services]]` entries require `runtime = \"compose\"`, got `{}`",
           self.environment.runtime
       ));
   }
   ```

3. Add per-service validation (place after the sessions validation block):
   ```rust
   if self.environment.runtime == "compose" {
       for (i, svc) in self.services.iter().enumerate() {
           if svc.name.trim().is_empty() {
               errors.push(format!("services[{i}].name: must not be empty"));
           }
           if svc.image.trim().is_empty() {
               errors.push(format!("services[{i}].image: must not be empty"));
           }
       }
   }
   ```

4. Add `VALID_COMPOSE_MANIFEST` constant to the test module — a minimal manifest with `runtime = "compose"` and two `[[services]]` entries (postgres with extra fields, redis bare). Include one extra field of each type: string, integer, boolean, and array (to prove type fidelity).

5. Write the 9 new tests in the `#[cfg(test)]` block:
   - `test_compose_manifest_roundtrip_with_services` — parse `VALID_COMPOSE_MANIFEST`; assert `services.len() == 2`, `services[0].name == "postgres"`, `services[0].image == "postgres:16"`, `services[0].extra` has keys for the extra fields, `services[0].extra` does NOT contain "name" or "image"
   - `test_compose_manifest_roundtrip_no_services` — parse `VALID_MANIFEST` (runtime=docker, no `[[services]]`); assert `services.is_empty()`
   - `test_compose_service_extra_does_not_contain_name_or_image` — parse a single-service compose manifest; assert `!extra.contains_key("name") && !extra.contains_key("image")`
   - `test_compose_service_passthrough_types` — parse a service with `port = 5432` (integer), `restart = true` (boolean), `command = ["pg_isready", "-U", "postgres"]` (array); assert the `extra` values are `toml::Value::Integer(5432)`, `toml::Value::Boolean(true)`, `toml::Value::Array([...])`
   - `test_validate_compose_service_missing_name` — `[[services]]` entry with `name = ""`, `image = "img"`; assert validation error contains `"services[0].name"`
   - `test_validate_compose_service_missing_image` — `[[services]]` entry with `name = "svc"`, `image = ""`; assert validation error contains `"services[0].image"`
   - `test_validate_services_require_compose_runtime` — `runtime = "docker"` with `[[services]]`; assert validation error contains `"services:"` and mentions `runtime = "compose"`
   - `test_validate_compose_empty_services_allowed` — `runtime = "compose"` with no `[[services]]`; assert `validate()` returns `Ok(())`
   - `test_validate_runtime_unknown_rejected` — `runtime = "kubernetes"`; assert validation error contains `"environment.runtime"`

6. Run `cargo test -p smelt-core --lib` and `cargo test --workspace` — both must pass cleanly.

## Must-Haves

- [ ] `validate()` rejects unknown `runtime` values with an error containing `"environment.runtime"`
- [ ] `validate()` rejects non-empty `services` with `runtime != "compose"` with an error containing `"services:"`
- [ ] `validate()` rejects `ComposeService` with empty `name` with an error containing `"services[N].name"`
- [ ] `validate()` rejects `ComposeService` with empty `image` with an error containing `"services[N].image"`
- [ ] `validate()` returns `Ok(())` for `runtime = "compose"` with empty services list
- [ ] `test_compose_service_extra_does_not_contain_name_or_image` passes — serde flatten contract verified
- [ ] `test_compose_service_passthrough_types` passes — integer, boolean, array extra fields serialize to correct `toml::Value` variants
- [ ] `cargo test -p smelt-core --lib` — all tests pass (121 existing + 9 new = 130 total)
- [ ] `cargo test --workspace` — zero regressions in any crate

## Verification

- `cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"` — shows `ok`, no FAILED
- `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` — shows all `ok`, no FAILED
- `cargo test -p smelt-core --lib 2>&1 | grep "passed"` — count is ≥ 130 (121 existing + 9 new)

## Observability Impact

- Signals added/changed: validation now produces errors for unknown runtimes and compose-service violations; these appear in `SmeltError::Manifest { message }` per D018 (collect-all-errors)
- How a future agent inspects this: `cargo test -p smelt-core --lib --test test_validate_runtime_unknown_rejected` to spot-check a specific validation path
- Failure state exposed: each new validation rule has a named test; running any single test by name immediately localizes which invariant broke

## Inputs

- `crates/smelt-core/src/manifest.rs` — existing `validate()` body with established error-push pattern; `ComposeService` struct and `services` field from T01
- S01-RESEARCH.md — pitfall notes on `deny_unknown_fields` + flatten interaction (avoid on `ComposeService`); zero-services-with-compose is allowed; serde flatten means `name`/`image` not in `extra`
- `.kata/DECISIONS.md` — D018 (collect all errors), D073 (IndexMap passthrough, no deny_unknown_fields)

## Expected Output

- `crates/smelt-core/src/manifest.rs` — `validate()` has 3 new compose rule blocks; 9 new tests in the `#[cfg(test)]` module; `VALID_COMPOSE_MANIFEST` constant with representative TOML
- `cargo test -p smelt-core --lib` — passes with ≥ 130 tests
- `cargo test --workspace` — zero regressions
