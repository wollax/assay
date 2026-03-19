# S03: Gossip Mode — UAT

**Milestone:** M004
**Written:** 2026-03-18

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: R037 and R038 are fully exercised at the filesystem contract level by two integration tests using mock session runners (`crates/assay-core/tests/gossip_integration.rs`). These tests exercise real `thread::scope` parallelism, real coordinator mpsc channel, real atomic `knowledge.json` writes, and real PromptLayer injection — no mocking of the file system or threading model. Real Claude invocation is not needed to prove the coordinator protocol; it would only be needed for UAT of the actual gossip coordination experience (cross-pollination quality), which is a qualitative human concern deferred to live runtime.

## Preconditions

- `cargo build --workspace --features orchestrate` succeeds (workspace compiles)
- `just ready` exits 0 (all checks green)
- Working directory is the project root

## Smoke Test

```bash
cargo test -p assay-core --features orchestrate --test gossip_integration
```

Expected output: `test result: ok. 2 passed; 0 failed` — confirms both integration tests pass.

## Test Cases

### 1. Knowledge manifest populated with all session entries

```bash
cargo test -p assay-core --features orchestrate --test gossip_integration \
  test_gossip_mode_knowledge_manifest -- --nocapture
```

1. Inspect output for the `<assay_dir>/orchestrator/<run_id>/gossip/knowledge.json` path
2. Verify the test confirms: `knowledge.json` exists, deserializes as `KnowledgeManifest`, has 3 entries (alpha, beta, gamma), and `state.json.gossip_status.sessions_synthesized == 3`
3. **Expected:** Test passes; `knowledge_manifest_path` in `gossip_status` ends with `gossip/knowledge.json`

### 2. Gossip knowledge manifest path in session prompt layers

```bash
cargo test -p assay-core --features orchestrate --test gossip_integration \
  test_gossip_mode_manifest_path_in_prompt_layer -- --nocapture
```

1. Verify the test confirms: each of the 2 mock sessions receives a PromptLayer named `"gossip-knowledge-manifest"` containing a line starting with `"Knowledge manifest: "`
2. Verify the path in that line is under the test's temp assay directory
3. Verify runner was called exactly 2 times (both sessions launched)
4. **Expected:** Test passes; no layer_errors; runner_call_count == 2

### 3. Schema snapshot tests pass

```bash
cargo test -p assay-types --features orchestrate -- \
  knowledge_entry_schema_snapshot knowledge_manifest_schema_snapshot \
  gossip_status_schema_snapshot orchestrator_status_schema_snapshot
```

1. **Expected:** All 4 snapshot tests pass without requesting updates

### 4. OrchestratorStatus gossip_status absent in DAG runs

1. Run `cargo test -p assay-core --features orchestrate -- orchestrate` (existing orchestration tests)
2. **Expected:** All tests pass; DAG runs do not include `gossip_status` in serialized `OrchestratorStatus` (field is `skip_serializing_if = "Option::is_none"`)

### 5. Full workspace health

```bash
just ready
```

1. **Expected:** `fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓` — exit 0 with ≥1264 total tests

## Edge Cases

### Coordinator processes completions when all workers finish before recv_timeout

The drain loop (`while let Ok(c) = rx.try_recv()`) handles this. Verified by `test_gossip_mode_knowledge_manifest` where 3 fast mock sessions all complete nearly simultaneously — the coordinator still synthesizes all 3 entries.

### Sessions with depends_on in gossip mode

1. Construct a manifest with `mode = "gossip"` and a session with non-empty `depends_on`
2. **Expected:** `tracing::warn!` is emitted per session with deps; `depends_on` is silently ignored; all sessions still launch in parallel

### Session failure (runner returns Err)

1. Mock runner returns `Err(PipelineError { ... })` for one session
2. **Expected:** That session's `state.json` entry shows `Failed`; coordinator still synthesizes remaining successful sessions; `knowledge.json` has entries only for sessions that completed successfully; `gossip_status.sessions_synthesized` reflects only successful completions

## Failure Signals

- `knowledge.json` missing after `run_gossip()` returns — coordinator thread failed to write initial manifest or gossip directory was not created
- `gossip_status` is `null` in `state.json` — executor ran in wrong mode or `gossip_status` was not passed through to final `OrchestratorStatus`
- `sessions_synthesized` less than number of sessions that succeeded — drain loop dropped completions or workers failed to send before channel closed
- `runner_call_count == 0` in prompt layer test — sessions were never launched (stub behavior leaked into production path)
- Schema snapshot tests request `INSTA_UPDATE` — types were modified without regenerating snapshots

## Requirements Proved By This UAT

- R037 (Gossip mode execution) — `test_gossip_mode_knowledge_manifest` proves: parallel launch of all sessions (no DAG ordering), coordinator thread synthesizes each completed session into `knowledge.json`, `gossip_status.sessions_synthesized` reflects actual completion count, state persisted to `state.json`
- R038 (Gossip knowledge manifest injection) — `test_gossip_mode_manifest_path_in_prompt_layer` proves: `"gossip-knowledge-manifest"` PromptLayer injected at session launch time, manifest path encoded as `"Knowledge manifest: <path>"` line, path is under the run's orchestrator directory (accessible to the running session)

## Not Proven By This UAT

- Actual cross-pollination quality: whether real Claude agents reading `knowledge.json` during execution actually make better decisions based on peer findings — this is a live-runtime qualitative UAT requiring real Claude invocations
- Mid-run manifest reads by still-running sessions: mock runners complete instantly; a real agent reading the manifest mid-execution while other sessions are still running is not tested
- Large-scale parallelism: integration tests use 2-3 sessions; behavior with 10+ concurrent sessions is not exercised
- `orchestrate_status` MCP tool surfacing `gossip_status` to real callers: S04 is needed for this surface

## Notes for Tester

- The integration tests use `tempfile::tempdir()` — each run gets a fresh isolated directory; no cleanup needed
- `--nocapture` is useful to see the full `knowledge.json` path if a test fails
- The knowledge manifest path format is always `<assay_dir>/orchestrator/<run_id>/gossip/knowledge.json` — predictable from the run_id used in `state.json`
- `gossip_status.coordinator_rounds` may be 1 in tests because fast mock runners complete before the first coordinator timeout; this is correct behavior (coordinator processes completions on its first `recv_timeout` or earlier)
