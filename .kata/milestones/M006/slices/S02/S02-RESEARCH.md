# S02: In-TUI Authoring Wizard — Research

**Researched:** 2026-03-20
**Domain:** Ratatui TUI form state machine, crossterm key events, assay-core wizard integration
**Confidence:** HIGH

## Summary

S01 has not yet been merged; the TUI crate still holds the 42-line stub. S02 planning must model the S01 outputs (App struct, Screen enum, project_root, etc.) per the M006 boundary map, then build on top of them. The wizard form itself is a pure Rust state machine — no third-party text input widget is needed, and none is available in the registry anyway (`tui-textarea` is not a workspace dep and is explicitly excluded by the S02 context).

The core integration is straightforward: `WizardState` holds all step data in a `Vec<Vec<String>>` (one inner `Vec<String>` per step; steps with multi-line input such as criteria accumulate entries, single-line steps hold one entry). `handle_wizard_event` is a pure function that mutates `WizardState` in-place and returns a `WizardAction`. On `WizardAction::Submit`, the caller assembles `WizardInputs` from the collected fields and calls `create_from_inputs`. The only risk is the step-routing logic for the variable number of criterion-input steps (one step per chunk for criteria collection); this is O(chunk_count) dynamic, not a fixed step count.

The integration test is the slice's primary proof. It lives in `crates/assay-tui/tests/wizard_round_trip.rs` and does not require a terminal — it instantiates `WizardState` directly, drives `handle_wizard_event` with synthetic `crossterm::event::KeyEvent` values, waits for `Submit(WizardInputs)`, calls `create_from_inputs`, then asserts that the milestone TOML and all chunk `gates.toml` files exist in a `TempDir`. This is the same pattern used by `crates/assay-core/tests/wizard.rs` and requires only `tempfile.workspace = true` added to `assay-tui`'s `[dev-dependencies]`.

## Recommendation

Implement `WizardState` with a fixed step model: step 0 = milestone name, step 1 = description (optional), step 2 = chunk count digit, steps 3..3+N-1 = chunk names (N = chunk_count), steps 3+N..3+2N-1 = criteria per chunk (multi-line, blank Enter to finish). Use `fields: Vec<Vec<String>>` where each entry is allocated dynamically after the user sets chunk count. For cursor positioning, track `cursor: usize` as the character count in the active single-line buffer; use `frame.set_cursor_position((area.x + prompt_width + cursor as u16, row))` in `draw_wizard`. For the popup layout, use `Layout::vertical/horizontal` with `Flex::Center` and fixed `Constraint::Length` dimensions (60 wide × 12 tall is ample). Use `frame.render_widget(Clear, area)` before rendering the popup block so the popup overwrites the dashboard behind it.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Wizard pure logic (file writes) | `assay_core::wizard::create_from_inputs(WizardInputs, assay_dir, specs_dir)` | Tested, atomic, produces milestone TOML + per-chunk gates.toml. S02 collects inputs and calls this; it must not re-implement writing. |
| Slug derivation and preview | `assay_core::wizard::slugify(s: &str) -> String` | Already imported in CLI wizard; derive slug from each name as user types and show as dim hint on the same line. |
| Popup overlay clearing | `ratatui::widgets::Clear` | Renders before the popup block to reset the cell buffer so the popup doesn't bleed over dashboard text. Use pattern: `frame.render_widget(Clear, area); frame.render_widget(block, area)`. |
| Cursor positioning in field | `frame.set_cursor_position((x, y))` on `&mut Frame` | `ratatui-core-0.1.0` provides `set_cursor_position<P: Into<Position>>`. Call this at end of `draw_wizard` to show a blinking cursor at the right offset. |
| Centered popup layout | `Layout::vertical/horizontal` with `Flex::Center` and `Constraint::Length(n)` | `Flex::Center` distributes remaining space symmetrically. Chain `.flex(Flex::Center)` on the Layout builder. |
| Post-Submit dashboard reload | `assay_core::milestone::milestone_scan(assay_dir)` | Called in `handle_event` after receiving `Submit` → runs `create_from_inputs` → reloads milestones list → sets `App.screen = Screen::Dashboard` with new selection. |

## Existing Code and Patterns

- `crates/assay-core/src/wizard.rs` — Public API: `WizardInputs { slug, name, description, chunks }`, `WizardChunkInput { slug, name, criteria: Vec<String> }`, `create_from_inputs(inputs, assay_dir, specs_dir) -> Result<WizardResult>`, `slugify(s) -> String`. The TUI wizard assembles these types from collected fields, calls `create_from_inputs`, and re-enters the dashboard on success.
- `crates/assay-cli/src/commands/plan.rs` — Reference implementation of the wizard flow using dialoguer. Maps directly to S02's step sequence: step 0 = milestone name (dialoguer::Input), step 1 = description confirm + text, step 2 = chunk count via Select (S02 uses digit input instead), steps 3..N = per-chunk name + criteria loop. S02 replicates this logic in a Ratatui state machine.
- `crates/assay-core/tests/wizard.rs` — Integration test pattern to follow: `TempDir::new()`, build `WizardInputs` directly, call `create_from_inputs`, assert with `milestone_load` and `gates_path.exists()`. S02's integration test will be identical in structure, but driven by synthetic key events instead of direct struct construction.
- `crates/assay-tui/src/main.rs` — 42-line stub (S01 not yet executed). S02 works on the S01 output: an `App` struct with `screen: Screen` and `Screen::Wizard(WizardState)` variant. The S01 summary (not yet written) will be the canonical reference; until it exists, design against the M006 boundary map.
- `crates/assay-core/src/milestone/mod.rs` — `milestone_scan(assay_dir: &Path) -> Result<Vec<Milestone>>` returns milestones sorted by slug. Called after successful wizard submission to reload the dashboard list.
- `ratatui-core-0.1.0/src/terminal/frame.rs` — `frame.set_cursor_position((x, y))` shows the hardware cursor at a specific position during the current frame. Call at the end of `draw_wizard` so the cursor appears inside the active input field.
- `ratatui-widgets-0.3.0/src/clear.rs` — `Clear` widget resets all cells in an area to default before overdrawing. Required for popup rendering to avoid the dashboard text bleeding through.
- `crossterm-0.28.1/src/event.rs` — `KeyCode` variants relevant to wizard: `Char(char)` (append to buffer), `Backspace` (delete last char or go back a step), `Enter` (advance step or finish criteria), `Esc` (cancel wizard). `KeyEventKind::Press` is the only kind to act on — guard with `if key.kind != KeyEventKind::Press { return WizardAction::Continue; }`.

## Constraints

- **D090** — `WizardState` lives in `assay-tui`, not `assay-core`. Core provides `create_from_inputs`; TUI provides all form rendering and event handling. Do NOT add any TUI types to assay-core.
- **D001 / D089** — `draw_wizard` must be a free function `fn draw_wizard(frame: &mut Frame, state: &WizardState)`, not a `Widget` impl. No trait objects or trait impls on wizard state.
- **D091** — `create_from_inputs` must be called in `handle_event` / `handle_wizard_event`, not inside `terminal.draw()`. The draw callback holds a mutable borrow on the terminal — blocking I/O there causes starvation.
- **S01 prerequisite** — `Screen::Wizard(WizardState)` variant and `App.project_root: Option<PathBuf>` must exist before S02 begins. Do not attempt S02 on a branch that lacks S01's outputs. The `project_root` is `None` on no-project guard; the wizard keybinding (`n`) must be unreachable from `Screen::NoProject`.
- **No `dialoguer`** — The context and research both flag `dialoguer` as incompatible with Ratatui raw mode (Pitfall 2, M006-RESEARCH.md). S02 must use crossterm `KeyEvent`s and Vec<String> buffers exclusively.
- **tempfile as dev-dependency** — The integration test needs `tempfile`. Add `tempfile.workspace = true` to `[dev-dependencies]` in `crates/assay-tui/Cargo.toml`. It is already in the workspace deps.
- **KeyEventKind filtering** — crossterm can emit `Repeat` and `Release` events in some terminal configurations. Always guard: `if key.kind != KeyEventKind::Press { return WizardAction::Continue; }` at the top of `handle_wizard_event`.

## Common Pitfalls

- **Variable step count causes off-by-one in step routing** — Step count = 3 (name, description, chunk-count) + N (chunk names) + N (criteria) = 3 + 2*N where N = chunk_count. This is only known after step 2. Allocate `fields` dynamically when chunk_count is confirmed: push `Vec::new()` for each chunk-name step, then `Vec::new()` for each criteria step. Index arithmetic must account for the confirmed chunk_count stored in `WizardState.chunk_count`. A simple guard: define named helper methods `current_step_kind(&self) -> StepKind` to avoid raw index arithmetic in `draw_wizard` and `handle_wizard_event`.
- **Criteria step multi-line vs single-line confusion** — Criteria fields hold multiple entries (one per criterion); single-line fields (name, description) hold exactly one entry. Using the same `Vec<Vec<String>>` for both means the criteria step appends to the inner `Vec` on each Enter, while name/description steps replace the single entry. This asymmetry must be explicit in the code: a `StepKind::Criteria` variant advances to next step on blank Enter; all other steps advance on any Enter.
- **Backspace-on-empty goes back a step** — This is expected UX but easy to implement wrongly. When `fields[current_step]` is empty (or the only entry is an empty string) and the user presses Backspace, decrement `step` (not into negative). For criteria steps: backspace when the last criterion is empty removes that entry and stays in the criteria step; only when the criteria list is itself empty does backspace go back to the chunk-name step.
- **Slug preview flicker** — Computing `slugify(buffer)` on every keypress and showing it as a dim hint is correct, but if `buffer` is empty the slug preview should show a placeholder (e.g., `(slug will appear here)`) rather than panicking. Guard `slugify` call: only call when the buffer is non-empty.
- **create_from_inputs error handling in wizard** — On error (e.g. slug collision), stay in the wizard with `state.error = Some(message)`. Do NOT switch to dashboard. Clear `state.error` on the next keypress. Verify this is exercised by a test: feed a duplicate slug and confirm the wizard stays at step 0 with an error, not at `Screen::Dashboard`.
- **Popup dimensions exceed terminal** — A 60×12 popup is ample for most terminals, but a 40-column terminal would clip it. Guard popup area: use `area.width.min(60)` and `area.height.min(12)` when computing layout dimensions. This prevents the `Layout` constraint solver from panicking on undersized areas.
- **integration test does not need a terminal** — `handle_wizard_event` is a pure state machine function that takes `&mut WizardState` and `KeyEvent`, not a `&mut Terminal`. The test instantiates `WizardState::new()`, calls `handle_wizard_event` in a loop with synthetic `KeyEvent`s, waits for `WizardAction::Submit(inputs)`, then calls `create_from_inputs`. No terminal init needed. `draw_wizard` is not called in the test; its correctness is verified by visual UAT.

## Open Risks

- **S01 outputs diverge from boundary map** — S02 design assumes the M006 boundary map exactly (D089: `Screen::Wizard(WizardState)`, `App.project_root`, `milestone_scan` reload pattern). If S01 implementation deviates (e.g. renames variants, changes how `project_root` is stored), S02 planning must reconcile before task decomposition. Read S01-SUMMARY.md before writing S02-PLAN.md.
- **Dynamic step count increases planner complexity** — The boundary map describes `fields: Vec<Vec<String>>` but leaves step routing implementation-defined. 3 + 2*N total steps (N confirmed at step 2) is more complex to route than a fixed step count. Plan a dedicated `step_kind()` helper and add a unit test for step transitions at each N in {1,2,3,7} to cover edge cases.
- **Integration test binary registration** — `crates/assay-tui/tests/wizard_round_trip.rs` will be auto-discovered by cargo as an integration test binary. No `[[test]]` section in Cargo.toml is needed, but the test binary will link against the entire `assay-tui` crate (including main.rs). Ensure `main.rs` does not contain unconditional `ratatui::init()` at module level — it must stay in `fn main()` so integration tests can import TUI functions without initializing a terminal.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | (none installed) | None found — no dedicated Ratatui skill in `~/.kata-cli/agent/skills/` |
| Rust crossterm | (none installed) | No skill; crossterm is simple enough to reference directly from registry source |

## Sources

- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ratatui-core-0.1.0/src/terminal/frame.rs` — `set_cursor_position` API, popup layout with `Flex::Center`, `Clear` widget (HIGH confidence)
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/crossterm-0.28.1/src/event.rs` — `KeyCode`, `KeyEventKind`, `KeyEvent` struct fields (HIGH confidence)
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ratatui-widgets-0.3.0/src/clear.rs` — `Clear` widget for popup overdraw (HIGH confidence)
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ratatui-core-0.1.0/src/layout/flex.rs` — `Flex::Center` for centered popup layout (HIGH confidence)
- Codebase: `crates/assay-core/src/wizard.rs` — Full public API: `WizardInputs`, `WizardChunkInput`, `create_from_inputs`, `slugify` (HIGH confidence)
- Codebase: `crates/assay-cli/src/commands/plan.rs` — Reference dialoguer wizard flow; maps to S02 step sequence (HIGH confidence)
- Codebase: `crates/assay-core/tests/wizard.rs` — Integration test pattern with TempDir + milestone_load assertions (HIGH confidence)
- M006-RESEARCH.md (preloaded) — Pitfall 2 (dialoguer incompatibility), step-count complexity risk, WizardState field layout (HIGH confidence)
- S02-CONTEXT.md — Authoritative step sequence, scope constraints, integration point definitions (HIGH confidence)
