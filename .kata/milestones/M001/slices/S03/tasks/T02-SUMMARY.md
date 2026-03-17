---
id: T02
parent: S03
milestone: M001
provides:
  - merge_settings() function with replace semantics for Vec and overlay for Option fields
  - Hook contract validation tests proving HookContract/HookEvent types sufficient for S04
key_files:
  - crates/assay-harness/src/settings.rs
key_decisions:
  - Explicit struct construction in merge_settings (no ..base) for compile-time field coverage
patterns_established:
  - Vec fields use replace semantics (non-empty override wins entirely, empty preserves base)
  - Option fields use overlay semantics (Some wins, None falls through to base)
observability_surfaces:
  - none — pure functions with no runtime state
duration: 5m
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T02: Implement settings merger and hook contract validation tests

**Implemented `merge_settings()` with replace/overlay semantics and validated hook contracts via serialization round-trips.**

## What Happened

Implemented `merge_settings(base, overrides) -> SettingsOverride` in `settings.rs` using explicit struct construction for compile-time safety. Added 6 settings merger tests covering: empty overrides, full override, partial model override, partial max_turns override, Vec replace semantics, and empty Vec preservation. Added 4 hook contract tests: PreTool field verification, PostTool serialization round-trip, Stop serialization round-trip, and a realistic `HarnessProfile` with all three hook types round-tripping through JSON.

## Verification

- `cargo test -p assay-harness` — 17 tests pass (7 prompt + 6 settings + 4 hook)
- `just ready` — all checks pass (fmt, clippy, test, deny)

## Diagnostics

- `cargo test -p assay-harness -- settings --nocapture` shows individual settings/hook test results
- No runtime state or failure surfaces — pure functions

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-harness/src/settings.rs` — implemented `merge_settings()` and added 10 tests (6 settings merger + 4 hook contract)
