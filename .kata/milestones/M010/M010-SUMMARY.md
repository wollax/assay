---
id: M010
provides:
  - StateBackend trait (7 sync methods, object-safe, Send+Sync) in assay_core::state_backend
  - CapabilitySet flags struct with all()/none() constructors
  - NoopBackend test helper (all capabilities false, all methods no-op)
  - LocalFsBackend with real filesystem persistence for all 7 StateBackend methods
  - Arc<dyn StateBackend> on OrchestratorConfig with manual Clone/Debug impls
  - RunManifest.state_backend: Option<StateBackendConfig> field with backward-compatible serde
  - StateBackendConfig enum (LocalFs / Custom { name, config: Value }) in assay_types, schema-snapshot-locked
  - All persist_state() callsites replaced by backend.push_session_event() across executor/mesh/gossip
  - CapabilitySet capability checks in run_mesh() (supports_messaging) and run_gossip() (supports_gossip_manifest) with graceful degradation and warn! events
  - plugins/smelt-agent/AGENTS.md and three skills (run-dispatch, backend-status, peer-message)
  - D149–D159 decisions documented
key_decisions:
  - D149 — StateBackend is a deliberate, scoped exception to D001 (zero-trait convention)
  - D150 — Trait methods are sync; async backends internalize their runtime (preserves D007)
  - D151 — Box<dyn StateBackend> (superseded by D156: Arc<dyn StateBackend>) in OrchestratorConfig
  - D152 — Tier-1/Tier-2 split: heartbeats and per-tick routing are LocalFsBackend internals
  - D153 — StateBackendConfig as enum with LocalFs + Custom variants
  - D154 — state_backend module feature-gated behind orchestrate
  - D155 — Object-safety compile guard fn _assert_object_safe at module level
  - D156 — Arc<dyn StateBackend> in OrchestratorConfig (supersedes D151 Box)
  - D157 — OrchestratorConfig Default uses placeholder .assay path for LocalFsBackend
  - D158 — persist_state removed from pub(crate) API after backend wiring
  - D159 — Feature-gated RunManifest fields require split schema snapshot tests
patterns_established:
  - Arc<dyn StateBackend> cloned before spawning into thread closures (backend_coord, backend_worker)
  - Feature-gated RunManifest fields need separate schema snapshot tests gated with cfg(feature)
  - NoopBackend as test helper for capability degradation proofs
  - Capability check before optional backend methods: if !backend.capabilities().supports_X { warn!(); skip }
observability_surfaces:
  - backend.capabilities() returns CapabilitySet — all four bool flags inspectable at backend construction time
  - warn! events emitted when supports_messaging or supports_gossip_manifest is false
  - AssayError::io / AssayError::json with path+operation context on every LocalFsBackend I/O failure
  - grep -rn "persist_state" crates/assay-core/src/orchestrate/ returns empty — confirms all writes go through backend
requirement_outcomes:
  - id: R071
    from_status: active
    to_status: validated
    proof: "S01 — StateBackend trait defined with 7 sync methods, object safety proven via _assert_object_safe compile guard, CapabilitySet::all()/none() verified by contract tests, StateBackendConfig serde schema snapshot locked, 1473 workspace tests pass, just ready green"
  - id: R072
    from_status: active
    to_status: validated
    proof: "S02 — backward-compat round-trip tests (manifest without field → None), 16/16 LocalFsBackend contract tests, 5+2+2 integration tests all pass unchanged, zero persist_state references in orchestrate/ src (grep confirmed), just ready green with 1481 tests"
  - id: R073
    from_status: active
    to_status: validated
    proof: "S02 — all 11 persist_state() callsites replaced by config.backend.push_session_event() across executor/mesh/gossip, LocalFsBackend retains filesystem behavior, zero persist_state references in orchestrate/ src confirmed by grep"
  - id: R074
    from_status: active
    to_status: validated
    proof: "S03 — run_mesh() checks supports_messaging before spawning routing thread; run_gossip() checks supports_gossip_manifest before injecting PromptLayer and writing manifests; NoopBackend test helper proves degradation; test_mesh_degrades_gracefully_without_messaging and test_gossip_degrades_gracefully_without_manifest both pass; all existing integration tests pass unchanged; just ready green with 1488 tests"
  - id: R075
    from_status: active
    to_status: validated
    proof: "S04 — plugins/smelt-agent/AGENTS.md (27 lines, ≤60 cap) exists with skill table, MCP tool table, and workflow overview; skills/run-dispatch.md, skills/backend-status.md, skills/peer-message.md all exist with valid YAML frontmatter; tool names (orchestrate_run, orchestrate_status, run_manifest) verified against server.rs; just ready green"
duration: ~2h total (S01: 30min, S02: 52min, S03: 25min, S04: 20min)
verification_result: passed
completed_at: 2026-03-26
---

# M010: Pluggable State Backend

**`StateBackend` trait with `LocalFsBackend` wiring all orchestrator state writes, `CapabilitySet` graceful degradation in Mesh/Gossip modes, and `smelt-agent` plugin — zero regression across 1488 tests, `just ready` green.**

## What Happened

**S01** established the trait API surface: `StateBackend` (7 sync methods, object-safe), `CapabilitySet` flags struct with `all()`/`none()` constructors, `LocalFsBackend` skeleton, and `StateBackendConfig` schema-locked enum (LocalFs + Custom variants) in `assay-types`. Six contract tests proved the API surface. D149 documented the deliberate exception to D001 (zero-trait convention). `just ready` green with 1473 tests.

**S02** replaced all stub method bodies in `LocalFsBackend` with real atomic filesystem persistence (NamedTempFile+rename pattern), added `Arc<dyn StateBackend>` to `OrchestratorConfig` (manual Clone/Debug required for trait object), and replaced all 11 `persist_state()` callsites across executor/mesh/gossip with `config.backend.push_session_event()`. `RunManifest.state_backend` field added behind `#[cfg(feature = "orchestrate")]` with backward-compatible serde. CLI/MCP construction sites updated with explicit `LocalFsBackend::new(assay_dir)`. Schema snapshots split into orchestrate/non-orchestrate variants. `just ready` green with 1481 tests.

**S03** added capability checks to `run_mesh()` and `run_gossip()` — if `supports_messaging` is false, the routing thread is not spawned; if `supports_gossip_manifest` is false, the knowledge manifest PromptLayer injection and all manifest writes are skipped. Both degradation paths emit `warn!` events. `NoopBackend` test helper added to `state_backend.rs` (all capabilities false, all methods no-op). Integration tests `test_mesh_degrades_gracefully_without_messaging` and `test_gossip_degrades_gracefully_without_manifest` prove sessions complete without panic in both degradation scenarios. Three new `NoopBackend` contract tests added to `state_backend.rs`. `just ready` green with 1488 tests.

**S04** created `plugins/smelt-agent/` with `AGENTS.md` (27 lines) and three skills: `run-dispatch.md` (loading RunManifest, configuring StateBackendConfig, dispatching via `orchestrate_run`/`run_manifest`), `backend-status.md` (polling `orchestrate_status`, interpreting `OrchestratorStatus` fields, reading CapabilitySet degradation signals), and `peer-message.md` (outbox/inbox directory convention, roster PromptLayer parsing, message send/receive lifecycle). All tool names verified against `server.rs`. No Rust code changes — pure documentation. `just ready` green.

## Cross-Slice Verification

### Success Criterion 1: `just ready` green with all 1400+ tests passing after every slice
- S01: 1473 tests ✅
- S02: 1481 tests ✅
- S03: 1488 tests ✅
- S04: 1488 tests ✅ (no Rust changes, same count)

### Success Criterion 2: `RunManifest` without `state_backend` deserializes identically to today
- Verified by `backward_compat_manifest_without_state_backend_deserializes_to_none` test (S02)
- `cargo test -p assay-core --features orchestrate --test state_backend backward_compat` — passes ✅

### Success Criterion 3: All orchestrator, mesh, gossip, and checkpoint writes flow through `StateBackend` methods
- `grep -rn "persist_state" crates/assay-core/src/orchestrate/` — empty result ✅
- 5 orchestrate_integration + 2 mesh_integration + 2 gossip_integration tests pass unchanged ✅

### Success Criterion 4: `CapabilitySet` flags are checked; Mesh degrades gracefully if `supports_messaging` is false
- `test_mesh_degrades_gracefully_without_messaging` — passes ✅
- `test_gossip_degrades_gracefully_without_manifest` — passes ✅
- Both tests prove sessions complete and no panic occurs

### Success Criterion 5: `plugins/smelt-agent/AGENTS.md` and at minimum three skills exist
- `ls plugins/smelt-agent/` → `AGENTS.md skills/` ✅
- `ls plugins/smelt-agent/skills/` → `run-dispatch.md backend-status.md peer-message.md` ✅
- `wc -l plugins/smelt-agent/AGENTS.md` → 27 lines (≤60) ✅
- All skills have valid YAML frontmatter with `name` and `description` fields ✅

### Milestone Definition of Done
- [x] `StateBackend` trait, `CapabilitySet`, `LocalFsBackend`, `StateBackendConfig`, `RunManifest.state_backend` all exist and schema-snapshot-locked
- [x] All orchestrator, mesh, gossip, checkpoint writes flow through `StateBackend` methods
- [x] `just ready` is green
- [x] `plugins/smelt-agent/` exists with AGENTS.md and 3 skills
- [x] D149 (StateBackend trait exception to D001) documented in DECISIONS.md
- [x] Existing orchestrate integration tests exercise the full LocalFsBackend code path and pass

## Requirement Changes

- R071: active → validated — StateBackend trait defined, CapabilitySet constructors verified, StateBackendConfig schema locked, 1473 tests pass
- R072: active → validated — backward-compat round-trip tests pass, 16/16 LocalFsBackend contract tests pass, all existing integration tests pass unchanged
- R073: active → validated — zero persist_state references in orchestrate/ src (grep confirmed), all callsites replaced by backend.push_session_event()
- R074: active → validated — capability checks in run_mesh()/run_gossip(), NoopBackend degradation tests prove graceful behavior
- R075: active → validated — plugins/smelt-agent/ with AGENTS.md + 3 skills, all tool names verified against server.rs

## Forward Intelligence

### What the next milestone should know
- `LocalFsBackend` is the sole concrete backend — M011 adds `LinearBackend`, `GitHubBackend`, etc. Each new backend must implement all 7 trait methods; the `NoopBackend` pattern in `state_backend.rs` is the template for stubs.
- `StateBackendConfig::Custom { name, config }` is the extension point for third-party backends — when a named variant is stabilized (e.g. `Linear`), add it to the enum and update schema snapshots (including the split orchestrate/non-orchestrate snapshot tests from D159).
- `Arc<dyn StateBackend>` in `OrchestratorConfig` already handles concurrent read access from worker threads — no locking is needed inside backend methods for read operations; writes should use atomic rename (NamedTempFile pattern).
- The capability check pattern is established: `if !config.backend.capabilities().supports_X { tracing::warn!(...); // skip the optional feature }` — use this in all new backend-touching code.
- `persist_state()` is gone — there is no fallback for direct filesystem writes from orchestrate/ code. Any new state persistence must go through the backend trait.

### What's fragile
- `poll_inbox` reads-then-deletes all inbox files in one pass — if a read succeeds but the delete fails, the message is returned but not cleaned up. No retry or deduplication. Adequate for LocalFsBackend; relevant for high-throughput smelt scenarios (deferred to M011+).
- `OrchestratorConfig::default()` uses `LocalFsBackend::new(".assay")` (relative path). Tests using `..Default::default()` without overriding `backend` will write to `.assay` relative to CWD — harmless in test contexts but confusing.
- The schema snapshot split (D159) means `run_manifest_schema_snapshot` tests must be run with and without `--features orchestrate` to get full coverage. `just ready` covers both, but ad-hoc `cargo test` may miss one variant.

### Authoritative diagnostics
- `grep -rn "persist_state" crates/assay-core/src/orchestrate/` — empty result means all writes go through backend
- `cargo test -p assay-core --features orchestrate --test state_backend` — 19 contract tests; all green means backend API is correct
- `cargo test -p assay-core --features orchestrate --test mesh_integration test_mesh_degrades` — proves capability degradation path works
- `cargo test -p assay-core --features orchestrate --test gossip_integration test_gossip_degrades` — proves gossip degradation path works

### What assumptions changed
- S01 plan assumed `_assert_object_safe` needed no `#[allow(dead_code)]` annotation; clippy required it.
- S01 plan assumed `CapabilitySet` needed only `PartialEq`; clippy's `derive_partial_eq_without_eq` required `Eq` too.
- S02 plan assumed 15 `persist_state` callsites; actual was 11 (3 executor + 3 mesh + 5 gossip).
- S02 plan assumed `Box<dyn StateBackend>` (D151); `OrchestratorConfig::clone()` requirement drove the change to `Arc<dyn StateBackend>` (D156).
- S02 T01 contract test assumed checkpoint backend wrote to `checkpoint.json`; actual output is `checkpoints/latest.md` — corrected in T02.

## Files Created/Modified

- `crates/assay-types/src/state_backend.rs` — new: StateBackendConfig enum with schema registry entry
- `crates/assay-types/src/lib.rs` — added pub mod state_backend and pub use state_backend::StateBackendConfig
- `crates/assay-types/tests/schema_snapshots.rs` — added state_backend_config_schema_snapshot test
- `crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap` — locked JSON Schema snapshot
- `crates/assay-types/src/manifest.rs` — added state_backend: Option<StateBackendConfig> field
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap` — new orchestrate-gated snapshot
- `crates/assay-core/src/state_backend.rs` — new: StateBackend trait, CapabilitySet, LocalFsBackend (real impls), NoopBackend test helper
- `crates/assay-core/src/lib.rs` — added feature-gated pub mod + re-exports including NoopBackend
- `crates/assay-core/src/orchestrate/executor.rs` — added Arc<dyn StateBackend> backend field, manual Clone/Debug, removed persist_state, replaced 3 callsites
- `crates/assay-core/src/orchestrate/mesh.rs` — capability check for supports_messaging, routing thread gated on capability, replaced 3 persist_state callsites
- `crates/assay-core/src/orchestrate/gossip.rs` — capability check for supports_gossip_manifest, PromptLayer injection + manifest writes gated on capability, replaced 5 persist_state callsites
- `crates/assay-core/tests/state_backend.rs` — 16→19 tests: backward-compat round-trips, LocalFsBackend contracts, NoopBackend contract tests
- `crates/assay-core/tests/mesh_integration.rs` — added test_mesh_degrades_gracefully_without_messaging
- `crates/assay-core/tests/gossip_integration.rs` — added test_gossip_degrades_gracefully_without_manifest
- `crates/assay-cli/src/commands/run.rs` — updated 3 OrchestratorConfig sites with explicit LocalFsBackend
- `crates/assay-mcp/src/server.rs` — updated 3 OrchestratorConfig sites with explicit LocalFsBackend, reordered assay_dir
- `plugins/smelt-agent/AGENTS.md` — new: smelt-agent system prompt with skill/MCP tool tables
- `plugins/smelt-agent/skills/run-dispatch.md` — new: RunManifest reading, StateBackendConfig setup, dispatch via MCP tools
- `plugins/smelt-agent/skills/backend-status.md` — new: orchestrate_status interpretation, OrchestratorStatus schema, CapabilitySet degradation
- `plugins/smelt-agent/skills/peer-message.md` — new: outbox/inbox file convention, roster PromptLayer parsing, message lifecycle
