# S01: App Scaffold, Dashboard, and Binary Fix — UAT

**Milestone:** M006
**Written:** 2026-03-20

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: The critical must-haves (binary produced, no-project guard, navigation) are fully covered by unit and integration tests. Visual inspection confirms that the rendering actually produces a readable dashboard — this cannot be verified programmatically without a headless terminal harness. The UAT is optional confirmation, not a gate.

## Preconditions

1. `cargo build -p assay-tui` has been run (binary present at `target/debug/assay-tui`)
2. A project with `.assay/milestones/` exists (e.g. the assay repo itself, or a fixture directory)
3. For the gate counts test: at least one milestone has gate history in `.assay/results/`

## Smoke Test

Run `cargo run -p assay-tui` in the assay repo root. The TUI should launch, show a dashboard with at least one milestone, and quit cleanly on `q`.

## Test Cases

### 1. Binary produced, no collision

```bash
cargo build -p assay-tui && ls -la target/debug/assay-tui
cargo build -p assay-cli && ls -la target/debug/assay
```

**Expected:** Both files exist; no build error; sizes are distinct.

### 2. Dashboard launches on a project with milestones

```bash
cd /path/to/project/with/.assay/
cargo run -p assay-tui
```

**Expected:** TUI launches; shows milestone list with name, `[Status]` badge, `done/total` chunk fraction, and `✓N ✗N` gate counts; selection highlight is visible on the first item.

### 3. Arrow-key navigation

While dashboard is showing:
1. Press `↓` several times
2. Press `↑` several times
3. Press `↓` past the last item

**Expected:** Selection highlight moves down and up; wraps from last item back to first on `↓`; wraps from first to last on `↑`.

### 4. Quit with `q`, `Q`, and `Esc`

1. Press `q` from dashboard
2. Relaunch; press `Q`
3. Relaunch; press `Esc`

**Expected:** All three quit cleanly, terminal is restored (no raw-mode artifacts).

### 5. No-project guard

```bash
cd /tmp && cargo run -p assay-tui
```

**Expected:** Screen shows "Not an Assay project — run `assay init` first" (or similar actionable message); pressing `q` exits cleanly; no panic.

### 6. Empty milestone list

Launch on a project where `.assay/` exists but `.assay/milestones/` is empty or missing.

**Expected:** Screen shows "No milestones — run `assay plan`" message; pressing `q` exits cleanly.

## Edge Cases

### Missing gate history

Launch on a project with milestones but no `.assay/results/` directory (fresh project, no gate runs yet).

**Expected:** Dashboard shows `✓0 ✗0` for all milestones; no panic; no error message.

### Malformed config.toml

Introduce a syntax error in `.assay/config.toml` before launching.

**Expected:** TUI launches with empty config (silently degrades); dashboard still shows milestones; no panic.

### Terminal resize

Resize the terminal window while the dashboard is visible.

**Expected:** Dashboard reflows to fit new dimensions; no panic; no display corruption lasting more than one frame.

## Failure Signals

- Panic on launch (any screen) — indicates unguarded unwrap on missing files or bad data
- Blank screen (no content rendered) — indicates draw dispatch is broken
- Terminal raw mode not restored after quit — indicates `ratatui::restore()` is not being called on all exit paths
- `target/debug/assay-tui` absent after `cargo build -p assay-tui` — `[[bin]]` declaration missing or broken
- `target/debug/assay` absent or renamed — collision regression in assay-cli's Cargo.toml

## Requirements Proved By This UAT

- R049 (TUI project dashboard) — live visual confirmation that milestones, status badges, chunk fractions, and gate counts render correctly from real `.assay/` data; keyboard navigation works; no-project guard works; quit is clean

## Not Proven By This UAT

- Wizard form interaction (R050) — S02
- Chunk/criteria detail navigation (R051) — S03
- Provider configuration persistence (R052) — S04
- Help overlay and status bar (S05)
- Correctness of gate counts beyond zero values — unit test `test_gate_data_loaded_from_history` proves the data path; visual UAT only confirms rendering

## Notes for Tester

The `cargo deny check` step in `just ready` currently fails on pre-existing `aws-lc-sys` RUSTSEC-2026-0044..0048 advisories (pulled by `jsonschema` dev-dep of `assay-types`). This is pre-existing and unrelated to S01. All other checks (`fmt`, `clippy`, `test`) pass.

For a quick fixture, the assay repo itself has milestones under `.assay/milestones/` — launch from the repo root for a real dashboard view.
