---
id: T03
parent: S02
milestone: M010
provides:
  - "OrchestratorConfig.backend: Arc<dyn StateBackend> field with Default impl"
  - "All persist_state() callsites replaced by backend.push_session_event()"
  - "persist_state() function removed entirely from executor.rs"
key_files:
  - crates/assay-core/src/orchestrate/executor.rs
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-core/src/orchestrate/gossip.rs
  - crates/assay-core/tests/orchestrate_integration.rs
key_decisions:
  - "Kept Default impl for OrchestratorConfig with Arc::new(LocalFsBackend::new(PathBuf::from(\".assay\"))) as default backend — minimizes changes to ~20 ::default() callsites across tests"
  - "Removed persist_state() entirely rather than keeping it private — all callsites now use backend trait, no reason to keep dead code"
  - "Manual Clone + Debug impls for OrchestratorConfig instead of derive — Arc<dyn Trait> requires manual Clone (Arc::clone), and Debug needs custom formatting since dyn StateBackend is not Debug"
patterns_established:
  - "Clone Arc<dyn StateBackend> into thread closures before spawning (backend_coord, backend_worker, backend) — required for move closures in thread::scope workers"
  - "Use ..Default::default() in struct-literal OrchestratorConfig construction sites to forward-compat with new fields"
observability_surfaces:
  - "Backend write failures propagate as AssayError at each callsite (same error semantics as before, now routed through trait object)"
  - "grep -rn persist_state crates/assay-core/src/orchestrate/ returns empty — confirms no direct filesystem writes remain outside the backend"
duration: 12 minutes
verification_result: passed
completed_at: 2026-03-26
blocker_discovered: false
---

# T03: Wired Arc<dyn StateBackend> into OrchestratorConfig and replaced all persist_state callsites

**Added `backend: Arc<dyn StateBackend>` to `OrchestratorConfig`, replaced all 15 `persist_state()` callsites across executor/mesh/gossip with `config.backend.push_session_event()`, and removed the `persist_state` function entirely.**

## What Happened

1. Added `backend: Arc<dyn StateBackend>` field to `OrchestratorConfig` with manual `Clone` (Arc::clone) and `Debug` impls, plus a `Default` that creates `LocalFsBackend::new(".assay")`.

2. Replaced all 3 `persist_state()` callsites in `executor.rs` with `config.backend.push_session_event()`. The worker thread closure clones the Arc before the `move` boundary.

3. Replaced all 3 `persist_state()` callsites in `mesh.rs`. Removed the `persist_state` import. Cloned `config.backend` into the worker thread closure.

4. Replaced all 5 `persist_state()` callsites in `gossip.rs`. Removed the `persist_state` import. Cloned `config.backend` into both the coordinator thread (`backend_coord`) and worker thread (`backend_worker`) closures.

5. Removed the `persist_state()` function entirely from `executor.rs` (along with its now-unused `std::io::Write` and `tempfile::NamedTempFile` imports).

6. Updated all struct-literal `OrchestratorConfig { ... }` construction sites (3 in `orchestrate_integration.rs`, 10 in `executor.rs` embedded tests) to use `..Default::default()` for the new backend field. The ~20 `::default()` sites needed no changes.

## Verification

- `cargo check -p assay-core --features orchestrate` — clean, zero warnings
- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — 5 passed
- `cargo test -p assay-core --features orchestrate --test mesh_integration` — 2 passed
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — 2 passed
- `cargo test -p assay-core --features orchestrate --test orchestrate_spans` — 5 passed
- `cargo test -p assay-core --features orchestrate --test integration_modes` — 3 passed
- `cargo test -p assay-core --features orchestrate --test state_backend` — 16 passed
- `cargo test -p assay-core --features orchestrate` — 881 total tests passed
- `grep -rn "persist_state" crates/assay-core/src/orchestrate/` — empty (confirmed removal)

## Diagnostics

- `config.backend.push_session_event()` errors propagate as `AssayError` with path/operation context at each callsite
- `state.json` files are still written to the same paths — behavior is identical, just routed through the trait object
- Backend errors in best-effort callsites (`let _ = ...`) are silently dropped (same as before)

## Deviations

- Plan said 15 callsites; actual count was 11 (3 executor + 3 mesh + 5 gossip). The plan may have counted differently or included the function definition itself.
- Removed `persist_state` entirely instead of making it private — it was dead code after all callsites were replaced.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/executor.rs` — Added `backend: Arc<dyn StateBackend>` to OrchestratorConfig, manual Clone/Debug impls, removed persist_state function, replaced 3 callsites
- `crates/assay-core/src/orchestrate/mesh.rs` — Removed persist_state import, replaced 3 callsites with backend.push_session_event()
- `crates/assay-core/src/orchestrate/gossip.rs` — Removed persist_state import, replaced 5 callsites with backend.push_session_event()
- `crates/assay-core/tests/orchestrate_integration.rs` — Added ..Default::default() to 3 struct-literal OrchestratorConfig sites
