---
id: S02
milestone: M011
status: ready
---

# S02: Full tracing migration + flaky test fix — Context

## Goal

Replace all `eprintln!` calls in smelt-cli (including the `main.rs` error handler) with tracing events, configure a dual-mode formatter (bare text by default, full tracing when `SMELT_LOG` is set), and fix the flaky `test_cli_run_invalid_manifest` timeout.

## Why this Slice

S01 completed the structural decomposition. S02 is independent (no dependency on S01) and tackles the two remaining non-structural quality issues: scattered `eprintln!` output and a flaky test. S03 (health endpoint) depends on S02 being complete so the final verification pass runs against a fully migrated codebase.

## Scope

### In Scope

- Replace all ~52 `eprintln!` calls across 8 files in `crates/smelt-cli/src/` with appropriate `tracing::info!`/`tracing::warn!`/`tracing::error!` events
- Migrate the `main.rs` top-level error handler from `eprintln!("Error: {e:#}")` to `tracing::error!`
- Implement a dual-mode tracing subscriber in `main.rs`:
  - **No `SMELT_LOG` set (default):** minimal bare formatter — progress messages print as plain text to stderr with no timestamps, levels, or module targets (matches current `eprintln!` UX)
  - **`SMELT_LOG` explicitly set:** standard `tracing_subscriber::fmt` format with timestamps, levels, and targets (operator mode)
- Auto-detect which mode to use based on whether `SMELT_LOG` env var is present (no additional CLI flags or env vars)
- Fix `test_cli_run_invalid_manifest` by increasing the subprocess timeout from 10s to 30s
- Update the flaky test's stderr assertion to check for actual error content (e.g. manifest path or "No such file") rather than the literal word "Error"
- Preserve the existing TUI file-appender branch (D107) — the dual-mode formatter applies only to the non-TUI code path

### Out of Scope

- JSON structured log output format
- OpenTelemetry or distributed tracing integration
- Custom log format CLI flag (`--log-format`) or `SMELT_LOG_FORMAT` env var
- Changes to the TUI file-appender tracing path (D107 unchanged)
- Metrics or Prometheus endpoint

## Constraints

- D107: `tracing_subscriber::fmt().init()` can only be called once per process — the existing branched init in `main.rs` must be extended (not replaced) to support the dual-mode formatter
- D127: `#![deny(missing_docs)]` enforced on smelt-cli — all new public items need docs
- The minimal formatter must be reliable enough to handle fatal errors (since `main.rs` error handler is also migrated)
- The `main.rs` error handler migration means tracing must be initialized before any command dispatch — if subscriber init itself fails, the error path must still produce output

## Integration Points

### Consumes

- `crates/smelt-cli/src/main.rs` — current tracing subscriber init (branched for TUI vs non-TUI per D107)
- `crates/smelt-cli/src/commands/run/phases.rs` — 33 `eprintln!` calls (largest migration surface)
- `crates/smelt-cli/src/commands/watch.rs` — 10 `eprintln!` calls
- `crates/smelt-cli/src/commands/{init.rs, list.rs, status.rs}` — scattered `eprintln!` calls
- `crates/smelt-cli/src/commands/run/dry_run.rs` — 2 `eprintln!` calls
- `crates/smelt-cli/src/serve/tui.rs` — 1 `eprintln!` call
- `crates/smelt-cli/tests/docker_lifecycle.rs` — flaky test at line 807

### Produces

- Updated tracing subscriber config in `main.rs` with dual-mode formatter (bare default / full when SMELT_LOG set)
- All `eprintln!` calls in smelt-cli replaced with `tracing::info!`/`tracing::warn!`/`tracing::error!`
- Zero `eprintln!` calls remaining in `crates/smelt-cli/src/` (no exceptions)
- Fixed `test_cli_run_invalid_manifest` (30s timeout, resilient assertion)
- All tests passing with 0 failures

## Open Questions

- None — all behavioral decisions settled during discussion.
