# S02: Compose File Generation

**Goal:** Implement `generate_compose_file()` — a pure function in a new `smelt_core::compose` module that takes a `JobManifest`, project name, and resolved credential env vars, and returns a valid Docker Compose YAML string with all `[[services]]` entries passed through and a `smelt-agent` service injected. Snapshot tests prove the YAML structure, insertion order, and `serde_yaml` type fidelity for all value kinds (array, integer, boolean, nested table).

**Demo:** `cargo test -p smelt-core --lib` passes including 6 new snapshot tests covering Postgres-only, Postgres + Redis, empty-services (agent-only), type fidelity (integer/boolean/array extra fields), nested healthcheck table, and empty credential env. `cargo test --workspace` shows zero regressions.

## Must-Haves

- `serde_yaml = "0.9"` added to `smelt-core` production dependencies (not workspace, not dev-only)
- `pub struct ComposeProvider {}` stub exists in `compose.rs` with a doc comment (RuntimeProvider impl deferred to S03)
- `pub fn generate_compose_file(manifest: &JobManifest, project_name: &str, extra_env: &HashMap<String, String>) -> crate::Result<String>` implemented and exported
- Generated YAML: `services:` section with user services first (name as key, `image:` first, then extra fields in alphabetical order), followed by `smelt-agent:` with `image`, `volumes`, optional `environment:` (only when `extra_env` non-empty), optional `depends_on:` (only when services non-empty), and `networks:`; top-level `networks:` section with `smelt-<project_name>: {}`
- `toml::Value` serializes to correct YAML types: arrays → YAML sequences, integers → YAML integers, booleans → YAML booleans (not strings)
- `extra_env` is iterated in sorted (BTreeMap) order for deterministic `environment:` output
- `compose.rs` re-exported from `lib.rs` as `pub mod compose;` with `pub use compose::ComposeProvider;`
- All 6 snapshot tests pass, zero workspace regressions

## Proof Level

- This slice proves: contract
- Real runtime required: no (pure function — no Docker dependency)
- Human/UAT required: no

## Verification

```
cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"
# → test result: ok. 137 passed (or more); 0 failed

# Named snapshot tests must appear in output:
cargo test -p smelt-core --lib -- compose 2>&1 | grep -E "test compose::"
# → test compose::tests::test_generate_compose_postgres_only ... ok
# → test compose::tests::test_generate_compose_postgres_and_redis ... ok
# → test compose::tests::test_generate_compose_empty_services ... ok
# → test compose::tests::test_generate_compose_type_fidelity ... ok
# → test compose::tests::test_generate_compose_nested_healthcheck ... ok
# → test compose::tests::test_generate_compose_empty_extra_env ... ok

cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
# → all crates: test result: ok. N passed; 0 failed
```

## Observability / Diagnostics

- Runtime signals: `generate_compose_file()` returns `crate::Result<String>` — `Err` carries the `SmeltError::Manifest` payload from `resolve_repo_path()` for bad repo paths
- Inspection surfaces: generated YAML string can be printed to stderr during dry-run (wired in S04); for S02, tests print the function output on assertion failure automatically via `assert_eq!` diff
- Failure visibility: `SmeltError::Manifest { field: "job.repo", message: ... }` propagates repo path errors with the invalid value included; `serde_yaml` serialization errors are surfaced as `SmeltError::Provider` via `.map_err(SmeltError::provider)`
- Redaction constraints: `extra_env` values (resolved credentials) must not appear in error messages — errors should reference keys only, never values

## Integration Closure

- Upstream surfaces consumed: `ComposeService` type and `JobManifest.services` (from S01), `resolve_repo_path()` (existing in `manifest.rs`), `SmeltError` (existing in `error.rs`)
- New wiring introduced in this slice: `compose.rs` module added to `lib.rs`; `ComposeProvider` stub exported from crate root; `generate_compose_file()` callable by any code with a `&JobManifest`
- What remains before the milestone is truly usable end-to-end: S03 (ComposeProvider implements RuntimeProvider — provision + exec + teardown with real Docker); S04 (run.rs dispatches to ComposeProvider on `runtime = "compose"`, dry-run extended with compose services section)

## Tasks

- [x] **T01: Add serde_yaml dep, implement generate_compose_file(), wire into lib.rs** `est:45m`
  - Why: Core deliverable of the slice — the pure YAML generation function and module skeleton that S03/S04 depend on
  - Files: `crates/smelt-core/Cargo.toml`, `crates/smelt-core/src/compose.rs`, `crates/smelt-core/src/lib.rs`
  - Do: Add `serde_yaml = "0.9"` to `[dependencies]` in smelt-core/Cargo.toml (not workspace, not dev-only). Create `compose.rs` with `ComposeProvider` stub (doc comment required — `#![deny(missing_docs)]` enforced), private `toml_to_yaml(v: &toml::Value) -> serde_yaml::Value` helper, and full `generate_compose_file()` implementation (see research for exact YAML structure rules: image first, extra fields in alphabetical order via BTreeMap, `extra_env` sorted via BTreeMap iteration, `depends_on` omitted when services empty, `environment` omitted when `extra_env` empty, `networks` always present). Add `pub mod compose;` to `lib.rs` and `pub use compose::ComposeProvider;`. Write one minimal smoke test to verify the module compiles and the function is callable.
  - Verify: `cargo build -p smelt-core` succeeds; `cargo test -p smelt-core --lib -- compose::tests::smoke` passes; `cargo test --workspace` shows zero FAILED lines
  - Done when: `cargo build -p smelt-core` exits 0; `compose::ComposeProvider` is importable from outside the crate; workspace test suite is green

- [x] **T02: Write 6 snapshot tests and confirm all pass** `est:45m`
  - Why: Contract proof — exact YAML output for all service configurations and edge cases. Retires the "TOML → YAML type fidelity" roadmap risk.
  - Files: `crates/smelt-core/src/compose.rs`
  - Do: Write all 6 `#[test]` functions listed in the research (postgres_only, postgres_and_redis, empty_services, type_fidelity, nested_healthcheck, empty_extra_env). For each test: construct a minimal `JobManifest` using a real local path for `job.repo` (use `env!("CARGO_MANIFEST_DIR")` so `resolve_repo_path()` succeeds), call `generate_compose_file()`, print the result via `eprintln!("{}", result)` on the first run to capture actual output, then write `assert_eq!(result.unwrap(), expected)` with the exact YAML string. Key invariants to assert per test: (1) postgres_only — `depends_on:` present, no `environment:` block; (2) postgres_and_redis — both service names appear as YAML keys, `depends_on:` has both names; (3) empty_services — no `depends_on:` in smelt-agent, no user service keys; (4) type_fidelity — `port: 5432` is an integer (not string), `restart: true` is a boolean (not string), `command:` is a YAML sequence; (5) nested_healthcheck — `healthcheck:` sub-keys appear in alphabetical order (`interval`, `retries`, `test`); (6) empty_extra_env — no `environment:` key on smelt-agent. Remove the `eprintln!` calls after tests pass.
  - Verify: `cargo test -p smelt-core --lib -- compose 2>&1 | grep -E "FAILED|ok"` shows all 6 tests as `ok`; `cargo test --workspace` all green
  - Done when: All 6 named snapshot tests pass; no `eprintln!` debug output remains; `cargo test --workspace` exits 0

## Files Likely Touched

- `crates/smelt-core/Cargo.toml` — add `serde_yaml = "0.9"` to `[dependencies]`
- `crates/smelt-core/src/compose.rs` — new module: ComposeProvider stub, generate_compose_file(), toml_to_yaml(), tests
- `crates/smelt-core/src/lib.rs` — add `pub mod compose;` and `pub use compose::ComposeProvider;`
