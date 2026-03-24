# S01: Structured tracing foundation and eprintln migration — Research

**Date:** 2026-03-24

## Summary

S01 replaces ~106 `eprintln!` calls across 4 crates (assay-cli, assay-core, assay-tui, and one stray in assay-mcp's init fallback) with `tracing::*` macros, and establishes a centralized `init_tracing()` function in a new `assay-core::telemetry` module. The workspace already has `tracing = "0.1"` and `tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }` in workspace deps. assay-core already depends on `tracing`; assay-cli already depends on `tracing-subscriber`. Two existing subscriber init sites exist (MCP serve and context guard) that must be consolidated.

The primary risk is that many CLI `eprintln!` calls are **user-facing progress output** (gate run progress with carriage returns and ANSI codes, orchestration phase banners, session result tables) — not diagnostic logging. These must remain on stderr and be visible at the default log level. The fmt subscriber writes to stderr by default, so the transport is preserved, but the output format changes from raw text to structured `tracing` events. The key decision is mapping each call to the right tracing level: user-facing progress → `info` or `warn`, internal diagnostics → `debug`, errors → `error`.

## Recommendation

1. Create `assay-core::telemetry` module with `init_tracing(config) -> TracingGuard` free function. The initial implementation (S01) sets up a single `fmt` layer with `EnvFilter` support. S04/S05 will add JSON file and OTLP layers to this same function.
2. `TracingGuard` holds a `tracing_appender::non_blocking::WorkerGuard` (already a workspace dep) to flush on drop.
3. Add `tracing` dep to assay-cli and assay-tui. Add `tracing-subscriber` dep to assay-core (needed for `init_tracing()`).
4. Call `init_tracing()` early in CLI `main()` and TUI `main()`. Remove the two existing ad-hoc subscriber inits (MCP serve, context guard).
5. Migrate eprintln calls file-by-file, categorizing each as `info!`, `warn!`, `error!`, or `debug!`.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Layered subscriber composition | `tracing_subscriber::Registry` + `.with()` layers | Already in workspace deps; supports adding JSON/OTel layers later without changing init site |
| Non-blocking writer with flush guard | `tracing_appender::non_blocking()` → `WorkerGuard` | Already a workspace dep (`tracing-appender = "0.2"`); ensures buffered events flush on drop |
| Env-based log level filtering | `tracing_subscriber::EnvFilter` | Already enabled via `features = ["env-filter"]`; `RUST_LOG` convention preserved |
| Per-layer filtering | `tracing_subscriber::filter::Targets` or per-layer `EnvFilter` | Built into tracing-subscriber; needed in S04/S05 when fmt and JSON layers want different levels |

## Existing Code and Patterns

- `crates/assay-cli/src/commands/mcp.rs:33-45` — `init_mcp_tracing()`: creates `EnvFilter` (default `warn`), `fmt()` to stderr, `with_ansi(false)`. **Must be replaced** by centralized `init_tracing()`.
- `crates/assay-cli/src/commands/context.rs:630-633` — Guard daemon tracing init: uses `tracing_subscriber::fmt()` with `non_blocking` writer from `tracing-appender`. **Must be replaced** or delegated to `init_tracing()` with a log-file option.
- `crates/assay-core/src/guard/daemon.rs` — Uses `tracing::{error, info, warn}` with structured fields. **Good pattern to follow** for migration.
- `crates/assay-mcp/src/server.rs` — ~25 `tracing::warn/info` calls already using structured fields. **No migration needed** — already correct.
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — ~10 `tracing::*` calls. **No migration needed**.
- `crates/assay-core/src/work_session.rs` — ~8 `tracing::*` calls. **No migration needed**.
- `crates/assay-core/src/worktree.rs` — ~5 `tracing::*` calls. **No migration needed**.

### eprintln Distribution (106 total across production code)

| File | Count | Category |
|------|-------|----------|
| `assay-cli/src/commands/run.rs` | 31 | User-facing: phase banners, session results, merge summaries |
| `assay-cli/src/commands/gate.rs` | 19 | User-facing: criterion progress with ANSI/carriage returns |
| `assay-cli/src/commands/harness.rs` | 11 | User-facing: file listings, diff output |
| `assay-cli/src/commands/context.rs` | 8 | Mixed: guard startup info + error reporting |
| `assay-core/src/history/analytics.rs` | 7 | Diagnostic: skip warnings during aggregation |
| `assay-cli/src/commands/worktree.rs` | 7 | Mixed: warnings + interactive prompts |
| `assay-cli/src/commands/milestone.rs` | 4 | Error: serialization/domain errors |
| `assay-cli/src/commands/history.rs` | 4 | Error/info: missing project, suggestions |
| `assay-cli/src/main.rs` | 3 | Error: no project, help errors |
| `assay-tui/src/app.rs` | 2 | Diagnostic: cycle status / config load warnings |
| `assay-tui/src/main.rs` | 1 | Diagnostic: gh CLI not found warning |
| Others (6 files) | 9 | Mixed: warnings, errors, info |

### Special Cases

- **`gate.rs` progress display**: Uses `eprint!` (no newline) + `\r\x1b[K` carriage return for live criterion status updates. These are interactive TUI-like progress, not logging. **Decision needed:** keep as `eprint!` (not traceable) or convert to `info!` (loses carriage return semantics). Recommendation: convert to `info!` — the fmt subscriber renders to stderr with newlines; the carriage-return progress UX is a nice-to-have that tracing's fmt layer doesn't support. The `--verbose` flag already controls evidence display.
- **`worktree.rs` interactive prompts**: `eprint!("Remove all? [y/N] ")` — these are interactive input prompts. **Keep as `eprint!`** — they are not logging events. (Only 2 occurrences.)
- **`run.rs` result tables**: Structured output like `[✓] name — completed`. These are user-facing summaries. Convert to `info!` with structured fields.

## Constraints

- **tracing-subscriber must go to assay-core** for `init_tracing()` to live there. Currently only assay-cli has this dep. Adding it to assay-core is acceptable since core already depends on `tracing` and `tracing-appender`.
- **assay-tui needs `tracing` dep** (D131 explicitly supersedes D125). Add `tracing.workspace = true` to assay-tui Cargo.toml.
- **assay-types must NOT gain tracing dep** — keep it dependency-free per existing convention.
- **Default `RUST_LOG` level for CLI**: Must be `info` (not `warn`) so user-facing progress output is visible by default. MCP server stays at `warn` default (stdout reserved for JSON-RPC).
- **Subscriber init must happen before any tracing macro call** — call `init_tracing()` at the very top of `main()`.
- **`TracingGuard` must be held for the lifetime of `main()`** — dropping it early flushes and drops the subscriber.
- **The fmt layer must write to stderr** — stdout is reserved for JSON output in `--json` modes and for JSON-RPC in MCP.
- **Guard daemon has a special log-file requirement** — context guard uses `tracing-appender` for file logging. `init_tracing()` needs an option to add a file writer layer, or the guard can call `init_tracing()` with a config that includes file output.

## Common Pitfalls

- **Mapping all eprintln→info loses error semantics** — `eprintln!("Error: {e}")` should map to `tracing::error!`, not `tracing::info!`. Audit each call individually, not bulk-replace.
- **Missing subscriber in tests** — Tests that trigger `tracing::*` calls without a subscriber silently drop events (acceptable). But tests that *assert* on stderr output will break. Check for any test that captures stderr and asserts on eprintln output.
- **Non-blocking writer drops events on crash** — `WorkerGuard` must be held. If the guard is dropped early (e.g., in an early `return`), buffered events are lost. Hold it in the `main()` scope.
- **`eprint!` (no newline) cannot become a tracing macro** — tracing events always produce complete records. The 3 `eprint!` calls (gate progress, worktree prompts) need special handling.
- **EnvFilter parsing failure** — If `RUST_LOG` contains invalid syntax, `EnvFilter::try_from_default_env()` returns `Err`. The existing MCP code handles this with a fallback; `init_tracing()` should do the same.
- **Double subscriber init** — If `init_tracing()` is called twice (e.g., MCP serve also calls it), `try_init()` silently succeeds only once. Use `try_init()`, not `init()`, to avoid panics.

## Open Risks

- **User-visible output format change**: Converting `eprintln!("Phase 1: Executing sessions...")` to `tracing::info!("Phase 1: Executing sessions...")` changes the output format from bare text to something like `2026-03-24T10:00:00.000Z INFO assay_cli::commands::run: Phase 1: Executing sessions...`. This may surprise CLI users. Mitigation: use `fmt::format::FmtSpan::NONE` and a compact format (`.compact()` or custom format without timestamps for non-verbose mode). **This is the biggest UX risk of S01.**
- **Test breakage from stderr format change**: Any test that captures stderr and matches on exact strings will break. Audit `assay-cli` and `assay-core` tests.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| tracing/tracing-subscriber | wshobson/agents@distributed-tracing (3.3K installs) | available — general distributed tracing skill, not Rust-specific |
| opentelemetry | bobmatnyc/claude-mpm-skills@opentelemetry (283 installs) | available — relevant for S05, not S01 |

No installed skills are directly relevant to S01's Rust `tracing` crate migration. The available skills are generic distributed-tracing guides, not Rust-specific. The `tracing` crate is well-documented and the workspace already has working examples (daemon.rs, server.rs).

## Sources

- Codebase grep: 106 eprintln! calls across 17 files in 4 crates
- Existing tracing usage: ~63 tracing::* calls in assay-core + assay-mcp (no migration needed)
- Two existing subscriber init sites: `mcp.rs:init_mcp_tracing()`, `context.rs:630`
- Workspace Cargo.toml: `tracing = "0.1"`, `tracing-subscriber = "0.3" [fmt, env-filter]`, `tracing-appender = "0.2"`
- Decisions register: D007 (sync core), D125→D131 (assay-tui gains tracing), D129 (telemetry module in assay-core)
