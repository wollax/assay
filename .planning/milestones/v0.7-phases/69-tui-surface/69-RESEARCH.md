# Phase 69: TUI Surface - Research

**Researched:** 2026-04-12
**Domain:** Ratatui TUI state machine — gate wizard screen in assay-tui
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Entry points & navigation**
- Dashboard keybinding `g` for new gate wizard — consistent with single-letter bindings (`n`/`s`/`m`/`a`/`t`)
- Slash command `/gate-wizard` accessible from any screen for new gate creation
- Edit mode triggered from ChunkDetail screen via `e` keybinding — natural "view then edit" flow
- Slash command `/gate-edit <name>` for direct edit from any screen
- New `Screen::GateWizard(GateWizardState)` variant — dedicated screen, not reusing `Screen::Wizard`

**Form layout & step flow**
- Linear step-by-step flow matching CLI wizard order: name → description → extends → includes → criteria → preconditions → confirm
- Optional steps (preconditions) skippable with y/N prompt — keeps happy path short
- Final step shows full summary of all entered values before write confirmation
- Edit mode pre-fills input buffers with current values — user can backspace to modify or Enter to keep

**Selection UX for extends/includes**
- Extends (single-select parent gate): scrollable ratatui List with highlight, `(none)` as first option, arrow keys navigate, Enter selects — reuses existing List + ListState pattern
- Includes (multi-select criteria libraries): toggle list with checkmarks, Space toggles, Enter confirms, arrow keys navigate — new pattern using ListState + HashSet<usize> for selected indices
- Library entries show slug + criterion count (e.g., "security-checks (3)")
- Empty lists: auto-skip step with brief message ("No existing gates found — skipping extends"), press any key to continue

**Criteria entry pattern**
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

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| WIZT-01 | User can create and edit gate definitions via TUI wizard screen | Full state machine design, integration with `apply_gate_wizard`, edit mode pre-fill via `load_spec_entry_with_diagnostics` |
| WIZT-02 | TUI wizard delegates all validation to core (no surface-specific logic) | `apply_gate_wizard` owns all slug validation; TUI only constructs `GateWizardInput` and delegates |
</phase_requirements>

---

## Summary

Phase 69 adds a new TUI screen (`Screen::GateWizard`) for creating and editing gate definitions. The implementation follows the established `WizardState`/`WizardAction` pattern from `crates/assay-tui/src/wizard.rs` exactly — a step-counter-based state machine, field buffers per step, an action enum, and matching draw/event-handler free functions.

The core contract is simple: the TUI constructs a `GateWizardInput` from its collected form state, calls `apply_gate_wizard(&input, &assay_dir, &specs_dir)`, and surfaces any returned `AssayError` inline. No validation logic lives in TUI code — all slug checking, collision detection, and file I/O happen exclusively in `assay-core::wizard::gate`. This is already proven by the CLI and MCP surfaces both using the same call site.

Two new UX patterns distinguish this wizard from the existing milestone wizard: (1) a single-select list for the `extends` parent gate using ratatui's existing `ListState`, and (2) a multi-select toggle list for `includes` criteria libraries using `ListState + HashSet<usize>`. Both patterns have direct precedents in the codebase (trace viewer uses `ListState`; toggle is a straightforward extension with space-key toggle). The slash command system needs two new variants (`GateWizard` and `GateEdit(String)`) with the argument-parsing complication that `gate-edit` takes a name parameter.

**Primary recommendation:** Create `crates/assay-tui/src/gate_wizard.rs` as a new module following `wizard.rs` exactly. Add `Screen::GateWizard(GateWizardState)` to `app.rs`, wire `g` on Dashboard and `e` on ChunkDetail, add slash command variants, and delegate everything to `apply_gate_wizard`.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 | TUI rendering — `List`, `ListState`, `Paragraph`, `Block`, `Layout` | Already in workspace; all existing TUI screens use it |
| crossterm | 0.28 | Key event types (`KeyCode`, `KeyEvent`, `KeyModifiers`) | Already in workspace; all event handlers use it |
| assay_core::wizard::apply_gate_wizard | current | Gate write entry point | The only permitted write path per Phase 67 contract |
| assay_core::spec::compose::scan_libraries | current | Populate includes list | Returns `Vec<CriteriaLibrary>` sorted by name |
| assay_core::spec::scan | current | Populate extends list | Returns `ScanResult` with `Vec<SpecEntry>` |
| assay_core::spec::load_spec_entry_with_diagnostics | current | Load gate for edit mode pre-fill | Used in existing ChunkDetail navigation |
| assay_types::{GateWizardInput, CriterionInput} | current | Input types to construct before calling core | Already defined, no changes needed |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::collections::HashSet | stdlib | Track selected indices in multi-select includes list | Toggle list for criteria libraries |
| assay_core::spec::compose::validate_slug | current | Inline slug preview hint (display only — not validation) | Show "→ slug: ..." hint below name field, matching milestone wizard pattern |

### Installation
No new dependencies needed. All libraries are already in the workspace.

---

## Architecture Patterns

### Recommended Project Structure
```
crates/assay-tui/src/
├── app.rs             # Add Screen::GateWizard(GateWizardState) variant,
│                      # wire handle_gate_wizard_event + draw_gate_wizard,
│                      # add 'g' Dashboard key, 'e' ChunkDetail key
├── gate_wizard.rs     # NEW: GateWizardState, GateWizardAction,
│                      # handle_gate_wizard_event(), draw_gate_wizard()
├── slash.rs           # Add SlashCmd::GateWizard, SlashCmd::GateEdit(String)
└── lib.rs             # Add: mod gate_wizard;
```

### Pattern 1: GateWizardState Layout
**What:** Step-indexed state machine carrying all form data, with sub-states for list selection and multi-select.
**When to use:** Direct analogue to `WizardState` in `wizard.rs`.

```rust
// Source: crates/assay-tui/src/wizard.rs (existing pattern)

pub struct GateWizardState {
    pub step: usize,
    // Text field buffers indexed by step. Steps with list selection
    // don't use these; steps with text input use index 0 of the inner Vec.
    pub(crate) fields: Vec<Vec<String>>,
    pub(crate) cursor: usize,

    // ── Composability selection state ────────────────────────────────
    // Available gates loaded at wizard open time (for extends single-select).
    pub(crate) available_gates: Vec<String>,            // slugs only
    pub(crate) extends_list_state: ListState,

    // Available libraries loaded at wizard open time (for includes multi-select).
    pub(crate) available_libs: Vec<assay_types::CriteriaLibrary>,
    pub(crate) includes_list_state: ListState,
    pub(crate) selected_includes: std::collections::HashSet<usize>,

    // ── Criteria sub-state ───────────────────────────────────────────
    pub(crate) criteria: Vec<assay_types::CriterionInput>,
    pub(crate) criteria_sub_step: CriteriaSubStep, // Name | Desc | Cmd

    // ── Edit mode ────────────────────────────────────────────────────
    // None = new gate creation; Some(slug) = edit existing gate
    pub(crate) edit_slug: Option<String>,

    // ── Error display ────────────────────────────────────────────────
    pub error: Option<String>,
}

#[derive(Default)]
pub(crate) enum CriteriaSubStep {
    #[default]
    Name,
    Description,
    Cmd,
    AddAnother,  // "Add another? (y/N)" prompt
    EditExisting { idx: usize }, // edit mode: reviewing existing criterion idx
}
```

### Pattern 2: GateWizardAction Enum
**What:** Tri-variant action enum matching `WizardAction` — Continue, Submit, Cancel.
**When to use:** Returned by `handle_gate_wizard_event`, consumed by `app.rs` dispatch.

```rust
// Source: crates/assay-tui/src/wizard.rs (existing pattern)

pub enum GateWizardAction {
    Continue,
    Submit(assay_types::GateWizardInput),
    Cancel,
}
```

### Pattern 3: Free-Function Event Handler
**What:** `handle_gate_wizard_event(state: &mut GateWizardState, event: KeyEvent) -> GateWizardAction`
**When to use:** Matches `handle_wizard_event` and `handle_mcp_panel_event` patterns — no methods on state, borrow-split friendly.

The step sequence for new mode:
- Step 0: Name (text input, required)
- Step 1: Description (text input, optional)
- Step 2: Extends selection (ListState list; auto-skips with message if no gates exist)
- Step 3: Includes selection (toggle list; auto-skips if no libraries exist)
- Step 4: Criteria loop (name → description → cmd → "Add another? y/N" repeat)
- Step 5: Preconditions (y/N prompt — if yes, sequential requires/commands inputs)
- Step 6: Confirm summary (shows all collected data, Enter to write, Esc to cancel)

Edit mode: same steps, all fields pre-filled from loaded `GatesSpec`.

### Pattern 4: App.rs Dispatch — Extracted Method
**What:** Complex screen event handling extracted as `handle_gate_wizard_event` method on `App`, matching `handle_mcp_panel_event` / `handle_trace_viewer_event` pattern.
**When to use:** Avoids borrow-splitting issues in the main `handle_event` match when fields from `self` (project_root, etc.) are needed alongside mutable screen state.

```rust
// Source: crates/assay-tui/src/app.rs lines 428-558 (handle_mcp_panel_event pattern)

impl App {
    fn handle_gate_wizard_event(&mut self, key: KeyEvent) -> bool {
        // 1. Extract action from state machine (borrow-split-safe)
        // 2. On Submit(input): call apply_gate_wizard, surface error inline
        // 3. On Cancel: return to previous screen
        // 4. On Continue: no-op (state already mutated)
        false
    }
}
```

### Pattern 5: Draw Function Signature
**What:** `draw_gate_wizard(frame: &mut Frame, area: Rect, state: &GateWizardState)`
**When to use:** Matches `draw_wizard` and all other draw functions — individual fields passed, not `&self`, enabling flexible borrowing.

For list steps, use `frame.render_stateful_widget(list, area, &mut state.extends_list_state)`. This requires `&mut` on the list state, which the existing trace viewer draw function handles by taking `list_state: &mut ListState`.

### Pattern 6: Slash Command Argument Parsing
**What:** `gate-edit` takes a name argument — the existing COMMANDS table only supports exact-match commands with no arguments.
**When to use:** New pattern needed for `SlashCmd::GateEdit(String)`.

```rust
// Source: crates/assay-tui/src/slash.rs (existing parse_slash_cmd)

// Extend parse_slash_cmd to handle "gate-edit <slug>":
pub fn parse_slash_cmd(input: &str) -> Option<SlashCmd> {
    let trimmed = input.trim().strip_prefix('/').unwrap_or(input.trim()).trim();

    // Check parameterized commands first
    if let Some(rest) = trimmed.strip_prefix("gate-edit") {
        let arg = rest.trim().to_string();
        return Some(SlashCmd::GateEdit(arg));
    }

    // Fall through to existing COMMANDS table lookup
    let lower = trimmed.to_lowercase();
    COMMANDS.iter().find(|(name, _)| *name == lower).map(|(_, cmd)| cmd.clone())
}
```

For tab completion, `gate-edit` should complete to `gate-edit ` (trailing space) when the user types `gate-e`.

### Pattern 7: Edit Mode Pre-Fill
**What:** Load existing `GatesSpec` to populate `GateWizardState` field buffers before opening.
**When to use:** Both `e` from ChunkDetail and `/gate-edit <slug>` slash command.

```rust
// Source: crates/assay-tui/src/app.rs line 1129 (load_spec_entry_with_diagnostics usage)

// In App::handle_event for ChunkDetail 'e' key:
if let Some(ref root) = self.project_root {
    let specs_dir = root.join(".assay").join("specs");
    let assay_dir = root.join(".assay");
    if let Ok(SpecEntry::Directory { gates, slug, .. }) =
        load_spec_entry_with_diagnostics(&chunk_slug, &specs_dir)
    {
        let available_gates = collect_gate_slugs(&specs_dir);
        let available_libs = scan_libraries(&assay_dir).unwrap_or_default();
        let state = GateWizardState::from_existing(gates, slug, available_gates, available_libs);
        self.screen = Screen::GateWizard(state);
    }
}
```

`GateWizardState::from_existing()` constructor pre-fills all field buffers from the `GatesSpec`, sets `edit_slug = Some(slug)`, and initializes the selection states to match the current `extends`/`include` values.

### Pattern 8: Multi-Select Toggle List Rendering
**What:** Render includes list with checkmarks using `ListState` + `HashSet<usize>`.
**When to use:** Step 3 of the wizard.

```rust
// Source: ratatui documentation pattern; ListState used in trace_viewer.rs

let items: Vec<ListItem> = state.available_libs.iter().enumerate().map(|(i, lib)| {
    let check = if state.selected_includes.contains(&i) { "☑" } else { "☐" };
    let label = format!("{} {} ({})", check, lib.name, lib.criteria.len());
    ListItem::new(label)
}).collect();

let list = List::new(items)
    .highlight_style(Style::default().bold().reversed())
    .block(Block::default().borders(Borders::ALL).title(" Include Libraries "));

frame.render_stateful_widget(list, area, &mut state.includes_list_state);
```

### Anti-Patterns to Avoid
- **Validation in TUI:** Never call `compose::validate_slug` as a validation gate — only use it for the slug-preview hint display. All real validation happens in `apply_gate_wizard`.
- **Borrow fights in handle_event match:** Extract complex screen handling to `handle_gate_wizard_event(&mut self, key)` to avoid the need to clone screen data. See `handle_mcp_panel_event` at app.rs:428.
- **Changing app.rs `draw()` signature:** Draw function already pattern-matches on all Screen variants; just add the `Screen::GateWizard(state)` arm.
- **Modifying assay-core:** Phase 67 explicitly states "Phase 69 (TUI) will consume `apply_gate_wizard()` with zero changes." Any decision that requires changes to core is wrong.
- **Blocking on empty lists:** Auto-skip steps with a brief message when `available_gates` or `available_libs` is empty — don't leave the user stuck on an empty list.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Gate TOML write | Custom `fs::write` for `gates.toml` | `apply_gate_wizard` | Atomic write via tempfile, slug validation, AlreadyExists check all included |
| Slug validation | TUI-side regex/check | Display hint via `compose::validate_slug` return; submit through `apply_gate_wizard` | Core owns validation contract; TUI must not duplicate |
| Gate list enumeration | `fs::read_dir` in TUI | `spec::scan(&specs_dir)` | Returns `ScanResult` with parsed slugs; handles errors gracefully |
| Library list | `fs::read_dir` in TUI | `compose::scan_libraries(&assay_dir)` | Returns `Vec<CriteriaLibrary>` sorted by name; handles missing dir as empty |
| Existing gate loading | Custom TOML parse in TUI | `load_spec_entry_with_diagnostics(&slug, &specs_dir)` | Already used in ChunkDetail; handles Legacy vs Directory formats |

**Key insight:** The TUI's entire job is building a `GateWizardInput` struct. Everything else — validation, TOML write, path construction — is core's responsibility.

---

## Common Pitfalls

### Pitfall 1: render_stateful_widget requires &mut ListState
**What goes wrong:** `frame.render_stateful_widget` requires a `&mut ListState`, but the `draw_gate_wizard` function receives `state: &GateWizardState` (immutable).
**Why it happens:** Ratatui's stateful widget API requires mutable state to update scroll position. The `draw()` call in `app.rs` already uses `&mut self.screen` for this purpose in the trace viewer.
**How to avoid:** Take `state: &mut GateWizardState` in `draw_gate_wizard`, matching how `draw_trace_viewer` handles list state mutation. The `app.rs` `draw()` method already re-borrows `&mut self.screen` for mutable draw operations.
**Warning signs:** Compiler error "cannot borrow `state.extends_list_state` as mutable because `state` is behind a `&` reference".

### Pitfall 2: Borrow splitting in handle_event match
**What goes wrong:** Attempting to access `self.project_root` while `self.screen` is mutably borrowed in the main `handle_event` match arm.
**Why it happens:** Rust's borrow checker cannot split struct fields across a match arm that holds a mutable borrow on `self.screen`.
**How to avoid:** Follow the `handle_mcp_panel_event` pattern: extract into a method `fn handle_gate_wizard_event(&mut self, key: KeyEvent) -> bool` that takes `&mut self` and resolves all borrows internally. In `handle_event`, call `self.handle_gate_wizard_event(key)`.
**Warning signs:** "cannot borrow `*self` as immutable because it is also borrowed as mutable".

### Pitfall 3: Auto-skip for empty lists must set step correctly
**What goes wrong:** When `available_gates` is empty and the extends step is auto-skipped, the step counter must advance past the step correctly, or the wizard loops on the auto-skip message.
**Why it happens:** The auto-skip message needs a "press any key" continuation, not an Enter-to-advance, or the step must unconditionally advance.
**How to avoid:** When `available_gates` is empty at the extends step, display a 1-frame informational message and advance the step immediately on the next keypress (any key). Set a `auto_skip_msg: Option<String>` sub-field on the step, displayed in the hint area. The first keypress after an auto-skip message advances regardless of key.

### Pitfall 4: Edit mode overwrite flag
**What goes wrong:** Calling `apply_gate_wizard` with `overwrite: false` in edit mode results in `AlreadyExists` error.
**Why it happens:** Default `GateWizardInput` has `overwrite: false`. Edit mode must set `overwrite: true`.
**How to avoid:** In `assemble_gate_input(state: &GateWizardState) -> GateWizardInput`, set `overwrite: state.edit_slug.is_some()`.

### Pitfall 5: Slash command `gate-edit` without argument
**What goes wrong:** User types `/gate-edit` with no following slug — `SlashCmd::GateEdit("")` is dispatched and core returns an error or loads nothing.
**Why it happens:** Argument parsing accepts empty string.
**How to avoid:** In `execute_slash_cmd`, if `GateEdit(slug)` has an empty slug, return an early error string "Usage: /gate-edit <slug>" without calling core.

### Pitfall 6: Pre-filling the includes toggle list for edit mode
**What goes wrong:** The `selected_includes` HashSet must be initialized from the loaded `GatesSpec.include` list by matching slugs against `available_libs` indices.
**Why it happens:** The HashSet tracks indices into `available_libs`, not slug strings.
**How to avoid:** In `GateWizardState::from_existing`, iterate `available_libs` with `enumerate()` and add index `i` to `selected_includes` if `available_libs[i].name` is in `gates.include`.

---

## Code Examples

Verified patterns from existing codebase:

### Wizard step advance (from wizard.rs)
```rust
// Source: crates/assay-tui/src/wizard.rs:247-250
state.step = 1;
state.fields.push(vec![String::new()]);
state.cursor = 0;
state.error = None;
```

### ListState navigation (from app.rs Dashboard)
```rust
// Source: crates/assay-tui/src/app.rs:872-886
KeyCode::Down => {
    let i = self.list_state.selected()
        .map(|s| (s + 1).min(self.milestones.len().saturating_sub(1)))
        .unwrap_or(0);
    self.list_state.select(Some(i));
}
```

### Stateful list render (from trace_viewer pattern — render_stateful_widget)
```rust
// ratatui 0.30 pattern — list state must be &mut
frame.render_stateful_widget(list, area, &mut state.trace_list_state);
```

### Error display inline (from wizard.rs draw)
```rust
// Source: crates/assay-tui/src/wizard.rs:405-409
let hint_text = if let Some(ref err) = state.error {
    Text::from(Line::from(Span::styled(
        err.as_str(),
        Style::default().fg(Color::Red),
    )))
} else { /* key hint */ };
```

### apply_gate_wizard call site (from wizard/gate.rs)
```rust
// Source: crates/assay-core/src/wizard/gate.rs:25-29
pub fn apply_gate_wizard(
    input: &GateWizardInput,
    _assay_dir: &Path,
    specs_dir: &Path,
) -> Result<GateWizardOutput>
```

### assemble_gate_input pattern (equivalent to assemble_inputs in wizard.rs)
```rust
// Pattern: mirror assemble_inputs() in wizard.rs lines 97-157
fn assemble_gate_input(state: &GateWizardState) -> GateWizardInput {
    let selected_libs: Vec<String> = state.available_libs.iter().enumerate()
        .filter(|(i, _)| state.selected_includes.contains(i))
        .map(|(_, lib)| lib.name.clone())
        .collect();

    let extends = {
        let sel = state.extends_list_state.selected().unwrap_or(0);
        // Index 0 = "(none)"; index N = available_gates[N-1]
        if sel == 0 { None } else { state.available_gates.get(sel - 1).cloned() }
    };

    GateWizardInput {
        slug: state.fields[0].last().cloned().unwrap_or_default(),
        description: {
            let d = state.fields[1].last().cloned().unwrap_or_default();
            if d.is_empty() { None } else { Some(d) }
        },
        extends,
        include: selected_libs,
        criteria: state.criteria.clone(),
        preconditions: state.collected_preconditions.clone(),
        overwrite: state.edit_slug.is_some(),
    }
}
```

---

## Integration Points Summary

### Files to Modify
1. `crates/assay-tui/src/lib.rs` — add `pub mod gate_wizard;`
2. `crates/assay-tui/src/app.rs` — add `Screen::GateWizard(GateWizardState)` variant; add `use crate::gate_wizard::{...}` import; add `g` key in Dashboard arm; add `e` key in ChunkDetail arm; add `draw_gate_wizard` call in `draw()`; add `handle_gate_wizard_event` method
3. `crates/assay-tui/src/slash.rs` — add `SlashCmd::GateWizard` and `SlashCmd::GateEdit(String)` variants; update `parse_slash_cmd` for parameterized gate-edit; update `execute_slash_cmd`; update `tab_complete`

### File to Create
- `crates/assay-tui/src/gate_wizard.rs` — contains `GateWizardState`, `GateWizardAction`, `CriteriaSubStep`, `handle_gate_wizard_event()`, `draw_gate_wizard()`, `assemble_gate_input()`, `GateWizardState::new()`, `GateWizardState::from_existing()`

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + integration test files |
| Config file | `crates/assay-tui/Cargo.toml` (no separate test config — Rust standard) |
| Quick run command | `cargo test -p assay-tui --test gate_wizard` |
| Full suite command | `cargo test -p assay-tui` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WIZT-01 | Step through new gate wizard, confirm writes gates.toml | integration | `cargo test -p assay-tui --test gate_wizard_round_trip` | Wave 0 |
| WIZT-01 | Edit mode pre-fills fields from existing GatesSpec | integration | `cargo test -p assay-tui --test gate_wizard_round_trip -- test_edit_mode_prefill` | Wave 0 |
| WIZT-01 | Cancel at any step returns to previous screen | unit | `cargo test -p assay-tui --test gate_wizard_round_trip -- test_cancel_returns_to_dashboard` | Wave 0 |
| WIZT-01 | Dashboard 'g' key opens GateWizard screen | integration | `cargo test -p assay-tui --test gate_wizard_app -- test_g_key_opens_gate_wizard` | Wave 0 |
| WIZT-01 | `/gate-wizard` slash command opens GateWizard screen | integration | `cargo test -p assay-tui --test gate_wizard_app -- test_slash_gate_wizard` | Wave 0 |
| WIZT-01 | `/gate-edit <slug>` opens GateWizard in edit mode | integration | `cargo test -p assay-tui --test gate_wizard_app -- test_slash_gate_edit` | Wave 0 |
| WIZT-02 | Invalid slug is rejected by core (not TUI) with inline error | integration | `cargo test -p assay-tui --test gate_wizard_round_trip -- test_invalid_slug_shows_error` | Wave 0 |
| WIZT-02 | No validation logic in gate_wizard.rs (structural check) | unit | `cargo test -p assay-tui --test gate_wizard_round_trip -- test_no_tui_side_validation` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p assay-tui`
- **Per wave merge:** `cargo test -p assay-tui && cargo test -p assay-core`
- **Phase gate:** `just ready` (fmt-check + lint + test + deny) before `/kata:verify-work`

### Wave 0 Gaps
- [ ] `crates/assay-tui/tests/gate_wizard_round_trip.rs` — unit/integration tests for the state machine (step advance, assemble_gate_input, cancel, backspace)
- [ ] `crates/assay-tui/tests/gate_wizard_app.rs` — App-level integration tests for keybindings and slash commands

*(Framework is already installed — only test files are missing)*

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `CriterionInput` in assay-core | `CriterionInput` re-homed to assay-types | Phase 67-01 | Import from `assay_types::CriterionInput` (re-exported via `assay_core::wizard::CriterionInput`) |
| Wizard writes directly | `apply_gate_wizard` (surface-agnostic core) | Phase 67 | TUI constructs input, delegates all I/O to core |
| No `extends`/`include` fields | `GatesSpec.extends` + `GatesSpec.include` | Phase 64-65 | Must expose these in the wizard form |
| No preconditions | `SpecPreconditions` with `requires`/`commands` | Phase 66 | Optional step in wizard with y/N prompt |

---

## Open Questions

1. **Preconditions UI depth**
   - What we know: `SpecPreconditions` has `requires: Vec<String>` and `commands: Vec<String>` — both are free-text lists
   - What's unclear: CONTEXT.md says "optional steps skippable with y/N prompt" — should requires and commands each be a separate sequential input loop, or one combined step?
   - Recommendation: Treat them as two sequential sub-loops under the preconditions step (requires first, then commands), each terminated by blank Enter, matching the criteria entry pattern. This keeps the wizard consistent.

2. **Criteria in edit mode: deletion UX**
   - What we know: CONTEXT.md says "walk through each existing criterion pre-filled, with `d` to delete, then 'Add another?' for new ones"
   - What's unclear: After pressing `d` to delete a criterion mid-walk, should the walk continue with the next criterion or jump to "Add another?"
   - Recommendation: Delete removes from `state.criteria` immediately and advances to the next criterion (or to "Add another?" if that was the last). This keeps the flow linear.

3. **Confirm step rendering**
   - What we know: CONTEXT.md says "final step shows full summary of all entered values before write confirmation"
   - What's unclear: How to display `criteria` (could be many) and `preconditions` in the limited confirm screen space
   - Recommendation: Show a compact summary: name, description (truncated), extends, includes (comma-joined), criteria count (e.g., "3 criteria"), and preconditions summary. Full criteria details are implicit from the preceding steps.

---

## Sources

### Primary (HIGH confidence)
- Codebase: `crates/assay-tui/src/wizard.rs` — full WizardState pattern studied
- Codebase: `crates/assay-tui/src/app.rs` — Screen enum, all event dispatch patterns, handle_mcp_panel_event, draw dispatch
- Codebase: `crates/assay-tui/src/slash.rs` — SlashCmd, parse, tab_complete, execute patterns
- Codebase: `crates/assay-core/src/wizard/gate.rs` — `apply_gate_wizard` signature and contract
- Codebase: `crates/assay-types/src/wizard_input.rs` — `GateWizardInput`, `CriterionInput` struct definitions
- Codebase: `crates/assay-core/src/spec/compose.rs` — `scan_libraries`, `validate_slug` signatures
- Codebase: `crates/assay-tui/tests/wizard_round_trip.rs` — test patterns to follow
- Codebase: `crates/assay-tui/tests/app_wizard.rs` — App-level test patterns

### Secondary (MEDIUM confidence)
- ratatui 0.30 `render_stateful_widget` + `ListState` pattern confirmed via existing trace_viewer.rs and dashboard usage in app.rs

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries verified in Cargo.toml and existing code
- Architecture: HIGH — all patterns directly copied from existing codebase; no new frameworks
- Pitfalls: HIGH — borrow-split, overwrite flag, auto-skip all verified from existing code patterns
- Validation architecture: HIGH — test framework confirmed; test file gaps correctly identified

**Research date:** 2026-04-12
**Valid until:** 2026-05-12 (stable codebase, no external dependencies changing)
