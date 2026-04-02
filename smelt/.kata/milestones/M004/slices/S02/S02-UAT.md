# S02: Compose File Generation — UAT

**Milestone:** M004
**Written:** 2026-03-21

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S02 is a pure function with no Docker or network dependency. All acceptance criteria are verified by deterministic `assert_eq!` snapshot tests on the exact YAML output. No runtime environment, Docker daemon, or human interaction is required to prove correctness.

## Preconditions

- Rust toolchain installed (`cargo` available)
- Working directory: `smelt` project root
- No Docker daemon required

## Smoke Test

```
cargo test -p smelt-core --lib -- compose::tests::test_generate_compose_postgres_only
```

Expected: `test compose::tests::test_generate_compose_postgres_only ... ok`

## Test Cases

### 1. All 6 named snapshot tests pass

```
cargo test -p smelt-core --lib -- compose 2>&1 | grep -E "test compose::|FAILED"
```

Expected (in any order, no FAILED lines):
```
test compose::tests::smoke_empty_services_compiles ... ok
test compose::tests::test_generate_compose_empty_extra_env ... ok
test compose::tests::test_generate_compose_empty_services ... ok
test compose::tests::test_generate_compose_nested_healthcheck ... ok
test compose::tests::test_generate_compose_postgres_and_redis ... ok
test compose::tests::test_generate_compose_postgres_only ... ok
test compose::tests::test_generate_compose_type_fidelity ... ok
```

### 2. Zero workspace regressions

```
cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
```

Expected: All crates show `test result: ok.` with `0 failed`. No FAILED lines anywhere.

### 3. serde_yaml is a production dependency (not dev-only)

```
grep -A5 '\[dependencies\]' crates/smelt-core/Cargo.toml | grep serde_yaml
```

Expected: `serde_yaml = "0.9"` appears under `[dependencies]`, not under `[dev-dependencies]`.

### 4. ComposeProvider is importable from the crate root

```
grep 'pub use compose::ComposeProvider' crates/smelt-core/src/lib.rs
```

Expected: line is present.

## Edge Cases

### Type fidelity — integers and booleans are not quoted

The `test_generate_compose_type_fidelity` test asserts that `port: 5432` in the manifest produces `port: 5432` in YAML (integer, not `"5432"` string) and `restart: true` produces `restart: true` (boolean, not `"true"` string). This is verified by `assert_eq!` against the exact expected YAML string.

### environment: key absent when extra_env is empty

The `test_generate_compose_empty_extra_env` test asserts `!yaml.contains("environment:")` when `extra_env` is empty. Docker Compose treats absent `environment:` identically to `environment: {}` — no functional impact, but the generated file is clean.

### depends_on: absent when no services

The `test_generate_compose_empty_services` test asserts `!yaml.contains("depends_on:")` when `manifest.services` is empty. The smelt-agent service is the only container in that case.

## Failure Signals

- Any `FAILED` line in `cargo test` output
- `serde_yaml` under `[dev-dependencies]` instead of `[dependencies]`
- Missing `pub use compose::ComposeProvider` in `lib.rs`
- Missing `pub mod compose` in `lib.rs`
- Integer TOML values serialized as quoted strings (e.g. `port: "5432"` instead of `port: 5432`)

## Requirements Proved By This UAT

- R020 (partial) — `generate_compose_file()` implements the TOML→YAML passthrough and smelt-agent injection required by the compose runtime; TOML→YAML type fidelity for arrays, integers, booleans, and nested tables is proven by snapshot tests; M004 roadmap risk "TOML → YAML type fidelity" is retired

## Not Proven By This UAT

- R020 full validation requires S03: `ComposeProvider: RuntimeProvider` with real Docker provision/teardown, healthcheck polling, and live service networking
- `docker compose config` validation of generated YAML — the snapshot tests prove structural correctness but not Docker Compose schema acceptance
- Ctrl+C teardown behavior — S03 and S04
- `smelt run --dry-run` compose services section — S04
- `smelt run examples/job-manifest-compose.toml` end-to-end — S03 + S04

## Notes for Tester

All S02 verification is automated. Run `cargo test --workspace` to confirm everything is green. No manual steps are needed — this slice has no runtime or human-experience component.
