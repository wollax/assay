---
estimated_steps: 5
estimated_files: 3
---

# T02: Wire slash overlay into App event handling and drawing

**Slice:** S03 — Slash Command Overlay
**Milestone:** M007

## Description

Connect the slash module to the App lifecycle. Add `App.slash_state: Option<SlashState>` field, the guard in `handle_event` (after help guard, before wizard check), the `/` key handler in applicable screens, `draw_slash_overlay` call in `draw()`, and the real `handle_slash_event` and `draw_slash_overlay` implementations. After this task, all 6 T01 integration tests pass and the slash overlay is fully functional.

## Steps

1. Add `slash_state: Option<SlashState>` field to `App` struct in `app.rs`. Initialize to `None` in `with_project_root` and `new()`. Add `use crate::slash::*;` import.

2. Add slash overlay guard in `handle_event`, placed after the help guard and before the wizard check:
   ```
   if let Some(ref mut state) = self.slash_state {
       let action = handle_slash_event(state, key);
       match action {
           SlashAction::Continue => {}
           SlashAction::Close => { self.slash_state = None; }
           SlashAction::Execute(cmd) => {
               if let Some(root) = &self.project_root {
                   let result = execute_slash_cmd(cmd, root);
                   if let Some(ref mut st) = self.slash_state {
                       st.result = Some(result);
                   }
               }
           }
       }
       return false;
   }
   ```

3. Add `/` key handler: In the `match self.screen` arms for Dashboard, MilestoneDetail, ChunkDetail, AgentRun, and Settings, add `KeyCode::Char('/')` that sets `self.slash_state = Some(SlashState::default())`. Do NOT add in Wizard, NoProject, or LoadError arms.

4. Implement real `handle_slash_event` in `slash.rs`:
   - `Esc` → `SlashAction::Close`
   - `Enter` → parse input, if `Some(cmd)` return `SlashAction::Execute(cmd)`, else set `state.error = Some("Unknown command")` and return `Continue`
   - `Tab` → call `tab_complete(&state.input)`, if Some set `state.input` to suggestion, clear error
   - `Char(c)` → append to `state.input`, update suggestion via `tab_complete`, clear result/error
   - `Backspace` → pop from `state.input`, update suggestion, clear result/error

5. Implement real `draw_slash_overlay` in `slash.rs`:
   - Bottom-aligned overlay: compute height based on result presence (2 lines min for input + hint, up to 12 lines with result)
   - `Clear` the area, render `Block` with borders titled "Command"
   - Input line: show `state.input` with cursor; if suggestion, show dimmed completion hint after input
   - Result area: if `state.result.is_some()`, render as Paragraph (truncate to ~10 lines)
   - Error area: if `state.error.is_some()`, render in red
   - Hint line at bottom: "Enter: run │ Tab: complete │ Esc: close"

6. Add `draw_slash_overlay` call in `App::draw()`: after the screen-specific draw, before the help overlay draw, check `if let Some(ref state) = self.slash_state { draw_slash_overlay(frame, frame.area(), state); }`

## Must-Haves

- [ ] `App.slash_state: Option<SlashState>` initialized to `None`
- [ ] Slash guard in `handle_event` intercepts all keys when overlay is active (D104 pattern)
- [ ] `/` key opens overlay from Dashboard, MilestoneDetail, ChunkDetail, AgentRun, Settings
- [ ] `/` key does NOT open overlay from Wizard, NoProject, LoadError
- [ ] `handle_slash_event` handles Esc (close), Enter (dispatch), Tab (complete), Char (append), Backspace (delete)
- [ ] `execute_slash_cmd` result is stored in `SlashState.result` after Enter dispatch
- [ ] `draw_slash_overlay` renders bottom-aligned with input, result, and hint areas
- [ ] All 6 T01 integration tests pass
- [ ] All existing 31+ assay-tui tests pass (zero regressions)
- [ ] `cargo clippy -p assay-tui -- -D warnings` clean

## Verification

- `cargo test -p assay-tui --test slash_commands` — all 6 tests pass
- `cargo test -p assay-tui` — all tests pass (31+ existing + 6 new), zero regressions
- `cargo build -p assay-tui` — zero warnings
- `cargo clippy -p assay-tui -- -D warnings` — clean

## Observability Impact

- Signals added/changed: `App.slash_state` field is the primary observable surface — `Some(SlashState { result, error, .. })` exposes command outcome; `None` means overlay closed
- How a future agent inspects this: Check `app.slash_state.is_some()` for overlay visibility; read `slash_state.result` for last command output; read `slash_state.error` for last error
- Failure state exposed: Unknown command → `state.error = "Unknown command: /foo"`; execute failure → `state.result` contains error string from assay-core (e.g., "No active milestone"); no panics on any path

## Inputs

- `crates/assay-tui/src/slash.rs` — T01 output: SlashCmd, SlashState, SlashAction, parse_slash_cmd, execute_slash_cmd, tab_complete, handle_slash_event (stub), draw_slash_overlay (stub)
- `crates/assay-tui/src/app.rs` — current App struct, handle_event guard pattern (help overlay at line 278), draw() method, Screen enum
- `crates/assay-tui/tests/slash_commands.rs` — T01 output: 6 integration tests (3 passing, 3 failing)
- S01-SUMMARY.md — handle_event signature, event_tx guard pattern, App field conventions

## Expected Output

- `crates/assay-tui/src/app.rs` — gains `slash_state` field, slash guard in handle_event, `/` key handler in 5 screen arms, draw_slash_overlay call in draw()
- `crates/assay-tui/src/slash.rs` — stubs replaced with real handle_slash_event and draw_slash_overlay implementations
- `crates/assay-tui/tests/slash_commands.rs` — all 6 tests now passing
