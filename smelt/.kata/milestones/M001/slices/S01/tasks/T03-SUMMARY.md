---
id: T03
parent: S01
milestone: M001
provides:
  - RuntimeProvider trait with async lifecycle methods (provision/exec/collect/teardown)
  - Expanded SmeltError with Provider, Credential, Config variants plus convenience constructors
  - SmeltConfig loader from .smelt/config.toml with sensible defaults and deny_unknown_fields
  - Opaque ContainerId, ExecHandle, CollectResult types for provider contract
key_files:
  - crates/smelt-core/src/provider.rs
  - crates/smelt-core/src/error.rs
  - crates/smelt-core/src/config.rs
  - crates/smelt-core/src/lib.rs
key_decisions:
  - Used impl Future in trait methods (RPITIT) instead of async_trait macro — Rust 2024 edition supports this natively, avoids the boxing overhead and macro dependency
  - RuntimeProvider requires Send + Sync via trait bound for concurrent session execution
  - SmeltConfig returns defaults when .smelt/config.toml is missing (non-fatal), errors only on malformed files
  - ContainerId display truncates to 12 chars (matching Docker short ID convention)
patterns_established:
  - SmeltError convenience constructors (provider(), provider_with_source(), credential(), config()) for ergonomic error creation
  - Config loading pattern: load from project root -> .smelt/config.toml, missing file = defaults, bad parse = error
observability_surfaces:
  - SmeltError variants carry structured context (operation, field, path, provider) for machine-readable error inspection
  - Provider errors optionally carry a source error chain via #[source]
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: RuntimeProvider trait, error types, and SmeltConfig

**Created RuntimeProvider async trait, expanded SmeltError with Provider/Credential/Config variants, and added SmeltConfig TOML loader with defaults.**

## What Happened

Three new modules added to smelt-core:

1. **provider.rs** — Defines `RuntimeProvider` trait with four async lifecycle methods: `provision()`, `exec()`, `collect()`, `teardown()`. Uses RPITIT (return-position impl trait in trait) instead of async_trait macro since the workspace targets Rust 2024 edition. Also defines `ContainerId` (opaque container handle), `ExecHandle` (running command handle), and `CollectResult` (session output with exit code, stdout/stderr, artifacts).

2. **error.rs** — Expanded from 5 to 8 variants. Added `Provider { operation, message, source? }`, `Credential { provider, message }`, and `Config { path, message }`. Added convenience constructors: `SmeltError::provider()`, `provider_with_source()`, `credential()`, `config()`. Kept all existing Git and Manifest variants unchanged.

3. **config.rs** — `SmeltConfig` loads from `.smelt/config.toml` with fields: `default_image`, `credential_sources`, `default_resources`, `default_timeout`. Uses `deny_unknown_fields`. Missing file returns defaults (non-fatal). 9 unit tests cover all paths.

Updated `lib.rs` to export all new types.

## Verification

- `cargo build --workspace` — zero errors, zero warnings
- `cargo test -p smelt-core` — 58 tests pass (32 git + 17 manifest + 9 config)
- `cargo test --workspace` — all tests pass
- RuntimeProvider trait compiles with Send + Sync bounds and is importable from lib.rs

### Slice-level verification (intermediate — T03 of T04):
- ✅ `cargo test -p smelt-core` — all tests pass
- ⏳ `cargo test -p smelt-cli` — no dry-run integration tests yet (T04)
- ✅ `cargo build --workspace` — zero errors, zero warnings
- ⏳ `cargo run -- run examples/job-manifest.toml --dry-run` — not yet wired (T04)
- ⏳ `cargo run -- run examples/bad-manifest.toml --dry-run` — not yet wired (T04)

## Diagnostics

- All SmeltError variants carry structured fields (operation, field, path, provider) for machine-readable error inspection
- `SmeltError::Provider` optionally carries `#[source]` error chain
- SmeltConfig parse errors include the file path and TOML parse error detail

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/provider.rs` — new: RuntimeProvider trait, ContainerId, ExecHandle, CollectResult
- `crates/smelt-core/src/error.rs` — expanded: added Provider, Credential, Config variants with convenience constructors
- `crates/smelt-core/src/config.rs` — new: SmeltConfig loader with 9 unit tests
- `crates/smelt-core/src/lib.rs` — updated: exports for config, provider modules and key types
