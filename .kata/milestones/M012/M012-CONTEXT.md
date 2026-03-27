# M012: Checkpoint Persistence on Remote Backends — Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

## Project Description

Assay is a spec-driven development platform for AI-augmented workflows. M011 delivered three production `StateBackend` implementations — `LinearBackend`, `GitHubBackend`, `SshSyncBackend` — and wired `backend_from_config()` into all CLI/MCP orchestration construction sites.

`StateBackend` has a `save_checkpoint_summary()` method and a `CapabilitySet::supports_checkpoints` flag, but nothing in the runtime actually routes checkpoint writes through the backend. Two call sites bypass the abstraction:

1. **`GuardDaemon::try_save_checkpoint`** — calls `crate::checkpoint::save_checkpoint()` directly on the local filesystem, ignoring the backend entirely.
2. **Orchestrators** — `executor.rs`, `mesh.rs`, `gossip.rs` never call `backend.save_checkpoint_summary()` at all; the method exists on the trait but has zero callers in the orchestration path.

M012 closes that gap: route both call sites through the backend and prove the wiring works end-to-end.

## Why This Milestone

The `StateBackend` trait promised that checkpoint persistence is pluggable — smelt workers on remote machines should be able to push checkpoint state to the controller via `SshSyncBackend`, or surface it as a Linear comment. Today that promise is broken: `save_checkpoint_summary` exists on the trait but is never called in production code paths.

M012 makes the abstraction real: guard daemon checkpoints and orchestrator-triggered checkpoints both flow through `backend.save_checkpoint_summary()` when `capabilities().supports_checkpoints` is true, and degrade gracefully when false.

## User-Visible Outcome

### When this milestone is complete, the user can:
- Configure `state_backend = { type = "ssh", ... }` in their `RunManifest`, run a guarded session on a remote machine, and find checkpoint files appearing under the remote `.assay/checkpoints/` directory via `SshSyncBackend`.
- See `assay context guard start` route its checkpoint writes through whatever backend is configured, not hardcoded to local filesystem.
- Run `just ready` and see all 1526+ tests pass with zero regression.

### Entry point / environment
- Entry point: `assay context guard start` (daemon path) and orchestrated `assay run` (orchestrator path)
- Environment: Rust codebase with test suite; real SSH/remote checkpoint validation is UAT only
- Live dependencies: None for contract tests; real SSH host for UAT

## Completion Class

- Contract complete means: `GuardDaemon` has a `backend: Arc<dyn StateBackend>` field; `try_save_checkpoint` calls `backend.save_checkpoint_summary()` when `supports_checkpoints` is true; contract tests with `NoopBackend` and a mock backend prove the routing; orchestrators call `backend.save_checkpoint_summary()` at appropriate points; `just ready` green.
- Integration complete means: All existing orchestrate, mesh, gossip, and guard tests pass unchanged. `OrchestratorConfig` constructed with `SshSyncBackend` and exercised by mock-runner tests.
- Operational complete means: N/A — real remote checkpoint validation is UAT only.

## Final Integrated Acceptance

To call this milestone complete:
- `GuardDaemon` accepts a `backend: Arc<dyn StateBackend>` at construction; `try_save_checkpoint` routes through it when `supports_checkpoints` is true, falls back to local `save_checkpoint` otherwise
- At least one orchestration path (DAG executor) calls `backend.save_checkpoint_summary()` at an appropriate lifecycle point (e.g. per-session on completion, or on milestone boundaries)
- Contract tests prove `save_checkpoint_summary` is called with the correct arguments via a mock/spy backend
- `just ready` green with 1526+ tests — zero regression
- `assay context guard start` CLI wiring passes the configured backend to `GuardDaemon`

## Risks and Unknowns

- **GuardDaemon construction sites** — `GuardDaemon::new` currently takes no backend parameter. Adding one requires updating every construction site (guard/mod.rs, guard/daemon.rs tests). Must find all sites.
- **Which orchestration lifecycle point to call `save_checkpoint_summary`** — The orchestrators don't extract `TeamCheckpoint` themselves; the checkpoint extractor (`assay_core::checkpoint::extractor`) reads from a JSONL session file. Orchestrated runs don't have a JSONL file in the same way a guard-watched session does. Need to decide: do orchestrators call `save_checkpoint_summary` with a synthesized checkpoint from `OrchestratorStatus`, or only the guard daemon uses it?
- **Backend availability in GuardDaemon** — `start_guard` is called from the CLI with only `assay_dir` and `GuardConfig`. The CLI doesn't currently have a `StateBackendConfig` to pass. Need to decide: does the guard read a `RunManifest` from disk to get the backend config, or does it read from `config.toml`, or does it default to `LocalFsBackend`?
- **Capability check ordering** — `supports_checkpoints` is checked at runtime; `try_save_checkpoint` must not panic when `supports_checkpoints` is false but fallback behavior differs per backend.

## Existing Codebase / Prior Art

- `crates/assay-core/src/state_backend.rs` — `StateBackend::save_checkpoint_summary()` method (line ~123), `LocalFsBackend::save_checkpoint_summary()` (calls `crate::checkpoint::persistence::save_checkpoint`), `SshSyncBackend::save_checkpoint_summary()` (scp pushes JSON to remote dir)
- `crates/assay-core/src/guard/daemon.rs` — `GuardDaemon` struct (line 20), `try_save_checkpoint` (line 310) calls `crate::checkpoint::save_checkpoint()` directly — this is the primary call site to reroute
- `crates/assay-core/src/guard/mod.rs` — `start_guard(session_path, assay_dir, project_dir, config)` public API — needs backend parameter or fallback
- `crates/assay-core/src/orchestrate/executor.rs` — `run_orchestrated()` and per-session workers; no checkpoint call today
- `crates/assay-core/src/orchestrate/mesh.rs` — `run_mesh()`; no checkpoint call today
- `crates/assay-core/src/orchestrate/gossip.rs` — `run_gossip()`; no checkpoint call today
- `crates/assay-backends/src/ssh.rs` — `SshSyncBackend::save_checkpoint_summary()` already implemented (pushes JSON to remote checkpoints dir)
- `crates/assay-cli/src/commands/context.rs` — `handle_guard_start` (line 594) constructs `GuardDaemon` via `start_guard`; would need to thread backend through
- D149: `StateBackend` is the deliberate exception to D001 (zero-trait convention)
- D156: `Arc<dyn StateBackend>` in `OrchestratorConfig`
- D163: SshSyncBackend uses `Command::arg()` chaining for scp
- D167: Capability guard pattern — capture bool before `thread::scope`, guard all feature-specific sites

> See `.kata/DECISIONS.md` for all architectural decisions.

## Relevant Requirements

- R080 (new) — Checkpoint persistence routed through StateBackend: `GuardDaemon` and orchestrators call `backend.save_checkpoint_summary()` when `supports_checkpoints` is true; graceful degradation when false

## Scope

### In Scope

- `GuardDaemon` gains `backend: Arc<dyn StateBackend>`; `try_save_checkpoint` calls `backend.save_checkpoint_summary()` when capability set
- `start_guard()` signature extended to accept `Arc<dyn StateBackend>` (or defaulting to `LocalFsBackend`)
- CLI `handle_guard_start` passes a `LocalFsBackend` (default) to `start_guard` — no RunManifest parsing for guard
- At least one orchestration path calls `backend.save_checkpoint_summary()` at a checkpoint-appropriate lifecycle point
- Contract tests proving the routing with a mock/spy backend
- `just ready` green with 1526+ tests

### Out of Scope / Non-Goals

- Real remote SSH validation (UAT only)
- GuardDaemon reading a RunManifest to determine which backend to use (too complex; CLI passes backend explicitly)
- Orchestrators synthesizing `TeamCheckpoint` from `OrchestratorStatus` (checkpoint extraction requires JSONL; orchestrators don't have that; the guard is the checkpoint extraction path)
- LinearBackend `save_checkpoint_summary` becoming meaningful (capability stays false; Linear has no checkpoint storage concept)
- GitHubBackend `save_checkpoint_summary` (capability stays false)

## Technical Constraints

- D001: Zero-trait convention. `StateBackend` is the sole exception.
- D007: Sync core. `GuardDaemon` is async (tokio), but `save_checkpoint_summary` is sync — this is fine; sync calls are safe inside async contexts.
- D149: `StateBackend` is the deliberate trait exception.
- D156: `Arc<dyn StateBackend>` in `OrchestratorConfig`.
- D167: Capability guard pattern — read `capabilities().supports_checkpoints` once before the guard event loop, pass the bool down.

## Open Questions

- Should orchestrators call `save_checkpoint_summary`? The orchestrators don't have `TeamCheckpoint` data — they have `OrchestratorStatus`. Decision: **No**, orchestrators do not call `save_checkpoint_summary`. The guard daemon is the checkpoint path; orchestrators only call `push_session_event`. The method exists for guard integration only. This keeps the scope tight.
- Should `start_guard` default to `LocalFsBackend` when called without a backend, or require one? Decision: **Default to `LocalFsBackend`** to preserve backward compatibility and keep CLI wiring simple. Callers that have a backend can pass it; others get the same behavior as today.
