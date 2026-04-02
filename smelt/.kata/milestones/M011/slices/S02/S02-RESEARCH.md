# S02: Full tracing migration + flaky test fix — Research

**Date:** 2026-03-27
**Domain:** Rust tracing, CLI output, subprocess testing
**Confidence:** HIGH

## Summary

S02 has two independent workstreams: (1) replacing all 52 `eprintln!` calls in smelt-cli with `tracing` events, and (2) fixing the flaky `test_cli_run_invalid_manifest` 10s timeout. Both are straightforward with well-understood solutions. The tracing migration is the larger effort but mechanically simple — the main risk is breaking integration tests that assert on specific stderr strings.

The tracing infrastructure is already in place: `tracing`, `tracing-subscriber`, and `tracing-appender` are workspace deps, and `main.rs` already initializes a `tracing_subscriber::fmt()` subscriber with `SMELT_LOG` env filter (D107). The key design decision (D139) is full migration — every `eprintln!` becomes a tracing event.

The critical constraint is the **subscriber format layer**. The default `tracing_subscriber::fmt()` format includes timestamps, log levels, and targets — e.g. `2026-03-27T10:00:00Z  INFO smelt_cli::commands::run::phases: Provisioning container...`. For user-facing progress messages, this is unacceptable noise. The subscriber must be configured with `.without_time().with_target(false).with_level(false)` for the default (non-SMELT_LOG) path, producing bare message output identical to the current `eprintln!` behavior. When `SMELT_LOG` is explicitly set, the full format (with levels and targets) should activate so operators get structured diagnostic output.

## Recommendation

**Two-phase approach:**

1. **Configure subscriber for clean default output.** Update `main.rs` to use `.without_time().with_target(false).with_level(false)` when SMELT_LOG is NOT set. When SMELT_LOG is set, use the current format (with time, target, level) for operator diagnostics. This way the default user experience is identical to today's `eprintln!` output, but `SMELT_LOG=info` activates full structured tracing.

2. **Mechanical eprintln! → tracing replacement.** Map each `eprintln!` to the appropriate tracing level:
   - Progress messages ("Provisioning container...") → `tracing::info!`
   - Warnings ("Warning: teardown failed...") → `tracing::warn!`
   - Errors ("Validation failed...") → `tracing::error!`
   - User-facing status ("No running job.") → `tracing::warn!` or `tracing::info!`

3. **Fix flaky test.** Increase `test_cli_run_invalid_manifest` timeout from 10s to 30s (matching the pattern of `test_cli_run_lifecycle` at 120s). The test spawns a subprocess that may need to link the binary on first run.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Structured CLI logging | `tracing` + `tracing-subscriber` (already in deps) | Already initialized in `main.rs`; just need format config |
| Env-based log filtering | `tracing_subscriber::EnvFilter` (already wired) | `SMELT_LOG` / `RUST_LOG` already supported via D107 |
| Subprocess test timeouts | `assert_cmd::Command::timeout()` (already in dev-deps) | Just increase the duration constant |

## Existing Code and Patterns

- `crates/smelt-cli/src/main.rs:38-62` — Tracing subscriber init with branched TUI/stderr writers (D107). This is the single point where format must change.
- `crates/smelt-cli/src/commands/run/phases.rs` — 33 `eprintln!` calls. Largest migration target. Already imports `tracing::{error, info}` but only uses them for 2 calls.
- `crates/smelt-cli/src/commands/watch.rs` — 10 `eprintln!` calls. Mix of error conditions and progress.
- `crates/smelt-cli/src/commands/status.rs` — 3 `eprintln!` calls. Warning and "no job" messages.
- `crates/smelt-cli/src/commands/init.rs` — 1 `eprintln!` call. "File already exists" error.
- `crates/smelt-cli/src/commands/list.rs` — 1 `eprintln!` call. Warning on state read failure.
- `crates/smelt-cli/src/commands/run/dry_run.rs` — 2 `eprintln!` calls. Validation failure output.
- `crates/smelt-cli/src/serve/tui.rs` — 1 `eprintln!` call. TUI error after `ratatui::restore()`.
- `crates/smelt-cli/src/main.rs:76` — 1 `eprintln!` call. Top-level error handler (EXCLUDED per M011 context).
- `crates/smelt-cli/tests/docker_lifecycle.rs:470-490` — Integration test `test_cli_run_lifecycle` asserts `stderr.contains("Writing manifest...")`, `"Executing assay run..."`, `"Assay complete"`, `"Container removed"`. These strings must remain in the tracing output.
- `crates/smelt-cli/tests/docker_lifecycle.rs:807-826` — `test_cli_run_invalid_manifest` with 10s timeout (line 813). Asserts `stderr.contains("Error")`.

## Constraints

- **D107 (single init):** `tracing_subscriber::fmt().init()` can only be called once. All format configuration must be decided in the `match &cli.command` block in `main.rs`.
- **D139 (full migration):** All `eprintln!` replaced — no partial approach.
- **D127 (deny(missing_docs)):** All new public items need doc comments.
- **main.rs error handler excluded:** The `eprintln!("Error: {e:#}")` at line 76 stays as-is per M011 context.
- **TUI branch writes to file appender:** The TUI branch in `main.rs` writes to `.smelt/serve.log` — format changes apply there too but readability is less critical (log file, not terminal).
- **Integration tests assert stderr strings:** `test_cli_run_lifecycle` checks for exact substrings like `"Writing manifest..."` and `"Container removed"` on stderr. The tracing format MUST preserve these strings in the output. With `.without_time().with_target(false).with_level(false)`, the output will be just the message text — identical to `eprintln!`. But if SMELT_LOG is set during tests, the format changes and assertions could break. Tests should NOT set SMELT_LOG.
- **`test_cli_run_invalid_manifest` checks `stderr.contains("Error")`** — after migration, the validation failure will emit `tracing::error!` which in bare format produces the message text. The word "Error" must appear in the message, not just in the level prefix. Current message: `"Validation failed for \`{}\`:\n"` — need to ensure `"Error"` appears. Actually, the top-level error handler (`eprintln!("Error: {e:#}")` in main.rs) is what produces "Error" on stderr — and that handler is EXCLUDED from migration. So this test should still pass.

## Common Pitfalls

- **Tracing format breaks test assertions** — If the subscriber includes timestamps/levels/targets, `stderr.contains("Writing manifest...")` still passes (substring match), but the output looks noisy to users. The fix is the bare format for default mode. Risk: low if implemented correctly.
- **SMELT_LOG set in CI/test environment** — If CI sets `SMELT_LOG=debug`, the full format activates and adds prefixes. The test assertions use `contains()` so they'll still pass (the message text is a substring). But warn level in default mode means some messages won't appear unless the filter is set. **Key risk: the default filter is `warn` — `info!` messages won't appear at all unless SMELT_LOG is set.** This would break the integration tests that expect progress messages. Solution: default filter must be `info` (not `warn`) for non-TUI CLI commands, or use a custom filter that shows `info` for `smelt_cli` target and `warn` for everything else.
- **`serve/tui.rs` eprintln after ratatui::restore()** — This `eprintln!` runs AFTER the TUI has been torn down and ratatui::restore() has been called. Tracing is still initialized to the file appender. Using `tracing::error!` here would write to the log file, not stderr. This one might need to stay as `eprintln!` since it's a terminal cleanup error that must be visible to the user — or use `eprintln!` as a deliberate exception alongside the main.rs error handler.
- **Default log level `warn` hides `info!` progress messages** — The current default env filter is `warn`. After migration, all progress messages (`"Provisioning container..."`, `"Writing manifest..."`, etc.) become `tracing::info!`. With the default `warn` filter, these messages vanish. **This is the #1 risk.** Fix: change the default filter from `"warn"` to `"info"` for the smelt_cli target, or to just `"info"` globally.

## Open Risks

- **Default filter level change impact** — Changing from `warn` to `info` may surface tracing events from dependencies (tokio, hyper, bollard, etc.) at info level, creating noise. Mitigation: use a targeted filter like `"smelt_cli=info,smelt_core=info,warn"` as the default, keeping dependency crates at `warn`.
- **TUI error after restore** — The `eprintln!("TUI error: {e}")` in `tui.rs` may need to be kept as `eprintln!` (second exception alongside main.rs) since tracing writes to the file appender in TUI mode, and this error should be visible on the terminal after TUI exit. Alternatively, since `ratatui::restore()` has already run, the terminal is back to normal and stderr writing works — but the tracing subscriber is still pointing at the file. This is a genuine edge case.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust tracing | `wshobson/agents@distributed-tracing` (3.4K installs) | Not relevant — distributed tracing, not subscriber formatting |
| Rust | `apollographql/skills@rust-best-practices` (4.8K installs) | Available but not directly relevant to this slice's work |

No directly relevant skills found — this is standard Rust `tracing` usage with well-documented API.

## Sources

- Codebase exploration: `rg 'eprintln!'` across smelt-cli/src/ (52 total calls, 51 to migrate, 1 excluded in main.rs)
- `tracing-subscriber` API: `fmt::SubscriberBuilder` supports `.without_time()`, `.with_target(false)`, `.with_level(false)` for minimal output
- D107, D139, D140 in DECISIONS.md
