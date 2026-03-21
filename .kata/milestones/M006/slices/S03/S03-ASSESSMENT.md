---
id: S03-ASSESSMENT
slice: S03
milestone: M006
assessed_at: 2026-03-21
verdict: roadmap_unchanged
---

# S03 Post-Slice Roadmap Assessment

## Verdict: Roadmap Unchanged

S03 delivered exactly what it promised. The remaining slices (S04, S05) are unaffected in scope,
ordering, or boundary contracts.

## Success Criteria Coverage

All six milestone success criteria have remaining owners after S03:

- `assay-tui` dashboard with milestone status/chunk progress/gate counts → S01 ✓ (complete)
- Navigating into milestone/chunk shows criteria + latest gate run result → S03 ✓ (just completed)
- `n` wizard creates real milestone + chunk specs → S02 ✓ (complete)
- `s` settings screen persists to `.assay/config.toml` → **S04** (remaining, covered)
- No-project guard, clean exit → S01 ✓ (complete)
- `just ready` passes, binary name correct → S01 ✓ + **S05** (remaining, covered)

Coverage check passes. No criterion is unowned.

## Boundary Map Accuracy

**S04 contracts confirmed intact:**
- S03 forward intelligence: `draw()` now has proper `match &self.screen` for all 6 variants;
  `Screen::Settings` arm is currently a stub `Paragraph` — S04 replaces it with `draw_settings`.
  No refactor needed; slot was reserved in S01 and remains correct.
- `App.config: Option<Config>` loaded in S01 — confirmed still present.
- `App.project_root` for config save path — confirmed.
- D097 pattern (pass individual fields, not `&mut App`) applies to `draw_settings` exactly as
  documented.

**S05 contracts confirmed intact:**
- Full navigation graph (Dashboard → MilestoneDetail → ChunkDetail, all Esc chains) is complete.
  S05's help overlay and status bar overlay on top of the established screen dispatch — no
  structural changes required.
- `draw()` match is now a proper per-variant dispatch (not the D096 unconditional-Dashboard +
  conditional-wizard pattern). S05 adds `show_help` overlay after the match, consistent with
  existing `App.show_help: bool` slot from S01.

## Risk Assessment

S03's `risk:medium` (spec browser navigation complexity) is retired:

- Borrow-split: D097/D098 patterns established and proven. Clone-then-mutate in `handle_event`;
  `..` in `draw()` match arms. S04/S05 follow the same patterns with no new exposure.
- Criterion join mismatch: D100 (linear scan, None=Pending for unmatched) works correctly and is
  exercised by `chunk_detail_no_history_all_pending` test.
- No new risks surfaced that require reordering or splitting S04 or S05.

## Requirement Coverage

- R051 (TUI spec browser) — now **validated** by S03. Six spec_browser integration tests prove
  all navigation transitions and criterion display.
- R052 (TUI provider configuration) — **active**, owned by S04. Pre-existing mapping error fixed
  (was incorrectly listed as M006/S03 in REQUIREMENTS.md; corrected to M006/S04).
- All other M006 requirements unchanged.

## Pre-existing Mapping Fix

REQUIREMENTS.md had `R052.primary_owning_slice = M006/S03` — copy-paste error from when the
requirements were authored. S03 owned R051 (spec browser); R052 (provider config) is S04.
Corrected in REQUIREMENTS.md with this assessment.

## Conclusion

Proceed directly to S04 (Provider Configuration Screen). No roadmap edits required.
