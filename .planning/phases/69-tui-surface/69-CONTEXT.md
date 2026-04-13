# Phase 69: TUI Surface - Context

**Gathered:** 2026-04-12
**Status:** Ready for planning

<domain>
## Phase Boundary

TUI wizard state machine for creating and editing gate definitions. Deliverables: `GateWizardState`/`GateWizardAction` types, `handle_gate_wizard_event()` event handler, `draw_gate_wizard()` renderer, plus slash commands `/gate-wizard` and `/gate-edit <name>`. All field validation delegates to `assay-core::wizard::apply_gate_wizard()` — no validation logic lives in TUI code.

Out of scope: Criteria library management TUI (list/create), changes to existing screens, changes to `assay-core` or `assay-types`.

</domain>

<decisions>
## Implementation Decisions

### Entry points & navigation
- Dashboard keybinding `g` for new gate wizard — consistent with single-letter bindings (`n`/`s`/`m`/`a`/`t`)
- Slash command `/gate-wizard` accessible from any screen for new gate creation
- Edit mode triggered from ChunkDetail screen via `e` keybinding — natural "view then edit" flow
- Slash command `/gate-edit <name>` for direct edit from any screen
- New `Screen::GateWizard(GateWizardState)` variant — dedicated screen, not reusing `Screen::Wizard`

### Form layout & step flow
- Linear step-by-step flow matching CLI wizard order: name → description → extends → includes → criteria → preconditions → confirm
- Optional steps (preconditions) skippable with y/N prompt — keeps happy path short
- Final step shows full summary of all entered values before write confirmation
- Edit mode pre-fills input buffers with current values — user can backspace to modify or Enter to keep

### Selection UX for extends/includes
- Extends (single-select parent gate): scrollable ratatui List with highlight, `(none)` as first option, arrow keys navigate, Enter selects — reuses existing List + ListState pattern
- Includes (multi-select criteria libraries): toggle list with checkmarks, Space toggles, Enter confirms, arrow keys navigate — new pattern using ListState + HashSet<usize> for selected indices
- Library entries show slug + criterion count (e.g., "security-checks (3)")
- Empty lists: auto-skip step with brief message ("No existing gates found — skipping extends"), press any key to continue

### Criteria entry pattern
- Sequential field loop per criterion: name → description → optional cmd → "Add another? (y/N)"
- Mirrors the CLI wizard's inline loop and the existing milestone wizard's criteria entry pattern
- Edit mode: walk through each existing criterion pre-filled, with `d` to delete, then "Add another?" for new ones after existing criteria are processed
- Minimum criteria count delegated to core validation — Claude decides based on how `apply_gate_wizard` handles zero criteria (a gate with extends/include may have zero own criteria)

### Claude's Discretion
- Linear steps vs single-screen form choice (user said "you decide" — lean toward linear steps to match existing WizardState pattern)
- Exact `GateWizardState` struct layout (step counter, field buffers, sub-state for criteria/selection)
- `GateWizardAction` enum variants
- Whether the gate wizard module is a new file (`gate_wizard.rs`) or extends existing `wizard.rs`
- Internal sub-state tracking for criteria entry (name/desc/cmd sub-steps)
- Cursor position management within text input fields
- Error display for core validation failures (inline red text, matching existing patterns)
- How to load existing gate for edit mode (call `load_spec_entry_with_diagnostics` or similar)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `WizardState` (`crates/assay-tui/src/wizard.rs`): Existing multi-step form state machine for milestone creation — step counter, field buffers, cursor tracking, `WizardAction` enum. Direct pattern to follow.
- `ListState` + `List` widget (ratatui): Used extensively in Dashboard, MilestoneDetail, TraceViewer — reuse for extends single-select and includes multi-select
- `apply_gate_wizard()` (`assay-core/src/wizard/gate.rs:25`): Core entry point for gate creation/edit — TUI builds `GateWizardInput`, calls this, done
- `apply_criteria_wizard()` (`assay-core/src/wizard/criteria.rs:26`): Core entry point for library creation
- `compose::scan_libraries()` (`assay-core/src/spec/compose.rs:136`): Returns `Vec<CriteriaLibrary>` for populating includes list
- `spec::scan()` (`assay-core/src/spec/mod.rs:683`): Enumerates existing gates for populating extends list
- `load_spec_entry_with_diagnostics()`: Used in existing TUI ChunkDetail to load spec — reuse for edit mode
- `GateWizardInput` / `GateWizardOutput` (`assay-types/src/wizard_input.rs`): Input type the TUI must construct from form state
- `CriterionInput` (`assay-types/src/wizard_input.rs`): Per-criterion input struct (name, description, cmd)
- `SlashState` / `SlashCmd` (`crates/assay-tui/src/slash.rs`): Slash command infrastructure — add `GateWizard` and `GateEdit(String)` variants
- `compose::validate_slug` (`assay-core/src/spec/compose.rs:21`): For inline slug validation display (not validation logic — that's core's job on submit)

### Established Patterns
- `Screen` enum with embedded state (`Screen::Wizard(WizardState)`, `Screen::McpPanel { ... }`)
- Extracted event handlers for complex screens (`handle_mcp_panel_event()`, `handle_trace_viewer_event()`)
- Draw functions take individual fields, not `&self` — enables flexible borrowing
- Single-line text input: char append, Backspace delete, Enter advance
- `assemble_inputs()` pattern: convert form state → core input types on submit
- Style conventions: `Style::default().bold().reversed()` for highlight, `.dim()` for hints, `Color::Red` for errors
- No trait objects — all concrete types and direct function calls

### Integration Points
- `crates/assay-tui/src/app.rs`: Add `Screen::GateWizard(GateWizardState)` variant, dispatch in `handle_event()` and `draw()`
- `crates/assay-tui/src/gate_wizard.rs` (new): `GateWizardState`, `GateWizardAction`, `handle_gate_wizard_event()`, `draw_gate_wizard()`
- `crates/assay-tui/src/slash.rs`: Add `SlashCmd::GateWizard` and `SlashCmd::GateEdit(String)` variants
- `crates/assay-tui/src/lib.rs`: Add `mod gate_wizard;` declaration

</code_context>

<specifics>
## Specific Ideas

- The gate wizard should feel like the existing milestone wizard — same step-based metaphor, same key conventions (Enter to advance, Esc to cancel), same visual styling
- The multi-select toggle list for includes is a new TUI pattern — keep it simple (ListState + HashSet) rather than building a generic multi-select widget
- Edit mode is "full replacement" per Phase 67 decision: load existing GatesSpec, present all fields pre-filled, write complete replacement on confirm
- Phase 67 explicitly states: "Phase 69 (TUI) will consume `apply_gate_wizard()` with zero changes" — if any decision forces changes to core, it's wrong

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 69-tui-surface*
*Context gathered: 2026-04-12*
