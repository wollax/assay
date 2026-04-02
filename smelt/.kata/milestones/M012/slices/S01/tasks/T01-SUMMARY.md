---
id: T01
parent: S01
milestone: M012
provides:
  - Bare-message subscriber format (no timestamp/target/level) for default stderr output
  - Full-format subscriber path activated by SMELT_LOG or RUST_LOG env var
  - Target-scoped default filter "smelt_cli=info,smelt_core=info,warn"
  - TUI file appender always uses full format regardless of env var
key_files:
  - crates/smelt-cli/src/main.rs
key_decisions:
  - "Env var detection uses std::env::var().is_ok() before building filter — separates format selection from filter construction"
patterns_established:
  - "Three-way subscriber init: TUI file appender (full format), explicit env (full format to stderr), default (bare format to stderr)"
observability_surfaces:
  - "SMELT_LOG=debug activates full diagnostic format with timestamps/levels/targets on stderr"
  - "Default path shows info for smelt_cli/smelt_core, warn for deps — bare message only"
duration: 5min
verification_result: passed
completed_at: 2026-03-27T00:00:00Z
blocker_discovered: false
---

# T01: Configure tracing subscriber format and default filter

**Three-way subscriber init: bare-message default, full-format when SMELT_LOG/RUST_LOG set, full-format for TUI file appender**

## What Happened

Refactored the `tracing_subscriber` initialization in `main.rs` to branch on whether the user explicitly set `SMELT_LOG` or `RUST_LOG`. The init block now has three paths:

1. **TUI file appender** (`smelt serve` without `--no-tui`): full format to `.smelt/serve.log` — unchanged from before.
2. **Explicit env var set**: full format (timestamp, target, level) to stderr with user-provided filter.
3. **No env var (default)**: bare-message format (`.without_time().with_target(false).with_level(false)`) to stderr with default filter `"smelt_cli=info,smelt_core=info,warn"`.

The default filter changed from `"warn"` to `"smelt_cli=info,smelt_core=info,warn"`, so `info!` events from smelt crates are now visible by default while dependency noise stays suppressed.

## Verification

- `cargo clippy --workspace -- -D warnings` — 0 warnings
- `cargo test --workspace` — all tests pass (87 + 3 + 23 + 16 + 5 + 161 + 5 + 3 = 298+ tests, 0 failures)
- `cargo doc --workspace --no-deps` — clean
- Read `main.rs` lines 38-82: confirmed bare format path uses `.without_time().with_target(false).with_level(false)`, default filter is `"smelt_cli=info,smelt_core=info,warn"`

### Slice-level verification (partial — T01 of 4):
- ✅ `cargo test --workspace` — passes
- ✅ `cargo clippy --workspace -- -D warnings` — clean
- ✅ `cargo doc --workspace --no-deps` — clean
- ⏳ `eprintln!` count — still many remaining (T02/T03 scope)
- ⏳ `from_secs(10)` timeout — T04 scope

## Diagnostics

- Inspect subscriber init: read `crates/smelt-cli/src/main.rs` lines 38-82
- Test bare format: run `smelt run --dry-run` on a valid manifest — should see clean messages without timestamps/levels
- Test full format: run `SMELT_LOG=debug smelt run --dry-run` — should see timestamps, levels, and targets

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/main.rs` — Refactored subscriber init with three-way branching and new default filter
