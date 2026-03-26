---
estimated_steps: 4
estimated_files: 4
---

# T04: Update CLI, MCP, and TUI OrchestratorConfig construction sites and run just ready

**Slice:** S02 — LocalFsBackend implementation and orchestrator wiring
**Milestone:** M010

## Description

Completes the wiring by updating all OrchestratorConfig construction sites outside of assay-core (CLI: 3 sites in `run.rs`, MCP: 3 sites in `server.rs`, any TUI references). Accepts any snapshot changes with `INSTA_UPDATE=always`. Runs `just ready` for the authoritative green signal.

## Steps

1. Update `crates/assay-cli/src/commands/run.rs` — 3 `OrchestratorConfig {` construction sites (lines ~376, ~599, ~738):
   - Add `backend: Arc::new(LocalFsBackend::new(assay_dir.clone()))` to each struct literal.
   - Add `use std::sync::Arc;` and `use assay_core::state_backend::LocalFsBackend;` to imports.
   - The `assay_dir` is already available at each site (it's constructed as `cwd.join(".assay")` or similar).

2. Update `crates/assay-mcp/src/server.rs` — 3 `OrchestratorConfig {` construction sites (lines ~2954, ~2996, ~3044):
   - Same pattern: add `backend: Arc::new(LocalFsBackend::new(assay_dir.clone()))`.
   - Add necessary imports at the top of the file.

3. Run `INSTA_UPDATE=always cargo test --workspace` to accept any snapshot changes (the `run_manifest_schema_snapshot` was pre-existing-failing and may need one final update). Review snapshot diffs to confirm only expected changes.

4. Run `just ready` (fmt + lint + test + deny). Fix any remaining issues. Confirm zero `persist_state` references outside `state_backend.rs`.

## Must-Haves

- [ ] All 3 CLI `OrchestratorConfig` construction sites pass a `LocalFsBackend` backend
- [ ] All 3 MCP `OrchestratorConfig` construction sites pass a `LocalFsBackend` backend
- [ ] `cargo test --workspace` — all tests pass
- [ ] `just ready` — green (fmt + lint + test + deny)
- [ ] `grep -rn "persist_state" crates/` — only appears in `state_backend.rs` as an internal detail (or not at all)

## Verification

- `cargo build -p assay-cli` — compiles without error
- `cargo build -p assay-mcp` — compiles without error
- `cargo test --workspace` — all ~1476 tests pass
- `just ready` — green
- `grep -rn "persist_state" crates/assay-core/src/orchestrate/executor.rs crates/assay-core/src/orchestrate/mesh.rs crates/assay-core/src/orchestrate/gossip.rs` — zero matches (or only a private helper in state_backend.rs)

## Observability Impact

- Signals added/changed: None (this task is wiring, not new behavior)
- How a future agent inspects this: `just ready` is the authoritative green signal
- Failure state exposed: Compile errors at construction sites if `backend` field is wrong type

## Inputs

- T03 output: `OrchestratorConfig` with `backend: Arc<dyn StateBackend>` field, all assay-core sites updated
- `crates/assay-cli/src/commands/run.rs` — 3 construction sites needing `backend` field
- `crates/assay-mcp/src/server.rs` — 3 construction sites needing `backend` field

## Expected Output

- `crates/assay-cli/src/commands/run.rs` — all 3 sites updated with backend
- `crates/assay-mcp/src/server.rs` — all 3 sites updated with backend
- `just ready` green — the authoritative done signal for S02
