---
estimated_steps: 4
estimated_files: 3
---

# T04: Migrate assay-cli eprintln calls to tracing macros (batch 1: run, gate, harness)

**Slice:** S01 — Structured tracing foundation and eprintln migration
**Milestone:** M009

## Description

Migrate the 61 `eprintln!` calls in the three highest-count CLI command files: `run.rs` (31), `gate.rs` (19), `harness.rs` (11). These are mostly user-facing progress output (phase banners, session results, criterion status, file listings). Each call is individually mapped to the right tracing level. The 1 `eprint!` (no newline) in gate.rs for live criterion progress is preserved — it uses carriage returns for interactive display.

## Steps

1. Migrate `crates/assay-cli/src/commands/run.rs` (31 calls):
   - Phase banners ("Phase 1: Executing sessions...") → `tracing::info!`
   - Session result lines ("[✓] name — completed") → `tracing::info!` with structured fields (`session_name`, `status`)
   - Error reports → `tracing::error!`
   - Merge summaries → `tracing::info!`
   - Verify no ANSI escape codes are hardcoded in the message strings (fmt layer handles coloring)
2. Migrate `crates/assay-cli/src/commands/gate.rs` (19 calls):
   - Criterion pass/fail/skip lines → `tracing::info!` with `criterion_name`, `passed` fields
   - Summary lines ("X of Y criteria passed") → `tracing::info!`
   - Error lines → `tracing::error!`
   - Evidence lines (verbose) → `tracing::debug!`
   - **Keep the 1 `eprint!` (no newline)** at line ~262 for live progress — this uses `\r\x1b[K` carriage return semantics that tracing doesn't support
3. Migrate `crates/assay-cli/src/commands/harness.rs` (11 calls):
   - File listing output → `tracing::info!`
   - Diff output → `tracing::info!`
   - Error/warning → `tracing::warn!` or `tracing::error!`
4. Run `cargo test -p assay-cli` and check for any test that captured stderr and matched on exact eprintln strings. Fix assertions if needed.

## Must-Haves

- [ ] Zero `eprintln!` in run.rs, gate.rs, harness.rs
- [ ] 1 `eprint!` in gate.rs preserved (carriage return progress)
- [ ] Each migration uses the appropriate tracing level (info for user-facing, error for errors, debug for verbose/evidence)
- [ ] `cargo test -p assay-cli` passes

## Verification

- `grep -rn 'eprintln!' crates/assay-cli/src/commands/run.rs crates/assay-cli/src/commands/gate.rs crates/assay-cli/src/commands/harness.rs` returns zero
- `grep -c 'eprint!' crates/assay-cli/src/commands/gate.rs` returns 1 (the preserved progress line)
- `cargo test -p assay-cli` passes

## Observability Impact

- Signals added/changed: Pipeline run progress, gate evaluation results, and harness operations are now structured events. Gate results carry `criterion_name` and `passed` fields — filterable and machine-readable.
- How a future agent inspects this: `RUST_LOG=assay_cli::commands::gate=debug` shows evidence; `RUST_LOG=info` shows results and phase banners.
- Failure state exposed: Gate failures now structured as `tracing::error!` with criterion context, not raw text.

## Inputs

- T01/T02 output: telemetry module exists, subscriber initialized in CLI main
- `crates/assay-cli/src/commands/run.rs` — 31 eprintln calls
- `crates/assay-cli/src/commands/gate.rs` — 19 eprintln + 1 eprint (keep)
- `crates/assay-cli/src/commands/harness.rs` — 11 eprintln calls

## Expected Output

- `crates/assay-cli/src/commands/run.rs` — all 31 eprintln replaced
- `crates/assay-cli/src/commands/gate.rs` — 19 eprintln replaced, 1 eprint preserved
- `crates/assay-cli/src/commands/harness.rs` — all 11 eprintln replaced
