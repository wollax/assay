---
id: S01
parent: M012
milestone: M012
provides:
  - Bare-message subscriber format (no timestamp/target/level) for default stderr output
  - Full-format subscriber path activated by SMELT_LOG or RUST_LOG env var
  - Target-scoped default filter "smelt_cli=info,smelt_core=info,warn"
  - TUI file appender always uses full format regardless of env var
  - All CLI progress/warning/error messages routed through tracing macros (50 eprintln! calls migrated)
  - Exactly 2 eprintln! calls remain (main.rs top-level error handler, serve/tui.rs TUI error)
  - Flaky test timeout increased from 10s to 30s (R061 resolved)
  - 298 tests pass, 0 failures
requires: []
affects:
  - slice: S02
    provides: "Clean tracing infrastructure — no eprintln!/tracing conflicts during integration"
key_files:
  - crates/smelt-cli/src/main.rs
  - crates/smelt-cli/src/commands/run/phases.rs
  - crates/smelt-cli/src/commands/watch.rs
  - crates/smelt-cli/src/commands/status.rs
  - crates/smelt-cli/src/commands/run/dry_run.rs
  - crates/smelt-cli/src/commands/init.rs
  - crates/smelt-cli/src/commands/list.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - "Three-way subscriber init: TUI file appender (full format), explicit SMELT_LOG/RUST_LOG (full format to stderr), default (bare-message to stderr) — extends D107"
  - "Structured tracing fields for warn! calls in teardown paths: warn!(container = %container, error = %e, \"message\") — enables structured filtering"
  - "Default filter changed from 'warn' to 'smelt_cli=info,smelt_core=info,warn' — info! progress messages visible by default"
patterns_established:
  - "Level mapping: progress/status → info!, warnings → warn!, errors/failures → error!"
  - "Bare-message format (.without_time().with_target(false).with_level(false)) matches prior eprintln! UX exactly"
observability_surfaces:
  - "SMELT_LOG=debug activates full diagnostic format with timestamps/levels/targets on stderr"
  - "SMELT_LOG=smelt_cli=trace shows CLI-only events at maximum verbosity"
  - "Default shows info for smelt_cli/smelt_core, warn for deps — bare message only"
  - "Error-level events now filterable — SMELT_LOG=error surfaces only errors"
drill_down_paths:
  - .kata/milestones/M012/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M012/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M012/slices/S01/tasks/T03-SUMMARY.md
duration: 20min
verification_result: passed
completed_at: 2026-03-27T00:00:00Z
---

# S01: M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix

**Three-way tracing subscriber init, 50 eprintln! calls migrated to structured tracing macros, flaky test timeout fixed — 298 tests pass, R061 and R062 resolved.**

## What Happened

This was a cleanup slice with three tasks executed in sequence.

**T01 — Subscriber init refactor:** The `tracing_subscriber` initialization in `main.rs` was restructured into three distinct paths: (1) TUI mode (`smelt serve` without `--no-tui`) routes full-format output to `.smelt/serve.log` via file appender — unchanged from before; (2) when `SMELT_LOG` or `RUST_LOG` is explicitly set, full-format output (timestamp, target, level) goes to stderr with the user's filter; (3) by default (no env var), bare-message format (`.without_time().with_target(false).with_level(false)`) goes to stderr with target-scoped filter `"smelt_cli=info,smelt_core=info,warn"`. The default filter changed from `"warn"` to surface `info!` events from smelt crates without exposing dependency noise.

**T02 — eprintln! migration:** All 50 `eprintln!` calls across 6 source files were replaced with structured tracing macros applying the level mapping: progress/status → `info!`, warnings → `warn!`, errors/failures → `error!`. Teardown-path warnings use structured fields (`warn!(container = %container, error = %e, "message")`) to enable structured filtering. Exact message text was preserved for integration test compatibility. Two documented exceptions remain: `main.rs` line 95 (top-level error handler that runs before tracing may be initialized, D139) and `serve/tui.rs` (TUI error display).

**T03 — Flaky test fix:** Changed `Duration::from_secs(10)` to `Duration::from_secs(30)` in `test_cli_run_invalid_manifest` in `docker_lifecycle.rs`. The 10s timeout was too short during cold or incremental builds when cargo needs to link the binary. Full slice verification sweep confirmed all criteria met.

## Verification

All 5 verification checks passed:

| Check | Result |
|-------|--------|
| `rg 'from_secs(10)' crates/smelt-cli/tests/docker_lifecycle.rs` | 0 results ✓ |
| `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` | main.rs:1, serve/tui.rs:1 ✓ |
| `cargo clippy --workspace -- -D warnings` | 0 warnings ✓ |
| `cargo doc --workspace --no-deps` | 0 warnings ✓ |
| `cargo test --workspace` | 298 passed, 0 failed ✓ |

## Requirements Advanced

- R061 — Flaky test timeout increased from 10s to 30s; test no longer flakes on cold builds
- R062 — All eprintln! calls in smelt-cli replaced with structured tracing events; output remains clean at default level

## Requirements Validated

- R061 — Proven: `rg 'from_secs(10)' crates/smelt-cli/tests/docker_lifecycle.rs` returns 0 results; `cargo test --workspace` passes 298 tests
- R062 — Proven: `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` returns exactly `main.rs:1` and `serve/tui.rs:1`; integration test assertions on stderr substrings still pass; `SMELT_LOG=debug` activates full-format diagnostic output

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- Task plan estimated 51 eprintln! calls but actual count was 50 across the 6 target files (main.rs had 2 initial grep hits: 1 actual call + 1 comment mention of "eprintln! behavior"). The comment was updated to "stderr behavior" to avoid false grep matches. No impact on outcome.

## Known Limitations

- The bare-message format matches prior `eprintln!` UX exactly but loses context (no source file, no span) at the default log level. Operators who want richer output must set `SMELT_LOG=debug`. This is by design (D139).
- No new Docker tests were added — the existing integration tests exercise the subscriber initialization sufficiently.

## Follow-ups

- none (cleanup slice; all S01 work feeds clean infrastructure into S02)

## Files Created/Modified

- `crates/smelt-cli/src/main.rs` — Refactored subscriber init with three-way branching and new default filter; comment updated to avoid false grep match
- `crates/smelt-cli/src/commands/run/phases.rs` — 33 eprintln! → tracing macros (info/warn/error); added `warn` to import
- `crates/smelt-cli/src/commands/watch.rs` — 10 eprintln! → tracing macros; added tracing::{error, info, warn} import
- `crates/smelt-cli/src/commands/status.rs` — 3 eprintln! → tracing macros; added tracing::{error, warn} import
- `crates/smelt-cli/src/commands/run/dry_run.rs` — 2 eprintln! → error! (validation failures)
- `crates/smelt-cli/src/commands/init.rs` — 1 eprintln! → tracing::error!
- `crates/smelt-cli/src/commands/list.rs` — 1 eprintln! → tracing::warn!
- `crates/smelt-cli/tests/docker_lifecycle.rs` — Changed subprocess timeout from 10s to 30s

## Forward Intelligence

### What the next slice should know
- The default filter `"smelt_cli=info,smelt_core=info,warn"` means any new `info!` events in smelt_cli or smelt_core crates are visible to users by default — choose log levels carefully in S02+ code.
- Integration test assertions check for stderr substrings like "Writing manifest...", "Executing assay run...", "Assay complete", "Container removed". These exact strings are now emitted via `info!` macros — any message text change will break integration tests.
- The two remaining `eprintln!` calls are intentional exceptions — don't migrate them.

### What's fragile
- Bare-message format depends on exact tracing-subscriber version behavior. If `.without_time().with_target(false).with_level(false)` changes behavior in a future version, user-facing output will regress silently.
- The three-way branch in `main.rs` is order-sensitive: TUI branch must be checked first (before env var detection) because TUI always uses file appender.

### Authoritative diagnostics
- `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` — must always return exactly 2 lines
- `rg 'from_secs\(10\)' crates/smelt-cli/tests/docker_lifecycle.rs` — must return 0 results
- `SMELT_LOG=debug smelt run --dry-run <manifest>` — inspect full-format output with timestamps/levels/targets

### What assumptions changed
- Originally assumed 51 eprintln! calls; actual was 50 (one was a comment). Minor, no impact.
