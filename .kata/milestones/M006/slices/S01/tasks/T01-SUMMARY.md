---
id: T01
parent: S01
milestone: M006
provides:
  - "[[bin]] name = 'assay-tui' in Cargo.toml — explicit binary declaration, no naming collision"
  - "lib.rs with App, Screen, WizardState, draw, handle_event, run as public API"
  - "Screen enum with 6 variants: Dashboard, MilestoneDetail, ChunkDetail, Wizard(WizardState), Settings, NoProject"
  - "App struct with 6 fields: screen, milestones, list_state, project_root, config, show_help"
  - "Thin main.rs entry point delegating to assay_tui::run — no manual panic hook"
key_files:
  - crates/assay-tui/Cargo.toml
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/src/main.rs
key_decisions:
  - "Config imported from assay_types::Config (not assay_core::config::Config — that module has no Config struct)"
  - "assay-types added as workspace dependency to assay-tui/Cargo.toml (was missing)"
  - "All screen arms in draw() render placeholder Paragraph text — real content deferred to T02/T03"
  - "Unused List/ListState imports suppressed with let _ trick to avoid compiler errors until T02"
patterns_established:
  - "thin main.rs + lib.rs split — all logic in lib.rs so integration tests can import from assay_tui::"
  - "handle_event returns bool (false = quit) as the explicit control-flow signal"
observability_surfaces:
  - "App.screen field — read to determine current screen"
  - "App.milestones.len() — read to check data load status"
  - "handle_event bool return — false = terminate, true = continue; primary quit signal"
duration: 10min
verification_result: passed
completed_at: 2026-03-20T20:40:00Z
blocker_discovered: false
---

# T01: Cargo.toml binary fix, App/Screen types, and run loop skeleton

**Replaced 42-line stub with lib.rs holding App/Screen/WizardState types plus draw/handle_event/run free functions; added explicit [[bin]] declaration; main.rs is now a 12-line thin entry point.**

## What Happened

Added `[[bin]] name = "assay-tui" path = "src/main.rs"` to `crates/assay-tui/Cargo.toml` (D088 fix), also adding `assay-types.workspace = true` which was missing but required for `Config` and `Milestone` imports.

Created `crates/assay-tui/src/lib.rs` with:
- `WizardState` (Default derive, placeholder for S02)
- `Screen` enum with all 6 variants (Dashboard, MilestoneDetail, ChunkDetail, Wizard(WizardState), Settings, NoProject); `#[allow(dead_code)]` suppresses warnings for unused variants
- `App` struct with all 6 fields from D089
- `draw` free function matching on `app.screen`, rendering placeholder `Paragraph` text per screen
- `handle_event` free function returning `bool` — `q` always quits, `Esc` quits only on Dashboard
- `run` free function: draw loop → read event → handle_event → break if false

Rewrote `main.rs` as a 12-line thin entry point: `color_eyre::install()` → `ratatui::init()` → construct `App` → `assay_tui::run()` → `ratatui::restore()`. The manual `std::panic::set_hook` block from the original stub was removed; `ratatui::init()` installs its own hook.

One deviation from the task plan: `Config` is in `assay_types::Config`, not `assay_core::config::Config` (the core config module contains no `Config` struct). Import corrected accordingly.

## Verification

```
cargo check -p assay-tui 2>&1 | grep -c error   → 0
cargo build -p assay-tui → Finished; target/debug/assay-tui exists (8.4 MB)
cargo build -p assay-cli → Finished; target/debug/assay exists (no collision)
grep '[[bin]]' crates/assay-tui/Cargo.toml      → [[bin]]
grep -c 'set_hook' crates/assay-tui/src/main.rs → 0
```

All 9 must-haves confirmed:
- `[[bin]]` present before `[dependencies]` ✓
- `WizardState` defined; `Screen::Wizard(WizardState)` compiles ✓
- All 6 Screen variants present ✓
- All 6 App fields present ✓
- `draw` is a free fn ✓
- `handle_event` is a free fn returning bool ✓
- No `set_hook`/`take_hook` in main.rs ✓
- `target/debug/assay-tui` produced ✓
- `target/debug/assay` produced, no naming collision ✓

## Diagnostics

- `App.screen` — read the enum variant to determine current screen
- `App.milestones.len()` — 0 until data is loaded in T02
- `handle_event` bool return — false signals quit; this is the only control-flow observable

## Deviations

- `Config` imported from `assay_types::Config` instead of `assay_core::config::Config` (plan listed the wrong path; the core config module has no `Config` struct — it uses `assay_types::Config` internally)
- `assay-types` added to assay-tui's Cargo.toml dependencies (was not present in the original file; required for `Milestone` and `Config`)

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/Cargo.toml` — added `[[bin]]` section and `assay-types.workspace = true` dependency
- `crates/assay-tui/src/lib.rs` — new; contains all public types and logic
- `crates/assay-tui/src/main.rs` — rewritten as 12-line thin entry point
