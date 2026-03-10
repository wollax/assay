# Phase 31 Verification

**Phase:** 31-error-messages
**Verified:** 2026-03-10
**Status:** passed

## Must-Haves

### 1. Command-not-found error shows binary name and PATH hint
**Status:** PASS

**Evidence:**

`crates/assay-core/src/gate/mod.rs` implements the full pipeline:

- `classify_exit_code(code: i32) -> Option<CommandErrorKind>` maps exit code 127 → `NotFound`, 126 → `NotExecutable` (line 56–61).
- `format_command_error(cmd: &str, kind: CommandErrorKind) -> String` (line 68–78) extracts the binary name from the command string and returns:
  - `"command '{binary}' not found. Is it installed and in PATH?"` for `NotFound`
  - `"command '{binary}' found but not executable. Check file permissions."` for `NotExecutable`
- `enriched_error_display` (line 313–330) handles spawn failures (OS `io::ErrorKind::NotFound`) and similarly enriches the error with the actionable hint.

**CLI layer** (`crates/assay-cli/src/commands/gate.rs`, lines 252–260): `stream_criterion` inspects `result.exit_code`, calls `classify_exit_code`, then calls `format_command_error` and prints the hint to stderr indented under the failing criterion.

**Core layer** (`evaluate_criteria`, lines 246–256): For JSON/batch mode, the hint is appended directly to `gate_result.stderr` so it surfaces in structured output too.

Tests covering this: `classify_exit_code_127_is_not_found`, `classify_exit_code_126_is_not_executable`, `format_command_error_not_found_message`, `format_command_error_not_executable_message` (gate/mod.rs).

---

### 2. Spec-not-found shows available specs
**Status:** PASS

**Evidence:**

`crates/assay-core/src/spec/mod.rs` defines:

- `format_spec_not_found(name, specs_dir, available, invalid, suggestion)` (line 70): produces context-sensitive output:
  - Zero specs: `"No specs found in {specs_dir}."`
  - With specs: `"spec '{name}' not found. Available specs: {comma-list}"` (truncated to 10 with `(and N more)` if over limit)
  - With invalid specs: marks them as `"{name} (invalid)"` in the list
  - With suggestion: appends `"Did you mean '{suggestion}'?"` (fuzzy Levenshtein match, threshold ≤ 2 and ≤ name.len()/2)

- `AssayError::SpecNotFoundDiagnostic` variant (error.rs, line 114) delegates its `Display` to `format_spec_not_found`.

- `load_spec_entry_with_diagnostics(slug, specs_dir)` (line 416): wraps `load_spec_entry`, catches `SpecNotFound`, scans the directory, collects valid slugs + invalid slugs (from parse/validation errors), runs `find_fuzzy_match`, and returns `SpecNotFoundDiagnostic`.

**CLI:** `handle_spec_show` (spec.rs line 56) and `handle_gate_run` (gate.rs line 449) both call `load_spec_entry_with_diagnostics` — the enriched error propagates to the user.

**MCP:** `load_spec_entry_mcp` (server.rs line 1075–1083) also calls `load_spec_entry_with_diagnostics` and maps the error to a `CallToolResult` domain error via `domain_error(&e)`.

Tests covering this: `format_spec_not_found_zero_specs`, `format_spec_not_found_three_specs`, `format_spec_not_found_eleven_specs_truncated`, `format_spec_not_found_with_invalid`, `format_spec_not_found_with_suggestion`, and the matching `AssayError` display tests in error.rs.

---

### 3. TOML parse error shows file path, line number, and specific parse error message
**Status:** PASS

**Evidence:**

`crates/assay-core/src/config/mod.rs` defines `format_toml_error(content, err)` (line 97–117):
- Calls `err.message()` for the specific parse message.
- Uses `err.span()` to get the byte offset, then `translate_position` to convert to `(line, col)`.
- Extracts the source line from the content, truncates it to 80 chars via `truncate_source_line`, and returns a formatted string of the form:
  ```
  line {N}, column {M}: {message}
    |
  N | {source_line}
    |   ^
  ```
- Falls back to just the message if no span is available.

The file path is included by the **error variant display**:
- `AssayError::SpecParse` (error.rs line 60): `#[error("parsing spec `{path}`: {message}")]` — the `message` field is populated with `format_toml_error(...)` output (spec/mod.rs line 311–314).
- `AssayError::GatesSpecParse` (error.rs line 148): same pattern, used in `load_gates` (spec/mod.rs line 334–337).
- `AssayError::FeatureSpecParse` (error.rs line 130): same, used in `load_feature_spec` (spec/mod.rs line 357–360).
- `AssayError::ConfigParse` (error.rs line 38): same, used in `config::load()` (config/mod.rs line 194–197).

So the final display message is: `"parsing spec '<path>': line N, column M: <toml message>\n<gutter>\nN | <source>\n  | <caret>"` — containing path, line number, and specific error.

Tests covering this: `format_toml_error_with_span`, `format_toml_error_multiline`, `load_invalid_toml_returns_config_parse` (config/mod.rs), `load_invalid_toml_returns_spec_parse` (spec/mod.rs), and `translate_position_*` / `truncate_source_line_*` unit tests.

---

## Test Suite
**Status:** All 599 tests pass, 0 failures. `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`, and `cargo deny check` all pass.

Breakdown:
- assay-core: 401 tests OK
- assay-mcp: 53 tests OK
- assay-cli: 8 tests OK
- assay-tui: 26 tests OK
- assay-types: 34 tests OK
- assay-guard: 23 tests OK

## Summary
3/3 must-haves verified. Status: passed
