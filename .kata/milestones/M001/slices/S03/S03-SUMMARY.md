---
id: S03
parent: M001
milestone: M001
provides:
  - build_prompt() function assembling PromptLayer slices into ordered prompt string
  - merge_settings() function with replace/overlay semantics for SettingsOverride
  - Hook contract validation proving HookContract/HookEvent types sufficient for S04
requires:
  - slice: S02
    provides: assay-harness crate structure, HarnessProfile/PromptLayer/SettingsOverride/HookContract/HookEvent types
affects:
  - S04
key_files:
  - crates/assay-harness/src/prompt.rs
  - crates/assay-harness/src/settings.rs
key_decisions:
  - Explicit struct construction in merge_settings (no ..base) for compile-time field coverage safety
patterns_established:
  - Prompt layers formatted as `## {name}\n\n{content}` joined by `\n\n---\n\n`
  - Empty-content layers filtered before assembly
  - Vec fields use replace semantics (non-empty override wins entirely, empty preserves base)
  - Option fields use overlay semantics (Some wins, None falls through to base)
observability_surfaces:
  - none — pure functions with no runtime state
drill_down_paths:
  - .kata/milestones/M001/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M001/slices/S03/tasks/T02-SUMMARY.md
duration: 10m
verification_result: passed
completed_at: 2026-03-16
---

# S03: Prompt Builder, Settings Merger & Hook Contracts

**Prompt builder assembles layered prompts with priority ordering, settings merger combines base + overrides with replace/overlay semantics, hook contracts validated as sufficient for Claude Code adapter.**

## What Happened

T01 implemented `build_prompt(layers: &[PromptLayer]) -> String` in `prompt.rs` — filters empty-content layers, stable-sorts by priority ascending, formats each as `## {name}\n\n{content}`, joins with `\n\n---\n\n`. 7 tests cover empty input, single layer, priority ordering, equal-priority stability, empty-content skipping, negative priorities, and mixed kinds.

T02 implemented `merge_settings(base, overrides) -> SettingsOverride` in `settings.rs` using explicit struct construction (no `..base`) for compile-time safety when new fields are added. Override wins for `model` and `max_turns` when `Some`; override replaces (not extends) `permissions` and `tools` when non-empty; empty Vec preserves base. 6 settings tests + 4 hook contract tests validate serialization round-trips and realistic `HarnessProfile` construction with PreTool/PostTool/Stop hook events.

## Verification

- `cargo test -p assay-harness` — 17/17 tests pass (7 prompt + 6 settings + 4 hook contract)
- `just ready` — all checks pass (fmt, clippy, test, deny) — 934 total tests across workspace

## Requirements Advanced

- R005 (Layered prompt builder) — `build_prompt()` implemented with priority ordering and empty-layer filtering
- R006 (Layered settings merger) — `merge_settings()` implemented with replace/overlay semantics
- R007 (Hook contract definitions) — HookContract/HookEvent types validated by construction and serialization round-trip tests

## Requirements Validated

- R005 — prompt builder proven by 7 unit tests covering all edge cases (empty, single, ordering, stability, filtering, negatives, mixed kinds)
- R006 — settings merger proven by 6 unit tests covering overlay, replace, and preservation semantics
- R007 — hook contracts proven by 4 tests including realistic HarnessProfile with all lifecycle event types, JSON round-trip verification

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

None.

## Known Limitations

- Prompt builder uses a fixed `## {name}\n\n{content}` format — S04 may need to adjust if Claude Code CLAUDE.md has specific formatting requirements
- Settings merger uses replace semantics for Vec fields — if future use cases need merge/extend semantics, this will need a new function or mode parameter

## Follow-ups

- none

## Files Created/Modified

- `crates/assay-harness/src/prompt.rs` — implemented `build_prompt()` with doc comment and 7 inline tests
- `crates/assay-harness/src/settings.rs` — implemented `merge_settings()` with doc comment and 10 tests (6 settings + 4 hook contract)

## Forward Intelligence

### What the next slice should know
- `build_prompt()` returns a plain string — S04's Claude Code adapter just writes this to CLAUDE.md
- `merge_settings()` returns a `SettingsOverride` — S04 needs to translate this to Claude Code's settings format
- Hook contracts are validated as types but S04 needs to translate `HookContract`/`HookEvent` to Claude Code's `hooks.json` format

### What's fragile
- The prompt section separator (`\n\n---\n\n`) is a Markdown horizontal rule — if CLAUDE.md content contains `---` naturally, it could look odd but won't break functionality

### Authoritative diagnostics
- `cargo test -p assay-harness -- --nocapture` — shows individual test results with assertion details
- All functions are pure with no I/O — failures will always be test assertion failures, never runtime errors

### What assumptions changed
- No assumptions changed — the S02 types were exactly what was needed
