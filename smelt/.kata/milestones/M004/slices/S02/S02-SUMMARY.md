---
id: S02
parent: M004
milestone: M004
provides:
  - "`serde_yaml = \"0.9\"` production dependency in smelt-core/Cargo.toml"
  - "`pub struct ComposeProvider {}` stub in `smelt_core::compose` module (RuntimeProvider impl deferred to S03)"
  - "`pub fn generate_compose_file(manifest, project_name, extra_env) -> crate::Result<String>` — full implementation"
  - "`fn toml_to_yaml(v: &toml::Value) -> serde_yaml::Value` covering all 7 TOML value variants with BTreeMap key ordering"
  - "6 exact-string snapshot tests + portable `workspace_vol()` helper covering all YAML generation cases"
  - "`pub mod compose` + `pub use compose::ComposeProvider` wired into lib.rs"
requires:
  - slice: S01
    provides: "`ComposeService` type, `JobManifest.services: Vec<ComposeService>`, `resolve_repo_path()` for workspace volume path"
affects:
  - S03
  - S04
key_files:
  - crates/smelt-core/src/compose.rs
  - crates/smelt-core/src/lib.rs
  - crates/smelt-core/Cargo.toml
key_decisions:
  - "D076: serde_yaml added as production dep (not dev-only) — generate_compose_file() runs in the normal smelt run path"
  - "D078: environment: key on smelt-agent omitted when extra_env is empty; same logic as depends_on: omission"
  - "SmeltError::provider(\"serialize\", e.to_string()) used for serde_yaml failure wrapping — provider() takes two args (operation, message)"
  - "Snapshot tests use workspace_vol() with canonicalized env!(\"CARGO_MANIFEST_DIR\") to be portable across machines and CI"
patterns_established:
  - "toml_to_yaml(): match all 7 toml::Value variants; BTreeMap iteration on Table gives alphabetical key order for deterministic YAML"
  - "generate_compose_file(): image-first in per-service mapping, BTreeMap-sorted extra_env for environment block, omit depends_on/environment when empty"
  - "Snapshot test workflow: write with eprintln! + --nocapture, observe output, write assert_eq!, remove eprintln!"
  - "workspace_vol() helper: canonicalize env!(\"CARGO_MANIFEST_DIR\") + \":/workspace\" — reuse for future compose snapshot tests"
observability_surfaces:
  - "generate_compose_file() returns crate::Result<String>; SmeltError::Manifest propagates repo path errors; SmeltError::Provider wraps serde_yaml serialization failures"
  - "`cargo test -p smelt-core --lib -- compose` runs all 7 compose module tests with assert_eq! diff on failure"
drill_down_paths:
  - .kata/milestones/M004/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M004/slices/S02/tasks/T02-SUMMARY.md
duration: 35min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# S02: Compose File Generation

**Pure `generate_compose_file()` function delivers Docker Compose YAML with smelt-agent injection, credential env sorting, TOML→YAML type fidelity, and 6 passing snapshot tests.**

## What Happened

T01 added `serde_yaml = "0.9"` as a production dependency and created `crates/smelt-core/src/compose.rs` (~160 lines) with three public/private surfaces: `pub struct ComposeProvider {}` (stub for S03), `pub fn generate_compose_file()` (full implementation), and `fn toml_to_yaml()` (private helper covering all 7 `toml::Value` variants). `generate_compose_file()` resolves the repo path via the existing `resolve_repo_path()`, builds a `serde_yaml::Mapping`-based document with user services (image-first, BTreeMap-ordered extra fields), injects a `smelt-agent` service with workspace volume mount, credential env (sorted via BTreeMap iteration), conditional `depends_on` and `environment` sections, and a named network, then serializes to a YAML string. The module was wired into `lib.rs` with `pub mod compose;` and `pub use compose::ComposeProvider;` (doc comment required by `#![deny(missing_docs)]`).

T02 added two test helpers (`make_manifest` and `workspace_vol`) and 6 exact-string `assert_eq!` snapshot tests covering the full matrix: empty services (agent-only, no depends_on/environment), postgres-only (depends_on present), postgres + redis (multi-service + credential env sorted), type fidelity (integer `5432` not `"5432"`, boolean `true` not `"true"`, sequence `command:`), nested healthcheck (BTreeMap sub-key order: `interval`, `retries`, `test`), and empty extra_env (environment key absent). The `workspace_vol()` helper canonicalizes `env!("CARGO_MANIFEST_DIR")` at test runtime so expected strings are portable across machines and CI without sacrificing exactness.

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
# All crates: test result: ok. 0 failed — smelt-core: 144 passed in workspace run
```

## Requirements Advanced

- R020 — `generate_compose_file()` implements the TOML→YAML passthrough required by the compose runtime; TOML→YAML type fidelity risk from the M004 roadmap retired by snapshot tests

## Requirements Validated

- none — R020 requires S03 (real Docker provision/teardown) for full validation

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

`SmeltError::provider()` takes two arguments (`operation`, `message`), not one as the task plan sketch implied. Used `SmeltError::provider("serialize", e.to_string())` — consistent with the existing constructor signature in smelt-core. Captured in T01-SUMMARY.md.

## Known Limitations

- `ComposeProvider` is a stub struct with no `RuntimeProvider` implementation — deferred to S03.
- `generate_compose_file()` is tested purely as a unit function; integration with a live Docker Compose stack is S03's responsibility.
- The generated YAML has not been round-tripped through `docker compose config` — formal Docker Compose validation is part of S03's integration test suite.

## Follow-ups

- S03: Implement `ComposeProvider: RuntimeProvider` — provision (compose up + healthcheck poll), exec (delegate to bollard on agent container), teardown (compose down + temp dir cleanup)
- S03: Add `ComposeProjectState { project_name, compose_file_path, _temp_dir }` internal type with `HashMap<ContainerId, ComposeProjectState>` on provider
- S04: Wire `run.rs` dispatch and extend `print_execution_plan()` with `── Compose Services ──` section

## Files Created/Modified

- `crates/smelt-core/src/compose.rs` — new module: `ComposeProvider`, `generate_compose_file()`, `toml_to_yaml()`, helpers, 7 tests
- `crates/smelt-core/src/lib.rs` — added `pub mod compose;` and `pub use compose::ComposeProvider;`
- `crates/smelt-core/Cargo.toml` — added `serde_yaml = "0.9"` under `[dependencies]`

## Forward Intelligence

### What the next slice should know
- `generate_compose_file()` is a pure function returning `crate::Result<String>` — call it at provision time, write the output to `tempfile::NamedTempFile` (or a file inside a `TempDir`), then pass the path to `docker compose -f <path>` subprocesses
- The network name is `smelt-<project_name>` — the project name passed into `generate_compose_file()` is the same string that should be used as the `--project-name` flag to Docker Compose
- `smelt-agent` is always the last service in the generated `services:` block; `depends_on:` lists all other service names in the order they appear in `manifest.services` — this is the order that Docker Compose will use when waiting for dependencies
- `ComposeProvider` is currently an empty struct in `compose.rs` — S03 should add fields for the internal state map directly to this struct (no rename needed)

### What's fragile
- `workspace_vol()` in tests uses `std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))` — if the crate is ever moved to a different directory, existing snapshot test expected strings will break (they're format!()-generated at runtime, so they'll self-heal as long as workspace_vol() is called, not hardcoded)
- `toml_to_yaml()` handles `toml::Value::Datetime` as a string fallback — if Docker Compose fields ever need real YAML timestamp types this will need to be updated

### Authoritative diagnostics
- `cargo test -p smelt-core --lib -- compose` — runs all 7 compose tests; `assert_eq!` diff shows exact YAML mismatch vs. contract on failure; add `eprintln!("{}", result.unwrap())` + `--nocapture` to capture actual output for any regression
- `SmeltError::Manifest { field: "job.repo", message: ... }` — repo path errors include the invalid path value in the message
- `SmeltError::Provider { operation: "serialize", ... }` — serde_yaml failures include the serializer error message

### What assumptions changed
- Original assumption: `SmeltError::provider()` takes one arg — actual: takes two (`operation`, `message`); using `provider("serialize", e.to_string())` is the correct pattern for S03's compose process errors too
