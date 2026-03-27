---
id: T03
parent: S04
milestone: M011
provides:
  - All 6 CLI/MCP construction sites now use backend_from_config() instead of hardcoded LocalFsBackend::new()
  - manifest.state_backend field is respected at runtime with LocalFs fallback for backward compatibility
key_files:
  - crates/assay-cli/Cargo.toml
  - crates/assay-mcp/Cargo.toml
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - Removed unused Arc import from run.rs since backend_from_config returns Arc<dyn StateBackend> directly
patterns_established:
  - manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs) — consistent unwrap pattern at all 6 sites
observability_surfaces:
  - Factory tracing::warn! is now reachable from CLI and MCP paths when unsupported backend config is used (e.g. Ssh without --features ssh)
duration: 10 minutes
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T03: Wire backend_from_config into CLI and MCP construction sites

**Replaced all 6 hardcoded LocalFsBackend::new() callsites in CLI run.rs (3) and MCP server.rs (3) with backend_from_config(), making manifest state_backend field respected at runtime.**

## What Happened

1. Added `assay-backends = { workspace = true }` to both `crates/assay-cli/Cargo.toml` and `crates/assay-mcp/Cargo.toml`.
2. In `crates/assay-cli/src/commands/run.rs`: removed `use assay_core::state_backend::LocalFsBackend` and unused `use std::sync::Arc`, added `use assay_backends::factory::backend_from_config` and `use assay_types::StateBackendConfig`. Replaced 3 `Arc::new(LocalFsBackend::new(...))` callsites in `execute_orchestrated()`, `execute_mesh()`, and `execute_gossip()` with `backend_from_config(manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs), pipeline_config.assay_dir.clone())`.
3. In `crates/assay-mcp/src/server.rs`: same import swap. Replaced 3 `Arc::new(LocalFsBackend::new(...))` callsites (DAG path ~line 2961, Mesh arm ~line 3006, Gossip arm ~line 3058) with the same `backend_from_config(...)` pattern.
4. Ran `cargo fmt` to fix formatting, then `just ready` passed all checks.

## Verification

- `grep -r "LocalFsBackend::new" crates/assay-cli crates/assay-mcp` — returns no output ✓
- `grep -r "use assay_core::state_backend::LocalFsBackend" crates/assay-cli crates/assay-mcp` — returns no output ✓
- `cargo check -p assay-cli` — compiles clean ✓
- `cargo check -p assay-mcp` — compiles clean (pre-existing warnings only) ✓
- `just ready` — all checks passed, 1499 tests run, 1499 passed, 0 skipped ✓

### Slice-level verification (partial — T03 is final task):
- `cargo test -p assay-cli` — all CLI tests pass ✓
- `cargo test -p assay-mcp` — all MCP tests pass ✓
- `just ready` — green with 1499 tests ✓
- Factory tests (T02 scope) — already passing ✓

## Diagnostics

- A manifest TOML with `state_backend = { type = "ssh", ... }` now routes to SshSyncBackend at runtime (if `--features ssh` is compiled in)
- Factory `tracing::warn!` fires if a backend type is configured but the feature is not enabled — observable with RUST_LOG=warn
- Without a `state_backend` field in the manifest, behavior is identical to before (LocalFs default via unwrap_or)

## Deviations

- Removed unused `use std::sync::Arc` from run.rs — the import became dead after switching to `backend_from_config()` which returns `Arc<dyn StateBackend>` directly. Arc is still used in server.rs for other purposes.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/Cargo.toml` — added assay-backends workspace dependency
- `crates/assay-mcp/Cargo.toml` — added assay-backends workspace dependency
- `crates/assay-cli/src/commands/run.rs` — replaced 3 LocalFsBackend::new() callsites with backend_from_config(), removed dead imports
- `crates/assay-mcp/src/server.rs` — replaced 3 LocalFsBackend::new() callsites with backend_from_config(), swapped LocalFsBackend import for factory imports
