---
id: T02
parent: S06
milestone: M001
provides:
  - Manifest parsing, validation, and loading in assay-core
  - ManifestParse and ManifestValidation error variants in AssayError
  - ManifestError struct for field-level validation issues
key_files:
  - crates/assay-core/src/manifest.rs
  - crates/assay-core/src/error.rs
  - crates/assay-core/src/lib.rs
key_decisions:
  - Reused format_toml_error from config module via pub(crate) visibility (already accessible)
patterns_established:
  - Manifest module mirrors config module pattern exactly (from_str → validate → load)
  - ManifestError follows ConfigError struct pattern (field + message + Display)
observability_surfaces:
  - ManifestParse errors include file path + caret-pointer line/column display
  - ManifestValidation errors list all issues at once with field paths (e.g. sessions[0].spec)
duration: fast
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T02: Add manifest parsing, validation, error variants, and tests

**Implemented manifest from_str/validate/load with ManifestParse and ManifestValidation error variants, plus 13 comprehensive tests.**

## What Happened

Created `crates/assay-core/src/manifest.rs` with the full manifest parsing pipeline mirroring the config module:
- `ManifestError` struct with field + message + Display
- `from_str()` — raw TOML parse returning `toml::de::Error`
- `validate()` — collects all semantic errors (empty sessions, blank specs) without fail-fast
- `load()` — reads file, parses with `format_toml_error` for caret-pointer display, then validates

Added `ManifestParse` and `ManifestValidation` variants to `AssayError` in error.rs, following the ConfigParse/ConfigValidation pattern exactly. The `format_toml_error` function was already `pub(crate)` so no visibility change was needed there.

## Verification

- `cargo test -p assay-core -- manifest` — all 13 tests pass
- `cargo test -p assay-types -- schema_snapshots::run_manifest` — schema snapshot passes
- `cargo test -p assay-types -- schema_snapshots::manifest_session` — schema snapshot passes
- `just ready` — full suite green (fmt, clippy, 653+ tests, deny)

## Diagnostics

- `ManifestParse` error Display shows: `parsing manifest '{path}': line N, column M: {message}` with source line and caret pointer
- `ManifestValidation` error Display shows: `invalid manifest '{path}':` followed by indented `- field: message` lines
- `ManifestError` exposes `field` and `message` for programmatic inspection

## Deviations

- Test `from_str_valid_full` initially used incorrect field names (`timeout`, `pre_eval`, `label`) for SettingsOverride, HookContract, and PromptLayer. Fixed to use actual schema fields (`max_turns`, `pre-tool`/`command`, `kind`/`name`/`content`/`priority`).

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/manifest.rs` — New module: ManifestError, from_str, validate, load, 13 tests
- `crates/assay-core/src/error.rs` — Added ManifestParse and ManifestValidation variants to AssayError
- `crates/assay-core/src/lib.rs` — Added `pub mod manifest`
- `crates/assay-core/src/config/mod.rs` — Changed TruncatedLine struct visibility to pub(crate)
