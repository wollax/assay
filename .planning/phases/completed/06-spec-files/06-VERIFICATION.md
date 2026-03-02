# Phase 06 — Spec Files: Verification Report

**Date:** 2026-03-01
**Status:** passed

---

## Must-Have Checklist

| # | Requirement | Result | Evidence |
|---|-------------|--------|----------|
| 1 | Spec struct has `name`, `description`, and `criteria: Vec<Criterion>` fields | PASS | `crates/assay-types/src/lib.rs:14-25` |
| 2 | Spec and Criterion both have `#[serde(deny_unknown_fields)]` | PASS | `assay-types/src/lib.rs:13`; `assay-types/src/criterion.rs:12` |
| 3 | Spec `description` defaults to empty string and is skipped when empty in serialized output | PASS | `assay-types/src/lib.rs:20` — `#[serde(default, skip_serializing_if = "String::is_empty")]` |
| 4 | `from_str()` parses valid TOML with `[[criteria]]` into a `Spec` struct | PASS | `assay-core/src/spec/mod.rs:42-44`; tests `from_str_valid_minimal`, `from_str_valid_with_description_and_cmd`, `from_str_valid_multiple_criteria` |
| 5 | `from_str()` rejects unknown fields on both Spec and Criterion | PASS | `assay-core/src/spec/mod.rs` tests `from_str_rejects_unknown_spec_field` (line 271) and `from_str_rejects_unknown_criterion_field` (line 293) |
| 6 | `validate()` rejects empty/whitespace-only name, zero criteria, and duplicate criterion names | PASS | `assay-core/src/spec/mod.rs:54-87`; tests `validate_empty_name`, `validate_whitespace_only_name`, `validate_zero_criteria`, `validate_duplicate_criterion_names`, `validate_empty_criterion_name` |
| 7 | `validate()` collects all validation errors at once (not fail-fast) | PASS | `assay-core/src/spec/mod.rs:51-88` — single `errors` Vec accumulated through all checks; test `validate_collects_all_errors_at_once` (line 417) asserts `errors.len() == 2` for spec with whitespace name + empty criteria |
| 8 | `load()` reads a single spec file, parses it, validates it, and returns `Spec` | PASS | `assay-core/src/spec/mod.rs:95-115`; test `load_valid_spec` (line 442) |
| 9 | `load()` returns `AssayError::SpecParse` with file path when TOML is invalid | PASS | `assay-core/src/spec/mod.rs:102-105`; test `load_invalid_toml_returns_spec_parse` (line 474) asserts path ends with `bad.toml` and message contains `"TOML parse error"` |
| 10 | `load()` returns `AssayError::SpecValidation` with all errors when spec is semantically invalid | PASS | `assay-core/src/spec/mod.rs:107-112`; test `load_valid_toml_invalid_semantics_returns_spec_validation` (line 495) |
| 11 | `scan()` finds all `.toml` files in a directory and returns `ScanResult` with parsed specs and errors | PASS | `assay-core/src/spec/mod.rs:122-178`; tests `scan_valid_specs`, `scan_mixed_valid_and_invalid`, `scan_ignores_non_toml_files` |
| 12 | `scan()` detects duplicate spec names across files and reports them as errors | PASS | `assay-core/src/spec/mod.rs:155-175`; test `scan_detects_duplicate_spec_names` (line 647) |
| 13 | `scan()` returns specs sorted by filename | PASS | `assay-core/src/spec/mod.rs:134` — `paths.sort()` before processing; test `scan_sorted_by_filename` (line 615) writes `zeta.toml` before `alpha.toml` and asserts `specs[0].0 == "alpha"` |
| 14 | `AssayError` has `SpecParse`, `SpecValidation`, and `SpecScan` variants | PASS | `assay-core/src/error.rs:49-73` — all three variants present with correct fields |
| 15 | `assay spec show <name>` displays a table with spec name, description, and criteria | PASS | `assay-cli/src/main.rs:88-194` — `handle_spec_show()` prints `"Spec: {name}"`, optional description, and a padded column table of criteria |
| 16 | `assay spec show <name> --json` outputs the parsed spec as pretty-printed JSON | PASS | `assay-cli/src/main.rs:114-121` — `serde_json::to_string_pretty(&spec)` when `json == true` |
| 17 | `assay spec show <name>` for a non-existent spec exits with a clear error | PASS | `assay-cli/src/main.rs:102-107` — matches `AssayError::Io { source, .. }` where `source.kind() == NotFound`, prints `"Error: spec '{name}' not found in {specs_dir}"` and exits 1 |
| 18 | `assay spec list` displays all spec names found in the specs directory | PASS | `assay-cli/src/main.rs:197-247` — `handle_spec_list()` prints each slug (with optional description column) |
| 19 | `assay spec list` with no specs prints a message indicating no specs found | PASS | `assay-cli/src/main.rs:221-224` — `println!("No specs found in {}", config.specs_dir)` |
| 20 | Terminal colors respect `NO_COLOR` env var | PASS | `assay-cli/src/main.rs:58-60` — `colors_enabled()` returns `false` when `NO_COLOR` is set (any value); used in `handle_spec_show` via `format_criteria_type` |
| 21 | Both commands resolve the specs directory from `.assay/config.toml` | PASS | `assay-cli/src/main.rs:90-97` (`handle_spec_show`) and `199-206` (`handle_spec_list`) — both call `assay_core::config::load(&root)` then `root.join(".assay").join(&config.specs_dir)` |

---

## Overall Assessment

All 21 must-haves are satisfied. Evidence is direct — read from source code, not from summaries.

**Key observations:**

- The type layer (`assay-types`) correctly separates `Spec` and `Criterion` with `deny_unknown_fields` on both.
- The domain layer (`assay-core/spec`) implements a clean three-function API: `from_str` (raw parse), `validate` (semantic checks, all-at-once), `load` (parse + validate with error wrapping), and `scan` (directory scan with duplicate detection).
- `scan()` uses `paths.sort()` on a `Vec<PathBuf>`, which gives correct lexicographic filename order.
- The CLI thin-wrapper pattern is respected: `assay-cli` delegates entirely to `assay-core::spec::load`/`scan` and `assay-core::config::load`.
- `NO_COLOR` is honoured per the spec at no-color.org — presence of the env var (any value) suppresses ANSI codes.
- The `schemas/spec.schema.json` is consistent with the Rust types: `additionalProperties: false` on both `Spec` and `Criterion`, `description` not required (defaults), `name` and `criteria` required.
- 78 tests were reported passing at last `just ready` run (phase 06-02 summary); the spec module alone contributes 17 unit tests covering all critical code paths.

**No gaps found.**
