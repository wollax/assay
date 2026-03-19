# S06: RunManifest Type & Parsing

**Goal:** TOML manifests with `[[sessions]]` parse into `RunManifest` types with validation and actionable error messages.
**Demo:** Round-trip parse tests pass, error-case tests produce caret-pointer diagnostics, schema snapshots lock the contract, `just ready` passes.

## Must-Haves

- `RunManifest` and `ManifestSession` types in `assay-types/src/manifest.rs` with full derives, `deny_unknown_fields`, and `inventory::submit!`
- Types re-exported from `assay-types/src/lib.rs`
- Schema snapshot tests for `RunManifest` and `ManifestSession`
- `from_str()`, `load()`, and `validate()` functions in `assay-core/src/manifest.rs`
- `ManifestParse` and `ManifestValidation` error variants in `assay-core/src/error.rs`
- `format_toml_error` reused for caret-pointer error display
- Validation: at least one session required, spec field required and non-empty per session
- Round-trip TOML tests (parse → serialize → parse)
- Error-case tests (unknown fields, missing required fields, empty sessions array)
- `[[sessions]]` array format even for single-session (forward-compatible for multi-agent, D004)
- All functions are plain functions, not trait methods (D001, R009)

## Proof Level

- This slice proves: contract (type system + parsing + validation, no runtime pipeline usage)
- Real runtime required: no (unit tests with TOML fixtures)
- Human/UAT required: no

## Verification

- `cargo test -p assay-types -- schema_snapshots::run_manifest` — schema snapshots pass
- `cargo test -p assay-types -- schema_snapshots::manifest_session` — schema snapshots pass
- `cargo test -p assay-core -- manifest` — all parsing, loading, validation, and error tests pass
- `just ready` — full suite passes (fmt, clippy, test, deny)

## Observability / Diagnostics

- Runtime signals: none (types and parsing only — no runtime behavior)
- Inspection surfaces: schema snapshot `.snap` files detect type drift; `cargo insta test` shows diffs
- Failure visibility: `ManifestParse` errors include file path + caret-pointer line display; `ManifestValidation` errors list all issues at once for single-pass fix
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `assay-types/src/harness.rs` — `SettingsOverride`, `PromptLayer`, `HookContract` types (referenced by `ManifestSession` optional overrides)
- New wiring introduced in this slice: `assay-core::manifest` module with `from_str()`, `load()`, `validate()` — no runtime consumers yet
- What remains before the milestone is truly usable end-to-end: S07 (pipeline consumes `load()` to drive worktree → harness → agent → gate → merge)

## Tasks

- [x] **T01: Define RunManifest and ManifestSession types with schema snapshots** `est:20m`
  - Why: Establishes the type contract that parsing and S07 pipeline depend on (R014, R016)
  - Files: `crates/assay-types/src/manifest.rs`, `crates/assay-types/src/lib.rs`, `crates/assay-types/tests/schema_snapshots.rs`
  - Do: Create `RunManifest` (top-level with `sessions: Vec<ManifestSession>`) and `ManifestSession` (spec, optional harness overrides) with full derives, `deny_unknown_fields`, `inventory::submit!`, re-export from `lib.rs`, add 2 schema snapshot tests
  - Verify: `cargo test -p assay-types -- schema_snapshots::run_manifest` and `manifest_session` pass; `cargo build -p assay-core` still compiles
  - Done when: types compile, re-exported, schema snapshots accepted and locked

- [x] **T02: Add manifest parsing, validation, error variants, and tests** `est:30m`
  - Why: Delivers the complete parsing and validation contract that S07 consumes (R015, R014, R016)
  - Files: `crates/assay-core/src/manifest.rs`, `crates/assay-core/src/lib.rs`, `crates/assay-core/src/error.rs`, `crates/assay-core/src/config/mod.rs`
  - Do: Add `ManifestParse`/`ManifestValidation` error variants; make `format_toml_error` accessible to manifest module; implement `from_str()`, `load()`, `validate()` following config pattern; add comprehensive tests (round-trip, unknown fields, missing spec, empty sessions, valid minimal, valid full, load from file, load error cases)
  - Verify: `cargo test -p assay-core -- manifest` passes; `just ready` passes
  - Done when: all parsing/validation/error tests pass, `just ready` green

## Files Likely Touched

- `crates/assay-types/src/manifest.rs` (new)
- `crates/assay-types/src/lib.rs`
- `crates/assay-types/tests/schema_snapshots.rs`
- `crates/assay-core/src/manifest.rs` (new)
- `crates/assay-core/src/lib.rs`
- `crates/assay-core/src/error.rs`
- `crates/assay-core/src/config/mod.rs`
