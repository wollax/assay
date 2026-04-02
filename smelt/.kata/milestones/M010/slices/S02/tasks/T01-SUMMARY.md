---
id: T01
parent: S02
milestone: M010
provides:
  - warn_teardown() async helper replacing 6 duplicated teardown blocks
  - .context() error chain preservation on all monitor.write()/set_phase() calls
  - Visible stderr warnings on teardown failures (previously silent)
key_files:
  - crates/smelt-cli/src/commands/run/phases.rs
key_decisions:
  - "warn_teardown prints 'Container removed.' only on teardown success — previously printed unconditionally after silent discard"
  - "Kept let _ = on outcome match block (GatesFailed/Complete/Failed/Timeout/Cancelled) — those are best-effort phase transitions, not teardown error paths"
patterns_established:
  - "warn_teardown(monitor, provider, container) pattern for early-return teardown in phases.rs"
observability_surfaces:
  - "eprintln!(Warning: ...) on teardown failures — provider.teardown, monitor.set_phase(TearingDown), monitor.cleanup"
  - ".context() on monitor.write()/set_phase() preserves SmeltError chain through anyhow"
duration: 10min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T01: Extract warn_teardown helper and replace silent let _ = in phases.rs

**Extracted `warn_teardown()` async helper consolidating 6 duplicated teardown blocks; replaced 5 `anyhow!("{e}")` with `.context()` preserving error chains**

## What Happened

Added a free `async fn warn_teardown()` at module level in `phases.rs` that handles the teardown sequence (set TearingDown phase → provider.teardown → monitor.cleanup) with `eprintln!` warnings on each failure. Replaced all 6 early-return teardown blocks (assay config write error/non-zero, specs dir error/non-zero, spec file loop, manifest write) with calls to this helper. Each callsite still sets `JobPhase::Failed` before calling the helper since that's a separate concern.

Replaced 5 `.map_err(|e| anyhow::anyhow!("{e}"))` calls on `monitor.write()` and `monitor.set_phase()` with `.context("descriptive message")`, preserving the original `SmeltError` chain through anyhow instead of stringifying it.

The final "always runs" teardown block at the end of the function was already properly logging warnings and was left unchanged — it has special logic to return an error if teardown fails on an otherwise successful run.

## Verification

- `cargo check --workspace` — compiles clean
- `cargo clippy --workspace` — no warnings
- `cargo test --workspace` — 155+ tests pass, 0 failures
- `rg 'let _ = provider\.teardown' phases.rs` — 0 matches ✓
- `rg 'anyhow!.*\{e\}' phases.rs` — 0 matches ✓
- `rg 'warn_teardown' phases.rs` — 6 callsites + 1 definition ✓

### Slice-Level Checks (Partial)
- `cargo test --workspace` — ✓ passes
- `cargo clippy --workspace` — ✓ clean
- `rg 'anyhow!.*\{e\}' phases.rs` — ✓ zero hits
- `rg 'let _ = provider\.teardown' phases.rs` — ✓ zero hits
- `cargo doc --workspace --no-deps` — not checked (will verify on final task)
- SSH arg builder DRY refactor — T02 scope

## Diagnostics

Teardown failures during `smelt run` error paths now produce visible stderr output:
- `Warning: failed to set TearingDown phase: <error>`
- `Warning: teardown failed: <error>`
- `Warning: monitor cleanup failed: <error>`

Previously these were silently discarded via `let _ =`.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run/phases.rs` — Added `warn_teardown()` helper, replaced 6 teardown blocks and 5 lossy error conversions; file reduced from ~400 to 359 lines
