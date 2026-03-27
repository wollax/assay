---
estimated_steps: 5
estimated_files: 4
---

# T03: Wire backend_from_config into CLI and MCP construction sites

**Slice:** S04 — SshSyncBackend and CLI/MCP factory wiring
**Milestone:** M011

## Description

Add `assay-backends` as a dependency to `assay-cli` and `assay-mcp`. Replace all 6 hardcoded `LocalFsBackend::new(...)` callsites in `run.rs` (3 sites) and `server.rs` (3 sites) with `backend_from_config(manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs), assay_dir.clone())`. Remove `use assay_core::state_backend::LocalFsBackend` imports from both files. Run `just ready` to confirm zero regression. This is the final assembly step that makes manifest `state_backend` fields actually respected at runtime.

## Steps

1. Add `assay-backends = { workspace = true }` to `[dependencies]` in `crates/assay-cli/Cargo.toml`. Add the same to `crates/assay-mcp/Cargo.toml`.

2. In `crates/assay-cli/src/commands/run.rs`:
   - Remove the import: `use assay_core::state_backend::LocalFsBackend;`
   - Add imports: `use assay_backends::factory::backend_from_config;` and `use assay_types::StateBackendConfig;` (check if `StateBackendConfig` is already imported via `assay_types::*` or explicitly — add only what's missing)
   - Find all 3 occurrences of `Arc::new(LocalFsBackend::new(pipeline_config.assay_dir.clone()))` and replace each with:
     ```rust
     backend_from_config(
         manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs),
         pipeline_config.assay_dir.clone(),
     )
     ```
     The three callsites are in `execute_orchestrated()` (~line 381), `execute_mesh()` (~line 605), and `execute_gossip()` (~line 745). Each function receives `manifest: &assay_types::RunManifest` as a parameter — the field access is valid.

3. In `crates/assay-mcp/src/server.rs`:
   - Remove the import: `use assay_core::state_backend::LocalFsBackend;`
   - Add imports: `use assay_backends::factory::backend_from_config;` and `use assay_types::StateBackendConfig;` (or verify they're accessible via existing wildcard imports)
   - Find all 3 occurrences of `Arc::new(LocalFsBackend::new(assay_dir.clone()))` and replace each with:
     ```rust
     backend_from_config(
         manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs),
         assay_dir.clone(),
     )
     ```
     The three callsites are: the main DAG `orchestrate_run` path (~line 2960), the `Mesh` mode arm (~line 3002), and the `Gossip` mode arm (~line 3051). Each callsite has `manifest` in scope (it's the loaded `RunManifest`). Verify `manifest` is accessible at each site — if the manifest was moved into a closure, clone `manifest.state_backend` before the closure.

4. Run `cargo check -p assay-cli --features orchestrate` and `cargo check -p assay-mcp` — fix any remaining import errors (e.g. `Arc` may already be imported, `StateBackendConfig` may need full path if not in scope).

5. Run `just ready` and verify: (a) all tests pass, (b) `grep -r "LocalFsBackend::new" crates/assay-cli crates/assay-mcp` returns no matches.

## Must-Haves

- [ ] `assay-backends = { workspace = true }` added to both `assay-cli/Cargo.toml` and `assay-mcp/Cargo.toml`
- [ ] Zero occurrences of `LocalFsBackend::new` in `run.rs` and `server.rs` after the change
- [ ] `use assay_core::state_backend::LocalFsBackend` removed from both files (no dead import)
- [ ] `manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs)` pattern used at all 6 sites — backward-compatible for manifests without `state_backend` field
- [ ] `just ready` green with 1499+ tests — zero regression

## Verification

- `grep -r "LocalFsBackend::new" crates/assay-cli crates/assay-mcp` — returns no output
- `cargo test -p assay-cli --features orchestrate` — all CLI tests pass
- `cargo test -p assay-mcp` — all MCP tests pass
- `just ready` — green with 1499+ tests

## Observability Impact

- Signals added/changed: existing `tracing::warn!` from `backend_from_config()` is now reachable from CLI and MCP paths when an unsupported backend config is used (e.g. `Ssh` variant without `--features ssh`) — previously these code paths were unreachable since the factory wasn't called
- How a future agent inspects this: manifest TOML with `state_backend = { type = "ssh", ... }` now routes to SshSyncBackend at runtime; the factory's tracing::warn! fires if the feature is not enabled
- Failure state exposed: factory fallback logic (tracing::warn + NoopBackend) becomes active when feature is not compiled in — previously all calls unconditionally used LocalFsBackend

## Inputs

- `crates/assay-cli/src/commands/run.rs` — 3 callsites to replace; `manifest` is in scope at each
- `crates/assay-mcp/src/server.rs` — 3 callsites to replace; `manifest` may need to be cloned if it was moved into a closure before the construction site
- `crates/assay-backends/src/factory.rs` (T02 output) — `backend_from_config()` fully implemented
- `crates/assay-types/src/manifest.rs` — confirms `RunManifest.state_backend: Option<StateBackendConfig>`

## Expected Output

- `crates/assay-cli/Cargo.toml` — assay-backends dep added
- `crates/assay-mcp/Cargo.toml` — assay-backends dep added
- `crates/assay-cli/src/commands/run.rs` — 3 callsites replaced, LocalFsBackend import removed
- `crates/assay-mcp/src/server.rs` — 3 callsites replaced, LocalFsBackend import removed
- `just ready` green with 1499+ tests
