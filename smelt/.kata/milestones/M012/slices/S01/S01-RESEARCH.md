# S01: M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix — Research

**Date:** 2026-03-27
**Domain:** Rust tracing, CLI output formatting, subprocess test reliability
**Confidence:** HIGH

## Summary

This slice resolves two M011 leftovers: R062 (full `eprintln!` → `tracing` migration in smelt-cli) and R061 (flaky `test_cli_run_invalid_manifest` 10s timeout). Both are well-understood, mechanically straightforward, and carry low risk.

The tracing infrastructure is already fully wired: `tracing`, `tracing-subscriber`, and `tracing-appender` are workspace deps; `main.rs` initializes `tracing_subscriber::fmt()` with `SMELT_LOG`/`RUST_LOG` env filter (D107); and the TUI branch already redirects to a file appender. The migration is 52 `eprintln!` calls across 8 files (51 to migrate, 1 excluded in `main.rs` per D139). The prior M011/S02 research (`.kata/milestones/M011/slices/S02/S02-RESEARCH.md`) contains a complete analysis including the **critical pitfall**: the default filter is `warn`, which would hide `info!`-level progress messages and break integration tests.

The flaky test fix is a single constant change: 10s → 30s timeout on `test_cli_run_invalid_manifest`.

## Recommendation

Follow the approach from M011/S02 research verbatim — it was well-analyzed but never executed:

1. **Subscriber format + default filter (main.rs):** Configure `.without_time().with_target(false).with_level(false)` for the default (non-SMELT_LOG) path so output is bare message text, identical to current `eprintln!`. Change default filter from `"warn"` to `"smelt_cli=info,smelt_core=info,warn"` so `info!` progress messages appear while dependency crate noise stays suppressed. When SMELT_LOG is explicitly set, use full format with time/target/level for operator diagnostics.

2. **Mechanical eprintln! → tracing replacement.** Level mapping:
   - Progress ("Provisioning container...", "Writing manifest...") → `info!`
   - Warnings ("Warning: teardown failed...") → `warn!`
   - Errors/failures ("Validation failed...") → `error!`
   - User-facing "not found" ("No running job.", "No state file...") → `warn!`

3. **Two exceptions stay as eprintln!:**
   - `main.rs:76` — top-level error handler (per D139 / M011 context)
   - `serve/tui.rs:27` — TUI error after `ratatui::restore()` (tracing subscriber still points at file appender; this must reach stderr)

4. **Flaky test fix:** Increase `test_cli_run_invalid_manifest` timeout from 10s to 30s.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Structured CLI logging | `tracing` + `tracing-subscriber` (workspace deps) | Already initialized in `main.rs`; format config only |
| Env-based log filtering | `tracing_subscriber::EnvFilter` (already wired) | `SMELT_LOG` / `RUST_LOG` supported via D107 |
| Subprocess test timeouts | `assert_cmd::Command::timeout()` (dev-dep) | Just change the duration constant |

## Existing Code and Patterns

- `crates/smelt-cli/src/main.rs:38-62` — Subscriber init with TUI/stderr branch (D107). Single point of change for format config.
- `crates/smelt-cli/src/commands/run/phases.rs` — 33 `eprintln!` calls. Largest target. Already imports `tracing::{error, info}`.
- `crates/smelt-cli/src/commands/watch.rs` — 10 `eprintln!` calls. Mix of error conditions and poll status.
- `crates/smelt-cli/src/commands/status.rs` — 3 calls. Warning and "no job" messages.
- `crates/smelt-cli/src/commands/run/dry_run.rs` — 2 calls. Validation failure.
- `crates/smelt-cli/src/commands/init.rs` — 1 call. "File already exists."
- `crates/smelt-cli/src/commands/list.rs` — 1 call. State read warning.
- `crates/smelt-cli/src/serve/tui.rs` — 1 call. TUI error (exception — stays as `eprintln!`).
- `crates/smelt-cli/src/main.rs:76` — 1 call. Top-level error handler (exception — stays as `eprintln!`).
- `crates/smelt-cli/tests/docker_lifecycle.rs:807-826` — Flaky test. Line 813: `timeout(std::time::Duration::from_secs(10))`.
- `crates/smelt-cli/tests/docker_lifecycle.rs:470-490` — `test_cli_run_lifecycle` asserts `stderr.contains("Writing manifest...")`, `"Executing assay run..."`, `"Assay complete"`, `"Container removed"`. These substrings must remain in tracing messages.

## Constraints

- **D107 (single init):** Subscriber initialized once in `main.rs` match block. All format config must be there.
- **D139 (full migration):** All `eprintln!` replaced except `main.rs` error handler.
- **D127 (deny(missing_docs)):** Any new public items need doc comments.
- **Default filter level:** Must change from `"warn"` to at least `"smelt_cli=info,warn"` or progress messages vanish (breaking tests and user output).
- **Integration test compatibility:** `test_cli_run_lifecycle` uses `stderr.contains("Writing manifest...")` — substring match. With bare format (no level/target/time), output is just the message text, so assertions pass. Tests must NOT set SMELT_LOG.
- **`test_cli_run_invalid_manifest` asserts `stderr.contains("Error")`** — "Error" comes from `main.rs:76` (excluded from migration), so this still works.
- **TUI mode file appender:** Format changes apply to `.smelt/serve.log` too but readability is less critical in a log file. TUI mode can use the full format (with levels) since it's not user-facing.

## Common Pitfalls

- **Default `warn` filter hides `info!` progress messages** — The #1 risk. Current default is `"warn"`. After migration, all progress messages are `info!`. Fix: target-scoped filter `"smelt_cli=info,smelt_core=info,warn"`. This lets smelt crate events through at info while keeping dependency noise (tokio, hyper, bollard) at warn.
- **Tracing format includes noisy prefixes** — Default `fmt()` includes timestamp, level, target. For user-facing CLI output, this is unacceptable. Fix: `.without_time().with_target(false).with_level(false)` for the default (stderr) path. **Only** activate full format when SMELT_LOG is explicitly set.
- **TUI error after restore()** — `serve/tui.rs` `eprintln!` runs after `ratatui::restore()`. Tracing subscriber still points to file appender. Must stay as `eprintln!` (second exception).
- **SMELT_LOG set in CI** — If CI sets `SMELT_LOG=debug`, full format activates with prefixes. Integration test assertions still pass (substring `contains()`), but consider documenting this.

## Open Risks

- **Targeted filter syntax correctness** — `"smelt_cli=info,smelt_core=info,warn"` must parse correctly with `EnvFilter::new()`. The `EnvFilter` docs confirm this comma-separated syntax works. Low risk but must test at runtime.
- **Existing `tracing::{error, info}` imports in phases.rs** — Some calls already use tracing (2 out of 35). Ensure no duplicate import issues when adding more tracing macros. Low risk — these files already import from tracing.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust tracing | `wshobson/agents@rust-async-patterns` (6.4K installs) | Available but not relevant — async patterns, not subscriber formatting |

No directly relevant skills found — this is standard `tracing-subscriber` configuration.

## Sources

- M011/S02 prior research: `.kata/milestones/M011/slices/S02/S02-RESEARCH.md` (complete analysis, never executed)
- Codebase: `rg 'eprintln!'` — 52 total calls (51 to migrate, 1 main.rs exception, 1 tui.rs exception)
- Test suite: `cargo test --workspace` — 298 tests pass (91 smelt-cli, 3 integration, 23+16 docker, 5 k8s, 5 compose, 161 smelt-core)
- Decisions register: D107 (subscriber branching), D139 (full migration), D127 (deny missing_docs)
