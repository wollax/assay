---
id: T03
parent: S01
milestone: M009
provides:
  - All 9 eprintln! calls in assay-core migrated to tracing::warn! with structured fields
  - Structured fields (path, error, milestone_slug) on all history/analytics warnings
  - Zero eprintln! remaining in crates/assay-core/src/
key_files:
  - crates/assay-core/src/history/analytics.rs
  - crates/assay-core/src/history/mod.rs
key_decisions:
  - "All assay-core eprintln calls mapped to warn! level — they are all skip/load warnings, none are fatal errors"
patterns_established:
  - "Structured tracing fields for file paths use `path = %path.display()` pattern; errors use `error = %e`"
observability_surfaces:
  - "RUST_LOG=assay_core::history=warn shows all history skip/load warnings with structured fields"
duration: 5min
verification_result: passed
completed_at: 2026-03-24T00:00:00Z
blocker_discovered: false
---

# T03: Migrate assay-core eprintln calls to tracing macros

**Migrated all 9 eprintln! calls in assay-core history/analytics modules to tracing::warn! with structured fields (path, error, milestone_slug)**

## What Happened

Replaced all 9 `eprintln!` calls across two files in assay-core:

- `history/analytics.rs` (7 calls): Results entry skip warnings, spec dir read failures, unreadable history records (read and parse), milestones dir read failure, and corrupt milestone timestamp warning. All mapped to `tracing::warn!` with structured fields like `path`, `error`, and `milestone_slug`.
- `history/mod.rs` (2 calls): History prune failure and history entry skip warning. Both mapped to `tracing::warn!` with `path`/`error` fields.

Added `use tracing::warn;` import to both files. The `tracing` crate was already a dependency of assay-core.

## Verification

- `grep -rn 'eprintln!' crates/assay-core/src/ --include='*.rs'` — zero matches ✓
- `cargo test -p assay-core --lib` — 690 passed, 0 failed ✓
- No tests assert on stderr content from these code paths, so no regressions.

### Slice-level verification (partial — T03 is intermediate):
- `grep -rn 'eprintln!' crates/assay-core/src/` — zero matches ✓ (assay-core clean)
- `grep -rn 'eprintln!' crates/assay-cli/src/ crates/assay-tui/src/ crates/assay-mcp/src/` — still has matches (T04/T05 scope)

## Diagnostics

`RUST_LOG=assay_core::history=warn` shows all history-related skip/load warnings with structured fields. Individual fields are filterable (e.g. by path or milestone_slug).

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/history/analytics.rs` — Replaced 7 eprintln! with tracing::warn!, added `use tracing::warn`
- `crates/assay-core/src/history/mod.rs` — Replaced 2 eprintln! with tracing::warn!, added `use tracing::warn`
