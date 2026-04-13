---
phase: 69-tui-surface
plan: "01"
subsystem: assay-tui
tags: [tui, wizard, gate-wizard, slash-commands]
dependency_graph:
  requires: []
  provides:
    - assay_tui::gate_wizard (GateWizardState, GateWizardAction, handle_gate_wizard_event, draw_gate_wizard, assemble_gate_input)
    - assay_tui::slash (SlashCmd::GateWizard, SlashCmd::GateEdit)
  affects:
    - crates/assay-tui/src/lib.rs (pub mod gate_wizard added)
    - crates/assay-tui/src/slash.rs (two new variants + parsing + completion)
tech_stack:
  added: []
  patterns:
    - ListState-based single/multi select in ratatui
    - Sub-step enum dispatch within a single wizard step
    - Auto-skip pattern for empty list steps with transient message
key_files:
  created:
    - crates/assay-tui/src/gate_wizard.rs
  modified:
    - crates/assay-tui/src/lib.rs
    - crates/assay-tui/src/slash.rs
decisions:
  - COMMANDS table lookup has priority over gate-edit prefix check in tab_complete — preserves alphabetical table ordering
  - criteria_edit_idx field reserved for Plan 02 edit-mode walk, suppressed with allow(dead_code)
  - assemble_gate_input collects selected_includes as Vec<String> (lib names) in arbitrary iteration order — ordering not semantically meaningful
metrics:
  duration_seconds: 271
  tasks_completed: 2
  files_changed: 3
  completed_date: "2026-04-13"
---

# Phase 69 Plan 01: Gate Wizard State Machine Summary

Gate wizard TUI module with 7-step state machine, event handler, renderer, and slash command variants — ready for integration in Plan 02.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Create gate_wizard.rs state machine module | f603cae | crates/assay-tui/src/gate_wizard.rs, crates/assay-tui/src/lib.rs |
| 2 | Add slash command variants for gate wizard | 94ed496 | crates/assay-tui/src/slash.rs |

## What Was Built

### gate_wizard.rs (858 lines)

Complete state machine module for the gate authoring wizard:

- `GateWizardState` — all form state for the 7-step wizard, including ratatui `ListState` fields for list steps, `HashSet<usize>` for multi-select includes, `CriterionInput` vec for criteria accumulation, and scratch arrays for in-progress criterion/precondition entry
- `GateWizardState::new()` — creates a fresh wizard; pre-selects extends index 0 ("(none)")
- `GateWizardState::from_existing()` — edit mode constructor; pre-fills all buffers from a `GatesSpec` and pre-selects extends/includes from existing values
- `CriteriaSubStep` enum (Name / Description / Cmd / AddAnother) and `PreconditionSubStep` enum (Ask / Requires / Commands) for sub-step dispatch within steps 4 and 5
- `GateWizardAction` enum (Continue / Submit(GateWizardInput) / Cancel)
- `handle_gate_wizard_event` — dispatches to per-step handlers; Esc at any step → Cancel; auto_skip_msg clears on any keypress and advances
- `assemble_gate_input` — converts form state into `GateWizardInput` with correct overwrite flag
- `draw_gate_wizard` — renders all 7 step variants with header/main/hint layout; uses `render_stateful_widget` for list steps; takes `&mut GateWizardState` for ListState mutability

### slash.rs (additions)

- `SlashCmd::GateWizard` and `SlashCmd::GateEdit(String)` variants added
- `COMMANDS` table extended with `("gate-wizard", SlashCmd::GateWizard)`
- `parse_slash_cmd` — checks `gate-edit` prefix before COMMANDS lookup
- `tab_complete` — COMMANDS table takes priority; `gate-edit` completes after no-table-match (preserves `"g"` → `/gate-check` behavior)
- `execute_slash_cmd` — placeholder strings for both new commands; GateEdit("") returns usage hint

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written.

### Minor Adaptations

**1. [Rule 2 - UX] tab_complete ordering**

The plan specified checking `gate-edit` before COMMANDS, but the existing test asserts `tab_complete("g") == Some("/gate-check")`. Since `"gate-edit"` also starts with `"g"`, the parameterized check had to move after the COMMANDS table lookup to preserve the test. This matches the intent: COMMANDS entries are "exact table" commands and should complete first alphabetically.

**2. [Rule 2 - Clarity] criteria_edit_idx dead_code suppression**

The `criteria_edit_idx` field is specified in the plan for Plan 02's edit-mode criterion walk. Since it's not used in Plan 01, it would trigger a dead_code warning that fails clippy `-D warnings`. Added `#[allow(dead_code)]` on the struct with a comment noting it's used in Plan 02.

## Self-Check: PASSED

- FOUND: crates/assay-tui/src/gate_wizard.rs
- FOUND: crates/assay-tui/src/lib.rs
- FOUND: crates/assay-tui/src/slash.rs
- FOUND commit f603cae (Task 1)
- FOUND commit 94ed496 (Task 2)
