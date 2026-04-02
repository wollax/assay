---
id: T01
parent: S02
milestone: M004
provides:
  - "serde_yaml = \"0.9\" production dependency in smelt-core"
  - "`compose.rs` module with `ComposeProvider {}` struct"
  - "`generate_compose_file(manifest, project_name, extra_env) -> crate::Result<String>` — full implementation"
  - "`toml_to_yaml()` private helper covering all 7 toml::Value variants"
  - "`pub mod compose` + `pub use compose::ComposeProvider` wired into lib.rs"
key_files:
  - crates/smelt-core/src/compose.rs
  - crates/smelt-core/src/lib.rs
  - crates/smelt-core/Cargo.toml
key_decisions:
  - "SmeltError::provider() requires two args (operation, message) — used provider(\"serialize\", e.to_string()) for serde_yaml failure wrapping"
  - "serde_yaml = \"0.9\" added to [dependencies] only (not workspace, not dev-dependencies), per D076"
  - "doc comment on pub use line placed before the re-export as required by #![deny(missing_docs)]"
patterns_established:
  - "toml_to_yaml(): match on all toml::Value variants, recurse for Array/Table; BTreeMap iteration gives alphabetical key order for Table variant"
  - "generate_compose_file(): image-first insertion in service mappings, BTreeMap sort of extra_env before building environment section, omit depends_on/environment when empty"
observability_surfaces:
  - "generate_compose_file() returns crate::Result<String>; SmeltError::Manifest propagates repo path errors with invalid path in message; SmeltError::Provider wraps serde_yaml failures"
  - "cargo test -p smelt-core --lib -- compose runs all compose module tests"
duration: 20min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Add serde_yaml dep, implement generate_compose_file(), wire into lib.rs

**New `smelt_core::compose` module delivers full Docker Compose YAML generation via `serde_yaml::Mapping` with TOML-to-YAML type conversion, credential env sorting, and correct omission of optional keys.**

## What Happened

Added `serde_yaml = "0.9"` to `[dependencies]` in `crates/smelt-core/Cargo.toml` (not workspace, not dev-dependencies). Created `crates/smelt-core/src/compose.rs` (~160 lines) containing:

- `pub struct ComposeProvider {}` — stub struct for the RuntimeProvider impl deferred to S03 (D019).
- `pub fn generate_compose_file(manifest, project_name, extra_env) -> crate::Result<String>` — resolves repo path via `resolve_repo_path()`, builds a `serde_yaml::Mapping`-based document with user services (image-first, then BTreeMap-ordered extra fields), injects `smelt-agent` with workspace volume, sorted credential env, conditional `depends_on`, and named network, then serializes.
- `fn toml_to_yaml(v: &toml::Value) -> serde_yaml::Value` — handles all 7 variants: String, Integer, Float, Boolean, Array (recursive), Table (BTreeMap → Mapping, alphabetical keys), Datetime (string fallback).

Wired into `lib.rs`: `pub mod compose;` inserted alphabetically between `collector` and `config`; `pub use compose::ComposeProvider;` added to re-exports with a `///` doc comment (required by `#![deny(missing_docs)]`).

## Verification

```
cargo build -p smelt-core          # exit 0, no errors
cargo test --workspace             # all crates: test result: ok, 0 failed
  - smelt-core: 138 passed; 0 failed
  - smoke_empty_services_compiles: ok

grep -n 'pub mod compose\|pub use compose' crates/smelt-core/src/lib.rs
  # 37: pub mod compose;
  # 50: pub use compose::ComposeProvider;

grep -n 'serde_yaml' crates/smelt-core/Cargo.toml
  # 25: serde_yaml = "0.9"   (under [dependencies], not [dev-dependencies])
```

All must-haves confirmed. Slice-level snapshot tests are in T02.

## Diagnostics

- `cargo test -p smelt-core --lib -- compose` runs all compose module tests.
- `SmeltError::Manifest { field: "job.repo", message: ... }` propagates repo path errors with the invalid path value.
- `SmeltError::Provider { operation: "serialize", message: <serde_yaml error>, ... }` surfaces any serialization failures.
- Both variants are pattern-matchable on `crate::SmeltError`.

## Deviations

`SmeltError::provider()` takes two arguments (`operation: impl Into<String>`, `message: impl Into<String>`), not one as the task plan sketch suggested. Used `SmeltError::provider("serialize", e.to_string())` — consistent with the existing constructor signature.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/compose.rs` — new module (~160 lines): `ComposeProvider`, `generate_compose_file()`, `toml_to_yaml()`, smoke test
- `crates/smelt-core/src/lib.rs` — added `pub mod compose;` and `pub use compose::ComposeProvider;`
- `crates/smelt-core/Cargo.toml` — added `serde_yaml = "0.9"` under `[dependencies]`
