# S02 Post-Slice Roadmap Assessment

**Date:** 2026-03-20
**Verdict:** Roadmap is still good — no changes needed.

## What S02 Actually Delivered

S02 retired its designated risk (wizard form state machine) with 5 integration tests proving
the round-trip through real filesystem writes. It also delivered the full `App` struct + `Screen`
enum + event loop that S01 had scoped but never implemented (D-S02-6). The interfaces match
the S01 boundary map spec exactly — they just arrived one slice late.

## Boundary Map Accuracy

S03 and S04 both "Consume from S01" per the boundary map. All those contracts are now in
place — `App.screen`, `App.project_root`, `App.config`, navigation event dispatch, `Screen`
enum with extensible variants. The fact that S02 delivered them instead of S01 is a sequencing
detail, not a contract change. Remaining slices can proceed against the same interfaces.

## Success Criteria Coverage

| Criterion | Remaining owner(s) |
|-----------|-------------------|
| Dashboard with milestones, status badges, chunk progress fractions, gate counts | S03 (history loading), S05 (polish) |
| Navigate milestone → chunks → criteria + latest gate result | S03 |
| `n` opens wizard, files written, dashboard refreshes immediately | **Delivered by S02** |
| `s` opens settings, persists to config.toml, survives restart | S04 |
| No `.assay/` shows clean message, no panic | **Delivered by S02** (Screen::NoProject) |
| `just ready` passes, binary `assay-tui` produced | S05 |

All six success criteria have at least one remaining owning slice or are already delivered.
Coverage check passes.

## Requirement Coverage

- R049 (TUI dashboard): partially validated by S02's scaffold; completion via S03+S05
- R050 (TUI interactive wizard): **validated by S02**
- R051 (TUI spec browser): owned by S03 — unchanged
- R052 (TUI provider configuration): owned by S04 — unchanged

Requirement coverage is sound. No ownership changes needed in REQUIREMENTS.md.

## Remaining Slice Order

S03 → S04 → S05 remains correct. No risks emerged that would justify reordering.
S03 and S04 are independent (both depend only on S01/S02 foundation); either can go first.

---

## Deferred Backlog (from second PR review pass)

Minor cleanup items deferred from the second review cycle. Address before M006 closes.

- `#[allow(dead_code)]` on `WizardAction` — remove; all variants used; add `#[derive(Debug)]`
- `cursor` field — remove or document as reserved; it tracks `current_line.len()` only and is never read by the renderer
- `step_prompt` unused `chunk_count` — rename to `_chunk_count` or remove the parameter
- Double-reverse `.rev().skip(1).rev()` in `draw_wizard` — replace with `&fields[..len - 1]`
- Duplicate keyboard-hint string — extract as `const KEY_HINT`
- `assemble_inputs` verbose description — collapse to `.filter(|s| !s.is_empty()).cloned()`
- `assert!(state.step >= 6)` in test — change to `assert_eq!(state.step, 6)`
- Unreachable `WizardAction::Submit(_) => unreachable!()` in test match arm — use `_ => panic!(…)`
- `total_steps` placeholder 7 — add comment explaining why 7; or render `"?"` for steps 0–2
