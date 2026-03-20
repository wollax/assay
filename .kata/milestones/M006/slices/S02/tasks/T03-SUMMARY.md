---
id: T03
parent: S02
milestone: M006
provides:
  - "crates/assay-tui/src/wizard_draw.rs — draw_wizard(frame, state) free function; centered 64×14 popup via Clear + Block; step counter, prompt, accumulated criteria, active input buffer, slug hint, inline error, key hints; hardware cursor positioned via frame.set_cursor_position"
  - "crates/assay-tui/src/app.rs — Screen::Wizard(WizardState) variant added; n keybinding opens wizard from Dashboard; handle_wizard_key dispatches WizardAction::Continue/Cancel/Submit; on Submit calls create_from_inputs, reloads milestones, returns to Dashboard; on error sets state.error and stays in wizard"
  - "crates/assay-tui/src/lib.rs — pub mod wizard_draw; added; library now exports wizard and wizard_draw modules for binary and integration test use"
  - "merge of origin/main S01 work (app.rs scaffold) into S02 branch — S02 branch was branched before S01 PR #158 merged; merge commit 8751d74 brings in App/Screen/run/draw/handle_event foundation"
  - "3 new App-level tests: test_n_key_opens_wizard_from_dashboard, test_esc_in_wizard_returns_to_dashboard, wizard_error_submit_stays_in_wizard"
key_files:
  - crates/assay-tui/src/wizard_draw.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/src/wizard.rs
  - crates/assay-tui/tests/wizard_round_trip.rs
key_decisions:
  - "app.rs lives in the binary module tree (mod app; in main.rs); it imports wizard types via assay_tui::wizard:: using the lib crate — the standard Rust combined bin+lib pattern"
  - "draw() avoids borrow conflicts by checking screen type via matches!() then calling draw_dashboard unconditionally (wizard overlays dashboard), then conditionally calling draw_wizard if Screen::Wizard"
  - "Popup geometry computed manually (centered Rect) rather than via Layout::flex — avoids unused imports and is simpler for a fixed 64×14 popup"
  - "n keybinding fires only when screen is Dashboard (which implies .assay/ exists); no separate project_root.is_some() check needed"
  - "Pre-existing cargo deny failures (aws-lc-sys CVEs via jsonschema dev-dep) are not introduced by T03; fmt + lint + test all pass clean"
patterns_established:
  - "Popup overlay pattern: clear behind popup with Clear widget, then render Block, then render content into block.inner(); call set_cursor_position at end"
  - "Screen-dispatched event handling: check screen variant first with if let, extract wizard state mutably, call pure handle_wizard_event, match owned WizardAction after borrow ends (NLL)"
observability_surfaces:
  - "state.error: Option<String> rendered inline in wizard popup as red text — I/O and slug-collision errors surface immediately without leaving the wizard"
  - "app.milestones reloaded from milestone_scan after Submit success — dashboard reflects new state without restart"
  - "cargo test -p assay-tui wizard_round_trip exercises the Submit path end-to-end"
duration: 90min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T03: Implement draw_wizard and wire into App

**`draw_wizard` popup + App wiring complete — `n` opens wizard, Submit writes milestone and reloads dashboard, Esc cancels; 23 assay-tui tests + 1333 workspace tests all green.**

## What Happened

S02 branch was created before S01's PR (#158) merged to origin/main. First action was merging origin/main to bring in `app.rs` (App struct, Screen enum, run/draw/handle_event). Resolved conflicts in Cargo.toml (combined `[lib]` + `[[bin]]`), .kata/ files (kept S02 STATE.md, took origin/main for DECISIONS/PROJECT/REQUIREMENTS).

Created `crates/assay-tui/src/wizard_draw.rs` with `draw_wizard(frame, state)`:
- Popup area computed as centered 64×14 Rect with width/height clamped to terminal bounds
- `Clear` rendered first to erase dashboard text behind popup
- `Block` with title `" New Milestone "` wraps content
- Content lines: step counter (dim), step prompt, accumulated criteria with `•` prefix (for Criteria steps), active input buffer `> <buf>`, slug hint (dim, Name/ChunkName steps only), error in red (when `state.error.is_some()`), key hints (dim)
- `frame.set_cursor_position` called at end for hardware cursor in active field

Updated `lib.rs` to add `pub mod wizard_draw;`. Updated `app.rs`:
- `Screen::Wizard(WizardState)` variant added to Screen enum
- `draw()` refactored to check NoProject first, always draw dashboard, then overlay wizard if Screen::Wizard — avoids borrow conflicts
- `handle_event()` dispatches to `handle_wizard_key()` for Screen::Wizard before any other processing
- `handle_wizard_key()` calls `handle_wizard_event`, matches owned WizardAction: Continue = no-op, Cancel = Screen::Dashboard, Submit = create_from_inputs → reload milestones → Screen::Dashboard (or set state.error and stay in wizard on Err)
- Dashboard handle_event: `n` → `Screen::Wizard(WizardState::new())` when screen is Dashboard
- Footer updated to include `n new milestone` hint
- 3 new tests added covering n-key, Esc-in-wizard, and error-stays-in-wizard paths

Fixed pre-existing clippy warning in wizard.rs: `c >= '1' && c <= '7'` → `('1'..='7').contains(&c)`.

## Verification

```
cargo test -p assay-tui       # 23 tests: 14 lib unit + 8 app unit + 1 integration — all pass
cargo build -p assay-tui      # exits 0; target/debug/assay-tui produced
cargo clippy --workspace --all-targets -- -D warnings   # clean
cargo fmt --all -- --check    # clean
cargo test --workspace        # 1356 tests total — all pass
```

`just ready` fmt ✓ lint ✓ test ✓; deny fails on 6 pre-existing `aws-lc-sys` CVEs (RUSTSEC-2026-0044 through -0049) via `jsonschema` dev-dep in assay-types — same 6 errors present on the commit before T03 began.

## Diagnostics

- `cargo test -p assay-tui wizard_round_trip -- --nocapture` — exercises Submit path end-to-end (no terminal required)
- `cargo test -p assay-tui` — all 23 tests including App wiring tests
- Visual UAT: launch `assay-tui` on `.assay/` project; `n` → popup appears with step counter and prompt; fill all steps; last blank Enter → wizard closes; new milestone in dashboard list; `Esc` mid-wizard → dashboard, no files written

## Deviations

- S01 app.rs had to be merged in from origin/main; S02 branch was created before PR #158 merged. Required resolving Cargo.toml conflict (combining `[lib]` + `[[bin]]` sections).
- Popup geometry uses manual centered Rect rather than `Layout::flex(Flex::Center)` — simpler, fewer imports, same visual result for a fixed-size popup.
- `project_root` is `PathBuf` (not `Option<PathBuf>`) in S01's actual implementation; `n` keybinding guards on `matches!(app.screen, Screen::Dashboard)` rather than `project_root.is_some()`.

## Known Issues

- `just ready` deny check fails on 6 pre-existing `aws-lc-sys` CVEs (RUSTSEC-2026-0044 to -0049) introduced via jsonschema dev-dep. Not introduced by T03; present since before this slice began. Requires `cargo update -p aws-lc-sys` and `cargo update -p rustls-webpki` in a separate task.

## Files Created/Modified

- `crates/assay-tui/src/wizard_draw.rs` — new; `draw_wizard` free function
- `crates/assay-tui/src/app.rs` — updated; `Screen::Wizard` variant, `n` keybinding, wizard draw/event dispatch, `create_from_inputs` + reload wiring, 3 new tests
- `crates/assay-tui/src/lib.rs` — updated; `pub mod wizard_draw;` added
- `crates/assay-tui/src/wizard.rs` — clippy fix: `('1'..='7').contains(&c)`
- `crates/assay-tui/tests/wizard_round_trip.rs` — unused import `WizardInputs` removed
- `crates/assay-tui/Cargo.toml` — merged: combined `[lib]` + `[[bin]]` + all deps
