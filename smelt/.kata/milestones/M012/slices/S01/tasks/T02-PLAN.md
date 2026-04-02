---
estimated_steps: 5
estimated_files: 6
---

# T02: Migrate all eprintln! calls to tracing macros

**Slice:** S01 — M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix
**Milestone:** M012

## Description

Replace all 51 `eprintln!` calls across 6 source files with the appropriate `tracing` macros (`info!`, `warn!`, `error!`). The two documented exceptions (`main.rs:76` top-level error handler and `serve/tui.rs` TUI error) remain as `eprintln!`.

Level mapping (from S01-RESEARCH.md):
- Progress messages ("Provisioning container...", "Writing manifest...", "Assay complete", "Container removed") → `info!`
- Warnings ("Warning: teardown failed...", "No running job.", "No state file...") → `warn!`
- Errors/failures ("Validation failed...", "Fatal error...") → `error!`

**Critical constraint:** Message text must be preserved exactly. Integration tests in `docker_lifecycle.rs` assert on stderr substrings like `"Writing manifest..."`, `"Executing assay run..."`, `"Assay complete"`, and `"Container removed"`. With bare format (T01), tracing output is just the message text — these assertions will pass as long as the message string is unchanged.

## Steps

1. Migrate `phases.rs` (33 calls) — the largest target. Map each `eprintln!` to the correct tracing level. Ensure `tracing::{info, warn, error}` are imported (some already are). Preserve exact message format strings.
2. Migrate `watch.rs` (10 calls) — mix of error conditions and poll status. Error conditions → `error!`/`warn!`; status updates → `info!`.
3. Migrate `status.rs` (3 calls), `dry_run.rs` (2 calls), `init.rs` (1 call), `list.rs` (1 call) — small files, straightforward replacements.
4. Verify exceptions remain: confirm `main.rs:76` and `serve/tui.rs` still use `eprintln!`. Run `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` and confirm exactly 2 results.
5. Run `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` to confirm no regressions.

## Must-Haves

- [ ] 51 `eprintln!` calls replaced with `info!`/`warn!`/`error!` across phases.rs, watch.rs, status.rs, dry_run.rs, init.rs, list.rs
- [ ] `main.rs:76` still uses `eprintln!` (D139 exception)
- [ ] `serve/tui.rs` still uses `eprintln!` (TUI exception)
- [ ] `rg 'eprintln!' crates/smelt-cli/src/` shows exactly 2 files with 1 match each
- [ ] Message text preserved exactly — no changes to format strings
- [ ] `cargo test --workspace` passes (all integration test stderr assertions still match)
- [ ] `cargo clippy --workspace -- -D warnings` clean

## Verification

- `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` — exactly `main.rs:1` and `serve/tui.rs:1`
- `cargo test --workspace` — 298+ tests pass, 0 failures
- `cargo clippy --workspace -- -D warnings` — 0 warnings

## Observability Impact

- Signals added/changed: All CLI output now flows through `tracing` — every progress, warning, and error message is a structured event that can be filtered by level and target
- How a future agent inspects this: `SMELT_LOG=debug` shows all events with full metadata; `SMELT_LOG=smelt_cli=trace` shows CLI-only events at maximum verbosity
- Failure state exposed: Error-level events in phases.rs now show up even when operator has `SMELT_LOG=error` — previously these were unconditional `eprintln!`

## Inputs

- `crates/smelt-cli/src/main.rs` — T01's refactored subscriber (bare format by default)
- S01-RESEARCH.md — complete level mapping for each `eprintln!` call
- D139 — full migration decision (all except main.rs error handler)

## Expected Output

- `crates/smelt-cli/src/commands/run/phases.rs` — 33 `eprintln!` → tracing macros
- `crates/smelt-cli/src/commands/watch.rs` — 10 `eprintln!` → tracing macros
- `crates/smelt-cli/src/commands/status.rs` — 3 `eprintln!` → tracing macros
- `crates/smelt-cli/src/commands/run/dry_run.rs` — 2 `eprintln!` → tracing macros
- `crates/smelt-cli/src/commands/init.rs` — 1 `eprintln!` → tracing macro
- `crates/smelt-cli/src/commands/list.rs` — 1 `eprintln!` → tracing macro
