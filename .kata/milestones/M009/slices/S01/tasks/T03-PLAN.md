---
estimated_steps: 3
estimated_files: 2
---

# T03: Migrate assay-core eprintln calls to tracing macros

**Slice:** S01 — Structured tracing foundation and eprintln migration
**Milestone:** M009

## Description

Migrate all 9 `eprintln!` calls in assay-core (7 in `history/analytics.rs`, 2 in `history/mod.rs`) to appropriate `tracing::*` macros. These are all diagnostic messages — analytics skip warnings and history load errors — so the mapping is straightforward.

## Steps

1. Open `crates/assay-core/src/history/analytics.rs`. Audit all 7 `eprintln!` calls. These are skip warnings during aggregation (e.g. "Skipping run with no results"). Map each to `tracing::warn!` with structured fields where useful (e.g. `run_id`, `spec_slug`, `file_path`). Confirm `tracing` is already in assay-core deps (it is — used in daemon.rs, worktree.rs, etc.).
2. Open `crates/assay-core/src/history/mod.rs`. Audit both `eprintln!` calls. Map load errors / skip warnings to `tracing::warn!` with structured fields (e.g. `path` of the problematic file).
3. Run `grep -rn 'eprintln!' crates/assay-core/src/` to confirm zero matches. Run `cargo test -p assay-core` to verify no test regressions (check if any test captures stderr and asserts on exact eprintln strings).

## Must-Haves

- [ ] Zero `eprintln!` calls in `crates/assay-core/src/`
- [ ] All replacements use appropriate tracing level (warn for skip/load warnings, error for failures)
- [ ] Structured fields added where they improve debuggability (file paths, slugs)
- [ ] `cargo test -p assay-core` passes with no regressions

## Verification

- `grep -rn 'eprintln!' crates/assay-core/src/ --include='*.rs'` returns zero matches
- `cargo test -p assay-core` — all tests pass (no stderr assertion breakage)

## Observability Impact

- Signals added/changed: Analytics skip warnings and history load warnings now appear as structured `tracing::warn!` events instead of raw stderr text. They carry structured fields (file paths, slugs) making them filterable.
- How a future agent inspects this: `RUST_LOG=assay_core::history=debug` shows all history-related events.
- Failure state exposed: History file load failures now visible via tracing filter, not just as noise on stderr.

## Inputs

- T01 output: telemetry module exists (subscriber will be initialized by binaries)
- `crates/assay-core/src/history/analytics.rs` — 7 eprintln calls
- `crates/assay-core/src/history/mod.rs` — 2 eprintln calls
- `tracing` already a dep of assay-core

## Expected Output

- `crates/assay-core/src/history/analytics.rs` — all eprintln replaced with tracing::warn!
- `crates/assay-core/src/history/mod.rs` — all eprintln replaced with tracing::warn!
