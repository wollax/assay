---
id: T02
parent: S01
milestone: M009
provides:
  - All three binaries (CLI, TUI, MCP) use centralized init_tracing()
  - Subcommand-aware config selection (MCP serve gets TracingConfig::mcp())
  - No ad-hoc tracing_subscriber::fmt() calls remain outside telemetry.rs
  - tracing dep added to assay-tui
  - Removed unused tracing-appender and tracing-subscriber direct deps from assay-cli
key_files:
  - crates/assay-cli/src/main.rs
  - crates/assay-cli/src/commands/mcp.rs
  - crates/assay-cli/src/commands/context.rs
  - crates/assay-tui/src/main.rs
  - crates/assay-tui/Cargo.toml
  - crates/assay-cli/Cargo.toml
key_decisions:
  - "Tracing init placed after CLI argument parsing (not top-of-main) so MCP serve gets TracingConfig::mcp() while all other subcommands get default — avoids double-init race"
  - "Removed tracing-appender and tracing-subscriber direct deps from assay-cli since they are only consumed transitively via assay-core::telemetry"
  - "Guard daemon file-based logging (tracing-appender) removed for now — stderr subscriber is sufficient; file logging revisited in S04"
patterns_established:
  - "tracing_config_for(&Option<Command>) helper in CLI main.rs — pattern for subcommand-specific tracing config selection"
observability_surfaces:
  - RUST_LOG=debug works consistently across all three binaries with identical output format
  - MCP serve defaults to warn level; CLI/TUI default to info level
duration: 10min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T02: Wire init_tracing into CLI, TUI, and MCP entry points

**Wired centralized init_tracing() into CLI (subcommand-aware), TUI, and MCP serve; deleted init_mcp_tracing() and guard daemon ad-hoc subscriber**

## What Happened

Added `tracing_config_for()` helper to CLI main.rs that inspects the parsed subcommand — returns `TracingConfig::mcp()` for `mcp serve` (warn level, no ANSI) and `TracingConfig::default()` for everything else (info level). The tracing guard is initialized after argument parsing but before command dispatch, ensuring all code paths get the subscriber.

Deleted `init_mcp_tracing()` from `commands/mcp.rs` entirely — the centralized init in main() handles MCP config. Replaced the guard daemon's ad-hoc `tracing_subscriber::fmt().with_writer(non_blocking).init()` in `commands/context.rs` with a comment noting file-based logging is deferred to S04.

Added `tracing.workspace = true` to assay-tui's Cargo.toml and placed `init_tracing(TracingConfig::default())` in TUI main after `color_eyre::install()`.

Cleaned up assay-cli's Cargo.toml: removed direct `tracing-appender` and `tracing-subscriber` deps since they're now only consumed transitively through assay-core::telemetry.

## Verification

- `cargo build -p assay-cli -p assay-tui` — success (no errors, only pre-existing assay-mcp warnings)
- `grep -rn 'init_mcp_tracing' crates/assay-cli/src/` — zero matches (deleted)
- `grep -rn 'tracing_subscriber::fmt' crates/assay-cli/src/` — zero matches (no ad-hoc inits)
- `grep -rn 'tracing_subscriber::fmt' crates/ --include='*.rs' | grep -v telemetry.rs` — zero matches (only telemetry.rs has it)
- `cargo test -p assay-cli` — 45 passed, 0 failed

### Slice-level verification (partial — T02 is not final task):
- `cargo build -p assay-cli` ✓
- `cargo build -p assay-tui` ✓
- `grep -rn 'eprintln!' crates/` — not yet checked (T03/T04 handle eprintln migration)
- `just ready` — deferred to final task

## Diagnostics

Run any binary with `RUST_LOG=debug` to see structured tracing output on stderr. MCP serve: `RUST_LOG=debug assay mcp serve`. Per-crate: `RUST_LOG=assay_core=debug,assay_cli=info`.

## Deviations

- Removed `tracing-appender` and `tracing-subscriber` from assay-cli Cargo.toml — not in the plan but correct since those crates are no longer directly imported in assay-cli source (consumed via assay-core::telemetry).
- Guard daemon: rather than calling `init_tracing()` again (which would be a no-op since main() already initialized), left a comment placeholder for S04 file logging.

## Known Issues

- Guard daemon no longer writes to `guard.log` file — it uses stderr via the centralized subscriber. File-based logging deferred to S04.

## Files Created/Modified

- `crates/assay-cli/src/main.rs` — Added tracing_config_for() helper and init_tracing() call after arg parsing
- `crates/assay-cli/src/commands/mcp.rs` — Deleted init_mcp_tracing(), simplified to just call assay_mcp::serve()
- `crates/assay-cli/src/commands/context.rs` — Replaced ad-hoc tracing_subscriber::fmt() block with comment for S04
- `crates/assay-tui/src/main.rs` — Added init_tracing(TracingConfig::default()) at top of main
- `crates/assay-tui/Cargo.toml` — Added tracing.workspace = true
- `crates/assay-cli/Cargo.toml` — Added tracing.workspace = true, removed tracing-appender and tracing-subscriber direct deps
