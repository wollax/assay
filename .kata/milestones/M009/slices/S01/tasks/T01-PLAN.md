---
estimated_steps: 5
estimated_files: 3
---

# T01: Create telemetry module with init_tracing and TracingGuard

**Slice:** S01 — Structured tracing foundation and eprintln migration
**Milestone:** M009

## Description

Create the `assay-core::telemetry` module — the centralized subscriber initialization that all binaries (CLI, TUI, MCP) will call. This module provides `init_tracing(config) -> TracingGuard` with a layered `fmt` subscriber writing to stderr, `EnvFilter` support via `RUST_LOG`, and a non-blocking writer with flush-on-drop guard. The architecture is designed for S04/S05 to add JSON file and OTLP layers without changing the call sites.

## Steps

1. Add `tracing-subscriber = { workspace = true }` to `crates/assay-core/Cargo.toml` dependencies.
2. Create `crates/assay-core/src/telemetry.rs` with:
   - `TracingConfig` struct: `default_level: &str` (default `"info"`), `ansi: bool` (default `true`), `with_target: bool` (default `false`).
   - `TracingGuard` struct wrapping `tracing_appender::non_blocking::WorkerGuard`.
   - `init_tracing(config: TracingConfig) -> TracingGuard` free function that:
     - Creates `EnvFilter` from `RUST_LOG` env var, falling back to `config.default_level` on parse error.
     - Creates a `non_blocking` stderr writer via `tracing_appender::non_blocking(std::io::stderr())`.
     - Builds a `tracing_subscriber::fmt` layer with the non-blocking writer, env filter, ansi config, stderr target.
     - Uses `tracing_subscriber::registry().with(layer).try_init()` (not `init()`) to avoid panics on double-init.
     - Returns `TracingGuard` holding the `WorkerGuard`.
   - Provide `TracingConfig::default()` (level=info, ansi=true, with_target=false) and `TracingConfig::mcp()` (level=warn, ansi=false, with_target=false).
3. Add `pub mod telemetry;` to `crates/assay-core/src/lib.rs`.
4. Write unit tests in `telemetry.rs`:
   - `test_default_config` — verifies `TracingConfig::default()` has expected field values.
   - `test_mcp_config` — verifies `TracingConfig::mcp()` has `warn` level and `ansi=false`.
   - `test_init_tracing_returns_guard` — calls `init_tracing(TracingConfig::default())` and confirms guard is returned (single-init test).
5. Run `cargo test -p assay-core telemetry` and `cargo build -p assay-core` to verify.

## Must-Haves

- [ ] `crates/assay-core/src/telemetry.rs` exists with `TracingConfig`, `TracingGuard`, `init_tracing()` 
- [ ] `tracing-subscriber` is a dependency of assay-core
- [ ] `init_tracing()` uses `try_init()` (not `init()`) — safe for double-init
- [ ] `EnvFilter` falls back to `config.default_level` on invalid `RUST_LOG`
- [ ] `TracingGuard` holds `WorkerGuard` for flush-on-drop
- [ ] Unit tests pass for config creation and guard initialization

## Verification

- `cargo test -p assay-core telemetry` — all tests pass
- `cargo build -p assay-core` — no compilation errors
- `grep 'pub mod telemetry' crates/assay-core/src/lib.rs` — module is public

## Observability Impact

- Signals added/changed: `init_tracing()` establishes the subscriber that all future `tracing::*` calls will emit to. This is the foundational observability primitive.
- How a future agent inspects this: `RUST_LOG=debug` enables all events; per-crate filtering via `RUST_LOG=assay_core=debug,assay_cli=info`.
- Failure state exposed: Invalid `RUST_LOG` falls back to default level (no crash). Double-init is a silent no-op (no panic).

## Inputs

- Workspace deps: `tracing = "0.1"`, `tracing-subscriber = "0.3" [fmt, env-filter]`, `tracing-appender = "0.2"` (already defined)
- Existing pattern: `crates/assay-cli/src/commands/mcp.rs:init_mcp_tracing()` (reference for EnvFilter + fmt + stderr)
- D129: telemetry module lives in assay-core

## Expected Output

- `crates/assay-core/src/telemetry.rs` — new module (~80-120 lines) with TracingConfig, TracingGuard, init_tracing()
- `crates/assay-core/src/lib.rs` — gains `pub mod telemetry;` line
- `crates/assay-core/Cargo.toml` — gains `tracing-subscriber.workspace = true`
