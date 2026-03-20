# S02: In-TUI Authoring Wizard — UAT

**Milestone:** M006
**Written:** 2026-03-20

## UAT Type

- UAT mode: mixed (artifact-driven + human-experience)
- Why this mode is sufficient: The state machine correctness and filesystem output are fully verified by `wizard_round_trip` integration test (artifact-driven). The popup rendering, visual feedback, and keyboard feel require a human with a live terminal (human-experience). Both components are needed to validate R050.

## Preconditions

1. `cargo build -p assay-tui` succeeds and `target/debug/assay-tui` exists
2. A project with `.assay/` directory and at least one milestone (e.g. the assay repo itself, or a tempdir seeded with `assay init`)
3. Terminal at least 80 columns wide (wizard popup is 64 wide; needs margin)

## Smoke Test

Run `cargo test -p assay-tui wizard_round_trip -- --nocapture` — must pass and show no panics. If this fails, do not proceed with interactive UAT.

## Test Cases

### 1. Open wizard from dashboard

1. Launch `./target/debug/assay-tui` on a project with `.assay/`
2. Confirm dashboard is visible with milestone list
3. Press `n`
4. **Expected:** A centered popup appears with title `New Milestone`, showing "Step 1 of N" and a prompt for milestone name. Hardware cursor visible in the input field.

### 2. Fill all wizard steps (2 chunks)

1. From wizard open (Test Case 1), type `Auth Layer` and press Enter
2. Type a description (or press Enter to leave blank)
3. Type `2` and press Enter for chunk count
4. Type `login` for first chunk name, press Enter
5. Type `register` for second chunk name, press Enter
6. Type `User can log in with valid credentials`, press Enter; press Enter again (blank = done with criteria)
7. Type `User can register a new account`, press Enter; press Enter again
8. **Expected:** Wizard closes; dashboard reappears; new milestone `auth-layer` appears in the list immediately (no restart required)

### 3. Verify files were written

1. After completing Test Case 2, check the filesystem:
   - `.assay/milestones/auth-layer.toml` exists and contains `name = "Auth Layer"` with `chunks = ["login", "register"]`
   - `.assay/specs/login/gates.toml` exists with at least one criterion
   - `.assay/specs/register/gates.toml` exists with at least one criterion
2. **Expected:** All three files present with valid TOML content

### 4. Esc cancels without writing files

1. Launch `assay-tui`, press `n` to open wizard
2. Type `cancel-test` for the name
3. Press `Esc`
4. **Expected:** Wizard closes; dashboard shows; no `.assay/milestones/cancel-test.toml` file created

### 5. Backspace navigates back between steps

1. Open wizard (`n`), type a name, press Enter (advances to Step 2)
2. Press Backspace on empty Step 2 field
3. **Expected:** Returns to Step 1; name field content preserved; cursor at end of name text

### 6. Error display on slug collision

1. Complete a wizard creating milestone `test-milestone` (or pick a slug that already exists)
2. Open wizard again (`n`), type the same name (producing the same slug)
3. Fill all steps and press Enter on final blank criteria
4. **Expected:** Wizard stays open (does NOT return to dashboard); red error text appears inline in the popup (e.g. "milestone already exists"); no duplicate file written

## Edge Cases

### Single-chunk milestone

1. Open wizard, name `solo-chunk`, enter `1` for chunk count, enter one chunk name, add one criterion, blank Enter
2. **Expected:** `.assay/milestones/solo-chunk.toml` with `chunks = ["<name>"]`; one `gates.toml` created

### Invalid chunk count ignored

1. Open wizard, reach chunk count step, type `9` (invalid)
2. **Expected:** Field shows `9` but pressing Enter shows a validation error or silently replaces — must NOT advance to chunk name steps with count=9; try typing `2` to recover

### Minimal input (description blank, one criterion)

1. Open wizard, enter a name, press Enter on description (blank), enter `1` chunk, enter chunk name, enter one criterion, blank Enter
2. **Expected:** Milestone created with no description field in TOML; gates.toml has one criterion

## Failure Signals

- Wizard popup does not appear when pressing `n` — keybinding not wired or Screen::Dashboard guard failing
- Wizard does not close after final blank Enter — submit path not triggered or WizardAction::Submit not matched
- New milestone does not appear in dashboard after wizard closes — `milestone_scan` reload not called after submit
- Dashboard visible through popup with no Clear clearing — Clear widget not rendered
- Hardware cursor not visible in active input field — `set_cursor_position` not called or wrong coordinates
- Panic on any keypress — event handling bug

## Requirements Proved By This UAT

- R050 (TUI interactive wizard) — interactive form inside TUI creates real milestone + chunk spec files; completing it updates dashboard without restart; Esc cancels without writing; error stays in wizard on collision

## Not Proven By This UAT

- Automated CI proof of wizard filesystem output — covered by `wizard_round_trip` integration test (not UAT)
- Terminal resize during wizard interaction — not tested; resize handling is implicit via ratatui re-render
- Wizard behavior with very long milestone names (>64 chars) — no truncation or validation for display overflow
- Non-ASCII input in wizard fields — not tested; depends on terminal encoding

## Notes for Tester

- The automated `wizard_round_trip` test is the real correctness proof. UAT verifies the interactive experience and rendering quality.
- The slug preview hint (dim text below the input showing the derived slug from `slugify()`) only appears on Name and ChunkName steps when the input buffer is non-empty — look for it as a UX confirmation signal.
- Criteria description fields accept freeform text but the generated `gates.toml` will have descriptions only, no `cmd` field — this is intentional (D076). The generated spec is a starting point that requires manual `cmd` editing to be runnable.
- Pre-existing `aws-lc-sys` CVE deny failures do not affect runtime behavior — ignore `cargo deny` output during UAT.
