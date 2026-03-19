---
estimated_steps: 5
estimated_files: 1
---

# T02: Implement settings merger and hook contract validation tests

**Slice:** S03 — Prompt Builder, Settings Merger & Hook Contracts
**Milestone:** M001

## Description

Implement `merge_settings()` in `assay-harness/src/settings.rs` and add hook contract validation tests. The settings merger uses replace semantics for Vec fields (override wins entirely when non-empty) and Option overlay for scalar fields. Hook contract tests validate that the existing types from S02 are sufficient for S04's Claude Code adapter translation.

## Steps

1. Read `crates/assay-types/src/harness.rs` to confirm `SettingsOverride` fields: `model: Option<String>`, `permissions: Vec<String>`, `tools: Vec<String>`, `max_turns: Option<u32>`
2. Implement `merge_settings(base: &SettingsOverride, overrides: &SettingsOverride) -> SettingsOverride`:
   - Use explicit struct construction (not `..base.clone()`) for compile-time safety when fields are added
   - `model`: override wins when `Some`, else base
   - `max_turns`: override wins when `Some`, else base
   - `permissions`: override replaces base when non-empty, else base
   - `tools`: override replaces base when non-empty, else base
   - Document the replace-not-extend semantics in the doc comment
3. Add `#[cfg(test)]` module for settings merger tests:
   - `empty_overrides` — all-default overrides returns base unchanged
   - `full_override` — all fields overridden
   - `partial_override_model` — only model overridden, rest from base
   - `partial_override_max_turns` — only max_turns overridden
   - `vec_replace_semantics` — non-empty override Vec replaces base Vec entirely
   - `empty_vec_preserves_base` — empty override Vec keeps base Vec
4. Add hook contract validation tests (in same file or `prompt.rs` depending on fit — likely a separate `#[cfg(test)]` section in `settings.rs`):
   - `hook_contract_pre_tool` — construct a PreTool hook with command and timeout, verify fields
   - `hook_contract_post_tool` — construct a PostTool hook, verify serialization round-trip
   - `hook_contract_stop` — construct a Stop hook, verify serialization round-trip
   - `hook_contracts_realistic_profile` — build a `HarnessProfile` with all three hook types, serialize to JSON and deserialize back, assert equality
5. Run `just ready` and fix any issues

## Must-Haves

- [ ] `merge_settings` uses explicit field construction (no `..base`)
- [ ] Vec fields use replace semantics (override wins when non-empty)
- [ ] Option fields use overlay semantics (override wins when Some)
- [ ] Doc comment documents merge semantics
- [ ] Settings merger tests cover all 6 cases
- [ ] Hook contract tests validate all 3 event types with serialization round-trip
- [ ] `just ready` passes

## Verification

- `cargo test -p assay-harness` — all tests pass (prompt + settings + hook)
- `just ready` — full suite passes

## Observability Impact

- Signals added/changed: None — pure functions with no runtime state
- How a future agent inspects this: read test output from `cargo test -p assay-harness -- --nocapture`
- Failure state exposed: None

## Inputs

- `crates/assay-types/src/harness.rs` — `SettingsOverride`, `HookContract`, `HookEvent`, `HarnessProfile` types (locked by S02 schema snapshots)
- `crates/assay-harness/src/settings.rs` — stub with doc comment, ready for implementation
- T01 output — prompt builder implemented (crate compiles)

## Expected Output

- `crates/assay-harness/src/settings.rs` — `merge_settings()` function implemented with 6 settings tests + 4 hook contract tests passing
- `just ready` green
