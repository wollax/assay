# S01: Structured tracing foundation and eprintln migration

**Goal:** All production crates (assay-core, assay-cli, assay-tui, assay-mcp) emit structured `tracing::*` events instead of `eprintln!`. A centralized `init_tracing()` function in `assay-core::telemetry` provides layered subscriber setup with `RUST_LOG` env filter support. Zero `eprintln!` calls remain in production code (except 2 interactive `eprint!` prompts in worktree.rs and 1 `eprint!` progress line in gate.rs which are kept as `eprint!`).
**Demo:** `RUST_LOG=debug assay gate run spec` produces leveled, structured output to stderr. `grep -rn 'eprintln!' crates/assay-{cli,core,tui,mcp}/src/` returns zero matches.

## Must-Haves

- `assay-core::telemetry` module exists with `init_tracing(config) -> TracingGuard` free function
- `TracingGuard` holds a `tracing_appender::non_blocking::WorkerGuard` and flushes on drop
- `init_tracing()` sets up a `fmt` layer writing to stderr with `EnvFilter` (default level `info` for CLI, `warn` for MCP)
- `tracing-subscriber` is a dependency of assay-core (needed for `init_tracing()`)
- `tracing` is a dependency of assay-cli and assay-tui
- Two existing ad-hoc subscriber init sites consolidated: `mcp.rs:init_mcp_tracing()` and `context.rs:630` guard daemon init
- All 106 `eprintln!` calls migrated to `tracing::info!`, `tracing::warn!`, `tracing::error!`, or `tracing::debug!` as appropriate
- 3 `eprint!` calls (gate progress, 2 worktree prompts) kept as-is — they are interactive, not logging
- `just ready` passes with zero warnings

## Proof Level

- This slice proves: contract + operational
- Real runtime required: yes (`just ready`, `cargo build`, manual `RUST_LOG=debug assay gate run` smoke test)
- Human/UAT required: no (zero-eprintln grep + `just ready` is sufficient)

## Verification

- `grep -rn 'eprintln!' crates/assay-cli/src/ crates/assay-core/src/ crates/assay-tui/src/ crates/assay-mcp/src/ --include='*.rs'` returns zero matches
- `cargo test -p assay-core telemetry` — unit tests for init_tracing, TracingGuard, EnvFilter fallback
- `just ready` passes (fmt, lint, test, deny — all green)
- `cargo build -p assay-cli` succeeds (tracing-subscriber wired correctly)
- `cargo build -p assay-tui` succeeds (tracing dep added)

## Observability / Diagnostics

- Runtime signals: All crates emit structured `tracing::*` events with level (info/warn/error/debug). Events carry structured fields where useful (e.g. `spec_slug`, `session_name`, `file_path`).
- Inspection surfaces: `RUST_LOG=debug assay <cmd>` shows all events. `RUST_LOG=assay_core=debug,assay_cli=info` for per-crate control.
- Failure visibility: `init_tracing()` returns `TracingGuard` — if dropped early or double-init attempted, `try_init()` silently succeeds only once (no panic). Invalid `RUST_LOG` values fall back to default level with a stderr warning.
- Redaction constraints: None for this slice (no secrets in tracing events).

## Integration Closure

- Upstream surfaces consumed: `tracing = "0.1"`, `tracing-subscriber = "0.3"`, `tracing-appender = "0.2"` (all already workspace deps)
- New wiring introduced in this slice: `assay-core::telemetry` module with `init_tracing()` called from CLI `main()`, TUI `main()`, and MCP serve. Guard daemon uses `init_tracing()` with file layer config.
- What remains before the milestone is truly usable end-to-end: S02 (pipeline spans), S03 (orchestration spans), S04 (JSON export + CLI), S05 (OTLP export + context propagation)

## Tasks

- [ ] **T01: Create telemetry module with init_tracing and TracingGuard** `est:45m`
  - Why: Foundation for all structured tracing — every other task and downstream slice depends on this module
  - Files: `crates/assay-core/src/telemetry.rs`, `crates/assay-core/src/lib.rs`, `crates/assay-core/Cargo.toml`
  - Do: Create `assay-core::telemetry` module. Add `tracing-subscriber` dep to assay-core. Implement `TracingConfig` struct (default_level, ansi, is_mcp), `TracingGuard` wrapping `WorkerGuard`, `init_tracing(config) -> TracingGuard` using `Registry` + `fmt` layer + `EnvFilter` + `non_blocking` stderr writer. Default level `info` (CLI) or `warn` (MCP). `try_init()` to avoid panics on double init. Add unit tests for guard creation, default config, EnvFilter fallback.
  - Verify: `cargo test -p assay-core telemetry` passes; `cargo build -p assay-core` succeeds
  - Done when: `init_tracing()` returns a valid `TracingGuard`, tests prove `EnvFilter` fallback on invalid `RUST_LOG`, module is `pub` in lib.rs

- [ ] **T02: Wire init_tracing into CLI, TUI, and MCP entry points** `est:30m`
  - Why: Subscribers must be initialized before any tracing macro fires — this wires the centralized init into all three binaries and removes the two ad-hoc subscriber inits
  - Files: `crates/assay-cli/src/main.rs`, `crates/assay-cli/src/commands/mcp.rs`, `crates/assay-cli/src/commands/context.rs`, `crates/assay-tui/src/main.rs`, `crates/assay-tui/Cargo.toml`, `crates/assay-cli/Cargo.toml`
  - Do: Add `tracing.workspace = true` to assay-tui Cargo.toml. Call `init_tracing(TracingConfig::default())` at top of CLI `main()`, hold `_guard` for lifetime. Call `init_tracing(TracingConfig { is_mcp: true, .. })` in MCP serve, remove `init_mcp_tracing()`. Replace guard daemon's ad-hoc subscriber init in `context.rs` with `init_tracing()` call (file layer support deferred — use stderr-only for now, guard daemon tracing-appender file writer is a S04 concern). Call `init_tracing()` in TUI `main()` before event loop. Add `tracing` dep to assay-cli Cargo.toml if not present.
  - Verify: `cargo build -p assay-cli -p assay-tui` succeeds; `init_mcp_tracing` function removed; no `tracing_subscriber::fmt()` direct calls remain outside `telemetry.rs`
  - Done when: All three binaries call `init_tracing()`, zero ad-hoc subscriber inits remain, builds pass

- [ ] **T03: Migrate assay-core eprintln calls to tracing macros** `est:30m`
  - Why: assay-core has 9 eprintln calls (7 in analytics.rs, 2 in history/mod.rs) that must become structured tracing events
  - Files: `crates/assay-core/src/history/analytics.rs`, `crates/assay-core/src/history/mod.rs`
  - Do: Audit each eprintln in assay-core. Map analytics skip warnings → `tracing::warn!`, history load errors → `tracing::warn!` or `tracing::error!`. Add structured fields where useful (e.g. `spec_slug`, `file_path`). Verify no test captures stderr and asserts on exact eprintln strings.
  - Verify: `grep -rn 'eprintln!' crates/assay-core/src/ --include='*.rs'` returns zero; `cargo test -p assay-core` passes
  - Done when: Zero eprintln in assay-core, all existing tests pass

- [ ] **T04: Migrate assay-cli eprintln calls to tracing macros (batch 1: run, gate, harness)** `est:45m`
  - Why: The three highest-count files (run.rs: 31, gate.rs: 19, harness.rs: 11) account for 61 of 94 CLI eprintln calls — this is the bulk migration
  - Files: `crates/assay-cli/src/commands/run.rs`, `crates/assay-cli/src/commands/gate.rs`, `crates/assay-cli/src/commands/harness.rs`
  - Do: Migrate each eprintln individually — user-facing progress banners → `info!`, error reports → `error!`, warnings → `warn!`, diagnostic details → `debug!`. Keep the 1 `eprint!` (no newline) in gate.rs for live criterion progress (it uses carriage returns — not a tracing event). For run.rs result tables (`[✓] name — completed`), use `info!` with structured fields. For gate.rs ANSI formatting, strip ANSI codes from tracing messages (fmt layer handles coloring). Check for any test that captures stderr — update assertions if needed.
  - Verify: `grep -rn 'eprintln!' crates/assay-cli/src/commands/run.rs crates/assay-cli/src/commands/gate.rs crates/assay-cli/src/commands/harness.rs` returns zero; `cargo test -p assay-cli` passes
  - Done when: Zero eprintln in all three files, 1 eprint! in gate.rs preserved, all tests pass

- [ ] **T05: Migrate remaining assay-cli and assay-tui eprintln calls** `est:45m`
  - Why: Finish the migration — 33 remaining eprintln calls across context.rs (8), worktree.rs (7), milestone.rs (4), history.rs (4), main.rs (3), pr.rs (2), init.rs (2), plan.rs (1), spec.rs (1), mcp.rs (1), plus 3 in assay-tui
  - Files: `crates/assay-cli/src/commands/context.rs`, `crates/assay-cli/src/commands/worktree.rs`, `crates/assay-cli/src/commands/milestone.rs`, `crates/assay-cli/src/commands/history.rs`, `crates/assay-cli/src/main.rs`, `crates/assay-cli/src/commands/pr.rs`, `crates/assay-cli/src/commands/init.rs`, `crates/assay-cli/src/commands/plan.rs`, `crates/assay-cli/src/commands/spec.rs`, `crates/assay-cli/src/commands/mcp.rs`, `crates/assay-tui/src/app.rs`, `crates/assay-tui/src/main.rs`
  - Do: Migrate all remaining eprintln calls. Guard daemon messages in context.rs → `info!`/`error!`. Worktree warnings → `warn!`. Keep 2 `eprint!` interactive prompts in worktree.rs. Milestone/history/pr/init/plan/spec errors → `error!`. main.rs errors → `error!`. mcp.rs tracing init failure → `error!` (bootstrapping issue — use stderr fallback if subscriber not yet initialized). assay-tui app.rs cycle status/config warnings → `warn!`. assay-tui main.rs gh-not-found → `warn!` (D131 supersedes D125).
  - Verify: `grep -rn 'eprintln!' crates/assay-cli/src/ crates/assay-core/src/ crates/assay-tui/src/ crates/assay-mcp/src/ --include='*.rs'` returns zero; `just ready` passes
  - Done when: Zero eprintln in all 4 crates, 3 eprint! calls preserved (1 gate.rs, 2 worktree.rs), `just ready` green

## Files Likely Touched

- `crates/assay-core/src/telemetry.rs` (new)
- `crates/assay-core/src/lib.rs`
- `crates/assay-core/Cargo.toml`
- `crates/assay-cli/src/main.rs`
- `crates/assay-cli/src/commands/mcp.rs`
- `crates/assay-cli/src/commands/context.rs`
- `crates/assay-cli/src/commands/run.rs`
- `crates/assay-cli/src/commands/gate.rs`
- `crates/assay-cli/src/commands/harness.rs`
- `crates/assay-cli/src/commands/worktree.rs`
- `crates/assay-cli/src/commands/milestone.rs`
- `crates/assay-cli/src/commands/history.rs`
- `crates/assay-cli/src/commands/pr.rs`
- `crates/assay-cli/src/commands/init.rs`
- `crates/assay-cli/src/commands/plan.rs`
- `crates/assay-cli/src/commands/spec.rs`
- `crates/assay-cli/Cargo.toml`
- `crates/assay-tui/src/main.rs`
- `crates/assay-tui/src/app.rs`
- `crates/assay-tui/Cargo.toml`
- `crates/assay-core/src/history/analytics.rs`
- `crates/assay-core/src/history/mod.rs`
