---
id: S02
parent: M010
milestone: M010
provides:
  - "RunManifest.state_backend: Option<StateBackendConfig> field with backward-compatible serde"
  - "OrchestratorConfig.backend: Arc<dyn StateBackend> field with manual Clone/Debug impls"
  - "LocalFsBackend: real filesystem persistence for all 7 StateBackend trait methods"
  - "All persist_state() callsites replaced by backend.push_session_event() across executor/mesh/gossip"
  - "persist_state() function removed entirely from executor.rs"
  - "All CLI and MCP OrchestratorConfig construction sites use explicit LocalFsBackend with resolved assay_dir"
  - "Schema snapshot for orchestrate-gated RunManifest (separate orchestrate vs non-orchestrate snapshots)"
requires:
  - slice: S01
    provides: "StateBackend trait, CapabilitySet, LocalFsBackend skeleton, StateBackendConfig enum"
affects:
  - S03
  - S04
key_files:
  - crates/assay-types/src/manifest.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap
  - crates/assay-core/src/state_backend.rs
  - crates/assay-core/src/orchestrate/executor.rs
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-core/src/orchestrate/gossip.rs
  - crates/assay-core/tests/state_backend.rs
  - crates/assay-core/tests/orchestrate_integration.rs
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - "Separate schema snapshot tests for orchestrate/non-orchestrate feature flag variants — feature-gated RunManifest fields change the schema shape"
  - "Default OrchestratorConfig backend uses LocalFsBackend::new('.assay') — minimizes changes at ~20 ::default() callsites"
  - "persist_state() removed entirely (not made private) — dead code after all callsites replaced"
  - "Manual Clone + Debug impls for OrchestratorConfig — Arc<dyn Trait> requires manual Clone, dyn StateBackend is not Debug"
  - "Arc::clone backend into thread closures before move boundary — required for worker thread closures in mesh/gossip"
  - "CLI/MCP construction sites use explicit LocalFsBackend with resolved assay_dir path (not default .assay)"
patterns_established:
  - "Arc<dyn StateBackend> cloned before spawning into thread closures (backend_coord, backend_worker)"
  - "Feature-gated RunManifest fields need separate schema snapshot tests gated with cfg(feature) and cfg(not(feature))"
  - "OrchestratorConfig construction sites should always provide explicit backend with resolved project assay_dir path"
  - "Atomic tempfile-rename pattern used consistently for all LocalFsBackend write operations"
observability_surfaces:
  - "AssayError::io with path+operation context on every I/O failure in LocalFsBackend"
  - "AssayError::json with path+operation context on serialization/deserialization failure"
  - "state.json at run_dir/state.json confirms push_session_event writes; backend.read_run_state() reads it back"
  - "grep -rn persist_state crates/assay-core/src/orchestrate/ returns empty — confirms no direct filesystem writes remain"
drill_down_paths:
  - .kata/milestones/M010/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M010/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M010/slices/S02/tasks/T03-SUMMARY.md
  - .kata/milestones/M010/slices/S02/tasks/T04-SUMMARY.md
duration: ~52 min total (T01: 10m, T02: 5m, T03: 12m, T04: 5m)
verification_result: passed
completed_at: 2026-03-26
---

# S02: LocalFsBackend implementation and orchestrator wiring

**All orchestrator, mesh, gossip, and checkpoint writes now flow through `LocalFsBackend` via `Arc<dyn StateBackend>` on `OrchestratorConfig`; `RunManifest.state_backend` field added with backward-compatible serde; `just ready` green with 1481 tests passing.**

## What Happened

**T01** added `state_backend: Option<StateBackendConfig>` to `RunManifest` behind `#[cfg(feature = "orchestrate")]` with `serde(default, skip_serializing_if)`. All struct literal construction sites (~15) received `state_backend: None`. Schema snapshots were split into two variants: a non-orchestrate base snapshot and a new `run-manifest-orchestrate-schema` snapshot. Two backward-compat round-trip tests were written (green), plus 3 red-state integration contracts for LocalFsBackend that would turn green in T02.

**T02** replaced all 7 stub method bodies in `LocalFsBackend` with real filesystem persistence using the atomic `NamedTempFile` + `sync_all` + `persist` pattern established in the codebase. `push_session_event` serializes `OrchestratorStatus` to `state.json` atomically; `read_run_state` deserializes it; `save_checkpoint_summary` delegates to `checkpoint::persistence::save_checkpoint`; `send_message`/`poll_inbox` use per-message files in inbox directories; `annotate_run` writes the manifest path to `gossip_manifest_path.txt`. All 16 state_backend contract tests turned green.

**T03** added `backend: Arc<dyn StateBackend>` to `OrchestratorConfig` with manual `Clone` and `Debug` impls (derive is incompatible with `Arc<dyn Trait>`). A `Default` impl creates `LocalFsBackend::new(".assay")` to minimize changes at ~20 `::default()` callsites. All 11 `persist_state()` callsites across executor (3), mesh (3), and gossip (5) were replaced by `config.backend.push_session_event()`. The `persist_state()` function was removed entirely — dead code after all callsites were replaced. Thread closures in mesh and gossip clone the `Arc` before the `move` boundary. All 5 integration test suites passed without change.

**T04** updated all 6 `OrchestratorConfig` construction sites (3 CLI, 3 MCP) to use explicit `Arc::new(LocalFsBackend::new(assay_dir.clone()))` with the resolved project path instead of the default `.assay` relative path. In `server.rs`, `assay_dir` construction was reordered before the first `OrchestratorConfig` to make the variable available. `just ready` was green with all 1481 tests passing.

## Verification

- `cargo test -p assay-types --test schema_snapshots run_manifest_schema_snapshot` — ✅ passed
- `cargo test -p assay-types --features orchestrate --test schema_snapshots` — ✅ 71 passed
- `cargo test -p assay-core --features orchestrate --test state_backend` — ✅ 16 passed
- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — ✅ 5 passed
- `cargo test -p assay-core --features orchestrate --test mesh_integration` — ✅ 2 passed
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — ✅ 2 passed
- `cargo test -p assay-core --features orchestrate --test orchestrate_spans` — ✅ 5 passed
- `cargo test -p assay-core --features orchestrate --test integration_modes` — ✅ 3 passed
- `cargo test --workspace` — ✅ 1481 tests passed
- `just ready` — ✅ green (fmt + lint + test + deny)
- `grep -rn "persist_state" crates/assay-core/src/orchestrate/` — ✅ empty

## Requirements Advanced

- R072 — LocalFsBackend now fully implements all StateBackend methods with real persistence; OrchestratorConfig carries Arc<dyn StateBackend>; all existing integration tests pass unchanged; RunManifest without state_backend deserializes identically to before

- R073 — All Tier-2 event writes (session transitions, checkpoint summaries, gossip manifest annotations) now route through StateBackend methods; Tier-1 per-tick and per-message routing stays inside LocalFsBackend implementation details

## Requirements Validated

- R072 — Proved by: backward-compat round-trip tests (manifest without field → None), 16/16 state_backend contract tests, 5 orchestrate_integration + 2 mesh_integration + 2 gossip_integration tests all passing unchanged, just ready green with 1481 tests

- R073 — Proved by: zero `persist_state` references in orchestrate/ src files (grep confirmed), all callsites verified to use backend.push_session_event(), backend routes through LocalFsBackend which retains filesystem behavior

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- **Schema snapshot split**: Plan said "update existing `run_manifest_schema_snapshot`"; instead created a separate `run_manifest_orchestrate_schema_snapshot` test gated with `#[cfg(feature = "orchestrate")]` and gated the original with `#[cfg(not(feature = "orchestrate"))]`. Necessary because `cargo nextest run --workspace` runs without `--features orchestrate`, producing different schema shapes in each mode.
- **persist_state callsite count**: Plan estimated 15 callsites; actual was 11 (3+3+5). Plan may have counted the function definition or differently-structured call chains.
- **persist_state disposition**: Plan said "make private or remove"; removed entirely since it was dead code after all callsites were replaced.
- **MCP assay_dir reorder**: In `server.rs`, `let assay_dir = cwd.join(".assay")` was moved before the first `OrchestratorConfig` construction (was after). No semantic change — reorder was necessary for availability.

## Known Limitations

- `Default` impl for `OrchestratorConfig` uses a hardcoded `".assay"` relative path for the backend. Tests that use `OrchestratorConfig::default()` without setting a project root will write to `.assay` relative to CWD. All real CLI/MCP sites use explicit `assay_dir` — this is only a test-ergonomics concern.
- `send_message`/`poll_inbox` use a flat per-message file approach. When many messages accumulate in an inbox, poll_inbox reads-and-deletes all of them atomically per message. This is adequate for S02 but may need optimization for high-throughput smelt scenarios (deferred to M011+).

## Follow-ups

- S03: Add CapabilitySet capability checks to orchestrator before mesh routing and gossip manifest injection, with graceful degradation paths and `NoopBackend` tests.
- S04: Write smelt-agent plugin with AGENTS.md and 3 skills documenting the backend-aware API surface.

## Files Created/Modified

- `crates/assay-types/src/manifest.rs` — Added `state_backend: Option<StateBackendConfig>` field
- `crates/assay-types/tests/schema_snapshots.rs` — Added orchestrate-gated snapshot test, gated base test
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap` — New snapshot with state_backend field
- `crates/assay-core/src/state_backend.rs` — All 7 stub methods replaced with real filesystem implementations
- `crates/assay-core/src/orchestrate/executor.rs` — Added backend field, manual Clone/Debug, removed persist_state, replaced 3 callsites
- `crates/assay-core/src/orchestrate/mesh.rs` — Removed persist_state import, replaced 3 callsites
- `crates/assay-core/src/orchestrate/gossip.rs` — Removed persist_state import, replaced 5 callsites
- `crates/assay-core/tests/state_backend.rs` — 5 new tests (2 backward-compat + 3 integration contracts); checkpoint test expectation fixed
- `crates/assay-core/tests/orchestrate_integration.rs` — Added `..Default::default()` to 3 struct-literal sites
- `crates/assay-core/tests/mesh_integration.rs` — Added `state_backend: None` to struct literal
- `crates/assay-core/tests/gossip_integration.rs` — Added `state_backend: None` to struct literal
- `crates/assay-core/tests/orchestrate_spans.rs` — Added `state_backend: None` to struct literal
- `crates/assay-core/tests/integration_modes.rs` — Added `state_backend: None` to struct literals
- `crates/assay-core/tests/pipeline_spans.rs` — Added `state_backend: None` to struct literal
- `crates/assay-core/src/orchestrate/mesh.rs` — Added `state_backend: None` to test helper
- `crates/assay-core/src/orchestrate/gossip.rs` — Added `state_backend: None` to test helper
- `crates/assay-cli/src/commands/run.rs` — Added imports, updated 3 OrchestratorConfig sites with explicit backend
- `crates/assay-mcp/src/server.rs` — Added import, updated 3 OrchestratorConfig sites with explicit backend, reordered assay_dir

## Forward Intelligence

### What the next slice should know
- `OrchestratorConfig.backend` is `Arc<dyn StateBackend>` — cloning into thread closures requires `Arc::clone(&config.backend)` before the `move` boundary, not `config.backend.clone()` (same effect, but the pattern is explicit Arc cloning).
- `capabilities()` is already callable on any backend via the trait — S03 should call `config.backend.capabilities().supports_messaging` directly at the orchestrator dispatch point.
- The `Default` impl for `OrchestratorConfig` provides a `LocalFsBackend::new(".assay")` — tests using `..Default::default()` inherit this. For degradation tests with `NoopBackend`, the test must construct `OrchestratorConfig` with an explicit `backend` field.

### What's fragile
- `poll_inbox` reads-then-deletes all inbox files in one pass — if a read succeeds but the delete fails, the message is returned but not cleaned up. No retry or deduplication exists. Adequate for S02; relevant for S03/S04 if message reliability is a concern.
- The schema snapshot split (orchestrate vs non-orchestrate) means schema tests must be run with and without `--features orchestrate` to get full coverage. The CI `just ready` command runs both, but ad-hoc `cargo test` may miss one.

### Authoritative diagnostics
- `grep -rn "persist_state" crates/assay-core/src/orchestrate/` — empty result confirms all writes go through backend
- `cargo test -p assay-core --features orchestrate --test state_backend` — 16 contract tests; all green means backend is correctly wired
- `state.json` existence under the run_dir after an orchestrated run confirms push_session_event is being called

### What assumptions changed
- Plan assumed 15 persist_state callsites; actual was 11. The count difference likely came from counting the function definition or some indirect invocations that turned out not to exist.
- T01 contract test assumed checkpoint backend wrote to `checkpoint.json`; actual output is `checkpoints/latest.md`. The test was corrected in T02 to match `save_checkpoint()`'s actual output.
