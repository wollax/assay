# S05: TUI Analytics Screen — UAT

**Milestone:** M008
**Written:** 2026-03-24

## UAT Type

- UAT mode: mixed (artifact-driven + human-experience)
- Why this mode is sufficient: Integration tests prove screen transitions, data population, and rendering without panic. Visual verification of color coding and table layout is a nice-to-have human check but not required for correctness — the color logic matches S04 CLI thresholds which are already unit-tested.

## Preconditions

- `just build` succeeds (assay-tui binary compiled)
- A project directory with `.assay/` containing at least one milestone with gate history records (for real data)
- Alternatively: any project directory (empty analytics shows "No analytics data available" message)

## Smoke Test

1. Run `cargo run -p assay-tui` from a project with `.assay/` containing gate history
2. Press `a` on the Dashboard
3. **Expected:** Analytics screen appears with failure frequency and milestone velocity tables

## Test Cases

### 1. Analytics screen transition from Dashboard

1. Launch TUI in a project with `.assay/` directory
2. Press `a`
3. **Expected:** Screen transitions to Analytics showing bordered "Analytics" block with two tables

### 2. Return to Dashboard via Esc

1. From Analytics screen, press `Esc`
2. **Expected:** Returns to Dashboard with milestone list intact

### 3. Return to Dashboard via q

1. From Analytics screen, press `q`
2. **Expected:** TUI exits cleanly (q is quit signal)

### 4. Color-coded failure rates

1. From Analytics screen with gate history containing failures
2. **Expected:** Rate column shows red for >50% failure rate, yellow for >0%, green for 0%

### 5. Milestone velocity display

1. From Analytics screen with completed chunks
2. **Expected:** Velocity table shows milestone slug, chunks completed/total, days elapsed, and chunks/day rate

## Edge Cases

### No project directory

1. Launch TUI without a `.assay/` directory (or with `project_root` = None)
2. Press `a`
3. **Expected:** Nothing happens (no-op, same as `r` and `n` guards)

### Empty analytics data

1. Launch TUI in a project with `.assay/` but no gate history records
2. Press `a`
3. **Expected:** Analytics screen shows centered "No analytics data available" message

### Help overlay includes analytics

1. From Dashboard, press `?`
2. **Expected:** Help overlay includes `a → Analytics` row in the Dashboard section

## Failure Signals

- `a` key does nothing from Dashboard when project_root is set — key handler broken
- Analytics screen shows blank area instead of tables — draw_analytics not wired
- Panic on `a` key press — compute_analytics error not handled (missing `.ok()`)
- Missing `a → Analytics` in help overlay — help text not updated

## Requirements Proved By This UAT

- R059 (Gate history analytics) — TUI analytics screen renders failure frequency heatmap and milestone velocity, completing the requirement alongside the S04 CLI component

## Not Proven By This UAT

- Real-world visual quality of color coding (automated tests verify logic, not pixel rendering)
- Performance with very large history directories (synchronous load on `a` key)
- Interaction with live agent runs (analytics shows historical data, not live)

## Notes for Tester

- The wizard integration test has a pre-existing intermittent hang — if `just ready` stalls, it's not related to S05
- Color coding in the failure frequency table uses ratatui's `Color::Red`, `Color::Yellow`, `Color::Green` — actual appearance depends on terminal color scheme
- Analytics data comes from `.assay/history/` records — to see meaningful data, run some gate evaluations first
