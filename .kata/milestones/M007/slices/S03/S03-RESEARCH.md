# S03: Slash Command Overlay — Research

**Date:** 2026-03-23

## Summary

S03 adds a slash command overlay to the TUI — pressing `/` from any non-wizard screen opens a bottom-aligned input line with tab completion, dispatching to `assay-core` functions synchronously (D111). The slice is low-risk: it's a new `Screen::SlashCmd` variant with a new `SlashState` struct, a parser (`parse_slash_cmd`), a dispatcher (`execute_slash_cmd`), and a render function (`draw_slash_overlay`). All five commands (`/gate-check`, `/status`, `/next-chunk`, `/spec-show`, `/pr-create`) map directly to existing `assay-core` public functions with no new library dependencies.

The main structural question is where the slash overlay fits in the render/event pipeline. The help overlay pattern (D104/D105) provides the exact template: an `App`-level boolean/state field checked before the normal `match self.screen` dispatch. The slash overlay should use the same guard pattern — when `slash_state` is `Some`, intercept all keys in `handle_event` and route them to slash-specific handling. Unlike help (which is purely visual), slash needs mutable state (input buffer, result text), so it's modeled as `Option<SlashState>` on `App` rather than a `Screen` enum variant. This avoids the borrow-split problem (D097/D098) and means slash can overlay any screen.

The five commands all call synchronous `assay-core` functions that complete in milliseconds (file I/O only, no subprocess spawning). `/pr-create` is the exception — it shells out to `gh` which could take seconds. For S03, all commands block the TUI briefly; async execution (spawning in a thread and showing "Running…") is a potential follow-up but not needed given the expected latency.

## Recommendation

**Model slash as `App.slash_state: Option<SlashState>` (overlay on App, not a Screen variant).** Use the help overlay guard pattern: check `slash_state.is_some()` at the top of `handle_event` before the `match self.screen` dispatch. This means `/` works from Dashboard, MilestoneDetail, ChunkDetail, AgentRun, and Settings screens. Exclude Wizard and NoProject/LoadError.

**New module: `crates/assay-tui/src/slash.rs`** with `SlashState`, `SlashCmd` enum, `parse_slash_cmd`, `execute_slash_cmd`, `handle_slash_event`, and `draw_slash_overlay` as free functions. Export via `lib.rs` as `pub mod slash`. Integration tests in `crates/assay-tui/tests/slash_commands.rs`.

**Tab completion:** Simple prefix match against the known command list. One `Option<String>` suggestion field; Tab fills it in; no dropdown UI needed for 5 commands.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Popup/overlay rendering | `draw_help_overlay` in `app.rs` | Same Clear + Block + Paragraph pattern; just bottom-aligned instead of centered |
| Gate evaluation for `/gate-check` | `assay_core::gate::evaluate_all_gates` | Already handles spec loading, timeout config, and result summarization |
| Cycle status for `/status` | `assay_core::milestone::cycle_status` | Returns `Option<CycleStatus>` with all fields needed |
| Active chunk info for `/next-chunk` | `assay_core::milestone::cycle_status` + `load_spec_entry_with_diagnostics` | CycleStatus gives `active_chunk_slug`; spec loader gives criteria |
| PR gate check for `/pr-create` | `assay_core::pr::pr_check_milestone_gates` | Returns `Vec<ChunkGateFailure>` — display as pass/fail summary |
| Atomic file operations | `assay_core::pr::pr_create_if_gates_pass` | Full PR creation with gate guard, `gh` invocation, milestone mutation |

## Existing Code and Patterns

- `crates/assay-tui/src/app.rs` `draw_help_overlay` — Overlay rendering pattern: compute `Rect`, `Clear` the area, render `Block` + content. Reuse for slash overlay but bottom-aligned (y = area.bottom() - height) instead of centered.
- `crates/assay-tui/src/app.rs` `show_help` guard in `handle_event` (line 279) — Guard pattern: check overlay state before `match self.screen`. Slash overlay uses same pattern with `self.slash_state.is_some()`.
- `crates/assay-tui/src/wizard.rs` `handle_wizard_event` — External event handler returning an action enum. Slash should follow the same pattern: `handle_slash_event(state, key) -> SlashAction` where `SlashAction` is `Continue | Close | Execute(SlashCmd)`.
- `crates/assay-core/src/milestone/cycle.rs` `cycle_status()` — Returns `Option<CycleStatus>` with `milestone_slug`, `active_chunk_slug`, `phase`, `completed_count`, `total_count`. Used by `/status` and `/next-chunk`.
- `crates/assay-core/src/gate/mod.rs` `evaluate_all_gates()` — Synchronous gate evaluation. Used by `/gate-check`. Needs a loaded `GatesSpec` and `working_dir`.
- `crates/assay-core/src/pr.rs` `pr_check_milestone_gates()` — Pre-flight gate check for PR creation. Returns failures list. Used by `/pr-create` to show pass/fail before actually creating.
- `crates/assay-tui/tests/agent_run.rs` — Integration test pattern: `setup_project_with_milestone` helper creates a tempdir fixture; `App::with_project_root` constructs app; synthetic `KeyEvent` values drive the app. Slash tests follow the same pattern.

## Constraints

- **D001 (zero traits):** `SlashCmd` is an enum with free function dispatch, not a trait. `execute_slash_cmd` is a free function, not a method on a trait object.
- **D111 (synchronous dispatch):** All slash commands call `assay-core` functions synchronously. No `std::thread::spawn` for command execution. If a command needs async work, it should transition to a different screen (e.g., `Screen::AgentRun`), not run in the overlay.
- **D104 (help overlay guard pattern):** The slash overlay must use the same guard-at-top-of-handle_event pattern. When slash is active, only slash-relevant keys are processed.
- **D105 (draw functions accept explicit `area: Rect`):** `draw_slash_overlay` accepts `area: Rect` (from `frame.area()`, like help overlay — it spans the full terminal area to position at the bottom).
- **D097 (render fns take individual fields):** `draw_slash_overlay` takes `&SlashState`, not `&App`.
- **assay-core dep direction:** `assay-tui` depends on `assay-core` — can call all public functions. No reverse dependency.
- **`/gate-check` needs a loaded spec:** Must load the GatesSpec via `load_spec_entry_with_diagnostics` for the active chunk. Needs `specs_dir` and `working_dir` from `project_root`.
- **`/pr-create` shells out to `gh`:** This is the only command with external process dependency. Must handle `gh` not found gracefully (return error string, don't panic).

## Common Pitfalls

- **Borrow-split on `self.slash_state` during draw** — If `slash_state` were inside `Screen`, drawing it while also borrowing `self.milestones` etc. would trigger borrow checker errors (D097/D098). Solution: keep `slash_state` as a separate `App` field, not inside `Screen`. The draw function receives `&SlashState` directly.
- **Forgetting to exclude Wizard from `/` key** — The wizard uses character input; pressing `/` during wizard must type `/` into the wizard, not open the slash overlay. Guard: only open slash when `!matches!(self.screen, Screen::Wizard(_) | Screen::NoProject | Screen::LoadError(_))`.
- **Missing `project_root` during execute** — Slash commands need `project_root` for `assay-core` calls. The `/` key handler should only be active when `project_root.is_some()`. If somehow `execute_slash_cmd` is called without a root, return an error string.
- **Tab completion with empty input** — Pressing Tab on empty input should show the first command suggestion (alphabetically or by most-common). Pressing Tab with a partial match should fill the unique completion; with multiple matches, cycle through them.
- **`/gate-check` with no active chunk** — `cycle_status` may return `None` or `Some(cs)` with `active_chunk_slug: None`. Must handle both gracefully with "No active chunk" message.
- **Result display overflow** — Gate check results or error messages could be multi-line. The result area should be a fixed-height scrollable region (or truncated). For S03, truncation to ~10 lines is sufficient.

## Open Risks

- **`/pr-create` blocking the TUI** — `gh pr create` can take 2-5 seconds. During this time the TUI is frozen (no key events processed, no redraws). Acceptable for S03 given `/pr-create` is an infrequent operation. If this becomes a UX issue, wrap in `std::thread::spawn` and show "Creating PR…" (future follow-up).
- **`/gate-check` execution time** — Gate evaluation runs shell commands (`cmd` fields in criteria). Complex specs with many `cmd` criteria could take seconds. Same mitigation as above: acceptable for S03, async wrapper is a follow-up.
- **Screen variant count growing** — `Screen` enum already has 8 variants. Adding `SlashCmd` as a 9th would increase match exhaustion burden. Recommendation (overlay on App, not Screen) avoids this entirely.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | — | none found (not needed — existing patterns in codebase are sufficient) |
| Rust TUI | — | none found |

No external skills needed. This slice uses only existing crate APIs and established Ratatui patterns from M006/S01-S05 and M007/S01.

## Sources

- M007-ROADMAP.md boundary map: S03 produces `SlashCmd` enum, `parse_slash_cmd`, `execute_slash_cmd`, `SlashState`, `Screen::SlashCmd(SlashState)`, `draw_slash_overlay`, 6 integration tests
- S01-SUMMARY.md: forward intelligence on `TuiEvent` loop, `event_tx`, `handle_tui_event` dispatch
- D104: Help overlay guard pattern — keys bypass normal dispatch when overlay visible
- D105: All draw functions accept explicit `area: Rect`
- D111: Slash command dispatch is synchronous and in-process
- Existing codebase: `app.rs` (1267 lines), `wizard.rs`, `agent_run.rs` test patterns
