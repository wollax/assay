# S05: Help Overlay, Status Bar, and Integration Polish — UAT

**Milestone:** M006
**Written:** 2026-03-21

## UAT Type

- UAT mode: mixed (artifact-driven + human-experience)
- Why this mode is sufficient: The contract tests (`help_status.rs`) cover all toggle/state behavior programmatically. Human-experience UAT is required for visual verification of status bar appearance, help overlay layout, terminal resize behavior, and the full end-to-end navigation flow — these require a live terminal that contract tests cannot replicate.

## Preconditions

1. `cargo build -p assay-tui` produces `target/debug/assay-tui` successfully
2. A project with `.assay/` exists (the assay repo itself works: `cd /Users/wollax/Git/personal/assay`)
3. At least one milestone exists in `.assay/milestones/` — use the M005 test fixtures or the assay project's own milestones
4. `just ready` passes (already verified by S05 automation)

## Smoke Test

Run `./target/debug/assay-tui` in the assay project directory. Confirm: the dashboard loads without panic, a one-line status bar is visible at the bottom, and pressing `?` shows a centered keybindings overlay.

## Test Cases

### 1. Status bar visible on launch

1. From the assay project root: `./target/debug/assay-tui`
2. **Expected:** Bottom 1-line status bar visible showing project name (from `.assay/config.toml`) and key hints `? help  q quit` in dim text. If no InProgress milestone exists, cycle slug is blank; if one does, it appears between project name and hints.

### 2. Help overlay opens and closes with `?`

1. From the dashboard, press `?`
2. **Expected:** A centered bordered popup appears titled ` Keybindings ` with a two-column table of keybindings grouped by section (Global, Dashboard, Detail views, Wizard, Settings). The dashboard is still visible behind the overlay.
3. Press `?` again
4. **Expected:** Overlay dismisses; dashboard is active again with no visual artifacts.

### 3. Help overlay closes with `Esc`

1. From the dashboard, press `?` to open help
2. Press `Esc`
3. **Expected:** Overlay dismisses. Dashboard is active. Navigation did NOT change (not treated as "go back").

### 4. Keys are consumed while help overlay is open

1. Press `?` to open help
2. Press `↓` (down arrow)
3. **Expected:** Dashboard list selection does NOT move. Overlay stays open.
4. Press `q`
5. **Expected:** Application does NOT quit. Overlay stays open (only `?` or `Esc` dismiss).

### 5. Terminal resize produces no artifacts

1. With the TUI running, resize the terminal window (drag to smaller then larger)
2. **Expected:** TUI redraws cleanly after each resize. No leftover character artifacts. Status bar remains at bottom. Dashboard list remains correct.

### 6. Status bar shows active milestone slug

1. Ensure at least one milestone has `status = "in_progress"` in its TOML
2. Launch `./target/debug/assay-tui`
3. **Expected:** Status bar shows the InProgress milestone's slug in dim text between the project name and key hints.

### 7. Full navigation flow — dashboard → milestone detail → chunk detail → back

1. From the dashboard, press `↓` to select a milestone
2. Press `Enter`
3. **Expected:** Milestone detail screen shows chunk list for that milestone. Breadcrumb or title indicates the milestone. Status bar remains at bottom.
4. Press `↓` to select a chunk, press `Enter`
5. **Expected:** Chunk detail screen shows criteria table with ✓/✗/? icons from latest gate run (or all `?` if no run exists). Status bar remains at bottom.
6. Press `Esc` to return to milestone detail, press `Esc` again to return to dashboard
7. **Expected:** Dashboard shows with previous list position retained.

### 8. Wizard creates milestone visible in dashboard; status bar updates

1. From the dashboard, press `n` to open the authoring wizard
2. **Expected:** Wizard popup opens on top of dashboard. Status bar remains at bottom.
3. Complete the wizard (enter milestone name, chunk count, chunk names, criteria)
4. **Expected:** Dashboard shows the new milestone immediately. If the new milestone has `status = "in_progress"`, the status bar cycle slug updates.

### 9. Settings screen accessible and saveable

1. From the dashboard, press `s` to open settings
2. **Expected:** Full-screen settings view shows provider options (Anthropic, OpenAI, Ollama) and model fields.
3. Use `↑↓` to select a different provider, press `w` to save
4. **Expected:** Returns to dashboard. `.assay/config.toml` is updated with the new provider choice (verify with `cat .assay/config.toml`).

### 10. No panic on missing `.assay/` directory

1. Run `./target/debug/assay-tui` from a directory with no `.assay/` subdirectory (e.g. `/tmp`)
2. **Expected:** TUI shows a clear "Not an Assay project" message (NoProject screen). Does not panic. Press `q` to exit cleanly.

## Edge Cases

### Help overlay from MilestoneDetail screen

1. Navigate to a milestone detail screen (Enter from dashboard)
2. Press `?`
3. **Expected:** Help overlay opens on top of the milestone detail screen. Overlay shows the same keybinding table. `Esc` dismisses and returns to milestone detail (not dashboard).

### Empty project (no milestones)

1. Create a temp directory with an empty `.assay/milestones/` directory
2. Launch assay-tui from that directory
3. **Expected:** Dashboard shows empty state message (no panic, no blank screen). Status bar shows blank cycle slug.

### Config.toml without provider section

1. Ensure `.assay/config.toml` exists but has no `[provider]` section
2. Launch `./target/debug/assay-tui`
3. **Expected:** TUI launches without error. Settings screen shows default provider (Anthropic). No parse failure. Status bar shows project name from config.

## Failure Signals

- TUI panics on launch → regression in `with_project_root` or `milestone_scan`
- Status bar missing or showing "None" as the cycle slug → `cycle_slug` not being loaded correctly, or `unwrap_or` missing in status bar render
- `?` key navigates or quits instead of opening help → help-overlay event guard not placed at top of `handle_event`
- Pressing keys while help is open causes side effects → guard not returning `false` before screen dispatch
- Terminal artifacts after resize → `Event::Resize` arm missing or `terminal.clear()` not called
- `draw_*` function panics with area=0 → Ratatui saturation arithmetic should prevent this; if seen, check `Constraint::Length(1)` doesn't exceed terminal height
- Status bar renders at top instead of bottom → Layout order is wrong (`[status, content]` instead of `[content, status]`)

## Requirements Proved By This UAT

- R049 (TUI project dashboard) — visual verification that dashboard loads with real data, navigates, and shows status bar
- R050 (TUI interactive wizard) — visual verification that wizard opens from `n`, completes, and new milestone appears immediately
- R051 (TUI spec browser) — visual verification that milestone detail and chunk detail screens render criteria and gate results
- R052 (TUI provider configuration) — visual verification that settings screen opens, provider selection works, save persists to config.toml

## Not Proven By This UAT

- Live gate evaluation from within the TUI (deferred to M007/R053)
- Live refresh of dashboard data while TUI is open without user navigation (deferred to M007)
- Agent spawning from the TUI (deferred to M007)
- `cycle_slug` refresh in Settings save path (gap until S04 lands in this branch)
- MCP server management panel (deferred to M007/R055)
- Full `assay-tui` on a project with the exact M005 test fixtures (automated contract tests use tempdir fixtures; this UAT uses the assay project's own `.assay/`)

## Notes for Tester

- The assay project itself (this repo) is the best test subject — it has real milestones, chunk specs, and gate history from prior development.
- The status bar cycle slug will be blank if no milestone has `status = "in_progress"`. To see it populate, temporarily edit `.assay/milestones/m006.toml` to `status = "in_progress"`, then relaunch.
- `AppConfig` in this slice is a minimal local struct (`project_name` only), not the full `assay_core::config::Config`. If the project name in the status bar looks wrong, check that `.assay/config.toml` has a `project_name` field.
- The Settings screen (`s` from dashboard) requires `App.config` to be `Some` for the `w` save to work. If it shows an inline error on save, that means `config.toml` wasn't loaded on startup — check that the file exists and is valid TOML.
