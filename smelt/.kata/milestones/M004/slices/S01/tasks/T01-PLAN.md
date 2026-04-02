---
estimated_steps: 5
estimated_files: 3
---

# T01: Add ComposeService struct, indexmap dependency, and services field

**Slice:** S01 — Manifest Extension
**Milestone:** M004

## Description

Add `indexmap` as an explicit workspace dependency (it is already a transitive dep at v2.13.0 — this makes version control explicit). Define the `ComposeService` struct with a serde-flatten passthrough map. Add the `services` field to `JobManifest` with `#[serde(default)]` for backward compat. After this task, the data model compiles and all 121 existing tests continue to pass.

## Steps

1. Add `indexmap = { version = "2", features = ["serde"] }` to `[workspace.dependencies]` in `Cargo.toml`.
2. Add `indexmap.workspace = true` to `[dependencies]` in `crates/smelt-core/Cargo.toml`.
3. Add `use indexmap::IndexMap;` to the imports in `crates/smelt-core/src/manifest.rs`.
4. Define `ComposeService` struct after `MergeConfig` and before `ValidationErrors` in `manifest.rs`:
   - `pub name: String` with doc comment
   - `pub image: String` with doc comment
   - `#[serde(flatten)] pub extra: IndexMap<String, toml::Value>` with doc comment
   - Derives: `#[derive(Debug, Deserialize)]` — NO `deny_unknown_fields` (intentional passthrough per D073)
5. Add `#[serde(default)] pub services: Vec<ComposeService>` field to `JobManifest` after the `forge` field, with a doc comment explaining it's populated from `[[services]]` TOML array.

## Must-Haves

- [ ] `indexmap` appears in `[workspace.dependencies]` in root `Cargo.toml`
- [ ] `indexmap.workspace = true` appears in `crates/smelt-core/Cargo.toml` `[dependencies]`
- [ ] `ComposeService` struct is defined with `name`, `image`, and `#[serde(flatten)] extra: IndexMap<String, toml::Value>`
- [ ] `ComposeService` does NOT have `#[serde(deny_unknown_fields)]`
- [ ] `JobManifest.services` field has `#[serde(default)]` so existing manifests parse with `services: vec![]`
- [ ] `cargo build -p smelt-core` succeeds with no errors or warnings
- [ ] `cargo test -p smelt-core --lib` reports 121 passed, 0 failed

## Verification

- `cargo build -p smelt-core` — exits 0, no errors
- `cargo test -p smelt-core --lib 2>&1 | tail -3` — shows `test result: ok. 121 passed; 0 failed`

## Observability Impact

- Signals added/changed: None — compile-time data model addition only
- How a future agent inspects this: `grep -n "ComposeService\|services:" crates/smelt-core/src/manifest.rs`
- Failure state exposed: None (no runtime behavior added)

## Inputs

- `crates/smelt-core/src/manifest.rs` — existing `JobManifest` struct and serde patterns to follow
- `Cargo.toml` — workspace dep list (indexmap 2.13.0 already transitive; make explicit)
- S01-RESEARCH.md — D073 confirms `IndexMap<String, toml::Value>` with no `deny_unknown_fields`

## Expected Output

- `Cargo.toml` — `indexmap` added to `[workspace.dependencies]`
- `crates/smelt-core/Cargo.toml` — `indexmap.workspace = true` in `[dependencies]`
- `crates/smelt-core/src/manifest.rs` — `ComposeService` struct defined; `services` field on `JobManifest`; all 121 existing tests still pass
