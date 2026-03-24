---
id: T04
parent: S01
milestone: M009
provides:
  - All 31 eprintln! calls in run.rs migrated to tracing macros with structured fields
  - All 19 eprintln! calls in gate.rs migrated to tracing macros with structured fields
  - All 11 eprintln! calls in harness.rs migrated to tracing macros with structured fields
  - 1 eprint! in gate.rs preserved (carriage-return live progress)
  - Structured fields on session results (session_name, status), gate criteria (criterion_name, passed, advisory), harness operations (adapter, file_count)
key_files:
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-cli/src/commands/gate.rs
  - crates/assay-cli/src/commands/harness.rs
  - crates/assay-cli/src/commands/mod.rs
key_decisions:
  - "Phase banners and session results mapped to tracing::info!; errors to tracing::error!; evidence output to tracing::debug! (verbose gate output)"
  - "Gate criterion pass/fail/warn mapped to info!/error!/warn! respectively — advisory failures use warn!, required failures use error!"
  - "print_evidence function outputs at debug level — evidence is verbose detail, not primary output"
  - "format_warn marked #[allow(dead_code)] rather than deleted — may be useful for future non-tracing display paths"
patterns_established:
  - "Session result structured fields: session_name, status for completed; session_name, error for failed; session_name, reason for skipped"
  - "Gate criterion structured fields: criterion_name, passed, advisory — consistent across all gate evaluation paths"
observability_surfaces:
  - "RUST_LOG=assay_cli::commands::run=info shows pipeline progress and session results"
  - "RUST_LOG=assay_cli::commands::gate=debug shows gate evidence; =info shows pass/fail results"
  - "RUST_LOG=assay_cli::commands::harness=info shows harness generate/install/diff operations"
duration: 12min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T04: Migrated 61 eprintln! calls in run.rs, gate.rs, harness.rs to structured tracing macros

**Pipeline run progress, gate criterion evaluation, and harness operations now emit structured tracing events with session_name, criterion_name, passed, and adapter fields**

## What Happened

Migrated all `eprintln!` calls in the three highest-count CLI command files:

- **run.rs (31→0):** Phase banners to `info!`, session results to `info!` with `session_name`/`status` fields, errors to `error!` with `stage`/`message` fields, summaries to `info!` with structured counts. All four execution paths (sequential, orchestrated, mesh, gossip) follow the same pattern.

- **gate.rs (19→0, 1 eprint! preserved):** Criterion pass/fail/warn to appropriate levels with `criterion_name`/`passed`/`advisory` fields. Evidence output to `debug!` level. Spec scan warnings and history warnings to `warn!`. The single `eprint!` for live carriage-return progress is preserved — tracing doesn't support `\r\x1b[K` semantics.

- **harness.rs (11→0):** File listing/generation to `info!` with `path`/`bytes` fields. Diff output to `info!` with `change` field. Install summaries to `info!` with `file_count`/`adapter`.

Also removed unused `format_warn` import from gate.rs and marked the function `#[allow(dead_code)]` in mod.rs.

## Verification

- `grep -rn 'eprintln!' crates/assay-cli/src/commands/run.rs crates/assay-cli/src/commands/gate.rs crates/assay-cli/src/commands/harness.rs` returns zero matches ✓
- `grep -c 'eprint!' crates/assay-cli/src/commands/gate.rs` returns 1 (preserved progress line) ✓
- `cargo test -p assay-cli` — 45 passed, 0 failed ✓
- `cargo build -p assay-cli` — clean (no warnings for assay-cli) ✓

### Slice-level verification (partial — T04 is intermediate):
- `grep -rn 'eprintln!' crates/assay-core/src/` — zero (done in T03) ✓
- `grep -rn 'eprintln!' crates/assay-cli/src/commands/{run,gate,harness}.rs` — zero ✓
- Remaining eprintln! in other CLI files (main.rs, spec.rs, plan.rs, history.rs, milestone.rs, init.rs, worktree.rs) — T05 scope
- `cargo build -p assay-cli` succeeds ✓
- `cargo test -p assay-cli` passes ✓

## Diagnostics

- `RUST_LOG=assay_cli::commands::run=info` — shows pipeline phase banners, session results, summaries
- `RUST_LOG=assay_cli::commands::gate=info` — shows criterion pass/fail/warn with structured fields
- `RUST_LOG=assay_cli::commands::gate=debug` — additionally shows evidence output
- `RUST_LOG=assay_cli::commands::harness=info` — shows generate/install/diff operations

## Deviations

- Removed `format_warn` import from gate.rs and added `#[allow(dead_code)]` to the function definition in mod.rs — it was only used in the now-replaced `eprintln!` display paths. Kept the function rather than deleting it since it may be useful for future non-tracing display paths.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/commands/run.rs` — 31 eprintln! → tracing macros with structured fields
- `crates/assay-cli/src/commands/gate.rs` — 19 eprintln! → tracing macros; 1 eprint! preserved
- `crates/assay-cli/src/commands/harness.rs` — 11 eprintln! → tracing macros with structured fields
- `crates/assay-cli/src/commands/mod.rs` — Added #[allow(dead_code)] on format_warn
