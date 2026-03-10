# Phase 31 Plan 02: Wire Error Formatting into Call Sites Summary

**One-liner:** Connected all Plan 01 pure functions to production error paths across CLI and MCP surfaces

## Metadata

- **Phase:** 31-error-messages
- **Plan:** 02
- **Subsystem:** error-handling
- **Tags:** error-messages, cli, mcp, diagnostics
- **Completed:** 2026-03-10
- **Duration:** ~9 minutes

## What Was Wired

### ERR-01: Exit Code Detection (Task 1)

- `evaluate_criteria()` in gate/mod.rs now appends actionable hints to stderr for exit 127/126
- `stream_criterion()` in CLI gate.rs prints hint below FAIL line for exit 127/126
- `enriched_error_display()` added for spawn errors (NotFound/PermissionDenied)
- `CommandErrorKind`, `classify_exit_code`, `format_command_error` promoted to `pub`

### ERR-02: Spec-Not-Found Diagnostics (Task 2)

- `load_spec_entry_with_diagnostics()` created in spec/mod.rs: wraps `load_spec_entry()`, catches SpecNotFound, scans directory, runs fuzzy match, returns SpecNotFoundDiagnostic
- 4 CLI gate.rs call sites replaced bail!() with diagnostics propagation
- 1 CLI spec.rs call site replaced bail!() with diagnostics propagation
- MCP server.rs `load_spec_entry_mcp()` updated to use diagnostics version
- Invalid specs extracted from scan errors with path stem matching

### ERR-03: TOML Error Formatting (Task 3)

- config/mod.rs `load()`: `e.to_string()` replaced with `format_toml_error(&content, &e)`
- spec/mod.rs `load()`: same for SpecParse
- spec/mod.rs `load_gates()`: same for GatesSpecParse
- spec/mod.rs `load_feature_spec()`: same for FeatureSpecParse

## Deviations

None. Plan executed as specified.

## Decisions

- Made `CommandErrorKind`, `classify_exit_code`, `format_command_error`, and `enriched_error_display` `pub` (not `pub(crate)`) so CLI crate can use them directly for streaming display
- MCP not-found tests updated to accept either "No specs found" or spec name in diagnostic, since zero-specs directories produce different messages

## Test Updates

- config/mod.rs `load_invalid_toml_returns_config_parse`: assertion changed from "TOML parse error" to "line" (new format)
- spec/mod.rs `load_invalid_toml_returns_spec_parse`: same pattern
- MCP `test_load_spec_entry_not_found`, `spec_get_missing_spec_returns_error`, `gate_run_nonexistent_spec_returns_error`: updated for enriched diagnostic messages

## Commits

| Hash | Description |
|------|-------------|
| `820c987` | feat(31-02): wire ERR-01 exit code detection into gate evaluation |
| `2669a47` | feat(31-02): wire ERR-02 spec-not-found diagnostics into all call sites |
| `6ec9d2c` | feat(31-02): wire ERR-03 TOML error formatting into all parse sites |

## Verification

- `just ready` passes (fmt-check + lint + test + deny)
- All 3 tasks verified individually with `just test` + `just lint`
- Zero new workspace dependencies
- All `#[allow(dead_code)]` annotations removed from Plan 01 functions
