# M010: Pluggable State Backend

**Vision:** Introduce a `StateBackend` trait in `assay-core` that abstracts where orchestrator state goes — local files today, Linear/GitHub/SSH tomorrow. All existing state writes (session transitions, mesh routing, gossip manifest, checkpoints) flow through the trait. A `LocalFsBackend` wraps current behavior so zero tests break. A smelt-agent plugin documents how smelt workers interact with the backend API surface. The abstraction is the foundation; concrete remote backends come in M011+.

## Success Criteria

- `just ready` green with all 1400+ tests passing after every slice
- `RunManifest` without `state_backend` deserializes identically to today (backward-compatible)
- `OrchestratorStatus` writes, mesh outbox/inbox routing, gossip knowledge manifest writes, and checkpoint saves all flow through `StateBackend` methods on `LocalFsBackend`
- `CapabilitySet` flags are checked by the orchestrator before using optional capabilities; Mesh degrades gracefully if `supports_messaging` is false
- `plugins/smelt-agent/AGENTS.md` and at minimum three skills exist, covering run dispatch, backend status queries, and agent-to-agent messaging concepts

## Key Risks / Unknowns

- D001 violation — `StateBackend` is a trait; the project's zero-trait convention is a firm pattern. This is a deliberate, scoped exception that must be justified and documented.
- Async trait surface — backends like Linear need async I/O, but the core is sync (D007). Trait methods must stay sync; async backends own their runtime internally.
- Generic vs trait-object — `Box<dyn StateBackend>` in `OrchestratorConfig` vs `OrchestratorConfig<B: StateBackend>`. Generic is viral; trait object has vtable cost. Decision locks the API surface for all downstream backends.
- `RunManifest` field addition — `deny_unknown_fields` is NOT set on `RunManifest`; confirm before adding field. If it is set, adding `state_backend` with `serde(default)` must not break deserialization of existing manifests without the field.

## Proof Strategy

- D001/trait exception → retire in S01 by defining the trait, having it reviewed (test-first contract), and documenting D149
- Generic vs trait-object → retire in S01 by locking `OrchestratorConfig` shape with a contract test
- `RunManifest` backward-compat → retire in S02 by a round-trip test: manifest without `state_backend` deserializes, runs, produces identical `OrchestratorStatus` to M009 baseline

## Verification Classes

- Contract verification: unit tests on `StateBackend` trait, `CapabilitySet` flags, `LocalFsBackend` method implementations, `StateBackendConfig` serde round-trip, `RunManifest` backward-compat
- Integration verification: existing `tests/orchestrate_integration.rs` passes unchanged; `tests/mesh_integration.rs` and `tests/gossip_integration.rs` exercise backend routing paths
- Operational verification: none (no daemon lifecycle in this milestone)
- UAT / human verification: smelt plugin authoring — a human reads the AGENTS.md and skills and confirms they accurately describe how to use the backend-aware API surface

## Milestone Definition of Done

This milestone is complete only when all are true:

- `StateBackend` trait, `CapabilitySet`, `LocalFsBackend`, `StateBackendConfig`, and `RunManifest.state_backend` field all exist and are schema-snapshot-locked
- All orchestrator, mesh, gossip, and checkpoint writes flow through `StateBackend` methods — no direct filesystem writes from executor/mesh/gossip code (LocalFsBackend encapsulates those)
- `just ready` is green (fmt + lint + test + deny)
- `plugins/smelt-agent/` directory exists with AGENTS.md and skills
- D149 (StateBackend trait exception to D001) is documented in DECISIONS.md
- Final integrated acceptance: existing orchestrate integration tests exercise the full LocalFsBackend code path and pass

## Requirement Coverage

- Covers: R071, R072, R073, R074, R075
- Partially covers: none
- Leaves for later: concrete remote backends (LinearBackend, GitHubBackend, SshSyncBackend) — M011+
- Orphan risks: multi-machine smelt integration testing — UAT only in this milestone

## Slices

- [x] **S01: StateBackend trait and CapabilitySet** `risk:high` `depends:[]`
  > After this: `StateBackend` trait, `CapabilitySet` struct, `StateBackendConfig` enum, and `LocalFsBackend` skeleton exist in `assay-core`; contract tests prove the API surface; `just ready` green (no orchestrator wiring yet).

- [x] **S02: LocalFsBackend implementation and orchestrator wiring** `risk:high` `depends:[S01]`
  > After this: all orchestrator, mesh, gossip, and checkpoint writes flow through `LocalFsBackend`; `RunManifest.state_backend` field added; existing orchestrate integration tests pass unchanged.

- [ ] **S03: CapabilitySet degradation paths** `risk:medium` `depends:[S02]`
  > After this: orchestrator checks `supports_messaging` before mesh routing and degrades gracefully; `supports_gossip` guards knowledge manifest writes; each degradation path has a test proving it produces a clear warning and not a panic.

- [ ] **S04: smelt-agent plugin** `risk:low` `depends:[S02]`
  > After this: `plugins/smelt-agent/AGENTS.md` and skills covering run dispatch, backend status queries, and agent-to-agent messaging exist; a human can read them and understand how to use the backend-aware API surface from a smelt worker.

## Boundary Map

### S01 → S02

Produces:
- `assay_core::state_backend::StateBackend` — trait with sync methods: `push_session_event`, `read_run_state`, `send_message`, `poll_inbox`, `annotate_run`, `save_checkpoint_summary`
- `assay_core::state_backend::CapabilitySet` — flags struct: `supports_messaging: bool`, `supports_gossip_manifest: bool`, `supports_annotations: bool`, `supports_checkpoints: bool`
- `assay_core::state_backend::LocalFsBackend` — skeleton struct (fields: `assay_dir: PathBuf`), all methods returning `Ok(())` or reading from filesystem; `capabilities()` returns all-true
- `assay_types::state_backend::StateBackendConfig` — enum: `LocalFs`, `Custom { name: String, config: serde_json::Value }`; serde round-trip tested
- Contract tests: trait object construction (`Box<dyn StateBackend>`), `CapabilitySet::all()` / `CapabilitySet::none()`, `StateBackendConfig` serde round-trip

Consumes:
- nothing (first slice)

### S02 → S03

Produces:
- `RunManifest.state_backend: Option<StateBackendConfig>` field with `serde(default, skip_serializing_if = "Option::is_none")` — backward-compat round-trip test: manifest without field deserializes and runs
- `OrchestratorConfig.backend: Box<dyn StateBackend>` — orchestrator carries the backend; all `save_orchestrator_status` calls replaced by `backend.push_session_event(...)`
- Mesh routing thread uses `backend.send_message` / `backend.poll_inbox` for Tier 2 coalesced events (Tier 1 per-tick file routing unchanged)
- Gossip coordinator uses `backend.annotate_run` for knowledge manifest path notification
- Checkpoint saves use `backend.save_checkpoint_summary`
- All existing `tests/orchestrate_integration.rs`, `mesh_integration.rs`, `gossip_integration.rs` pass unchanged

Consumes from S01:
- `StateBackend` trait (for `Box<dyn StateBackend>` in OrchestratorConfig)
- `LocalFsBackend` (instantiated as default when `state_backend` is None or `LocalFs`)
- `StateBackendConfig` (for `RunManifest` field type)

### S03 → S04

Produces:
- `OrchestratorConfig` checks `backend.capabilities().supports_messaging` before routing mesh messages; emits `warn!` and no-ops if false
- `OrchestratorConfig` checks `backend.capabilities().supports_gossip_manifest` before writing knowledge manifest path; degrades to omitting the PromptLayer if false
- Two new tests: `test_mesh_degrades_gracefully_without_messaging`, `test_gossip_degrades_gracefully_without_manifest`
- `NoopBackend` test helper (all methods no-op, `capabilities()` returns `CapabilitySet::none()`) for degradation tests

Consumes from S02:
- `CapabilitySet` flags (checked at runtime by orchestrator)
- `backend.capabilities()` method

### S04 → (M011)

Produces:
- `plugins/smelt-agent/AGENTS.md` — system prompt describing how a smelt worker uses assay MCP tools + backend events
- `plugins/smelt-agent/skills/run-dispatch.md` — how to read a RunManifest, configure a backend, and dispatch a run
- `plugins/smelt-agent/skills/backend-status.md` — how to query `read_run_state`, interpret `OrchestratorStatus`, and report back
- `plugins/smelt-agent/skills/peer-message.md` — how to use `send_message` / `poll_inbox` for agent-to-agent coordination across machines

Consumes from S02:
- `StateBackend` API surface (method signatures, CapabilitySet, payload types)
- `OrchestratorStatus`, `SessionStatus`, `MeshStatus` schemas (documented in the skills)
