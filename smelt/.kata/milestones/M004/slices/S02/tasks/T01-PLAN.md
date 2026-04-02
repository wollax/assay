---
estimated_steps: 7
estimated_files: 3
---

# T01: Add serde_yaml dep, implement generate_compose_file(), wire into lib.rs

**Slice:** S02 — Compose File Generation
**Milestone:** M004

## Description

This task creates the `smelt_core::compose` module from scratch and delivers the core YAML generation function. It adds `serde_yaml = "0.9"` as a production dependency, implements `generate_compose_file()` with full YAML generation logic (user services passthrough, smelt-agent injection, named network), and wires the module into `lib.rs`. A minimal smoke test confirms the module compiles and is callable.

The research has fully resolved the design (see S02-RESEARCH.md). Key implementation contracts:
- Build YAML via `serde_yaml::Mapping` — no hand-built strings
- Extra fields come from `ComposeService.extra: IndexMap<String, toml::Value>` deserialized with serde flatten — they arrive in BTreeMap (alphabetical) order because TOML's internal table type is BTreeMap
- `extra_env` must be sorted via BTreeMap before building the environment mapping for deterministic output
- `depends_on:` omitted entirely when `manifest.services` is empty
- `environment:` omitted entirely when `extra_env` is empty
- `networks:` on smelt-agent always present; top-level `networks:` section always present
- Return type is `crate::Result<String>` — `resolve_repo_path()` can fail

## Steps

1. Add `serde_yaml = "0.9"` to `[dependencies]` in `crates/smelt-core/Cargo.toml`. Do NOT add to `[workspace.dependencies]` (D076 — only smelt-core needs it). Confirm it is NOT in `[dev-dependencies]`.

2. Create `crates/smelt-core/src/compose.rs` with the module skeleton:
   - Top-level doc comment for the module
   - `use` imports: `std::collections::HashMap`, `std::collections::BTreeMap`, `indexmap::IndexMap` (or just rely on toml::Value's BTreeMap), `serde_yaml`, `crate::manifest::JobManifest`, `crate::manifest::resolve_repo_path`, `crate::Result`
   - `/// Docker Compose runtime provider.` doc comment on `pub struct ComposeProvider {}`
   - Signature for `pub fn generate_compose_file(manifest: &JobManifest, project_name: &str, extra_env: &HashMap<String, String>) -> crate::Result<String>` — leave body as `todo!()` for now
   - Signature for `fn toml_to_yaml(v: &toml::Value) -> serde_yaml::Value` — leave body as `todo!()` for now

3. Implement `fn toml_to_yaml(v: &toml::Value) -> serde_yaml::Value` using a match on all `toml::Value` variants:
   - `toml::Value::String(s)` → `serde_yaml::Value::String(s.clone())`
   - `toml::Value::Integer(i)` → `serde_yaml::Value::Number(serde_yaml::Number::from(*i))`
   - `toml::Value::Float(f)` → `serde_yaml::Value::Number(serde_yaml::Number::from(*f))`
   - `toml::Value::Boolean(b)` → `serde_yaml::Value::Bool(*b)`
   - `toml::Value::Array(arr)` → `serde_yaml::Value::Sequence(arr.iter().map(toml_to_yaml).collect())`
   - `toml::Value::Table(table)` → build a `serde_yaml::Mapping` from the table's key-value pairs (BTreeMap iteration gives alphabetical order); return `serde_yaml::Value::Mapping(m)`
   - `toml::Value::Datetime(dt)` → `serde_yaml::Value::String(dt.to_string())` (edge case, not expected in practice)

4. Implement `pub fn generate_compose_file(...)`:
   - Call `resolve_repo_path(&manifest.job.repo)?` to get a `PathBuf`; format as `format!("{}:/workspace", repo_path.display())`
   - Build a `serde_yaml::Mapping` for the top-level `services:` entry
   - For each service in `manifest.services` (in manifest order):
     - Build a `serde_yaml::Mapping` for the service; insert `"image"` key first with `serde_yaml::Value::String(service.image.clone())`
     - Iterate `service.extra` (IndexMap, but deserialized in BTreeMap/alphabetical order via TOML flatten); for each `(k, v)`: insert `serde_yaml::Value::String(k.clone()) → toml_to_yaml(v)` into the service mapping
     - Insert the service mapping into `services_map` keyed by `service.name`
   - Build the `smelt-agent` service mapping:
     - Insert `"image"` → `manifest.environment.image.clone()`
     - Insert `"volumes"` → `serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(vol_string)])` where `vol_string` is the `<repo_path>:/workspace` string
     - If `!extra_env.is_empty()`: build environment mapping by iterating `extra_env.iter().collect::<BTreeMap<_,_>>().iter()` (sorted), inserting each key-value as `String → String`; insert `"environment"` key
     - If `!manifest.services.is_empty()`: build `depends_on` sequence from `manifest.services.iter().map(|s| serde_yaml::Value::String(s.name.clone())).collect()`; insert `"depends_on"` key
     - Insert `"networks"` → `serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(format!("smelt-{project_name}"))])`
   - Insert `"smelt-agent"` service into `services_map`
   - Build top-level document mapping: insert `"services" → serde_yaml::Value::Mapping(services_map)`
   - Build `networks:` section: one entry `format!("smelt-{project_name}") → serde_yaml::Value::Mapping(serde_yaml::Mapping::new())`
   - Insert `"networks"` into top-level mapping
   - Serialize: `serde_yaml::to_string(&serde_yaml::Value::Mapping(top_level)).map_err(|e| crate::SmeltError::provider(e.to_string()))`

5. Add `pub mod compose;` to `crates/smelt-core/src/lib.rs` (alphabetical position among existing `pub mod` declarations). Add `pub use compose::ComposeProvider;` to the re-exports block. Add a `///` doc comment before the `pub use` line.

6. Write one minimal smoke test in `compose.rs` under `#[cfg(test)]`:
   ```rust
   #[test]
   fn smoke_empty_services_compiles() {
       // Verify the module compiles and the function is reachable.
       // Full snapshot tests are in T02.
       let _ = std::collections::HashMap::<String, String>::new();
       let _provider = ComposeProvider {};
   }
   ```
   This is intentionally minimal — its only job is proving the module compiled and types are accessible.

7. Run `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` — confirm all crates pass, zero FAILED lines.

## Must-Haves

- [ ] `serde_yaml = "0.9"` in `[dependencies]` (not `[dev-dependencies]`, not workspace) of `crates/smelt-core/Cargo.toml`
- [ ] `pub struct ComposeProvider {}` with doc comment exists in `compose.rs`
- [ ] `pub fn generate_compose_file(manifest: &JobManifest, project_name: &str, extra_env: &HashMap<String, String>) -> crate::Result<String>` fully implemented (no `todo!()` or `unimplemented!()` remaining)
- [ ] `fn toml_to_yaml(v: &toml::Value) -> serde_yaml::Value` handles all 7 toml::Value variants
- [ ] `pub mod compose;` added to `lib.rs`; `pub use compose::ComposeProvider;` added to re-exports
- [ ] `cargo build -p smelt-core` exits 0 (including `#![deny(missing_docs)]` check)
- [ ] `cargo test --workspace` exits 0, zero FAILED lines

## Verification

- `cargo build -p smelt-core 2>&1 | grep -E "^error"` — should produce no output
- `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` — all `test result: ok`, no FAILED
- `grep -n 'pub mod compose\|pub use compose' crates/smelt-core/src/lib.rs` — both lines present
- `grep -n 'serde_yaml' crates/smelt-core/Cargo.toml` — appears under `[dependencies]`, not `[dev-dependencies]`

## Observability Impact

- Signals added/changed: `generate_compose_file()` returns `crate::Result<String>`; `SmeltError::Manifest` propagates from `resolve_repo_path()` with the invalid repo path value in the message; `SmeltError::Provider` wraps any serde_yaml serialization failure
- How a future agent inspects this: `cargo test -p smelt-core --lib -- compose` runs all compose module tests; generated YAML is printed by tests on failure via `assert_eq!` diff; `SmeltError` variants are pattern-matchable
- Failure state exposed: repo path errors include the invalid path string; serde_yaml errors include the serializer error message; both are `crate::Result<T>` Err variants

## Inputs

- `crates/smelt-core/src/manifest.rs` — `ComposeService`, `JobManifest`, `resolve_repo_path()`, `SmeltError::Manifest` — the types and functions this task depends on from S01
- `crates/smelt-core/src/error.rs` — `SmeltError::provider()` constructor for wrapping serde_yaml errors
- `S02-RESEARCH.md` — full YAML structure spec, key ordering rules, pitfalls (BTreeMap order, HashMap non-determinism, Mapping insertion order)
- `.kata/DECISIONS.md` — D073 (no deny_unknown_fields on ComposeService), D074 (extra_env to smelt-agent only), D076 (serde_yaml as production dep), D019 (RuntimeProvider impl deferred to S03)

## Expected Output

- `crates/smelt-core/Cargo.toml` — `serde_yaml = "0.9"` under `[dependencies]`
- `crates/smelt-core/src/compose.rs` — new module (~150 lines) with `ComposeProvider`, `generate_compose_file()`, `toml_to_yaml()`, smoke test
- `crates/smelt-core/src/lib.rs` — `pub mod compose;` and `pub use compose::ComposeProvider;` added
- `cargo test --workspace` all green
