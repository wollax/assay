---
estimated_steps: 4
estimated_files: 1
---

# T01: Implement prompt builder with tests

**Slice:** S03 — Prompt Builder, Settings Merger & Hook Contracts
**Milestone:** M001

## Description

Implement `build_prompt()` in `assay-harness/src/prompt.rs` — a pure function that assembles `PromptLayer` values into a single string ordered by priority. This is the first half of S03's contract and the input surface for S04's CLAUDE.md generation.

## Steps

1. Read `crates/assay-types/src/harness.rs` to confirm `PromptLayer` fields: `kind`, `name`, `content`, `priority`
2. Implement `build_prompt(layers: &[PromptLayer]) -> String`:
   - Clone and stable-sort layers by `priority` (ascending — lowest first)
   - Filter out layers with empty `content` (after trimming)
   - Format each layer as `## {name}\n\n{content}`
   - Join sections with `\n\n---\n\n`
   - Return the assembled string (empty string if no layers survive filtering)
3. Add `#[cfg(test)]` module with tests:
   - `empty_layers` — empty slice returns empty string
   - `single_layer` — single layer produces `## name\n\ncontent`
   - `priority_ordering` — layers sort by priority ascending
   - `equal_priority_stability` — same-priority layers preserve insertion order
   - `empty_content_skipped` — layers with empty/whitespace content are excluded
   - `negative_priority` — negative priorities sort before positive
   - `mixed_kinds` — different `PromptLayerKind` values don't affect ordering (only priority does)
4. Run `cargo test -p assay-harness -- prompt` and fix any failures

## Must-Haves

- [ ] `build_prompt` sorts by priority ascending with stable sort
- [ ] Empty-content layers are skipped
- [ ] Each layer formatted as `## {name}\n\n{content}` with `\n\n---\n\n` separator
- [ ] Doc comment on public function
- [ ] All 7 tests pass

## Verification

- `cargo test -p assay-harness -- prompt` — all tests pass
- `cargo clippy -p assay-harness` — no warnings

## Observability Impact

- Signals added/changed: None — pure function with no runtime state
- How a future agent inspects this: read test output from `cargo test -p assay-harness -- prompt --nocapture`
- Failure state exposed: None

## Inputs

- `crates/assay-types/src/harness.rs` — `PromptLayer`, `PromptLayerKind` types (locked by S02 schema snapshots)
- `crates/assay-harness/src/prompt.rs` — stub with doc comment, ready for implementation

## Expected Output

- `crates/assay-harness/src/prompt.rs` — `build_prompt()` function implemented with 7+ inline tests passing
