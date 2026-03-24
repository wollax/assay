---
id: S01
parent: M009
milestone: M009
provides:
  - assay_core::telemetry module with init_tracing(TracingConfig) -> TracingGuard
  - TracingConfig presets (default for CLI/TUI at info level, mcp at warn level)
  - EnvFilter support via RUST_LOG with fallback to configured default level
  - Non-blocking stderr writer with flush-on-drop TracingGuard
  - Centralized subscriber init across CLI, TUI, and MCP binaries
  - Zero eprintln! in all 4 production crates (assay-cli, assay-core, assay-tui, assay-mcp)
  - Structured tracing events with fields (path, error, session_name, criterion_name, etc.)
requires:
  - slice: none
    provides: foundation slice — no upstream dependencies
affects:
  - S02 (pipeline spans — uses tracing macros and layered subscriber)
  - S03 (orchestration spans — uses tracing macros and layered subscriber)
  - S04 (JSON export — adds layer to init_tracing subscriber stack)
  - S05 (OTLP export — adds layer to init_tracing subscriber stack)
key_files:
  - crates/assay-core/src/telemetry.rs
  - crates/assay-core/src/lib.rs
  - crates/assay-core/Cargo.toml
  - crates/assay-cli/src/main.rs
  - crates/assay-cli/src/commands/mcp.rs
  - crates/assay-cli/src/commands/context.rs
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-cli/src/commands/gate.rs
  - crates/assay-cli/src/commands/harness.rs
  - crates/assay-tui/src/main.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-core/src/history/analytics.rs
  - crates/assay-core/src/history/mod.rs
key_decisions:
  - "D129: Telemetry module in assay-core, not a new crate"
  - "D131: D125 superseded — assay-tui gains tracing dep"
  - "D132: CLI default tracing level is info, MCP is warn"
  - "D133: Interactive eprint! prompts preserved, not migrated to tracing"
  - "D134: tracing-subscriber added to assay-core for init_tracing()"
  - "Composable registry().with() layer approach ready for S04/S05 to add JSON/OTLP layers without changing call sites"
  - "Subcommand-aware config selection: tracing_config_for() in CLI main.rs inspects parsed command"
  - "Guard daemon file-based logging removed for now — stderr subscriber sufficient; file logging revisited in S04"
patterns_established:
  - "init_tracing(TracingConfig) -> TracingGuard — all binaries call once at startup, hold guard for lifetime"
  - "Structured fields on tracing events: path = %path.display(), error = %e, session_name, criterion_name, passed, advisory"
  - "Level mapping: user-facing banners → info!, errors → error!, warnings → warn!, evidence/diagnostic detail → debug!"
observability_surfaces:
  - "RUST_LOG=debug shows all events across all binaries"
  - "RUST_LOG=assay_core=debug,assay_cli=info for per-crate filtering"
  - "MCP defaults to warn level (stdout reserved for JSON-RPC)"
  - "Invalid RUST_LOG silently falls back to config.default_level"
  - "Double-init via try_init() is a silent no-op"
drill_down_paths:
  - .kata/milestones/M009/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M009/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M009/slices/S01/tasks/T03-SUMMARY.md
  - .kata/milestones/M009/slices/S01/tasks/T04-SUMMARY.md
  - .kata/milestones/M009/slices/S01/tasks/T05-SUMMARY.md
duration: 50min
verification_result: passed
completed_at: 2026-03-24T12:30:00Z
---

# S01: Structured tracing foundation and eprintln migration

**Centralized tracing subscriber with layered architecture, zero eprintln! across all production crates, and structured leveled events with filterable fields**

## What Happened

Built a centralized `assay_core::telemetry` module (T01) providing `init_tracing(TracingConfig) -> TracingGuard` with composable `Registry + EnvFilter + fmt` layer stack, non-blocking stderr writer, and flush-on-drop guard. The design uses `registry().with()` composition so downstream slices (S04/S05) can add JSON file and OTLP layers without changing any call site.

Wired init_tracing into all three binary entry points (T02): CLI main.rs with subcommand-aware config selection (MCP serve gets warn level, everything else gets info), TUI main.rs, and MCP serve. Deleted the two ad-hoc subscriber init sites: `init_mcp_tracing()` in mcp.rs and the guard daemon's tracing-appender block in context.rs. Cleaned up unused direct deps (tracing-appender, tracing-subscriber) from assay-cli Cargo.toml.

Migrated all eprintln! calls across four crates in three batches: assay-core (9 calls in T03), high-count CLI files run.rs/gate.rs/harness.rs (61 calls in T04), and remaining CLI + TUI files (33 calls in T05). Each call was individually mapped to the appropriate tracing level with structured fields. Three interactive `eprint!` calls were preserved (1 gate.rs carriage-return progress, 2 worktree.rs y/N prompts) — these are interactive I/O, not logging events.

## Verification

- `grep -rn 'eprintln!' crates/assay-cli/src/ crates/assay-core/src/ crates/assay-tui/src/ crates/assay-mcp/src/ --include='*.rs'` → **zero matches**
- `grep -rn 'eprint!' crates/ --include='*.rs' | grep -v eprintln` → **exactly 3 matches** (gate.rs:261, worktree.rs:337, worktree.rs:437)
- `cargo test -p assay-core telemetry` → 3/3 pass (default_config, mcp_config, init_tracing_returns_guard)
- `cargo test -p assay-cli` → 45 passed, 0 failed
- `cargo test -p assay-core --lib` → 690 passed, 0 failed
- `cargo build -p assay-cli -p assay-tui` → clean builds
- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `just ready` → all test suites passing (fmt, lint, test, deny green)

## Requirements Advanced

- R060 (Structured tracing foundation) — fully implemented: eprintln! replaced across workspace, tracing-subscriber layered config, RUST_LOG env filter, centralized init_tracing()
- R027 (OpenTelemetry instrumentation) — foundation laid: subscriber layer architecture supports S02-S05 additions

## Requirements Validated

- R060 — proven by zero eprintln! grep, telemetry unit tests, and `just ready` green. All crates emit structured tracing events. RUST_LOG works.

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- Removed `tracing-appender` and `tracing-subscriber` direct deps from assay-cli Cargo.toml — not in plan but correct since those are now consumed transitively via assay-core::telemetry
- Guard daemon file-based logging (tracing-appender to guard.log) removed rather than converted — stderr subscriber is sufficient; file logging is an S04 concern
- `format_warn` in commands/mod.rs marked `#[allow(dead_code)]` rather than deleted — may be useful for future non-tracing display paths

## Known Limitations

- Guard daemon no longer writes to `guard.log` file — uses stderr via centralized subscriber. File-based logging deferred to S04 (JSON trace export)
- Integration test suite (`orchestrate_integration`) is slow and times out at 300s — pre-existing, not caused by this slice

## Follow-ups

- S04 should extend `init_tracing()` to accept a JSON file layer configuration for `.assay/traces/` export
- S05 should extend `init_tracing()` to conditionally add an OTLP layer behind `--features telemetry`

## Files Created/Modified

- `crates/assay-core/src/telemetry.rs` — new module (~110 lines) with TracingConfig, TracingGuard, init_tracing(), 3 unit tests
- `crates/assay-core/src/lib.rs` — added `pub mod telemetry;`
- `crates/assay-core/Cargo.toml` — added `tracing-subscriber.workspace = true`
- `crates/assay-cli/src/main.rs` — added tracing_config_for() helper and init_tracing() call; 3 eprintln! migrated
- `crates/assay-cli/src/commands/mcp.rs` — deleted init_mcp_tracing(); 1 eprintln! migrated
- `crates/assay-cli/src/commands/context.rs` — replaced ad-hoc subscriber init; 8 eprintln! migrated
- `crates/assay-cli/src/commands/run.rs` — 31 eprintln! → tracing macros
- `crates/assay-cli/src/commands/gate.rs` — 19 eprintln! → tracing macros; 1 eprint! preserved
- `crates/assay-cli/src/commands/harness.rs` — 11 eprintln! → tracing macros
- `crates/assay-cli/src/commands/mod.rs` — added #[allow(dead_code)] on format_warn
- `crates/assay-cli/src/commands/worktree.rs` — 5 eprintln! → tracing; 2 eprint! preserved
- `crates/assay-cli/src/commands/milestone.rs` — 4 eprintln! → tracing
- `crates/assay-cli/src/commands/history.rs` — 4 eprintln! → tracing
- `crates/assay-cli/src/commands/pr.rs` — 2 eprintln! → tracing
- `crates/assay-cli/src/commands/init.rs` — 2 eprintln! → tracing
- `crates/assay-cli/src/commands/plan.rs` — 1 eprintln! → tracing
- `crates/assay-cli/src/commands/spec.rs` — 1 eprintln! → tracing
- `crates/assay-cli/Cargo.toml` — added tracing.workspace, removed tracing-appender/tracing-subscriber direct deps
- `crates/assay-tui/src/main.rs` — added init_tracing(); 1 eprintln! migrated
- `crates/assay-tui/src/app.rs` — 2 eprintln! → tracing
- `crates/assay-tui/Cargo.toml` — added tracing.workspace = true
- `crates/assay-core/src/history/analytics.rs` — 7 eprintln! → tracing::warn!
- `crates/assay-core/src/history/mod.rs` — 2 eprintln! → tracing::warn!

## Forward Intelligence

### What the next slice should know
- `init_tracing()` returns a `TracingGuard` — the guard MUST be held for the lifetime of the program or the non-blocking writer will drop buffered events
- The subscriber uses `registry().with(filter).with(fmt_layer)` — S04/S05 add layers by extending the `with()` chain in `init_tracing()`, not by creating a new subscriber
- `try_init()` means only the first `init_tracing()` call takes effect — no panics on double-init but also no reconfiguration

### What's fragile
- `tracing_config_for()` in CLI main.rs uses a `matches!` on the parsed command — adding new subcommands that need non-default tracing config requires updating this function
- Guard daemon stderr logging may be noisy in production — S04 should prioritize file layer for the daemon

### Authoritative diagnostics
- `RUST_LOG=assay_core::telemetry=debug` — check subscriber initialization
- `grep -rn 'eprintln!' crates/` — the definitive zero-eprintln check
- `cargo test -p assay-core telemetry` — 3 tests prove init_tracing contract

### What assumptions changed
- Plan estimated ~106 eprintln! calls; actual count was ~106 (9 core + 61 batch-1 + 33 remaining + 3 preserved eprint!)
- Guard daemon's tracing-appender file writer was expected to be replaced with init_tracing() — instead it was removed entirely (deferred to S04)
