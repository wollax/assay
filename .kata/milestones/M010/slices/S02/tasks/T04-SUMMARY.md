---
id: T04
parent: S02
milestone: M010
provides:
  - All CLI and MCP OrchestratorConfig construction sites use explicit LocalFsBackend with resolved assay_dir path
key_files:
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - Replaced ..Default::default() with explicit backend field at all 6 construction sites, using the resolved assay_dir path instead of the default relative ".assay" path
patterns_established:
  - OrchestratorConfig construction sites should always provide an explicit backend with the resolved project assay_dir path
observability_surfaces:
  - none (wiring task — no new observability)
duration: 5 min
verification_result: passed
completed_at: 2026-03-26
blocker_discovered: false
---

# T04: Update CLI, MCP, and TUI OrchestratorConfig construction sites and run just ready

**Updated all 6 OrchestratorConfig construction sites (3 CLI, 3 MCP) to use explicit `LocalFsBackend` with resolved `assay_dir` path, achieving `just ready` green.**

## What Happened

Updated `crates/assay-cli/src/commands/run.rs` (3 sites: orchestrated, mesh, gossip) and `crates/assay-mcp/src/server.rs` (3 sites: default, mesh, gossip) to replace `..Default::default()` with explicit `backend: Arc::new(LocalFsBackend::new(assay_dir.clone()))`. Added necessary imports (`std::sync::Arc`, `assay_core::state_backend::LocalFsBackend`) to both files. In the MCP server, moved `assay_dir` construction before the first `OrchestratorConfig` so the variable is available.

Confirmed zero `persist_state` references remain anywhere in the codebase.

## Verification

- `cargo build -p assay-cli -p assay-mcp` — compiles without error
- `cargo test --workspace` — all 1481 tests pass
- `just ready` — green (fmt + lint + test + deny): "All checks passed."
- `grep -rn "persist_state" crates/` — zero matches

### Slice Verification (final task — all must pass)
- `cargo test -p assay-types --test schema_snapshots run_manifest_schema_snapshot` — ✅ pass
- `cargo test -p assay-core --features orchestrate --test state_backend` — ✅ pass
- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — ✅ pass
- `cargo test -p assay-core --features orchestrate --test mesh_integration` — ✅ pass
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — ✅ pass
- `cargo test -p assay-core --features orchestrate --test orchestrate_spans` — ✅ pass
- `cargo test -p assay-core --features orchestrate --test integration_modes` — ✅ pass
- `cargo test --workspace` — ✅ pass (1481 tests)
- `just ready` — ✅ green

## Diagnostics

None — this is a wiring task. Existing diagnostics from T02/T03 (AssayError with path/operation context) remain unchanged.

## Deviations

- In MCP `server.rs`, moved `let assay_dir = cwd.join(".assay")` before the first `OrchestratorConfig` construction (it was previously after). This reorder was necessary so the variable is available for the backend field. No semantic change.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/commands/run.rs` — Added Arc + LocalFsBackend imports, updated 3 OrchestratorConfig sites with explicit backend
- `crates/assay-mcp/src/server.rs` — Added LocalFsBackend import, updated 3 OrchestratorConfig sites with explicit backend, reordered assay_dir construction
