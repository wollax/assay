# Phase 6 Plan 02: Wire CLI spec show and spec list subcommands Summary

**One-liner:** Two CLI subcommands (`assay spec show` and `assay spec list`) that delegate to the core spec module, with table/JSON output, ANSI color support respecting NO_COLOR, and clear error messages.

## Execution Details

| Field | Value |
|-------|-------|
| Phase | 06-spec-files |
| Plan | 02 |
| Type | execute |
| Duration | ~6 minutes |
| Completed | 2026-03-02 |
| Tasks | 2/2 (1 auto + 1 checkpoint:human-verify) |
| Tests Total (workspace) | 78 |

## What Was Built

### CLI Subcommands (assay-cli/src/main.rs)

| Command | Description |
|---------|-------------|
| `assay spec show <name>` | Display a spec's criteria in a formatted table |
| `assay spec show <name> --json` | Output the parsed spec as pretty-printed JSON |
| `assay spec list` | List all available specs with descriptions |

### Spec Show — Table Output

Displays spec name, description (if non-empty), and a table of criteria with columns: #, Criterion, Type, Command. Criteria types are color-differentiated:
- **executable** (has `cmd`) — green (`\x1b[32m`)
- **descriptive** (no `cmd`) — yellow (`\x1b[33m`)

Column widths are computed dynamically from data. Unicode box-drawing characters (`\u{2500}`) used for separators.

### Spec Show — JSON Output

Uses `serde_json::to_string_pretty()` to emit the parsed `Spec` struct directly. Fields match the TOML schema: `name`, `description` (omitted when empty), `criteria` array with `name`, `description`, `cmd` (omitted when None).

### Spec List Output

Displays all spec slugs (filenames without `.toml` extension) with descriptions, aligned in columns. Scan errors are printed as warnings to stderr. Empty directory prints `No specs found in {specs_dir}`.

### Error Handling

| Scenario | Behavior |
|----------|----------|
| Config not found | `Error: reading config at ...` + exit 1 |
| Spec not found | `Error: spec '{name}' not found in {specs_dir}` + exit 1 |
| Spec parse/validation error | Prints the AssayError Display + exit 1 |
| Scan I/O error | Prints the AssayError Display + exit 1 |

### NO_COLOR Support

`colors_enabled()` checks `std::env::var("NO_COLOR").is_err()` per the no-color.org convention. When set (any value), all ANSI escape codes are suppressed.

### Dependency Addition

`serde_json.workspace = true` added to `crates/assay-cli/Cargo.toml` (already a workspace dependency).

## Commits

| Hash | Type | Description |
|------|------|-------------|
| `98ae2bc` | feat | Add spec show and spec list CLI subcommands |

## Deviations from Plan

None — plan executed exactly as written.

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| ANSI escape codes directly, no color library | Plan specified no external color deps; raw codes are simple for two colors |
| `println!`-based formatting, no table library | Plan specified no table library; column alignment via format width specifiers |
| `project_root()` helper instead of combined resolve function | Avoids depending on `assay_types::Config` directly in CLI (not a direct dependency) |
| Dynamic column widths from data | Adapts to varying criterion name lengths without hardcoded widths |
| Unicode box-drawing separators (`─`) | Matches plan's table format example |

## Verification Results

All 7 end-to-end scenarios passed human verification:

1. `spec show hello-world` — table with colored types, 2 criteria displayed correctly
2. `spec show hello-world --json` — valid pretty-printed JSON output
3. `spec show nonexistent` — clear "spec not found" error, exit code 1
4. `spec list` — shows "hello-world" with description
5. `spec list` (no project) — config not found error, exit code 1
6. `NO_COLOR=1 spec show hello-world` — no ANSI escape codes in output
7. `just ready` — all checks passed (78 tests)

## Requirements Satisfied

- **SPEC-06**: `assay spec show <name>` displays table with spec name, description, and criteria
- **SPEC-06-json**: `assay spec show <name> --json` outputs parsed spec as pretty-printed JSON
- **SPEC-06-error**: Non-existent spec exits with clear error message
- **SPEC-06-list**: `assay spec list` displays all spec names found in specs directory
- **SPEC-06-list-empty**: Empty specs directory prints "no specs found" message
- **SPEC-06-color**: Terminal colors differentiate criteria types, respecting NO_COLOR
- **SPEC-06-config**: Both commands resolve specs directory from .assay/config.toml

## Key Files

### Modified
- `crates/assay-cli/Cargo.toml` (added serde_json dependency)
- `crates/assay-cli/src/main.rs` (Spec subcommand with Show and List variants, handlers, color support)
- `Cargo.lock` (updated for new dependency wiring)

## Key Links Verified

| From | To | Via |
|------|----|-----|
| `crates/assay-cli/src/main.rs` | `crates/assay-core/src/spec/mod.rs` | `assay_core::spec::{load, scan}` |
| `crates/assay-cli/src/main.rs` | `crates/assay-core/src/config/mod.rs` | `assay_core::config::load` |

## Phase 6 Completion

With Plan 02 complete, Phase 6 (Spec Files) is fully done:
- Plan 01: Spec type updates, error variants, spec module (from_str/validate/load/scan)
- Plan 02: CLI spec show and spec list subcommands

Phase 7 (Gate Evaluation) can proceed. No blockers or concerns.
