# Phase 31 Plan 01: Error Message Pure Functions Summary

**One-liner:** Levenshtein fuzzy matching, exit code classification, and TOML error formatting with source-line caret display

## Metadata

- **Phase:** 31-error-messages
- **Plan:** 01
- **Subsystem:** error-handling
- **Tags:** error-messages, levenshtein, toml, diagnostics
- **Completed:** 2026-03-10
- **Duration:** ~8 minutes

## What Was Built

### ERR-01: Command Error Classification (gate/mod.rs)

- `CommandErrorKind` enum: `NotFound` (exit 127), `NotExecutable` (exit 126)
- `extract_binary()`: extracts first whitespace-delimited token from command strings
- `classify_exit_code()`: maps shell exit codes to error kinds
- `format_command_error()`: produces actionable messages like "command 'cargo' not found. Is it installed and in PATH?"

### ERR-02: Spec-Not-Found Diagnostics (spec/mod.rs, error.rs)

- `levenshtein()`: standard 1D DP edit distance algorithm
- `find_fuzzy_match()`: returns single unambiguous suggestion (distance <= 2 AND <= name.len()/2)
- `format_spec_not_found()`: rich diagnostic with available specs list (truncated at 10), invalid spec markers, fuzzy suggestion
- `SpecNotFoundDiagnostic` error variant: delegates Display to format_spec_not_found

### ERR-03: TOML Parse Error Formatting (config/mod.rs)

- `TruncatedLine` struct for truncation results
- `translate_position()`: byte offset to (line, col) conversion
- `truncate_source_line()`: centers error column in ~80 char window with `...` ellipsis
- `format_toml_error()`: produces source-line + caret pointer display, falls back to message-only when no span

## Key Files

### Created
None

### Modified
- `crates/assay-core/src/gate/mod.rs` — ERR-01 functions + tests
- `crates/assay-core/src/spec/mod.rs` — ERR-02 functions + tests
- `crates/assay-core/src/error.rs` — SpecNotFoundDiagnostic variant + tests
- `crates/assay-core/src/config/mod.rs` — ERR-03 functions + tests

## Decisions Made

1. **All functions marked `pub(crate)` with `#[allow(dead_code)]`** — Plan 02 will wire them into call sites. The dead_code allows prevent clippy failures while keeping functions accessible within the crate.
2. **format_spec_not_found lives in spec/mod.rs** — called from error.rs via the SpecNotFoundDiagnostic Display impl using a crate-path reference.
3. **Fuzzy match threshold: distance <= 2 AND distance <= name.len()/2** — prevents suggesting "b" for "a" (short names) while catching typos like "auth-flw" -> "auth-flow".

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Dead code warnings broke `just lint`**
- **Found during:** Verification step
- **Issue:** All 10 new functions/types are `pub(crate)` but not yet called from production code (Plan 02 wires them in), causing `-D warnings` to fail on dead_code lint.
- **Fix:** Added `#[allow(dead_code)]` with comments explaining Plan 02 will remove them.
- **Commit:** 810a28e

## Test Coverage

- 11 tests for ERR-01 (extract_binary, classify_exit_code, format_command_error)
- 14 tests for ERR-02 (levenshtein: 6, find_fuzzy_match: 5, format_spec_not_found: 5, SpecNotFoundDiagnostic Display: 3 — some overlap)
- 13 tests for ERR-03 (translate_position: 6, truncate_source_line: 5, format_toml_error: 2)

## Verification

- `just test` — all tests pass
- `just lint` — no clippy warnings
- `just fmt-check` — formatting correct
