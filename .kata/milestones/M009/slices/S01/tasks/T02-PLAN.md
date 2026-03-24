---
estimated_steps: 5
estimated_files: 6
---

# T02: Wire init_tracing into CLI, TUI, and MCP entry points

**Slice:** S01 — Structured tracing foundation and eprintln migration
**Milestone:** M009

## Description

Wire the centralized `init_tracing()` into all three binary entry points (CLI main, TUI main, MCP serve) and remove the two ad-hoc subscriber initialization sites. Add `tracing` as a dependency of assay-tui (D131) and assay-cli. After this task, every binary uses the same subscriber setup and `RUST_LOG` works consistently everywhere.

## Steps

1. Add `tracing.workspace = true` to `crates/assay-tui/Cargo.toml` dependencies. Confirm `tracing` is already in `crates/assay-cli/Cargo.toml` — if not, add it.
2. In `crates/assay-cli/src/main.rs`: add `let _guard = assay_core::telemetry::init_tracing(assay_core::telemetry::TracingConfig::default());` at the very top of `main()`, before any other logic.
3. In `crates/assay-cli/src/commands/mcp.rs`: remove the `init_mcp_tracing()` function entirely. Replace its call in `McpCommand::Serve` with `let _guard = assay_core::telemetry::init_tracing(assay_core::telemetry::TracingConfig::mcp());`. Note: the CLI-level subscriber will already be initialized from `main()`, so `try_init()` will be a silent no-op — the MCP serve path should call `init_tracing` with MCP config _before_ the CLI-level init fires, or restructure so MCP serve skips the default CLI init. The cleanest approach: in `main()`, detect if the subcommand is `mcp serve` and use `TracingConfig::mcp()` instead of default. Alternatively, since `try_init()` only succeeds once, call MCP config init _inside_ the serve handler and it will be a no-op since CLI already initialized. The pragmatic fix: move init_tracing call from top-of-main to after subcommand parsing, passing MCP config when the command is `mcp serve`.
4. In `crates/assay-cli/src/commands/context.rs`: remove the guard daemon's ad-hoc `tracing_subscriber::fmt()` block (~lines 630-633). Replace with `let _guard = assay_core::telemetry::init_tracing(assay_core::telemetry::TracingConfig::default());`. The guard daemon's file-based logging via `tracing-appender` will be revisited in S04 — for now, stderr is sufficient.
5. In `crates/assay-tui/src/main.rs`: add `let _guard = assay_core::telemetry::init_tracing(assay_core::telemetry::TracingConfig::default());` at the top of `main()`, before the event loop setup.

## Must-Haves

- [ ] `init_tracing()` called in CLI main (or per-subcommand with appropriate config)
- [ ] `init_mcp_tracing()` function deleted from mcp.rs
- [ ] Guard daemon ad-hoc subscriber init in context.rs replaced with `init_tracing()`
- [ ] `init_tracing()` called in TUI main
- [ ] `tracing` dep added to assay-tui Cargo.toml
- [ ] No direct `tracing_subscriber::fmt()` calls remain outside `telemetry.rs`
- [ ] All three binaries build successfully

## Verification

- `cargo build -p assay-cli -p assay-tui` succeeds
- `grep -rn 'init_mcp_tracing' crates/assay-cli/src/` returns zero matches
- `grep -rn 'tracing_subscriber::fmt' crates/assay-cli/src/` returns zero matches (no ad-hoc inits)
- `cargo test -p assay-cli` passes (no regressions)

## Observability Impact

- Signals added/changed: All binaries now emit structured tracing events to stderr via the centralized subscriber. MCP serve uses `warn` default level; CLI and TUI use `info` default level.
- How a future agent inspects this: Run any binary with `RUST_LOG=debug` — consistent output format across all entry points.
- Failure state exposed: None new — this task is wiring, not new signals.

## Inputs

- T01 output: `assay_core::telemetry::{init_tracing, TracingConfig, TracingGuard}`
- Existing MCP init: `crates/assay-cli/src/commands/mcp.rs:33-50` (to be removed)
- Existing guard init: `crates/assay-cli/src/commands/context.rs:630-633` (to be removed)
- D131: assay-tui gains tracing dep

## Expected Output

- `crates/assay-cli/src/main.rs` — gains `init_tracing()` call
- `crates/assay-cli/src/commands/mcp.rs` — `init_mcp_tracing()` removed, uses centralized init with MCP config
- `crates/assay-cli/src/commands/context.rs` — ad-hoc subscriber init replaced
- `crates/assay-tui/src/main.rs` — gains `init_tracing()` call
- `crates/assay-tui/Cargo.toml` — gains `tracing.workspace = true`
- `crates/assay-cli/Cargo.toml` — potentially gains `tracing.workspace = true` if missing
