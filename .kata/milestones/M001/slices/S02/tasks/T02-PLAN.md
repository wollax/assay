---
estimated_steps: 5
estimated_files: 7
---

# T02: Scaffold assay-harness crate, add schema snapshots, pass just ready

**Slice:** S02 — Harness Crate & Profile Type
**Milestone:** M001

## Description

Create the `assay-harness` crate as a workspace leaf with module stubs for S03/S04, add schema snapshot tests for all 6 new harness types, accept the snapshots, and verify the full suite passes with `just ready`. This completes both R003 (harness crate exists) and R004 (HarnessProfile type with schema snapshot).

## Steps

1. Add `assay-harness = { path = "crates/assay-harness" }` to `[workspace.dependencies]` in root `Cargo.toml`.

2. Create `crates/assay-harness/Cargo.toml`:
   ```toml
   [package]
   name = "assay-harness"
   version.workspace = true
   edition.workspace = true
   license.workspace = true
   repository.workspace = true
   description = "Agent harness adapters for Assay: prompt building, settings merging, and agent-specific config generation"

   [dependencies]
   assay-core.workspace = true
   assay-types.workspace = true
   serde.workspace = true
   serde_json.workspace = true

   [lints]
   workspace = true
   ```

3. Create `crates/assay-harness/src/lib.rs` with `#![deny(missing_docs)]`, crate-level doc comment, and module stubs:
   - `pub mod prompt;` — will hold prompt builder in S03
   - `pub mod settings;` — will hold settings merger in S03
   - `pub mod claude;` — will hold Claude Code adapter in S04
   Each stub module file (`prompt.rs`, `settings.rs`, `claude.rs`) has a doc comment only.

4. Add 6 schema snapshot tests to `crates/assay-types/tests/schema_snapshots.rs`:
   - `harness_profile_schema_snapshot`
   - `prompt_layer_schema_snapshot`
   - `prompt_layer_kind_schema_snapshot`
   - `settings_override_schema_snapshot`
   - `hook_contract_schema_snapshot`
   - `hook_event_schema_snapshot`
   Follow the exact pattern of existing tests (e.g., `gate_eval_context_schema_snapshot`).

5. Run `cargo insta test -p assay-types --accept` to generate and accept snapshots, then `just ready` to verify everything passes.

## Must-Haves

- [ ] `crates/assay-harness/Cargo.toml` with workspace deps on `assay-core` and `assay-types`
- [ ] `crates/assay-harness/src/lib.rs` with `#![deny(missing_docs)]` and 3 module stubs
- [ ] `crates/assay-harness/src/prompt.rs`, `settings.rs`, `claude.rs` as stub modules
- [ ] `assay-harness` entry in root `[workspace.dependencies]`
- [ ] 6 schema snapshot tests added to `schema_snapshots.rs`
- [ ] All snapshots accepted (no pending)
- [ ] `just ready` passes

## Verification

- `cargo build -p assay-harness` succeeds
- `cargo insta test -p assay-types` reports no pending snapshots
- `cargo test -p assay-types -- schema_snapshots` — all tests pass (existing + 6 new)
- `just ready` passes (fmt, lint, test, deny)
- `ls crates/assay-harness/src/` shows `lib.rs`, `prompt.rs`, `settings.rs`, `claude.rs`

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: Schema snapshot `.snap` files provide deterministic JSON schema for all harness types; `cargo build -p assay-harness` validates dependency graph
- Failure state exposed: `cargo insta test` shows diffs for any schema drift

## Inputs

- `crates/assay-types/src/harness.rs` — types defined in T01 (must compile)
- `crates/assay-types/tests/schema_snapshots.rs` — existing test pattern
- `crates/assay-core/Cargo.toml` — reference for workspace dependency format
- Root `Cargo.toml` — workspace members and dependencies

## Expected Output

- `crates/assay-harness/Cargo.toml` — new crate manifest
- `crates/assay-harness/src/lib.rs` — crate root with module stubs
- `crates/assay-harness/src/prompt.rs` — empty module stub
- `crates/assay-harness/src/settings.rs` — empty module stub
- `crates/assay-harness/src/claude.rs` — empty module stub
- `crates/assay-types/tests/schema_snapshots.rs` — 6 new test functions
- `crates/assay-types/tests/snapshots/` — 6 new `.snap` files
- `Cargo.toml` — `assay-harness` in workspace dependencies
