# S01: App Scaffold, Dashboard, and Binary Fix — UAT

**Milestone:** M006
**Written:** 2026-03-20

## UAT Type

- UAT mode: mixed (artifact-driven + live-runtime)
- Why this mode is sufficient: Binary collision, navigation correctness, and empty-list guard are proven by `cargo build` output and `cargo test` (7 automated tests). Live-runtime checks cover the human-experience cases that cannot be asserted programmatically: visual dashboard rendering, no-project guard clean exit, and keyboard responsiveness.

## Preconditions

- `cargo build -p assay-tui` and `cargo build -p assay-cli` both succeed
- `cargo test -p assay-tui` → 7 passed, 0 failed
- `just ready` → All checks passed
- A terminal with at least 80×24 columns available
- The assay project root (`/Users/wollax/Git/personal/assay`) is available as a test fixture — it has a `.assay/` directory; milestone files may or may not exist depending on current dev state

## Smoke Test

```sh
cd /Users/wollax/Git/personal/assay && cargo run -p assay-tui
```

Expected: TUI launches, shows a "Dashboard" titled bordered panel (possibly empty milestone list or a list of milestones), no panic, exits cleanly on `q`.

## Test Cases

### 1. Binary naming — no collision

```sh
ls -la target/debug/assay target/debug/assay-tui
```

1. Confirm `target/debug/assay` exists (assay-cli binary)
2. Confirm `target/debug/assay-tui` exists (assay-tui binary)
3. **Expected:** Both files present; different sizes; `assay-tui` is approximately 10–15 MB

### 2. Dashboard with real milestone data

1. `cd /Users/wollax/Git/personal/assay`
2. `cargo run -p assay-tui`
3. **Expected:** Dashboard panel titled "Dashboard" renders; if `.assay/milestones/` contains TOML files, each milestone appears as a line showing `name  [StatusBadge]  done/total`; badges are one of `Draft`, `Active`, `Verify`, `Done`

### 3. Keyboard navigation — ↑↓ arrow keys

1. Launch `cargo run -p assay-tui` in a project with 2+ milestones (or create fixtures first)
2. Press `↓` repeatedly
3. **Expected:** Selection highlight moves down; wraps from last item back to first
4. Press `↑` repeatedly
5. **Expected:** Selection moves up; wraps from first item to last

### 4. Enter key — transitions to MilestoneDetail stub

1. Launch TUI; navigate to any milestone with `↓`
2. Press `Enter`
3. **Expected:** Screen changes to show "Milestone detail — coming in S03" (placeholder text); no crash

### 5. Esc key — returns to Dashboard

1. Press `Enter` to enter MilestoneDetail
2. Press `Esc`
3. **Expected:** Returns to Dashboard screen with "Dashboard" title visible

### 6. q key — quits from any screen

1. Press `Enter` to enter MilestoneDetail
2. Press `q`
3. **Expected:** TUI exits cleanly; terminal restored to normal state; no garbage characters left

## Edge Cases

### No .assay/ directory — clean exit message

```sh
mkdir /tmp/test-no-assay-tui && cd /tmp/test-no-assay-tui && cargo run --manifest-path /Users/wollax/Git/personal/assay/Cargo.toml -p assay-tui
```

1. Run from a directory with no `.assay/` subdirectory
2. **Expected:** TUI shows "Not an Assay project — run `assay init` first" (centered text); no panic; `q` exits cleanly

### Empty milestones list — no panic

1. `cd /Users/wollax/Git/personal/assay` (has `.assay/` but possibly zero milestone TOML files)
2. `cargo run -p assay-tui`
3. **Expected:** Dashboard renders with "No milestones — press n to create one" placeholder; no panic; navigation keys are no-ops; `q` exits cleanly

### Terminal restore after launch

1. Run `cargo run -p assay-tui` and then press `q`
2. Type `echo hello` in the terminal
3. **Expected:** Terminal is fully restored — `echo hello` prints normally; no raw-mode artifact

## Failure Signals

- Panic or `unwrap` output in terminal → `Screen::NoProject` guard or empty-list guard is missing
- Both `target/debug/assay` and the binary being named `assay` for the TUI → `[[bin]]` section missing from Cargo.toml
- Dashboard shows hardcoded text instead of milestone names → `milestone_scan` is not being called or its output is ignored
- Status badge shows a raw number or `?` → `MilestoneStatus` match arm missing
- Terminal not restored after `q` → `ratatui::restore()` not called in error path

## Requirements Proved By This UAT

- R049 (TUI project dashboard) — live dashboard loading real milestone data from `milestone_scan`; keyboard navigation; no-project guard; empty-state handled without panic. *Partially proves R049* — chunk detail view (S03) and agent spawning (M007/S01) remain before R049 is fully validated.

## Not Proven By This UAT

- `Screen::MilestoneDetail` content — renders placeholder text; real chunk list navigation deferred to S03
- `Screen::Wizard` — renders placeholder; multi-step form deferred to S02
- `Screen::Settings` — renders placeholder; provider config deferred to S04
- Help overlay (?) keybinding — `App.show_help` field exists but overlay deferred to S05
- Status bar — deferred to S05
- Terminal resize handling — implicit via ratatui automatic redraw; explicit clear-on-resize deferred to S05 if artifacts observed
- R050 (TUI wizard), R051 (TUI spec browser), R052 (TUI provider config) — entirely out of S01 scope

## Notes for Tester

- The assay project root has `.assay/` but milestone files may not exist depending on whether M005 fixtures were set up. If the dashboard appears empty, use the "Empty milestones list" edge case test — that is the expected behavior.
- The no-project test requires running from an arbitrary directory, not the assay repo root. The `--manifest-path` flag in the edge case command allows running the binary from `/tmp/test-no-assay-tui/` while still using the compiled binary from the assay workspace.
- If you see RUSTSEC advisory warnings from `cargo deny` in `just ready`, run `cargo update` to pull latest patch versions — these are transitive dep advisories unrelated to assay code.
