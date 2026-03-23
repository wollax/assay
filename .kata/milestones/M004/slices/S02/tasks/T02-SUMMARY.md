---
id: T02
parent: S02
milestone: M004
provides:
  - "6 snapshot tests for generate_compose_file() with exact assert_eq! YAML strings"
  - "make_manifest() helper using env!(\"CARGO_MANIFEST_DIR\") for portable path resolution"
  - "workspace_vol() helper that canonicalizes CARGO_MANIFEST_DIR for stable expected strings"
  - "Type-fidelity verification: integer, boolean, array, and nested table TOML → YAML"
  - "Confirmed: environment: absent when extra_env empty; depends_on: absent when no services"
key_files:
  - crates/smelt-core/src/compose.rs
key_decisions:
  - "Expected YAML strings use format!() with workspace_vol() rather than literal paths — makes tests portable across machines and CI"
  - "smoke_empty_services_compiles kept alongside 6 new tests — no removal needed, it still compiles cleanly"
patterns_established:
  - "Snapshot test workflow: write with eprintln! + --nocapture, observe output, write assert_eq!, remove eprintln!"
  - "workspace_vol() helper: canonicalize env!(\"CARGO_MANIFEST_DIR\") + append ':/workspace' — reuse for any future compose snapshot tests"
observability_surfaces:
  - "cargo test -p smelt-core --lib -- compose::tests — runs all 7 compose tests; assert_eq! diff shows exact YAML mismatch on failure"
  - "Re-add eprintln!(\"{}\", result.unwrap()) + --nocapture to debug any future generate_compose_file() output changes"
duration: 15min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T02: Write 6 snapshot tests and confirm all pass

**6 exact-string snapshot tests added to compose.rs; all pass with zero regressions (144 smelt-core tests, 215 workspace total)**

## What Happened

Added 6 `#[test]` functions and two helpers (`make_manifest`, `workspace_vol`) to the existing `#[cfg(test)]` block in `compose.rs`. The standard snapshot-test workflow was used: wrote all 6 tests with `eprintln!("{}", result.unwrap())`, ran `--nocapture` to capture actual YAML, transcribed exact strings into `assert_eq!` calls, removed `eprintln!`.

Key design choice: expected strings use `format!()` with `workspace_vol()` rather than hardcoded absolute paths. `workspace_vol()` canonicalizes `env!("CARGO_MANIFEST_DIR")` at test runtime — this makes the assertions portable across machines and CI without sacrificing exactness.

Tests cover the full matrix:
- `test_generate_compose_empty_services` — agent-only file; confirms no `depends_on:` or `environment:` 
- `test_generate_compose_postgres_only` — one service, no extra fields; confirms `depends_on:` appears
- `test_generate_compose_postgres_and_redis` — two services + credential env; confirms env sort and manifest-order `depends_on:`
- `test_generate_compose_type_fidelity` — integer `5432` (not `"5432"`), boolean `true` (not `"true"`), sequence `command:`; extra keys alphabetical
- `test_generate_compose_nested_healthcheck` — nested BTreeMap table; sub-keys `interval`, `retries`, `test` in alphabetical order
- `test_generate_compose_empty_extra_env` — postgres service, empty extra_env; `environment:` key is absent (confirmed with additional `assert!(!yaml.contains("environment:"))`)

## Verification

```
cargo test -p smelt-core --lib -- compose 2>&1 | grep -E "test compose::|FAILED"
# test compose::tests::smoke_empty_services_compiles ... ok
# test compose::tests::test_generate_compose_empty_extra_env ... ok
# test compose::tests::test_generate_compose_empty_services ... ok
# test compose::tests::test_generate_compose_nested_healthcheck ... ok
# test compose::tests::test_generate_compose_postgres_and_redis ... ok
# test compose::tests::test_generate_compose_postgres_only ... ok
# test compose::tests::test_generate_compose_type_fidelity ... ok
# (no FAILED lines)

cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
# All crates: test result: ok. 0 failed — smelt-core: 144 passed
```

## Diagnostics

- `cargo test -p smelt-core --lib -- compose` runs all compose tests; `assert_eq!` diff shows exactly what `generate_compose_file()` produces vs. what the contract requires
- To debug a future output change: add `eprintln!("{}", result.unwrap())` to the failing test + run with `--nocapture` to see actual YAML
- `SmeltError::Manifest` and `SmeltError::Provider` errors surface as panics via `.unwrap()` in tests — the panic message contains the error detail

## Deviations

None — followed the plan's observe-then-assert workflow exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/compose.rs` — Added `make_manifest()` and `workspace_vol()` helpers plus 6 snapshot tests (`test_generate_compose_empty_services`, `test_generate_compose_postgres_only`, `test_generate_compose_postgres_and_redis`, `test_generate_compose_type_fidelity`, `test_generate_compose_nested_healthcheck`, `test_generate_compose_empty_extra_env`) to existing `#[cfg(test)]` block
