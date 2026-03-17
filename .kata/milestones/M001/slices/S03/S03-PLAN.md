# S03: Prompt Builder, Settings Merger & Hook Contracts

**Goal:** Prompt builder assembles layered prompts from `PromptLayer` slices, settings merger combines base + override `SettingsOverride` values, and hook contract types are validated as sufficient for S04's Claude Code adapter.
**Demo:** `cargo test -p assay-harness` passes with tests covering priority ordering, empty-layer handling, settings override semantics, and realistic hook contract construction.

## Must-Haves

- `build_prompt(layers: &[PromptLayer]) -> String` assembles layers sorted by priority (lowest first), skips empty content, uses stable sort for deterministic output at equal priorities
- `merge_settings(base: &SettingsOverride, overrides: &SettingsOverride) -> SettingsOverride` overlays non-None/non-empty override fields onto base with replace semantics for Vec fields
- Hook contract types (`HookContract`, `HookEvent`) validated by tests constructing realistic Claude Code lifecycle configurations
- All public functions have doc comments (`#![deny(missing_docs)]` enforced)
- `just ready` passes

## Proof Level

- This slice proves: contract
- Real runtime required: no
- Human/UAT required: no

## Verification

- `cargo test -p assay-harness` — all prompt builder, settings merger, and hook contract tests pass
- `just ready` — full suite passes (fmt, clippy, test, deny)

## Observability / Diagnostics

- Runtime signals: none (pure functions, no I/O or state)
- Inspection surfaces: test output from `cargo test -p assay-harness -- --nocapture` shows individual test results
- Failure visibility: compile errors for missing docs; test assertion messages describe expected vs actual values
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `assay-types::harness` — `PromptLayer`, `PromptLayerKind`, `SettingsOverride`, `HookContract`, `HookEvent` (locked by schema snapshots from S02)
- New wiring introduced in this slice: `build_prompt()` and `merge_settings()` free functions in `assay-harness` — consumed by S04's Claude Code adapter
- What remains before the milestone is truly usable end-to-end: S04 (adapter generates files from profile), S05 (worktree enhancements), S06 (manifest parsing), S07 (pipeline assembly)

## Tasks

- [x] **T01: Implement prompt builder with tests** `est:20m`
  - Why: Delivers R005 — the prompt builder is required by S04 to assemble CLAUDE.md content from layered prompt sources
  - Files: `crates/assay-harness/src/prompt.rs`
  - Do: Implement `build_prompt(layers: &[PromptLayer]) -> String` — stable sort by priority (lowest first), skip empty-content layers, separate sections with `\n\n---\n\n`, format each layer as `## {name}\n\n{content}`. Add inline `#[cfg(test)]` module with tests: empty input, single layer, priority ordering, equal-priority stability, empty-content skipping, negative priorities, mixed kinds.
  - Verify: `cargo test -p assay-harness -- prompt` passes all tests
  - Done when: `build_prompt` is implemented, documented, and all prompt tests pass

- [x] **T02: Implement settings merger and hook contract validation tests** `est:20m`
  - Why: Delivers R006 (settings merger) and R007 (hook contract validation) — both required by S04's Claude Code adapter
  - Files: `crates/assay-harness/src/settings.rs`
  - Do: Implement `merge_settings(base: &SettingsOverride, overrides: &SettingsOverride) -> SettingsOverride` — use explicit field construction (not `..base`) for compile-time safety on new fields; override wins for `model` and `max_turns` when `Some`; override replaces (not extends) `permissions` and `tools` when non-empty. Add inline `#[cfg(test)]` module with tests: base-only (empty overrides), full override, partial override, Vec replace semantics, empty-Vec-does-not-clear-base. Add hook contract tests in same or separate test module: construct realistic `HookContract` values for PreTool/PostTool/Stop matching Claude Code lifecycle events, verify serialization round-trip, verify `HookEvent` coverage is sufficient for single-agent scope. Run `just ready`.
  - Verify: `cargo test -p assay-harness` passes all tests; `just ready` passes
  - Done when: `merge_settings` is implemented, hook contracts validated by tests, `just ready` green

## Files Likely Touched

- `crates/assay-harness/src/prompt.rs`
- `crates/assay-harness/src/settings.rs`
