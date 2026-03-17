# S03: Prompt Builder, Settings Merger & Hook Contracts — Research

**Date:** 2026-03-16

## Summary

S03 implements three pure-function modules in `assay-harness` (prompt builder, settings merger) and validates the hook contract types already defined in `assay-types`. All three requirements (R005, R006, R007) are implementation work against types that are already locked by schema snapshots from S02. The risk is low — this is pure logic with no I/O, no process management, no external dependencies.

The existing `build_evaluator_prompt()` in `assay-core/src/evaluator.rs` provides a strong pattern for prompt assembly (section-based string building), but the harness prompt builder has a different contract: it assembles `PromptLayer` structs ordered by priority into a single string, not a fixed-format evaluator prompt. The settings merger is a straightforward Option-overlay pattern. Hook contracts are already fully defined in `assay-types/src/harness.rs` — S03 only needs to verify the types are sufficient for S04's Claude Code adapter translation.

The primary open question — whether Claude Code's hooks.json format matches our `HookEvent` variants — could not be verified via web research (network unavailable). Based on training knowledge: Claude Code's `.claude/settings.json` supports a `hooks` object with keys like `PreToolUse`, `PostToolUse`, and `Stop`, each mapping to an array of hook definitions with `command` and optional `timeout` fields. Our `HookEvent` enum (`PreTool`, `PostTool`, `Stop`) maps cleanly to these with a simple name translation in S04's adapter. The `Notification` and `SubagentStop` events exist in Claude Code but are not needed for M001's single-agent scope.

## Recommendation

Implement three focused modules in `assay-harness`:

1. **`prompt.rs`**: `build_prompt(layers: &[PromptLayer]) -> String` — sort by priority, concatenate with section separators. Pure function, no side effects.
2. **`settings.rs`**: `merge_settings(base: &SettingsOverride, overrides: &SettingsOverride) -> SettingsOverride` — overlay non-None/non-empty fields from overrides onto base. Pure function.
3. **Hook contracts**: Already defined in `assay-types`. S03 validates they're sufficient by writing tests that construct realistic hook configurations matching Claude Code's actual lifecycle events. No new types needed unless testing reveals a gap.

All tests should be inline `#[cfg(test)]` modules following the codebase convention.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| n/a | n/a | All work is pure Rust logic with no external dependencies needed |

## Existing Code and Patterns

- `crates/assay-core/src/evaluator.rs:87-165` — `build_evaluator_prompt()` and `build_system_prompt()` show the section-based prompt assembly pattern; harness prompt builder follows similar structure but consumes `PromptLayer` types instead of raw strings
- `crates/assay-types/src/harness.rs` — All 6 types (`HarnessProfile`, `PromptLayer`, `PromptLayerKind`, `SettingsOverride`, `HookContract`, `HookEvent`) are defined with full derives, `deny_unknown_fields`, and schema snapshots locked
- `crates/assay-types/src/feature_spec.rs` — `FeatureSpec` shows how spec content is structured; the prompt builder may need to extract requirements/criteria text from specs to build `Spec`-kind prompt layers
- `crates/assay-harness/src/prompt.rs` — Stub with doc comment, ready for implementation
- `crates/assay-harness/src/settings.rs` — Stub with doc comment, ready for implementation
- `crates/assay-harness/Cargo.toml` — Already depends on `assay-core`, `assay-types`, `serde`, `serde_json`
- Inline `#[cfg(test)]` modules are the standard test pattern across the codebase (no separate `tests/` directories in core crates)

## Constraints

- **Zero new dependencies**: `assay-harness` already has `assay-core`, `assay-types`, `serde`, `serde_json` — these are sufficient for pure logic
- **Schema snapshots are locked**: Any change to `PromptLayer`, `SettingsOverride`, `HookContract`, or `HookEvent` will break snapshot tests. S03 should not modify these types unless a real gap is found during implementation
- **`deny_unknown_fields` on all structs**: Settings merger must produce valid `SettingsOverride` structs, not arbitrary JSON
- **`#![deny(missing_docs)]`** on `assay-harness`: Every public function and type needs doc comments
- **Zero-trait convention**: No trait objects; prompt builder and settings merger are free functions, not trait implementations
- **`PromptLayer.priority` is `i32`**: Lower values assemble first (documented in type). Negative priorities are valid for "always first" layers

## Common Pitfalls

- **Unstable sort for equal priorities** — Use `sort_by_key` which is stable in Rust, or explicitly define tiebreaking (e.g., by insertion order or by `PromptLayerKind` discriminant). Two layers with the same priority should produce deterministic output.
- **Empty content layers** — The prompt builder should handle layers with empty `content` gracefully (skip them or include as empty sections). Don't blindly concatenate.
- **Settings merger field-by-field oversight** — `SettingsOverride` has 4 fields (`model`, `permissions`, `tools`, `max_turns`). The merger must handle each explicitly. If a field is added later, a non-exhaustive match will silently miss it. Use struct construction (not `..base`) to get compile-time safety on new fields.
- **Vec merge semantics** — `permissions` and `tools` are `Vec<String>`. Decide: does override *replace* or *extend* the base list? Recommendation: replace (override wins entirely), which is simpler and matches Claude Code's settings override behavior. Document this clearly.

## Open Risks

- **Claude Code hook event coverage**: Our `HookEvent` enum has 3 variants (`PreTool`, `PostTool`, `Stop`). Claude Code may support additional events (`Notification`, `SubagentStop`, `PreToolUse` vs `PostToolUse` naming). S03 defines the Assay-side contract; S04 handles the translation. If Claude Code's format diverges significantly, S04 may need to extend `HookEvent` (which would require snapshot updates).
- **Network was unavailable for Claude Code docs verification**: The hook format mapping is based on training knowledge, not live documentation. S04 should verify against actual Claude Code docs or runtime behavior before committing the adapter translation.
- **Prompt layer separator format**: The choice of separator between assembled layers (e.g., `\n\n---\n\n` vs `\n\n`) affects how Claude Code interprets the system prompt. S04 may need to adjust the separator based on testing.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | `apollographql/skills@rust-best-practices` (2.5K installs) | available — not directly relevant (general Rust, not specific to this domain) |
| Rust | `jeffallan/claude-skills@rust-engineer` (1.2K installs) | available — not directly relevant |

No skills are needed for this slice. The work is pure Rust logic with types already defined.

## Sources

- `crates/assay-types/src/harness.rs` — all 6 harness types (source of truth for S03's input contract)
- `crates/assay-core/src/evaluator.rs:87-165` — existing prompt assembly pattern
- `crates/assay-types/tests/snapshots/schema_snapshots__hook-contract-schema.snap` — locked hook contract schema
- `crates/assay-types/tests/snapshots/schema_snapshots__hook-event-schema.snap` — locked hook event schema (pre-tool, post-tool, stop)
- Claude Code hooks documentation (from training knowledge, not verified live): hooks.json uses `PreToolUse`, `PostToolUse`, `Stop` as event keys with `{command, timeout}` definitions
