# S01: Manifest Extension — UAT

**Milestone:** M004
**Written:** 2026-03-21

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is pure data-model and validation logic — no Docker, no runtime, no UI. All acceptance criteria are machine-verifiable via `cargo test`. No human intuition is required to validate parsing or validation rules.

## Preconditions

- Rust toolchain installed (`cargo` on PATH)
- Working directory: `smelt/` repo root
- `cargo build -p smelt-core` succeeds

## Smoke Test

```
cargo test -p smelt-core --lib 2>&1 | tail -3
```

Expected: `test result: ok. 131 passed; 0 failed`

## Test Cases

### 1. Roundtrip: manifest with `[[services]]` parses correctly

```
cargo test -p smelt-core --lib --test test_compose_manifest_roundtrip_with_services 2>&1 | tail -3
```

**Expected:** `test result: ok. 1 passed; 0 failed`

Confirms: `ComposeService.name`, `.image`, and `.extra` are correctly populated; `extra` does not contain `name` or `image` keys (serde flatten exclusion contract holds).

### 2. Backward compatibility: existing docker manifest parses with empty services

```
cargo test -p smelt-core --lib --test test_compose_manifest_roundtrip_no_services 2>&1 | tail -3
```

**Expected:** `test result: ok. 1 passed; 0 failed`

Confirms: `#[serde(default)]` on `services` means existing manifests without `[[services]]` parse successfully.

### 3. Type fidelity: extra fields preserve integer, boolean, array, and string types

```
cargo test -p smelt-core --lib --test test_compose_service_passthrough_types 2>&1 | tail -3
```

**Expected:** `test result: ok. 1 passed; 0 failed`

Confirms: `port = 5432` → `Integer(5432)`, `restart = true` → `Boolean(true)`, `command = [...]` → `Array([...])`, `tag = "db"` → `String("db")`.

### 4. Validation rejects unknown runtime values

```
cargo test -p smelt-core --lib --test test_validate_runtime_unknown_rejected 2>&1 | tail -3
```

**Expected:** `test result: ok. 1 passed; 0 failed`

Confirms: `runtime = "kubernetes"` produces an error containing `"environment.runtime"`.

### 5. Validation rejects services with `runtime = "docker"`

```
cargo test -p smelt-core --lib --test test_validate_services_require_compose_runtime 2>&1 | tail -3
```

**Expected:** `test result: ok. 1 passed; 0 failed`

Confirms: non-empty `[[services]]` with `runtime = "docker"` produces an error containing both `"services:"` and `"compose"`.

### 6. Validation rejects compose services with missing name or image

```
cargo test -p smelt-core --lib --test test_validate_compose_service_missing_name -- --nocapture 2>&1 | tail -5
cargo test -p smelt-core --lib --test test_validate_compose_service_missing_image -- --nocapture 2>&1 | tail -5
```

**Expected:** both pass; errors contain `"services[0].name"` and `"services[0].image"` respectively.

### 7. Zero regressions across full workspace

```
cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
```

**Expected:** every line reads `test result: ok. N passed; 0 failed` with no `FAILED` lines.

## Edge Cases

### compose runtime with zero services is valid

```
cargo test -p smelt-core --lib --test test_validate_compose_empty_services_allowed 2>&1 | tail -3
```

**Expected:** `test result: ok. 1 passed; 0 failed` — `runtime = "compose"` with no `[[services]]` entries passes `validate()`.

### serde flatten excludes name and image from extra

```
cargo test -p smelt-core --lib --test test_compose_service_extra_does_not_contain_name_or_image 2>&1 | tail -3
```

**Expected:** `test result: ok. 1 passed; 0 failed` — `extra` map does not contain `"name"` or `"image"` keys.

## Failure Signals

- Any `FAILED` line in `cargo test --workspace` output
- `test result: ok. N passed` where N < 131 in `smelt-core --lib`
- Compile errors mentioning `ComposeService`, `services`, or `IndexMap` after dependency changes
- `extra` containing `"name"` or `"image"` keys (serde flatten regression)

## Requirements Proved By This UAT

- R020 (partial) — S01's UAT proves the manifest layer of R020: `[[services]]` entries are parsed, typed, and validated correctly. The full R020 requires live Docker integration (S02 YAML generation + S03 ComposeProvider + S04 CLI dispatch) before it can be marked validated.

## Not Proven By This UAT

- TOML → YAML type fidelity through `serde_yaml` serialization (deferred to S02 snapshot tests)
- `ComposeProvider` lifecycle (provision, exec, teardown) — deferred to S03
- `docker compose up/down` correctness — deferred to S03 integration tests
- `smelt run --dry-run` showing `── Compose Services ──` section — deferred to S04
- Live Docker environment behavior with real service containers — deferred to S03/S04

## Notes for Tester

All test cases are deterministic and require no Docker daemon. Run them in any order. The `test_cli_run_invalid_manifest` test in `docker_lifecycle.rs` is known to be flaky when the full workspace test suite runs under heavy parallelism (it passes in isolation); this is a pre-existing issue unrelated to S01.
