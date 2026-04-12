---
phase: 67-wizard-core-cli-surface
plan: "03"
subsystem: cli
tags: [wizard, assay-cli, gate, dialoguer, tdd]

requires:
  - phase: 67-02
    provides: "apply_gate_wizard, apply_criteria_wizard from assay_core::wizard"
  - phase: 67-01
    provides: "CriterionInput, GateWizardInput, GateWizardOutput from assay-types"

provides:
  - "GateCommand::Wizard { edit: Option<String> } variant in assay-cli"
  - "handle_wizard: TTY-guarded CLI entry point delegating to apply_gate_wizard"
  - "wizard_helpers module: prompt_slug, prompt_criteria_loop, select_from_list, multi_select_from_list (pub(crate))"
  - "load_gate_for_edit: private helper used by handle_wizard + tests"

affects:
  - "67-04-PLAN: CLI criteria wizard — import prompt_criteria_loop, prompt_slug from crate::commands::wizard_helpers"
  - "68-mcp-tools: wizard_helpers is CLI-only; MCP builds its own input payload"
  - "69-tui: wizard_helpers is CLI-only; TUI builds its own interactive flow"

tech-stack:
  added: []
  patterns:
    - "TTY guard first: std::io::stdin().is_terminal() checked before any I/O"
    - "Thin CLI surface: all persistence/validation delegated to assay_core::wizard::apply_gate_wizard"
    - "validate_with wrapping compose::validate_slug: inline slug rejection in dialoguer prompt"
    - "wizard_helpers pub(crate): shared between gate.rs (Plan 03) and Plan 04 criteria command"
    - "load_gate_for_edit: private fn called by handle_wizard and test module for isolation"

key-files:
  created:
    - "crates/assay-cli/src/commands/wizard_helpers.rs"
  modified:
    - "crates/assay-cli/src/commands/gate.rs"
    - "crates/assay-cli/src/commands/mod.rs"

key-decisions:
  - "handle_wizard_edit_non_tty test added (not in plan): TTY guard fires before spec loading in edit mode, so the test is zero-cost and verifies the guard path covers both create and edit modes"
  - "load_gate_for_edit is a private fn (not pub(crate)): only tests in the same module call it directly; Plan 04 does not need this helper"
  - "Legacy spec TOML in test requires cmd field: assay Spec validation requires at least one criterion with cmd or path; test updated to include cmd = 'echo ok'"

metrics:
  duration: 4min
  completed: "2026-04-12"
  tasks: 1
  files: 3
---

# Phase 67 Plan 03: Gate Wizard CLI Surface Summary

**GateCommand::Wizard with --edit mode implemented; wizard_helpers module created for Plan 04 reuse; 4 unit tests green; 2433 workspace tests pass**

## Performance

- **Duration:** 4 min
- **Started:** 2026-04-12T16:09:22Z
- **Completed:** 2026-04-12T16:13:53Z
- **Tasks:** 1
- **Files modified/created:** 3

## Accomplishments

- Added `GateCommand::Wizard { edit: Option<String> }` variant to `gate.rs`
- Implemented `handle_wizard`: TTY guard returns `Ok(1)` before any I/O, then drives a linear dialoguer flow collecting slug, description, extends (Select), include (MultiSelect), criteria (loop), and optional preconditions (MultiSelect + loop); delegates to `assay_core::wizard::apply_gate_wizard`
- Added `load_gate_for_edit`: calls `load_spec_entry_with_diagnostics` (fuzzy suggestion in error), rejects `SpecEntry::Legacy` with a clear message
- Added `prompt_preconditions`: MultiSelect of spec slugs + add-another command loop
- Created `wizard_helpers.rs` with four `pub(crate)` helpers for Plan 04 reuse
- Registered `wizard_helpers` in `commands/mod.rs`
- 4 unit tests: `handle_wizard_non_tty`, `handle_wizard_edit_non_tty`, `handle_wizard_edit_not_found`, `load_gate_for_edit_rejects_legacy`
- `just test` → 2433/2433 passed; clippy clean

## GateCommand::Wizard Clap Shape

```rust
// crates/assay-cli/src/commands/gate.rs
/// Interactively create or edit a gate definition
Wizard {
    /// Edit an existing gate by slug rather than creating a new one.
    #[arg(long)]
    edit: Option<String>,
},
```

Dispatch: `GateCommand::Wizard { edit } => handle_wizard(edit)`

## wizard_helpers Public Surface (for Plan 04)

```rust
// crates/assay-cli/src/commands/wizard_helpers.rs
pub(crate) fn prompt_slug(prompt: &str, initial: Option<&str>) -> Result<String>
pub(crate) fn prompt_criteria_loop(existing: &[CriterionInput]) -> Result<Vec<CriterionInput>>
pub(crate) fn select_from_list(prompt: &str, items: &[String], default_idx: usize) -> Result<usize>
pub(crate) fn multi_select_from_list(prompt: &str, items: &[String], preselected: &[usize]) -> Result<Vec<usize>>
```

Plan 04 imports: `use crate::commands::wizard_helpers::{prompt_criteria_loop, prompt_slug};`

## GatesSpec / Criterion Field Names (for Phase 68/69 Reference)

From `assay-types/src/gates_spec.rs`:
- `GatesSpec.description: String` (empty string = no description; NOT `Option<String>`)
- `GatesSpec.extends: Option<String>`
- `GatesSpec.include: Vec<String>`
- `GatesSpec.preconditions: Option<SpecPreconditions>`
- `GatesSpec.criteria: Vec<GateCriterion>` (= `Vec<Criterion>`)

From `assay-types/src/criterion.rs` (Criterion / GateCriterion):
- `Criterion.name: String`
- `Criterion.description: String`
- `Criterion.cmd: Option<String>` (field is named `cmd`, not `command`)

`GateWizardInput.description: Option<String>` (description field IS optional in the input type even though `GatesSpec.description` is a plain `String`).

## fuzzy-suggestion Phrasing

`SpecNotFoundDiagnostic` Display: `"Did you mean '{suggestion}'?"` (capital D, single-quoted suggestion). Test asserts `msg.contains("Did you mean") || msg.contains("not found")`.

## Task Commits

1. **Task 1: Extract wizard_helpers, implement GateCommand::Wizard** — `a5a9ea0` (feat)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Legacy spec TOML needed cmd field for validation to pass**
- **Found during:** Task 1 (test `load_gate_for_edit_rejects_legacy`)
- **Issue:** Test wrote `criteria = []` which failed assay's own spec validation ("must have at least one criterion") before reaching the Legacy branch check; subsequent fix with `[[criteria]] ... description = "A check"` still failed ("at least one criterion must have a cmd or path field")
- **Fix:** Added `cmd = "echo ok"` to test TOML to satisfy all three spec validation rules
- **Files modified:** `crates/assay-cli/src/commands/gate.rs`
- **Commit:** `a5a9ea0` (inline fix)

**2. [Rule 2 - Missing test] Added `handle_wizard_edit_non_tty` test**
- **Found during:** Task 1 implementation review
- **Issue:** Plan specified 3 tests; adding a 4th for edit-mode non-TTY was trivially low-cost and verified the guard covers both branches
- **Fix:** Added `handle_wizard_edit_non_tty` test
- **Files modified:** `crates/assay-cli/src/commands/gate.rs`
- **Commit:** `a5a9ea0`

## Self-Check

- [x] `crates/assay-cli/src/commands/wizard_helpers.rs` — FOUND
- [x] `crates/assay-cli/src/commands/gate.rs` — FOUND
- [x] `crates/assay-cli/src/commands/mod.rs` — FOUND
- [x] Commit `a5a9ea0` — FOUND

## Self-Check: PASSED
