---
estimated_steps: 7
estimated_files: 1
---

# T02: Write 6 snapshot tests and confirm all pass

**Slice:** S02 — Compose File Generation
**Milestone:** M004

## Description

This task writes the comprehensive snapshot test suite for `generate_compose_file()` that proves the YAML contract for all service configurations, including TOML → YAML type fidelity. Each test constructs a `JobManifest` directly, calls `generate_compose_file()`, and asserts the exact YAML string — no approximate matching.

The key challenge is getting the exact expected YAML strings right. The workflow is: run each test with `eprintln!("{}", result.unwrap())`, observe actual output, write the expected string to match, then remove the `eprintln!`. This is the standard snapshot-test workflow.

Six tests as specified in the research:
1. `test_generate_compose_postgres_only` — one service, no extra fields, no credential env
2. `test_generate_compose_postgres_and_redis` — two services, credential env present
3. `test_generate_compose_empty_services` — no user services (agent-only), no credential env
4. `test_generate_compose_type_fidelity` — one service with integer, boolean, and array extra fields
5. `test_generate_compose_nested_healthcheck` — one service with a nested TOML table extra field; keys appear in alphabetical order
6. `test_generate_compose_empty_extra_env` — verify `environment:` key is omitted when `extra_env` is empty

**Critical knowledge from research:**
- `toml::Value::Table` is `BTreeMap` internally — nested table sub-keys appear alphabetically in YAML regardless of TOML source order
- Top-level extra keys in `ComposeService.extra` also come out alphabetical (TOML flatten deserializes from BTreeMap)
- `extra_env` is sorted via BTreeMap before emitting — produce test values whose alphabetical sort order is predictable
- `depends_on:` absent when `manifest.services` is empty; `environment:` absent when `extra_env` is empty

## Steps

1. Create a shared helper in the `#[cfg(test)]` block of `compose.rs` to build a minimal `JobManifest` for tests:
   ```rust
   fn make_manifest(services: Vec<ComposeService>) -> JobManifest {
       // Uses env!("CARGO_MANIFEST_DIR") for job.repo so resolve_repo_path() succeeds
       // Fill all required fields with fixed test values
   }
   ```
   All 6 tests should use this helper to avoid repetition. The `job.repo` must be a real local path that exists — use `env!("CARGO_MANIFEST_DIR")` (the `crates/smelt-core` directory) so it always exists.

2. Write `test_generate_compose_empty_services` first (simplest case — no user services, no extra_env):
   - Call `generate_compose_file(&manifest, "myproj", &HashMap::new())`
   - Add `eprintln!("{}", result.as_ref().unwrap())` temporarily
   - Run `cargo test -p smelt-core --lib -- test_generate_compose_empty_services -- --nocapture`
   - Observe actual YAML; write `assert_eq!` with exact string; remove `eprintln!`
   - Must confirm: no `depends_on:` key, no `environment:` key on `smelt-agent`, `networks:` contains `smelt-myproj`, top-level `networks:\n  smelt-myproj: {}\n` present

3. Write `test_generate_compose_postgres_only`:
   - Service: `name = "postgres"`, `image = "postgres:16"`, no extra fields
   - `extra_env` = empty
   - Observe actual YAML; write `assert_eq!`
   - Must confirm: `depends_on:\n- postgres\n` present on smelt-agent; `image: postgres:16` in postgres service; `image` first in postgres service block

4. Write `test_generate_compose_postgres_and_redis`:
   - Services: postgres (no extra) and redis (no extra)
   - `extra_env` = `{"ANTHROPIC_API_KEY": "test-key"}` — single env var whose alphabetical sort is predictable
   - Observe actual YAML; write `assert_eq!`
   - Must confirm: both `postgres:` and `redis:` keys in services section; `depends_on:` lists both in manifest order (postgres first, redis second); `environment:\n  ANTHROPIC_API_KEY: test-key\n` present

5. Write `test_generate_compose_type_fidelity`:
   - Build a `ComposeService` directly with `extra` containing:
     - `"command"` → `toml::Value::Array(vec![toml::Value::String("CMD".into()), toml::Value::String("pg_isready".into())])`
     - `"port"` → `toml::Value::Integer(5432)`
     - `"restart"` → `toml::Value::Boolean(true)`
   - Note: insert into the `IndexMap` in any order — alphabetical output order is guaranteed by TOML's BTreeMap behavior regardless
   - Observe actual YAML; write `assert_eq!`
   - Must confirm: `port: 5432` (integer — no quotes), `restart: true` (boolean — no quotes), `command:` followed by sequence items

6. Write `test_generate_compose_nested_healthcheck`:
   - Build a `ComposeService` with `extra` containing `"healthcheck"` → `toml::Value::Table` with keys: `"interval"` → String, `"retries"` → Integer, `"test"` → Array
   - Must confirm: sub-keys appear as `interval:`, `retries:`, `test:` in alphabetical order in the YAML
   - Observe actual YAML; write `assert_eq!`

7. Write `test_generate_compose_empty_extra_env`:
   - Postgres-only service, `extra_env` = `HashMap::new()`
   - Observe actual YAML; write `assert_eq!`
   - Must confirm: NO `environment:` key anywhere in smelt-agent block
   - Run full workspace: `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` — all green

## Must-Haves

- [ ] All 6 tests exist: `test_generate_compose_postgres_only`, `test_generate_compose_postgres_and_redis`, `test_generate_compose_empty_services`, `test_generate_compose_type_fidelity`, `test_generate_compose_nested_healthcheck`, `test_generate_compose_empty_extra_env`
- [ ] All 6 tests use `assert_eq!` with exact expected YAML strings (no `contains` approximations)
- [ ] No `eprintln!` debug output left in tests (removed after expected strings are captured)
- [ ] `test_generate_compose_empty_services` confirms absence of `depends_on:` key
- [ ] `test_generate_compose_type_fidelity` confirms `port: 5432` is integer (not `"5432"`), `restart: true` is boolean (not `"true"`), `command:` is a YAML sequence
- [ ] `test_generate_compose_nested_healthcheck` confirms nested table sub-keys appear in alphabetical order
- [ ] `test_generate_compose_empty_extra_env` confirms absence of `environment:` key when `extra_env` is empty
- [ ] `cargo test -p smelt-core --lib -- compose 2>&1 | grep "FAILED"` produces no output
- [ ] `cargo test --workspace` exits 0, zero FAILED lines

## Verification

```
cargo test -p smelt-core --lib -- compose 2>&1 | grep -E "test compose::|FAILED"
# Expected output:
# test compose::tests::test_generate_compose_empty_extra_env ... ok
# test compose::tests::test_generate_compose_empty_services ... ok
# test compose::tests::test_generate_compose_nested_healthcheck ... ok
# test compose::tests::test_generate_compose_postgres_and_redis ... ok
# test compose::tests::test_generate_compose_postgres_only ... ok
# test compose::tests::test_generate_compose_type_fidelity ... ok

cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
# All crates: test result: ok. N passed; 0 failed
```

## Observability Impact

- Signals added/changed: 6 snapshot tests — any future change to `generate_compose_file()`'s YAML output causes an immediate test failure with exact diff showing what changed
- How a future agent inspects this: `cargo test -p smelt-core --lib -- compose` — the `assert_eq!` diff in the failure output shows exactly what the function produces vs what the contract requires; re-adding `eprintln!("{}", result.unwrap())` + `--nocapture` is the standard debugging path
- Failure state exposed: serde_yaml serialization errors and resolve_repo_path errors surface as `Err(...)` from `generate_compose_file()` — test `.unwrap()` calls will panic with the error message on unexpected failure

## Inputs

- `crates/smelt-core/src/compose.rs` — `generate_compose_file()`, `ComposeProvider`, and `toml_to_yaml()` from T01 (must be complete and passing smoke test before starting T02)
- `crates/smelt-core/src/manifest.rs` — `JobManifest`, `ComposeService`, `JobMeta`, `Environment`, `CredentialConfig`, `SessionDef`, `MergeConfig` — needed to construct test manifests via struct literals
- `S02-RESEARCH.md` — "Top-level extra fields are also alphabetical" pitfall; "toml::Value::Table key order is alphabetical" pitfall; exact YAML structure spec

## Expected Output

- `crates/smelt-core/src/compose.rs` — 6 new `#[test]` functions added to existing `#[cfg(test)]` block plus a shared `make_manifest()` helper; all 6 passing with exact expected YAML strings
- `cargo test --workspace` all green — 137 or more tests passing, zero failures
