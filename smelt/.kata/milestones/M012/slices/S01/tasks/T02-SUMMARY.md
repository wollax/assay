---
id: T02
parent: S01
milestone: M012
provides:
  - All CLI progress messages routed through tracing info! macro
  - All CLI warnings routed through tracing warn! macro
  - All CLI errors routed through tracing error! macro
  - Exactly 2 eprintln! calls remain (main.rs top-level error handler, serve/tui.rs TUI error)
  - Message text preserved exactly for integration test compatibility
key_files:
  - crates/smelt-cli/src/commands/run/phases.rs
  - crates/smelt-cli/src/commands/watch.rs
  - crates/smelt-cli/src/commands/status.rs
  - crates/smelt-cli/src/commands/run/dry_run.rs
  - crates/smelt-cli/src/commands/init.rs
  - crates/smelt-cli/src/commands/list.rs
key_decisions:
  - "Structured tracing fields (container, error) used for warn! calls in teardown paths instead of inline format strings — enables structured filtering"
  - "Validation error messages split into two error! calls (path+context, then detail) to preserve multi-line output behavior"
patterns_established:
  - "Level mapping convention: progress/status → info!, warnings → warn!, errors/failures → error!"
  - "tracing structured fields for contextual warnings: warn!(container = %container, error = %e, \"message\")"
observability_surfaces:
  - "SMELT_LOG=debug shows all CLI events with full metadata (timestamps, levels, targets)"
  - "SMELT_LOG=smelt_cli=trace shows CLI-only events at maximum verbosity"
  - "Error-level events now filterable — SMELT_LOG=error surfaces only errors"
duration: 10min
verification_result: passed
completed_at: 2026-03-27T12:00:00Z
blocker_discovered: false
---

# T02: Migrate all eprintln! calls to tracing macros

**Replaced 50 eprintln! calls across 6 source files with tracing info!/warn!/error! macros, preserving exact message text for integration test compatibility**

## What Happened

Migrated all `eprintln!` calls in the smelt-cli crate to structured `tracing` macros, applying the level mapping defined in S01-RESEARCH.md:
- **phases.rs** (33 calls): Progress messages (provisioning, writing manifest, executing, collecting, PR creation) → `info!`; teardown/cleanup warnings → `warn!` with structured fields; validation/runtime errors → `error!`; timeout/cancelled → `warn!`
- **watch.rs** (10 calls): Missing state/config errors → `error!`; poll failures → `warn!`; poll status updates and terminal states → `info!`
- **status.rs** (3 calls): Missing state file errors → `error!`; stale PID warning → `warn!`
- **dry_run.rs** (2 calls): Validation failure messages → `error!` (matching phases.rs pattern)
- **init.rs** (1 call): File-already-exists error → `tracing::error!`
- **list.rs** (1 call): Corrupt state skip → `tracing::warn!`

The two documented exceptions remain as `eprintln!`:
1. `main.rs` line 95 — top-level error handler (D139 exception, runs before tracing may be initialized)
2. `serve/tui.rs` — TUI error display

Also updated a comment in `main.rs` that mentioned "eprintln! behavior" to "stderr behavior" to avoid false matches in grep verification.

## Verification

- `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` → exactly `main.rs:1` and `serve/tui.rs:1` ✓
- `cargo clippy --workspace -- -D warnings` → 0 warnings ✓
- `cargo test --workspace` → 293 passed, 9 ignored, 0 failures ✓
- `cargo doc --workspace --no-deps` → 0 warnings ✓

## Diagnostics

- Inspect all CLI output: `SMELT_LOG=debug smelt run --dry-run <manifest>` — shows timestamps, levels, targets
- Filter by severity: `SMELT_LOG=error` surfaces only error-level events
- Filter by crate: `SMELT_LOG=smelt_cli=trace` shows CLI-only events at max verbosity
- Default (no env var): bare-message format — identical to previous eprintln! output

## Deviations

- Task plan estimated 51 eprintln! calls but actual count was 50 across the 6 target files (main.rs had 2 in the initial grep: 1 actual call + 1 comment mention). No impact on outcome.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run/phases.rs` — 33 eprintln! → tracing macros (info/warn/error); added `warn` to import
- `crates/smelt-cli/src/commands/watch.rs` — 10 eprintln! → tracing macros; added tracing::{error, info, warn} import
- `crates/smelt-cli/src/commands/status.rs` — 3 eprintln! → tracing macros; added tracing::{error, warn} import
- `crates/smelt-cli/src/commands/run/dry_run.rs` — 2 eprintln! → error! (validation failures)
- `crates/smelt-cli/src/commands/init.rs` — 1 eprintln! → tracing::error!
- `crates/smelt-cli/src/commands/list.rs` — 1 eprintln! → tracing::warn!
- `crates/smelt-cli/src/main.rs` — Updated comment to avoid false grep match
