---
id: T01
parent: S02
milestone: M006
provides:
  - assay-tui restructured as binary + library crate with `[lib]` section in Cargo.toml
  - src/lib.rs with `pub mod wizard;` declaration
  - src/wizard.rs with stub public types: WizardState, StepKind, WizardAction, handle_wizard_event
  - tests/wizard_round_trip.rs — full integration test in red state (panics with "T02")
  - tempfile added as dev-dependency in assay-tui
key_files:
  - crates/assay-tui/Cargo.toml
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/src/wizard.rs
  - crates/assay-tui/tests/wizard_round_trip.rs
key_decisions:
  - Wizard module declared in lib.rs without wizard_draw (which would cause a compile error since draw file doesn't exist yet — T03 adds it)
  - All stub functions use `todo!("T02")` to make the failing test's origin unambiguous
patterns_established:
  - Integration test follows assay-core/tests/wizard.rs pattern: TempDir, synthetic KeyEvents, assert on real filesystem artifacts
observability_surfaces:
  - "`cargo test -p assay-tui wizard_round_trip -- --nocapture` shows panic at todo!(\"T02\") — confirms which stub needs implementation"
duration: 15min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T01: Add library target and integration test contract

**Restructured assay-tui as binary + library crate and wrote the full integration test in red state; `cargo build` passes, test panics at `todo!("T02")`.**

## What Happened

`assay-tui` was a pure binary crate (`src/main.rs` only). Integration tests in `tests/` need to `use assay_tui::wizard::*`, which requires a library target. Added `[lib]` section to `Cargo.toml` pointing at `src/lib.rs`, and added `tempfile.workspace = true` under `[dev-dependencies]`.

Created `src/lib.rs` with a single `pub mod wizard;` declaration (no `wizard_draw` — that module file doesn't exist yet and declaring it would cause a compile error).

Created `src/wizard.rs` with all the public types T02 will implement: `WizardState` (step, fields, cursor, chunk_count, error), `StepKind` enum, `WizardAction` enum (Continue, Submit(WizardInputs), Cancel), and `handle_wizard_event`. All methods use `todo!("T02")` as the body.

Wrote `tests/wizard_round_trip.rs` following the exact pattern from `crates/assay-core/tests/wizard.rs`. The test drives two chunks ("Login", "Register") through all wizard steps via synthetic `KeyEvent`s, expects `WizardAction::Submit(inputs)`, calls `create_from_inputs`, and asserts three files exist in a TempDir.

## Verification

- `cargo build -p assay-tui` → exits 0 (library + binary both compile with stubs)
- `cargo test -p assay-tui wizard_round_trip` → exits non-zero; panics at `todo!("T02")` in `WizardState::new()` — confirms red state
- `cargo test -p assay-tui wizard_round_trip 2>&1 | grep "error\[E"` → empty (no compile errors)

## Diagnostics

`cargo test -p assay-tui wizard_round_trip -- --nocapture` will show the exact panic location once T02 begins advancing the implementation. The `"T02"` tag in all `todo!()` calls makes the missing implementation immediately obvious.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/Cargo.toml` — added `[lib]` section and `tempfile` dev-dependency
- `crates/assay-tui/src/lib.rs` — new; `pub mod wizard;`
- `crates/assay-tui/src/wizard.rs` — new; stub types and function (all `todo!("T02")`)
- `crates/assay-tui/tests/wizard_round_trip.rs` — new; full integration test in red state
