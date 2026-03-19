---
estimated_steps: 5
estimated_files: 4
---

# T02: Add manifest parsing, validation, error variants, and tests

**Slice:** S06 — RunManifest Type & Parsing
**Milestone:** M001

## Description

Implement `from_str()`, `load()`, and `validate()` for `RunManifest` in `assay-core/src/manifest.rs`, following the config module pattern exactly. Add `ManifestParse` and `ManifestValidation` error variants. Make `format_toml_error` accessible to the manifest module. Write comprehensive tests covering round-trip parsing, error formatting, validation rules, and file loading.

## Steps

1. Add `ManifestParse` and `ManifestValidation` error variants to `crates/assay-core/src/error.rs` mirroring `ConfigParse`/`ConfigValidation`:
   - `ManifestParse { path: PathBuf, message: String }` — with Display: `"parsing manifest '{path}': {message}"`
   - `ManifestValidation { path: PathBuf, errors: Vec<ManifestError> }` — with Display listing all errors
   - Define `ManifestError` (field + message) in the manifest module, import in error.rs
2. Make `format_toml_error` accessible to the manifest module — either promote `config::format_toml_error` to `pub(crate)` or extract as a shared utility within config module that manifest can call via `crate::config::format_toml_error`
3. Create `crates/assay-core/src/manifest.rs` with:
   - `ManifestError` struct (field, message) with Display impl
   - `from_str(s: &str) -> Result<RunManifest, toml::de::Error>` — raw TOML parse
   - `validate(manifest: &RunManifest) -> Result<(), Vec<ManifestError>>` — semantic checks: sessions non-empty, each session spec non-empty
   - `load(path: &Path) -> Result<RunManifest>` — read file → `from_str` (wrap in `ManifestParse`) → `validate` (wrap in `ManifestValidation`)
4. Add `pub mod manifest` to `crates/assay-core/src/lib.rs`
5. Write tests in `crates/assay-core/src/manifest.rs` `#[cfg(test)] mod tests`:
   - `from_str_valid_minimal` — single session with just spec
   - `from_str_valid_full` — session with all optional overrides (settings, hooks, prompt_layers)
   - `from_str_multiple_sessions` — two `[[sessions]]` entries
   - `from_str_round_trip` — parse → serialize → parse, assert equality
   - `from_str_rejects_unknown_fields` — unknown top-level key
   - `from_str_rejects_unknown_session_fields` — unknown key in `[[sessions]]`
   - `validate_empty_sessions_rejected` — sessions array empty
   - `validate_empty_spec_rejected` — session with blank spec
   - `validate_collects_all_errors` — multiple invalid sessions, all reported
   - `load_valid_manifest` — write to tempfile, load succeeds
   - `load_missing_file_returns_io_error` — path doesn't exist
   - `load_invalid_toml_returns_manifest_parse` — bad TOML, error includes caret-pointer
   - `load_valid_toml_invalid_semantics_returns_manifest_validation` — empty sessions

## Must-Haves

- [ ] `ManifestParse` and `ManifestValidation` error variants added to `AssayError`
- [ ] `ManifestError` struct with field + message + Display
- [ ] `from_str()` returns `toml::de::Error` on parse failure
- [ ] `validate()` collects all errors at once (not fail-fast)
- [ ] `load()` uses `format_toml_error` for caret-pointer display
- [ ] All 13 tests pass
- [ ] `just ready` passes (fmt, clippy, test, deny)

## Verification

- `cargo test -p assay-core -- manifest` — all 13 tests pass
- `just ready` — full suite green

## Observability Impact

- Signals added/changed: `ManifestParse` errors include file path + formatted caret-pointer line; `ManifestValidation` errors list all issues with field paths
- How a future agent inspects this: error Display output shows exact line/column for parse errors, field-level messages for validation errors
- Failure state exposed: `ManifestParse.message` contains line number, column, source line, and caret pointer; `ManifestValidation.errors` is a Vec for programmatic inspection

## Inputs

- `crates/assay-types/src/manifest.rs` — `RunManifest`, `ManifestSession` types (from T01)
- `crates/assay-core/src/config/mod.rs` — `format_toml_error()` function to reuse, `ConfigError` pattern to mirror
- `crates/assay-core/src/error.rs` — `ConfigParse`/`ConfigValidation` pattern to mirror

## Expected Output

- `crates/assay-core/src/manifest.rs` — `ManifestError`, `from_str()`, `validate()`, `load()`, 13 tests
- `crates/assay-core/src/lib.rs` — `pub mod manifest` added
- `crates/assay-core/src/error.rs` — `ManifestParse`, `ManifestValidation` variants added
- `crates/assay-core/src/config/mod.rs` — `format_toml_error` visibility adjusted to `pub(crate)` (or re-exported)
