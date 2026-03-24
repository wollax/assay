---
id: T01
parent: S01
milestone: M009
provides:
  - assay_core::telemetry module with init_tracing() and TracingGuard
  - TracingConfig with default() and mcp() presets
  - EnvFilter support via RUST_LOG with fallback to configured default level
  - Non-blocking stderr writer with flush-on-drop guard
  - try_init() for safe double-init (no panics)
key_files:
  - crates/assay-core/src/telemetry.rs
  - crates/assay-core/src/lib.rs
  - crates/assay-core/Cargo.toml
key_decisions:
  - "Used tracing_subscriber::registry().with(filter).with(fmt_layer).try_init() — composable layer approach ready for S04/S05 to add JSON/OTLP layers"
patterns_established:
  - "init_tracing(TracingConfig) -> TracingGuard pattern — all binaries call this once at startup, hold the guard for program lifetime"
observability_surfaces:
  - "RUST_LOG env var controls filtering at runtime (e.g. RUST_LOG=debug, RUST_LOG=assay_core=debug,warn)"
  - "Invalid RUST_LOG silently falls back to config.default_level — no crash, no panic"
  - "Double-init is a silent no-op via try_init()"
duration: 8min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T01: Create telemetry module with init_tracing and TracingGuard

**Centralized tracing subscriber in assay-core::telemetry with layered fmt subscriber, EnvFilter, non-blocking stderr writer, and flush-on-drop guard**

## What Happened

Created `crates/assay-core/src/telemetry.rs` with three public items: `TracingConfig` (configuration struct with `default()` and `mcp()` presets), `TracingGuard` (RAII wrapper around `WorkerGuard` for flush-on-drop), and `init_tracing()` (builds a layered fmt subscriber with non-blocking stderr writer and EnvFilter). The implementation uses `tracing_subscriber::registry().with()` composition so future slices (S04/S05) can add JSON file or OTLP layers without changing call sites. Added `tracing-subscriber.workspace = true` to assay-core's Cargo.toml and exported the module from lib.rs.

The existing `init_mcp_tracing()` in `crates/assay-cli/src/commands/mcp.rs` was used as reference for the EnvFilter + fmt + stderr pattern. The new module supersedes that pattern — T04/T05 will migrate call sites to use `init_tracing()`.

## Verification

- `cargo test -p assay-core telemetry`: 3/3 tests pass (test_default_config, test_mcp_config, test_init_tracing_returns_guard)
- `cargo build -p assay-core`: clean build, no errors
- `grep 'pub mod telemetry' crates/assay-core/src/lib.rs`: module is public

### Slice-level checks (partial — T01 scope):
- ✅ `cargo test -p assay-core telemetry` — unit tests pass
- ✅ `cargo build -p assay-core` — compiles
- ⏳ `grep -rn 'eprintln!' ...` — not yet relevant (migration is T02-T05)
- ⏳ `just ready` — deferred to final task
- ⏳ `cargo build -p assay-cli` / `cargo build -p assay-tui` — deferred to tasks that touch those crates

## Diagnostics

- `RUST_LOG=debug` enables all tracing events once a binary calls `init_tracing()`
- Per-crate filtering: `RUST_LOG=assay_core=debug,assay_cli=info`
- Invalid `RUST_LOG` values fall back to `config.default_level` (no crash)
- Double-init via `try_init()` is a silent no-op

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/telemetry.rs` — new module (~110 lines) with TracingConfig, TracingGuard, init_tracing(), and 3 unit tests
- `crates/assay-core/src/lib.rs` — added `pub mod telemetry;`
- `crates/assay-core/Cargo.toml` — added `tracing-subscriber.workspace = true`
