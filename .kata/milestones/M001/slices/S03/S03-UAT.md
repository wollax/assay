# S03: Prompt Builder, Settings Merger & Hook Contracts — UAT

**Milestone:** M001
**Written:** 2026-03-16

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All deliverables are pure functions with no I/O, no runtime state, and no side effects. Correctness is fully provable by unit tests — no live runtime or human experience verification needed.

## Preconditions

- Rust toolchain installed
- Working directory is the assay repo root
- `just ready` passes (confirms all workspace dependencies resolve)

## Smoke Test

Run `cargo test -p assay-harness` — all 17 tests pass, confirming prompt builder, settings merger, and hook contracts are functional.

## Test Cases

### 1. Prompt builder assembles layers in priority order

1. Run `cargo test -p assay-harness -- prompt::tests::priority_ordering`
2. **Expected:** Test passes — layers with priority 10, 0, 5 are assembled as priority 0 → 5 → 10

### 2. Prompt builder skips empty-content layers

1. Run `cargo test -p assay-harness -- prompt::tests::empty_content_skipped`
2. **Expected:** Test passes — layers with empty or whitespace-only content are excluded from output

### 3. Settings merger overlays Option fields

1. Run `cargo test -p assay-harness -- settings::tests::partial_override_model`
2. **Expected:** Test passes — override's `model: Some("fast")` wins over base's `model: Some("default")`

### 4. Settings merger uses replace semantics for Vec fields

1. Run `cargo test -p assay-harness -- settings::tests::vec_replace_semantics`
2. **Expected:** Test passes — non-empty override Vec replaces base Vec entirely (not appended)

### 5. Hook contracts support Claude Code lifecycle events

1. Run `cargo test -p assay-harness -- settings::tests::hook_contracts_realistic_profile`
2. **Expected:** Test passes — a `HarnessProfile` with PreTool, PostTool, and Stop hooks round-trips through JSON serialization

## Edge Cases

### Empty overrides preserve base entirely

1. Run `cargo test -p assay-harness -- settings::tests::empty_overrides`
2. **Expected:** Test passes — when all override fields are None/empty, base values are preserved unchanged

### Empty Vec does not clear base

1. Run `cargo test -p assay-harness -- settings::tests::empty_vec_preserves_base`
2. **Expected:** Test passes — an empty `permissions: []` override does not erase the base's non-empty permissions

### Equal-priority layers maintain insertion order

1. Run `cargo test -p assay-harness -- prompt::tests::equal_priority_stability`
2. **Expected:** Test passes — layers with equal priority appear in their original order (stable sort)

## Failure Signals

- Any test failure in `cargo test -p assay-harness` indicates a regression in prompt builder, settings merger, or hook contract types
- `cargo clippy -p assay-harness` warnings indicate missing doc comments or code quality issues
- Schema snapshot test failures in `assay-types` would indicate the upstream types changed unexpectedly

## Requirements Proved By This UAT

- R005 (Layered prompt builder) — `build_prompt()` assembles layers by priority with empty filtering, proven by 7 tests
- R006 (Layered settings merger) — `merge_settings()` combines base + overrides with correct precedence, proven by 6 tests
- R007 (Hook contract definitions) — HookContract/HookEvent types are sufficient for PreTool/PostTool/Stop lifecycle events, proven by 4 construction and round-trip tests

## Not Proven By This UAT

- Runtime integration with Claude Code's actual hooks.json format — deferred to S04
- Whether the prompt format is optimal for Claude Code's CLAUDE.md consumption — deferred to S04
- Settings merger translation to Claude Code's native settings format — deferred to S04

## Notes for Tester

All verification is automated via `cargo test`. No manual setup, external services, or browser interaction needed. The `just ready` command runs the full verification suite including fmt, clippy, all tests, and dependency audit.
