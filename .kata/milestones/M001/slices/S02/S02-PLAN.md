# S02: Harness Crate & Profile Type

**Goal:** `assay-harness` crate exists in the workspace and `HarnessProfile` type compiles in `assay-types` with full schema snapshot coverage.
**Demo:** `cargo build -p assay-harness` succeeds, `cargo insta test -p assay-types` has no pending snapshots, and `just ready` passes.

## Must-Haves

- `HarnessProfile` type with `PromptLayer`, `SettingsOverride`, `HookContract`, and `HookEvent` sub-types in `assay-types/src/harness.rs`
- All types have `Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq` derives and `#[serde(deny_unknown_fields)]`
- All types registered in schema registry via `inventory::submit!` with kebab-case names
- All types re-exported from `assay-types/src/lib.rs`
- Schema snapshot tests for all new types in `schema_snapshots.rs`
- `crates/assay-harness` crate exists with `Cargo.toml` depending on `assay-core` + `assay-types`
- `assay-harness` has module stubs for `prompt`, `settings`, `claude` (S03/S04 will fill these)
- `assay-harness` workspace dependency entry in root `Cargo.toml`
- `#![deny(missing_docs)]` on `assay-harness` with doc comments on all public items
- `just ready` passes (fmt, lint, test, deny)

## Proof Level

- This slice proves: contract (type compiles, schema snapshot deterministic, workspace dependency graph correct)
- Real runtime required: no (compilation + snapshot tests only)
- Human/UAT required: no

## Verification

- `cargo build -p assay-harness` — crate compiles with correct dependency edges
- `cargo insta test -p assay-types` — no pending snapshots (all new types have accepted snapshots)
- `cargo test -p assay-types -- schema_snapshots` — all snapshot tests pass
- `just ready` — full suite passes (fmt, lint, test, deny)
- `rg "HarnessProfile" crates/assay-types/src/lib.rs` — type is re-exported
- `rg "deny_unknown_fields" crates/assay-types/src/harness.rs` — every struct has the attribute

## Observability / Diagnostics

- Runtime signals: none (compile-time types only, no runtime behavior in this slice)
- Inspection surfaces: schema snapshot files in `crates/assay-types/tests/snapshots/` — human-readable JSON schemas for all new types
- Failure visibility: `cargo insta test` shows pending snapshots with diffs; `cargo build -p assay-harness` shows dependency errors
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `GateEvalContext` type from S01 (clean compilation foundation; no direct type dependency)
- New wiring introduced in this slice: `assay-harness` crate in workspace dependency graph; `HarnessProfile` type available via `assay_types::HarnessProfile`
- What remains before the milestone is truly usable end-to-end: S03 (prompt builder, settings merger, hook contracts fill the harness crate), S04 (Claude adapter uses the types), S06 (RunManifest references HarnessProfile), S07 (E2E pipeline composes everything)

## Tasks

- [x] **T01: Define HarnessProfile type system in assay-types** `est:25m`
  - Why: R003 and R004 require the type contract that all downstream slices (S03, S04, S06) consume. The types must exist before the crate that operates on them.
  - Files: `crates/assay-types/src/harness.rs`, `crates/assay-types/src/lib.rs`
  - Do: Create `harness.rs` with `HarnessProfile`, `PromptLayer`, `PromptLayerKind`, `SettingsOverride`, `HookContract`, `HookEvent` types. All types get standard derives, `deny_unknown_fields`, `inventory::submit!`, doc comments. Add `pub mod harness` to `lib.rs` and `pub use` re-exports. Follow `feature_spec.rs` pattern for nested struct composition and `work_session.rs` pattern for lifecycle enums.
  - Verify: `cargo build -p assay-types` compiles clean; `rg "HarnessProfile" crates/assay-types/src/lib.rs` shows re-export
  - Done when: All harness types compile with full derives and are re-exported from `assay-types`

- [x] **T02: Scaffold assay-harness crate, add schema snapshots, pass just ready** `est:20m`
  - Why: R003 requires the crate to exist as a workspace leaf. Schema snapshots lock the type contract for downstream consumers. `just ready` proves nothing is broken.
  - Files: `crates/assay-harness/Cargo.toml`, `crates/assay-harness/src/lib.rs`, `crates/assay-types/tests/schema_snapshots.rs`, `Cargo.toml`
  - Do: Create `crates/assay-harness/` with `Cargo.toml` (workspace deps: `assay-core`, `assay-types`, `serde`, `serde_json`) and `lib.rs` with module stubs (`pub mod prompt`, `pub mod settings`, `pub mod claude`) plus `#![deny(missing_docs)]`. Add `assay-harness = { path = "crates/assay-harness" }` to root workspace dependencies. Add schema snapshot tests for all new harness types. Run `cargo insta test -p assay-types --accept` then `just ready`.
  - Verify: `cargo build -p assay-harness` succeeds; `cargo insta test -p assay-types` no pending; `just ready` passes
  - Done when: Harness crate compiles, all schema snapshots accepted, full test suite green

## Files Likely Touched

- `crates/assay-types/src/harness.rs` (new)
- `crates/assay-types/src/lib.rs`
- `crates/assay-types/tests/schema_snapshots.rs`
- `crates/assay-harness/Cargo.toml` (new)
- `crates/assay-harness/src/lib.rs` (new)
- `crates/assay-harness/src/prompt.rs` (new — stub)
- `crates/assay-harness/src/settings.rs` (new — stub)
- `crates/assay-harness/src/claude.rs` (new — stub)
- `Cargo.toml` (workspace deps)
