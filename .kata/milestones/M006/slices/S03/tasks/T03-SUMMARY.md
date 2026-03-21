---
id: T03
parent: S03
milestone: M006
provides:
  - "join_results free function: joins Criterion slice with Option<&GateRunRecord> → Vec<(&Criterion, Option<bool>)>"
  - "draw_chunk_detail renderer: Table with icon/name/description columns, Paragraph fallback for None spec"
  - "MilestoneDetail Enter: loads spec via load_spec_entry_with_diagnostics, run via history::list+load, then transitions to Screen::ChunkDetail"
  - "ChunkDetail Esc: returns to Screen::MilestoneDetail preserving detail_milestone and detail_list_state"
  - "draw() ChunkDetail arm: wired to draw_chunk_detail with real spec/run state"
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/spec_browser.rs
key_decisions:
  - "GatesSpec validation requires at least one criterion with cmd/path/kind=AgentReport; test fixture must include cmd field on at least one criterion"
  - "Collapsed nested if-let in Dashboard Enter arm to satisfy clippy::collapsible_if (-D warnings)"
patterns_established:
  - "join_results pattern: iterate criteria, find by criterion_name in run.summary.results, map Option<GateResult> to Option<bool>"
  - "draw_chunk_detail layout: [title(1), table_or_message(fill), hint(1)] — same pattern as other draw_* fns in this file"
observability_surfaces:
  - "app.detail_spec.is_some() — confirms spec loaded as Directory entry"
  - "app.detail_spec_note.as_deref() — carries reason string when spec is None (Legacy or I/O/validation error)"
  - "app.detail_run.is_some() — confirms whether gate history exists for the chunk"
  - "cargo test -p assay-tui spec_browser::chunk_detail_no_history_all_pending — directly tests spec-loaded + no-run state"
duration: 30min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T03: ChunkDetail screen — spec+history load, criterion join, Table render

**ChunkDetail screen fully implemented: MilestoneDetail Enter loads spec via `load_spec_entry_with_diagnostics` and run via `history::list`+`load`, `draw_chunk_detail` renders a Ratatui Table with ✓/✗/? icons, all 6 spec_browser tests pass, `just ready` green.**

## What Happened

Added `join_results` (maps criteria to pass/fail/pending by name-joining against the run's `summary.results`) and `draw_chunk_detail` (renders a bordered `Table` with icon/name/description columns when spec is `Some`, or a dim `Paragraph` with the note when spec is `None`). Wired `draw_chunk_detail` into the `draw()` ChunkDetail arm by cloning `chunk_slug` first to release the borrow on `self.screen` before borrowing `self.detail_spec`.

The MilestoneDetail `Enter` handler was extended to call `load_spec_entry_with_diagnostics` (setting `detail_spec`/`detail_spec_note` depending on `SpecEntry` variant or error) and `history::list`+`load` (setting `detail_run = None` when history is absent — not an error). The ChunkDetail `Esc` handler (already wired in T02) was confirmed correct.

One fix required in the test fixture: `GatesSpec` validation rejects specs where no criterion has `cmd`, `path`, or `kind = AgentReport`. The `setup_project` fixture in `spec_browser.rs` had two criteria with only `name`/`description`. Added `cmd = "true"` to the first criterion to satisfy the validator.

One clippy fix: Dashboard `Enter` had nested `if let` blocks which clippy's `collapsible_if` lint (promoted to error via `-D warnings`) flagged. Collapsed to `if let Some(idx) = ... && let Some(ms) = ...`.

## Verification

```
cargo test -p assay-tui --test spec_browser
# test result: ok. 6 passed; 0 failed

cargo test -p assay-tui
# test result: ok. 9 passed; 0 failed (unit tests)
# test result: ok. 6 passed; 0 failed (spec_browser integration tests)

just ready
# fmt ✓, lint ✓, test ✓, deny ✓
# All checks passed.
```

## Diagnostics

- `app.detail_spec.is_some()` — `true` when MilestoneDetail Enter loaded a Directory-variant spec
- `app.detail_spec_note.as_deref()` — `Some("Legacy flat spec...")` or `Some("Failed to load spec: ...")` when spec is `None`
- `app.detail_run.is_some()` — `true` only when `history::list` returned non-empty IDs and `history::load` succeeded
- `cargo test -p assay-tui spec_browser::chunk_detail_no_history_all_pending -- --nocapture` — executable check for no-history state

## Deviations

- Test fixture required `cmd = "true"` on one criterion to pass GatesSpec validation — not mentioned in the task plan, which said "fix any test setup issues". Root cause: `load_spec_entry_with_diagnostics` runs full validation (including the "at least one executable criterion" rule), not just TOML parse.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — added imports (history, SpecEntry, load_spec_entry_with_diagnostics, Criterion, Cell, Row, Table, Color); join_results fn; draw_chunk_detail fn; MilestoneDetail Enter spec+run loading; draw() ChunkDetail arm wired; clippy collapsible_if fix
- `crates/assay-tui/tests/spec_browser.rs` — added `cmd = "true"` to first criterion in setup_project fixture
