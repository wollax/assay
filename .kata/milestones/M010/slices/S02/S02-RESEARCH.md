# S02: LocalFsBackend implementation and orchestrator wiring — Research

**Date:** 2026-03-26

## Summary

S02 is the highest-risk slice in M010: it wires `LocalFsBackend` into the three orchestration executors (DAG/Mesh/Gossip), replaces direct `persist_state()` calls with `backend.push_session_event()` / `backend.read_run_state()`, adds the `RunManifest.state_backend` field, and implements real method bodies on `LocalFsBackend`. The fundamental challenge is that `OrchestratorConfig` currently has `#[derive(Clone)]` and `Box<dyn StateBackend>` is not `Clone` — S01's summary already flagged `Arc<dyn StateBackend>` as the solution (the `_assert_object_safe` guard uses `Arc`). Every existing integration test (~1339 lines across 3 files) must pass unchanged after the wiring.

The existing code uses `persist_state()` (a `pub(crate)` free function in `executor.rs`) at 15 callsites across `executor.rs`, `mesh.rs`, and `gossip.rs`. Each call constructs an `OrchestratorStatus` and writes it to `state.json` via atomic tempfile-rename. The `LocalFsBackend::push_session_event` implementation should delegate to this exact function. Similarly, `read_run_state` reads and deserializes `state.json`. The Tier-1 filesystem ops in mesh (outbox→inbox routing at 50ms polling) and gossip (`persist_knowledge_manifest`) are internal to `LocalFsBackend` and NOT exposed as trait methods (D152).

The `RunManifest` struct does NOT have `deny_unknown_fields` and already uses `serde(default, skip_serializing_if)` for optional feature-gated fields — adding `state_backend: Option<StateBackendConfig>` follows the exact same pattern. There is a pre-existing snapshot test failure (`run_manifest_schema_snapshot`) that was already broken before S01 merged — updating the snapshot is safe.

## Recommendation

**Wire the backend incrementally:** (1) Add `state_backend` field to `RunManifest` with backward-compat round-trip test. (2) Add `backend: Arc<dyn StateBackend>` to `OrchestratorConfig`. (3) Implement `LocalFsBackend` method bodies by delegating to existing persistence functions. (4) Replace `persist_state()` callsites with `backend.push_session_event()` in all three executors. (5) Wire `orchestrate_status` MCP tool to use `backend.read_run_state()`. (6) Verify all ~1476 tests still pass.

Use `Arc<dyn StateBackend>` (not `Box<dyn StateBackend>`) because `OrchestratorConfig` derives `Clone`. `Arc` is the natural choice — the backend is shared across worker threads in `thread::scope` anyway.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic file writes | `persist_state()` in `executor.rs` — NamedTempFile + rename + sync_all | Battle-tested; identical pattern in `persist_knowledge_manifest()` and `save_checkpoint()` |
| OrchestratorStatus serialization | `serde_json::to_string_pretty()` + `serde_json::from_str()` already used everywhere | Schema-snapshot-locked; any format change breaks tests |
| Backward-compat optional fields on RunManifest | `#[serde(default, skip_serializing_if = "Option::is_none")]` on `mesh_config`, `gossip_config` | Proven pattern (D092/D056); RunManifest has no deny_unknown_fields |

## Existing Code and Patterns

- `crates/assay-core/src/orchestrate/executor.rs:111-133` — `persist_state(run_dir, status)` atomic write of OrchestratorStatus; use this as the body of `LocalFsBackend::push_session_event`
- `crates/assay-core/src/orchestrate/executor.rs:65-73` — `OrchestratorConfig { max_concurrency, failure_policy }` — add `backend: Arc<dyn StateBackend>` here
- `crates/assay-core/src/orchestrate/executor.rs:148-157` — `run_orchestrated()` signature takes `config: OrchestratorConfig` by value; `Arc` makes this cheap to clone
- `crates/assay-core/src/orchestrate/mesh.rs:28` — mesh imports `persist_state` from executor — replace with `config.backend.push_session_event()`
- `crates/assay-core/src/orchestrate/gossip.rs:32` — gossip imports `persist_state` from executor — same replacement
- `crates/assay-core/src/orchestrate/gossip.rs:50-75` — `persist_knowledge_manifest()` is a gossip-internal fn — stays as-is (Tier-1, not a trait method)
- `crates/assay-core/src/orchestrate/mesh.rs:220-250` — mesh routing thread with `fs::read_dir` + `fs::rename` at 50ms — stays as-is (Tier-1, D152)
- `crates/assay-core/src/checkpoint/persistence.rs:38-70` — `save_checkpoint()` atomic write pattern — reuse for `LocalFsBackend::save_checkpoint_summary`
- `crates/assay-mcp/src/server.rs:3285-3340` — `orchestrate_status()` reads `state.json` directly — consider delegating to `backend.read_run_state()` or keep as-is (MCP layer doesn't have a backend instance)
- `crates/assay-types/src/manifest.rs:33-55` — `RunManifest` struct with `#[derive(Default)]`, no `deny_unknown_fields`, existing `serde(default, skip_serializing_if)` pattern

## Constraints

- **`OrchestratorConfig` derives `Clone`** — `Box<dyn StateBackend>` is not `Clone`; must use `Arc<dyn StateBackend>` (S01 already validated object-safety with `Arc`)
- **D007 (sync core)** — all `StateBackend` methods must remain sync; no `async` infection
- **D152 (Tier-1/Tier-2 split)** — mesh routing (50ms polling, fs::rename) and gossip knowledge manifest writes are NOT trait methods; they stay as internal filesystem ops in `LocalFsBackend` and in the executor code
- **Feature-gated** — `state_backend` module is behind `#[cfg(feature = "orchestrate")]`; the `RunManifest.state_backend` field should also be feature-gated to match `mode`, `mesh_config`, `gossip_config`
- **Backward compatibility** — existing RunManifest TOML files without `state_backend` must deserialize without error; the field must use `serde(default, skip_serializing_if = "Option::is_none")`
- **Schema snapshot** — `run_manifest_schema_snapshot` is pre-existing-failing; adding the new field will change the snapshot; update and commit
- **`persist_state` is `pub(crate)`** — the function is shared across executor.rs, mesh.rs, and gossip.rs; after wiring, it can either become a private impl detail of `LocalFsBackend` or stay as a helper called by the backend
- **15 callsites** — `persist_state` is called 15 times across 3 files (5 in executor.rs, 6 in gossip.rs, 4 in mesh.rs); each must be replaced
- **Thread safety** — `Arc<dyn StateBackend>` is `Send + Sync` (trait bound); it will be cloned into `thread::scope` workers naturally
- **Integration tests construct `OrchestratorConfig` directly** — all test sites that create `OrchestratorConfig { max_concurrency, failure_policy }` must now also pass a `backend` field; use `Arc::new(LocalFsBackend::new(assay_dir))` at each test site

## Common Pitfalls

- **Forgetting `Default` for `OrchestratorConfig`** — it currently has `impl Default`; the new `backend` field has no sensible default (needs a `PathBuf`). Either remove `Default` or change it to use a temp directory. Removing `Default` may break test code that uses `..Default::default()`. Check all sites.
- **`persist_state` vs `push_session_event` naming mismatch** — callers construct full `OrchestratorStatus` structs inline and pass them to `persist_state`; `push_session_event` has the same signature (`run_dir, &OrchestratorStatus`) so the call-site changes are minimal (just change the function name to `config.backend.push_session_event()`), but verify the `run_dir` argument is the same path the backend expects
- **MCP `orchestrate_status` doesn't have a backend instance** — the MCP handler reads `state.json` directly from disk using a path derived from `cwd + .assay/orchestrator/<run_id>/state.json`. It does NOT go through the backend. For S02, this is fine — `LocalFsBackend::read_run_state` writes to the same path. In M011+ with remote backends, the MCP handler would need a backend instance. Leave the MCP handler reading the filesystem directly for now.
- **Gossip coordinator thread accessing backend** — the gossip coordinator runs in a dedicated thread spawned inside `thread::scope`. It calls `persist_state` for status snapshots. The `Arc<dyn StateBackend>` must be cloned into this thread. Make sure the coordinator captures a clone of the `Arc`, not a reference.
- **`run_manifest_schema_snapshot` snapshot delta** — the snapshot is already failing; adding `state_backend` changes the schema further. Run `INSTA_UPDATE=always` and commit the new snapshot. Verify no other snapshot tests break.
- **Test helpers that construct `OrchestratorConfig`** — the integration test files and embedded tests in executor.rs, mesh.rs, gossip.rs all construct config. Search for `OrchestratorConfig {` and `OrchestratorConfig::default()` to find all sites.

## Open Risks

- **`OrchestratorConfig::default()` removal cascade** — if `Default` is removed from `OrchestratorConfig`, every `..Default::default()` usage becomes a compile error. Need to audit all usage sites before deciding approach. Alternative: keep Default but use a no-op backend (e.g., a NullBackend that panics on any method call, only used as a default placeholder).
- **`send_message` / `poll_inbox` method bodies** — the S01 trait has these methods but the Tier-1/Tier-2 split (D152) means the mesh routing thread does NOT use them (it does raw `fs::rename`). The `send_message` / `poll_inbox` trait methods are for Tier-2 coalesced inter-machine messaging (M011+). In S02, the method bodies should do the same thing as the inline mesh routing code for LocalFs, but they won't be called by the mesh routing thread. Clarify in the plan that these methods implement the trait contract but aren't wired into mesh.rs yet — mesh routing stays file-based.
- **Mesh and gossip tests assert on `state.json` contents** — `mesh_integration.rs:591` reads `state.json` directly. The backend wiring must write to the same path or these tests break.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | N/A — core systems Rust, no external framework | none needed |

No professional agent skills are relevant for this slice. The work is pure Rust trait wiring and filesystem persistence — no external services, frameworks, or SDKs involved.

## Sources

- S01-SUMMARY.md Forward Intelligence (source: `.kata/milestones/M010/slices/S01/S01-SUMMARY.md`)
- M010-ROADMAP.md boundary map S01→S02 and S02→S03 (source: `.kata/milestones/M010/M010-ROADMAP.md`)
- D149–D153 decisions register (source: `.kata/DECISIONS.md`)
- Direct code exploration of executor.rs, mesh.rs, gossip.rs, manifest.rs, state_backend.rs
