---
id: T03
parent: S03
milestone: M004
provides:
  - crates/assay-core/src/orchestrate/gossip.rs — full run_gossip() implementation replacing stub
key_files:
  - crates/assay-core/src/orchestrate/gossip.rs
  - crates/assay-mcp/tests/mcp_handlers.rs
key_decisions:
  - Used mpsc::channel (unbounded) for worker→coordinator message passing; coordinator owns rx and exits cleanly on Disconnected after draining
  - drop(tx) called inside thread::scope immediately after spawning all workers, ensuring coordinator exits when all worker clones drop
  - GossipCompletion sent before updating session_statuses_arc to avoid coordinator waiting on locked status state
  - Extracted changed_files from MergeCheck.files (Vec<FileChange>.path), not a nonexistent .changed_files field
patterns_established:
  - Gossip mode follows mesh.rs blueprint: thread::scope + bounded semaphore + panic isolation; coordinator thread replaces routing thread
  - persist_knowledge_manifest() uses identical tempfile+rename+sync_all pattern as persist_state()
  - Coordinator drain loop (while let Ok(c) = rx.try_recv()) after Disconnected ensures no completions are lost on clean shutdown
observability_surfaces:
  - "tracing::info!(session = %name, \"gossip session starting\") per worker"
  - "tracing::debug!(sessions_synthesized, coordinator_rounds, \"gossip coordinator cycle\") per coordinator round"
  - "tracing::warn!(session, \"depends_on is ignored in Gossip mode\") per session with deps"
  - "cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status  →  { sessions_synthesized, knowledge_manifest_path, coordinator_rounds }"
  - "cat .assay/orchestrator/<run_id>/gossip/knowledge.json | jq '.entries | length'  →  synthesis count"
duration: ~25 minutes
verification_result: passed
completed_at: 2026-03-18
blocker_discovered: false
---

# T03: Implement run_gossip() with coordinator thread

**Replaced the `run_gossip()` stub with a full parallel implementation: thread::scope + mpsc coordinator + atomic knowledge.json, with both integration tests passing and zero clippy warnings.**

## What Happened

Replaced the placeholder gossip.rs stub with a complete implementation following the mesh.rs blueprint. Key additions:

1. **`GossipCompletion` struct** — carries per-session results (name, spec, gate counts, changed files, timestamp) from workers to coordinator via mpsc channel.

2. **`persist_knowledge_manifest()`** — writes `KnowledgeManifest` to `gossip_dir/knowledge.json` using the same tempfile+rename+sync_all atomic pattern as `persist_state()`.

3. **Setup phase** — generates run_id/run_dir/gossip_dir, injects `"gossip-knowledge-manifest"` PromptLayer into each session clone with the manifest path, writes empty initial `knowledge.json`, initializes `GossipStatus`, persists initial `state.json`.

4. **`thread::scope`** with:
   - **Coordinator thread** (owns `rx`): loops on `recv_timeout(coordinator_interval)`, synthesizing `KnowledgeEntry` records on Ok(completion), incrementing coordinator_rounds on timeout, breaking + draining on Disconnected.
   - **Worker threads**: acquire semaphore slot, mark Running, `catch_unwind` the session_runner call, extract gate_pass_count/gate_fail_count/changed_files, send `GossipCompletion` via `tx_worker` clone, update session_statuses_arc, best-effort persist_state snapshot, release semaphore.
   - **`drop(tx)`** called in parent scope immediately after spawning all workers — ensures coordinator exits when all worker tx clones drop.

5. **Post-scope** — builds final `OrchestratorStatus` with `gossip_status: Some(final_gossip)`, persists to state.json, builds and returns `OrchestratorResult`.

6. **Stub unit tests removed** (`run_gossip_returns_empty_result`, `run_gossip_emits_warn_for_depends_on`); replaced with `run_gossip_calls_runner` (verifies runner is actually invoked via `AtomicBool`).

Also patched `crates/assay-mcp/tests/mcp_handlers.rs` line 1668: the `OrchestratorStatus` construction site was missing `gossip_status: None` after T01 added the field.

## Verification

- `cargo test -p assay-core --features orchestrate --test gossip_integration` — both tests pass: `test_gossip_mode_knowledge_manifest` and `test_gossip_mode_manifest_path_in_prompt_layer`
- `cargo test -p assay-core --features orchestrate` — 769 unit tests pass + 2 integration tests pass, including new `run_gossip_calls_runner`
- `cargo clippy -p assay-core --features orchestrate -- -D warnings` — exits 0, zero warnings
- `cargo test -p assay-types --features orchestrate` — all schema snapshot tests pass (knowledge_entry, knowledge_manifest, gossip_status, orchestrator_status)
- `cargo test -p assay-mcp` — 141 tests pass
- `just ready` — fmt ✓, lint ✓, test ✓, deny ✓

## Diagnostics

```bash
# Live gossip progress during a run:
cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status
# → { "sessions_synthesized": N, "knowledge_manifest_path": "...", "coordinator_rounds": N }

# Knowledge manifest entries:
cat .assay/orchestrator/<run_id>/gossip/knowledge.json | jq '.entries | length'
cat .assay/orchestrator/<run_id>/gossip/knowledge.json | jq '[.entries[].session_name]'
```

## Deviations

- **MergeCheck field name**: The task plan said `result.merge_check.changed_files` but `MergeCheck` has `files: Vec<FileChange>` (each with `.path: String`). Fixed to use `mc.files.iter().map(|f| f.path.clone()).collect()`.
- **MCP test construction site**: T01 added `gossip_status` to `OrchestratorStatus` but one construction site in `crates/assay-mcp/tests/mcp_handlers.rs` was missed. Fixed by adding `gossip_status: None`.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/gossip.rs` — full implementation replacing stub: GossipCompletion struct, persist_knowledge_manifest(), full run_gossip(), run_gossip_calls_runner unit test; stub tests removed
- `crates/assay-mcp/tests/mcp_handlers.rs` — added missing `gossip_status: None` to OrchestratorStatus construction site at line 1668
