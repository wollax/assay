---
estimated_steps: 4
estimated_files: 1
---

# T01: Extract warn_teardown helper and replace silent let _ = in phases.rs

**Slice:** S02 — Teardown error handling + SSH DRY cleanup
**Milestone:** M010

## Description

Replace 6 identical teardown blocks in `phases.rs` that silently discard errors via `let _ =` with a single `warn_teardown()` async helper that logs warnings on failure. Also replace all 5 occurrences of `anyhow!("{e}")` on `monitor.write()` calls with `.context()` to preserve error chains.

## Steps

1. Read `phases.rs` to confirm the 6 teardown blocks (lines ~117–207) and 5 `anyhow!("{e}")` sites
2. Add an async `warn_teardown()` helper function that:
   - Calls `monitor.set_phase(JobPhase::TearingDown)` with `eprintln!` warning on error
   - Calls `provider.teardown(&container)` with `eprintln!` warning on error
   - Prints "Container removed." on success
   - Calls `monitor.cleanup()` with `eprintln!` warning on error
3. Replace all 6 teardown blocks (assay config write, specs dir, spec file loop, manifest write, post-manifest error, post-timeout/cancel) with calls to `warn_teardown()`
4. Replace all 5× `.map_err(|e| anyhow::anyhow!("{e}"))` on `monitor.write()` with `.context("failed to write monitor state")`
5. Keep `let _ =` on non-teardown `monitor.set_phase()` calls (GatesFailed, Complete, Failed, Timeout, Cancelled, TearingDown at line 346) — these are best-effort status updates in the outcome match block, not teardown error paths

## Must-Haves

- [ ] `warn_teardown()` helper function exists and logs warnings (not silent discards) on each failure
- [ ] All 6 duplicated teardown blocks replaced with `warn_teardown()` calls
- [ ] Zero occurrences of `let _ = provider.teardown(` in the file
- [ ] Zero occurrences of `anyhow!("{e}")` or `anyhow::anyhow!("{e}")` in the file
- [ ] `monitor.write()` failures use `.context()` preserving error chain
- [ ] `cargo test --workspace` passes with 0 failures
- [ ] `cargo check --workspace` compiles clean

## Verification

- `cargo test --workspace` — all 286+ tests pass, 0 failures
- `rg 'let _ = provider\.teardown' crates/smelt-cli/src/commands/run/phases.rs` — 0 matches
- `rg 'anyhow!.*\{e\}' crates/smelt-cli/src/commands/run/phases.rs` — 0 matches
- `rg 'warn_teardown' crates/smelt-cli/src/commands/run/phases.rs` — 6+ matches (callsites)

## Observability Impact

- Signals added/changed: `eprintln!("Warning: ...")` on teardown failures (provider.teardown, monitor.set_phase, monitor.cleanup) — previously silent
- How a future agent inspects this: stderr output during `smelt run` error paths shows teardown outcome
- Failure state exposed: orphaned containers or corrupt monitor state now produces visible stderr warnings instead of being swallowed

## Inputs

- `crates/smelt-cli/src/commands/run/phases.rs` — the 6 duplicated teardown blocks and 5 error-chain-losing `anyhow!("{e}")` calls

## Expected Output

- `crates/smelt-cli/src/commands/run/phases.rs` — teardown blocks collapsed to helper calls; error chains preserved; file ~80-100 lines shorter
