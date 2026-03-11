# Phase 37 Plan 01: Validation Types & Domain Logic Summary

**Structured validation diagnostics and core validation logic for static spec analysis**

## Frontmatter

- **Phase:** 37-spec-validation
- **Plan:** 01
- **Subsystem:** validation
- **Tags:** validation, diagnostics, cycle-detection, which, serde, schemars
- **Completed:** 2026-03-11
- **Duration:** ~15 minutes

### Dependency Graph

- **Requires:** Phase 35-36 (existing spec validation, gate infrastructure)
- **Provides:** ValidationResult, Diagnostic, Severity types; validate_spec, detect_cycles functions
- **Affects:** Phase 37 Plan 02 (MCP tool wiring uses validate_spec_with_dependencies)

### Tech Stack

- **Added:** `which` v7 (command binary PATH lookup)
- **Patterns:** Bridge pattern (SpecError -> Diagnostic conversion), DFS three-color cycle detection

### Key Files

- **Created:**
  - `crates/assay-types/src/validation.rs` — Severity, Diagnostic, ValidationResult, DiagnosticSummary types
  - `crates/assay-core/src/spec/validate.rs` — Core validation logic with 6 functions + 14 tests
- **Modified:**
  - `crates/assay-types/src/lib.rs` — Added validation module, depends field on Spec
  - `crates/assay-types/src/gates_spec.rs` — Added depends field on GatesSpec
  - `Cargo.toml` — Added `which = "7"` to workspace dependencies
  - `crates/assay-core/Cargo.toml` — Added `which` dependency
  - `crates/assay-core/src/spec/mod.rs` — Added `pub mod validate;`
  - `crates/assay-core/src/gate/mod.rs` — Updated struct literals with depends field
  - `crates/assay-mcp/src/server.rs` — Updated struct literals with depends field
  - `crates/assay-types/tests/schema_roundtrip.rs` — Updated struct literals with depends field
  - 3 schema snapshot files — Updated for new depends field in Spec/GatesSpec/Workflow schemas

## What Was Done

### Task 1: Validation Types in assay-types

Created `validation.rs` with four types registered in the schema registry:
- `Severity` (Error/Warning/Info) — `rename_all = "lowercase"` for JSON serialization
- `Diagnostic` — severity + location + message
- `ValidationResult` — spec slug, valid bool, diagnostics vec, summary
- `DiagnosticSummary` — error/warning/info counts

Added `depends: Vec<String>` field to both `Spec` and `GatesSpec` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. Placed after `gate` field, before `criteria`. Compatible with `deny_unknown_fields` — existing TOML files deserialize correctly (empty vec default).

Updated 34 struct literal constructions across 6 files and 3 schema snapshots.

### Task 2: Core Validation Logic

Created `spec::validate` module with:
- `spec_errors_to_diagnostics` — bridges existing SpecError to Diagnostic (all Error severity)
- `validate_agent_prompts` — warns when AgentReport criteria lack a prompt
- `validate_commands` — opt-in PATH validation using `which::which()` via `extract_binary()`
- `detect_cycles` — DFS three-color marking, reports full cycle path, warns on unknown deps
- `validate_spec` — main entry point, reuses existing validate/validate_gates_spec
- `validate_spec_with_dependencies` — adds cross-spec cycle detection by scanning specs_dir

14 unit tests covering all functions including edge cases (whitespace-only prompts, three-node cycles, unknown dependencies).

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Command-not-found is Warning (not Error) | Binary may exist in execution env but not validation env |
| Whitespace-only prompt treated same as missing | Empty/whitespace prompt provides no agent guidance |
| Cycle detection only runs when spec has dependencies | Avoids unnecessary full-directory scan for simple specs |
| `which` v7 chosen | Latest major version, cross-platform |

## Deviations from Plan

None — plan executed exactly as written.

## Next Phase Readiness

Plan 02 can proceed immediately. The `validate_spec_with_dependencies` function has the exact signature expected by Plan 02 Task 1: `pub fn validate_spec_with_dependencies(entry: &SpecEntry, check_commands: bool, specs_dir: &Path) -> ValidationResult`.
