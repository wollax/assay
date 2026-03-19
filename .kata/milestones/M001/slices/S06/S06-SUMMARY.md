---
id: S06
parent: M001
milestone: M001
provides:
  - RunManifest and ManifestSession types in assay-types with schema snapshots
  - Manifest parsing (from_str), validation (validate), and file loading (load) in assay-core
  - ManifestParse and ManifestValidation error variants with caret-pointer diagnostics
requires:
  - slice: S02
    provides: HarnessProfile, SettingsOverride, PromptLayer, HookContract types in assay-types
affects:
  - S07
key_files:
  - crates/assay-types/src/manifest.rs
  - crates/assay-core/src/manifest.rs
  - crates/assay-core/src/error.rs
key_decisions:
  - D014 — ManifestSession uses inline optional overrides rather than embedding HarnessProfile
patterns_established:
  - Manifest module mirrors config module pattern exactly (from_str → validate → load)
  - ManifestError follows ConfigError struct pattern (field + message + Display)
observability_surfaces:
  - Schema snapshot .snap files detect future type drift via cargo insta test
  - ManifestParse errors include file path + caret-pointer line/column display
  - ManifestValidation errors list all issues at once with field paths (e.g. sessions[0].spec)
drill_down_paths:
  - .kata/milestones/M001/slices/S06/tasks/T01-SUMMARY.md
  - .kata/milestones/M001/slices/S06/tasks/T02-SUMMARY.md
duration: 20m
verification_result: passed
completed_at: 2026-03-16
---

# S06: RunManifest Type & Parsing

**TOML manifest types with `[[sessions]]` array parsing, semantic validation, and caret-pointer error diagnostics.**

## What Happened

T01 created `RunManifest` and `ManifestSession` types in `assay-types/src/manifest.rs`. `RunManifest` is the top-level struct with `sessions: Vec<ManifestSession>`. `ManifestSession` has `spec: String` (required), `name: Option<String>`, and optional inline overrides (`settings`, `hooks`, `prompt_layers`) rather than embedding `HarnessProfile` directly. Both types have full derives, `deny_unknown_fields`, `inventory::submit!`, and schema snapshot tests.

T02 created `assay-core/src/manifest.rs` with `from_str()`, `validate()`, and `load()` following the config module pattern exactly. Two new error variants (`ManifestParse`, `ManifestValidation`) were added to `AssayError`. `ManifestParse` reuses `format_toml_error` for caret-pointer display. `ManifestValidation` collects all semantic errors (empty sessions, blank specs) without fail-fast so users can fix everything in one pass. 13 comprehensive tests cover round-trip parsing, unknown field rejection, missing required fields, empty sessions, valid minimal/full manifests, and file loading.

## Verification

- `cargo test -p assay-types -- manifest` — 2 schema snapshot tests pass
- `cargo test -p assay-core -- manifest` — 13 parsing/validation/loading tests pass
- `just ready` — full suite green (fmt, clippy, 653+ tests, deny)

## Requirements Advanced

- R014 — RunManifest type defined with `[[sessions]]` TOML array format, schema locked by snapshots
- R015 — Manifest parsing and validation with actionable caret-pointer errors and field-level validation
- R016 — Forward compatibility established via `[[sessions]]` array even for single-session manifests
- R009 — All manifest functions are plain functions, not trait methods (constraint continues)

## Requirements Validated

- R014 — Schema snapshot tests lock the type contract; types compile and round-trip through TOML
- R015 — 13 tests prove parsing, validation, and error quality (caret pointers, field paths, collect-all-errors)
- R016 — `[[sessions]]` array format verified by all test fixtures; single-session uses same array syntax

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

T02 initially used incorrect field names in the `from_str_valid_full` test (`timeout`, `pre_eval`, `label` instead of `max_turns`, `pre-tool`/`command`, `kind`/`name`/`content`/`priority`). Fixed during execution to match actual schema. No plan deviation.

## Known Limitations

- ManifestSession overrides are parsed but not consumed by any runtime pipeline yet (S07 will wire them)
- No manifest schema evolution strategy beyond `deny_unknown_fields` (sufficient for M001)

## Follow-ups

- S07 consumes `load()` to drive the end-to-end pipeline (manifest → worktree → harness → agent → gate → merge)

## Files Created/Modified

- `crates/assay-types/src/manifest.rs` — RunManifest and ManifestSession types with full derives and docs
- `crates/assay-types/src/lib.rs` — Added `pub mod manifest` and re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — 2 schema snapshot tests
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — locked schema
- `crates/assay-types/tests/snapshots/schema_snapshots__manifest-session-schema.snap` — locked schema
- `crates/assay-core/src/manifest.rs` — ManifestError, from_str, validate, load, 13 tests
- `crates/assay-core/src/error.rs` — ManifestParse and ManifestValidation variants
- `crates/assay-core/src/lib.rs` — Added `pub mod manifest`
- `crates/assay-core/src/config/mod.rs` — TruncatedLine visibility changed to pub(crate)

## Forward Intelligence

### What the next slice should know
- `assay_core::manifest::load(path)` is the entry point — returns `Result<RunManifest>` with validated contents
- `ManifestSession.settings`, `.hooks`, `.prompt_layers` map directly to `HarnessProfile` fields but are inline optional overrides, not a nested `HarnessProfile` — S07 must construct `HarnessProfile` from these fields plus defaults
- The `format_toml_error` function is `pub(crate)` in `assay-core::config` — already accessible from any `assay-core` module

### What's fragile
- `deny_unknown_fields` on both types means any field addition requires schema snapshot updates — intentional but easy to forget during rapid iteration

### Authoritative diagnostics
- `cargo insta test -p assay-types` — shows schema drift immediately if types change
- `ManifestValidation` Display output lists all field-level errors at once — trustworthy for debugging manifest authoring issues

### What assumptions changed
- No assumptions changed — this slice executed exactly as planned
