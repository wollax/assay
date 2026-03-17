---
id: T01
parent: S03
milestone: M001
provides:
  - build_prompt() function assembling PromptLayer slices into ordered prompt string
key_files:
  - crates/assay-harness/src/prompt.rs
key_decisions: []
patterns_established:
  - Prompt layers formatted as `## {name}\n\n{content}` joined by `\n\n---\n\n`
  - Empty-content layers filtered before assembly
observability_surfaces:
  - none — pure function with no runtime state
duration: 5m
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T01: Implement prompt builder with tests

**Implemented `build_prompt()` — stable-sorts `PromptLayer` slices by priority ascending and assembles them into a sectioned prompt string.**

## What Happened

Implemented `build_prompt(layers: &[PromptLayer]) -> String` in `crates/assay-harness/src/prompt.rs`. The function filters out empty-content layers, stable-sorts by priority (ascending), formats each as `## {name}\n\n{content}`, and joins with `\n\n---\n\n`. Added 7 inline tests covering all required scenarios.

## Verification

- `cargo test -p assay-harness -- prompt` — 7/7 tests pass (empty_layers, single_layer, priority_ordering, equal_priority_stability, empty_content_skipped, negative_priority, mixed_kinds)
- `cargo clippy -p assay-harness` — no warnings
- Slice-level: `cargo test -p assay-harness` passes for prompt tests; settings/hook tests not yet implemented (T02)

## Diagnostics

- `cargo test -p assay-harness -- prompt --nocapture` shows individual test results
- No runtime state or failure surfaces — pure function

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-harness/src/prompt.rs` — implemented `build_prompt()` with doc comment and 7 inline tests
