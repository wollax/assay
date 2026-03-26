# M010: Pluggable State Backend — Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

## Project Description

Assay is a spec-driven development platform. Multi-agent orchestrated runs (DAG/Mesh/Gossip) currently write all state — `OrchestratorStatus`, mesh outbox/inbox files, gossip knowledge manifest, team checkpoints — to the local filesystem under `.assay/orchestrator/<run_id>/`. Smelt (a sibling infrastructure layer that receives Assay manifests and executes them across multiple machines) dispatches workers via SSH and currently uses SCP to push state back to the controller. This works but hardwires a single transport and a single observer.

## Why This Milestone

The question behind this milestone: what happens when state doesn't live on a git-tracked filesystem? The prompt came from wanting to use Linear for run observability, GitHub Issues as a discussion/annotation channel, or Azure DevOps for enterprise integration — and from the realisation that the SCP-based state return from smelt is a transport detail that shouldn't be load-bearing.

The answer is a `StateBackend` abstraction in `assay-core` with a tiered capability model:

- **Tier 1 (fast path, always local):** Heartbeats, mesh message routing, per-tick gossip updates. File-backed. No backend API can match sub-second latency here.
- **Tier 2 (slow path, pluggable):** Session state transitions, run phase changes, checkpoint summaries, coalesced agent-to-agent messages. Fire at natural lifecycle boundaries. Any backend can handle this.

Workers push Tier 2 events directly to the configured backend (no SCP hop). The controller reads from the backend. The filesystem backend is the default and changes nothing for single-machine runs.

## User-Visible Outcome

### When this milestone is complete, the user can:
- Run a multi-machine smelt job and watch session transitions appear in Linear (if configured), without SCP back to the controller
- Use the local filesystem backend and see zero behavioral regression from M001–M009
- Add a custom backend by implementing `StateBackend` and registering it in the `RunManifest`

### Entry point / environment
- Entry point: `RunManifest.state_backend` field (or `assay run --state-backend <name>`)
- Environment: Rust codebase, test suite, CLI
- Live dependencies: None for LocalFs. Linear API key for LinearBackend (UAT only in this milestone)

## Completion Class

- Contract complete means: `StateBackend` trait defined, `CapabilitySet` flags struct, `LocalFsBackend` wrapping existing persistence, orchestrator routes all state writes through the trait, all 1400+ tests pass
- Integration complete means: `LocalFsBackend` round-trips identical state to what M009 produced; existing orchestrate integration tests exercise the backend interface
- Operational complete means: N/A for this milestone (no daemon lifecycle)

## Final Integrated Acceptance

To call this milestone complete:
- All orchestrate integration tests pass unchanged with `LocalFsBackend` (no observable regression)
- `OrchestratorStatus` writes, mesh outbox/inbox routing, gossip knowledge manifest writes, and team checkpoint saves all flow through `StateBackend` methods
- A `RunManifest` without a `state_backend` field defaults to `LocalFsBackend` (backward-compatible)
- The `smelt-agent` plugin (AGENTS.md + skills) is authored and covers the backend-aware API surface

## Risks and Unknowns

- **Zero-trait convention (D001)** — the codebase explicitly avoids trait abstractions. `StateBackend` *is* a trait. This is a deliberate, scoped exception to D001 because the whole point is a pluggable interface. Decision must be documented and the trait must be minimal.
- **Capability degradation paths** — if a backend doesn't support messaging, Mesh mode becomes file-only for routing. This must be explicit and tested, not silent.
- **Async trait leakage** — many backends (Linear, GitHub) will need async I/O. The core is sync (D007). The trait methods must be sync; async backends wrap a runtime internally (same pattern as D143/OTLP).
- **Schema stability** — `RunManifest` gains a `state_backend` field. `deny_unknown_fields` is not on RunManifest; verify the default serialization doesn't break existing manifests.

## Existing Codebase / Prior Art

- `crates/assay-core/src/checkpoint/persistence.rs` — atomic-write pattern (NamedTempFile+rename), the model for all Tier 1 state writes
- `crates/assay-core/src/orchestrate/executor.rs` — where `OrchestratorStatus` is written; this is the primary callsite to route through the backend
- `crates/assay-core/src/orchestrate/mesh.rs` — mesh outbox/inbox polling; routing thread to route through backend messaging capability
- `crates/assay-core/src/orchestrate/gossip.rs` — knowledge manifest writes; coordinator to route through backend
- `crates/assay-types/src/orchestrate.rs` — `OrchestratorStatus`, `SessionStatus`, `MeshStatus`, `GossipStatus` — these are the payloads the backend carries
- `crates/assay-types/src/manifest.rs` — `RunManifest` — gains `state_backend: Option<StateBackendConfig>` field
- `plugins/claude-code/` — model for the smelt plugin structure
- `plugins/codex/AGENTS.md` — model for the smelt AGENTS.md format

> See `.kata/DECISIONS.md` for all architectural and pattern decisions. D001 (zero-trait convention), D007 (sync core), D092 (backward-compatible Config extension pattern) are especially relevant.

## Relevant Requirements

- R071 — StateBackend trait and CapabilitySet
- R072 — LocalFsBackend: zero regression
- R073 — Tier-2 event routing through backend
- R074 — CapabilitySet and graceful degradation
- R075 — smelt-agent plugin

## Scope

### In Scope

- `StateBackend` trait definition in `assay-core`
- `CapabilitySet` flags struct
- `LocalFsBackend` implementing the trait (wraps all existing persistence)
- `StateBackendConfig` in `assay-types` (config enum: `local_fs`, extensible)
- `RunManifest.state_backend: Option<StateBackendConfig>` field (backward-compatible)
- Orchestrator, mesh, gossip, checkpoint code routed through the backend
- `smelt-agent` plugin: `plugins/smelt-agent/AGENTS.md` + skills covering backend-aware API surface
- Schema snapshots updated; all 1400+ tests pass

### Out of Scope / Non-Goals

- `LinearBackend`, `GitHubBackend`, `SshSyncBackend` — concrete non-local backends are M011+
- Multi-machine smelt integration — verified manually/UAT; not automated in this milestone
- Tier 1 (heartbeats, per-tick mesh routing) moving to a remote backend — explicitly deferred; the split-tier design means Tier 1 stays local always

## Technical Constraints

- D001: Zero-trait convention is a project-wide rule. `StateBackend` is a deliberate, scoped exception. It must be documented as D149 in DECISIONS.md.
- D007: Sync core. Backend trait methods must be sync. Async backends internalize their runtime.
- D092: serde(default) + skip_serializing_if pattern for any new optional fields on persisted types.
- `deny_unknown_fields` is NOT set on `RunManifest` — verify and document.

## Integration Points

- `assay-core::orchestrate::executor` — primary state write callsite
- `assay-core::orchestrate::mesh` — mesh routing writes
- `assay-core::orchestrate::gossip` — knowledge manifest writes
- `assay-core::checkpoint::persistence` — checkpoint saves
- `assay-types::manifest::RunManifest` — config entry point

## Open Questions

- Should `StateBackend` be a trait object (`Box<dyn StateBackend>`) or a generic parameter `<B: StateBackend>`? — Generic avoids vtable overhead but makes `OrchestratorConfig` generic, which is viral. Trait object preferred for the orchestrator; decide during S01 and document.
- Should `StateBackendConfig` be an enum or a string+map? — Enum is type-safe for known backends; map allows third-party extension. Start with enum, extensible variant (`Custom { name, config: serde_json::Value }`).
