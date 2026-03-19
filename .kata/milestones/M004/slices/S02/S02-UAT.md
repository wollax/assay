# S02: Mesh Mode — UAT

**Milestone:** M004
**Written:** 2026-03-18

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: Mesh mode coordination mechanics (message routing, membership state, state.json persistence) are fully observable via filesystem artifacts — inbox/outbox file presence, state.json contents, and integration test assertions. Real agent processes writing to outboxes are UAT-only; the routing mechanics are identical regardless of whether the writer is a mock closure or a live Claude agent.

## Preconditions

- `just build` succeeds (workspace compiles clean)
- `just ready` passes (fmt ✓ lint ✓ test ✓ deny ✓)
- All snapshot files committed: `find crates/assay-types/tests/snapshots -name "*.snap.new"` returns empty

## Smoke Test

```
cargo test -p assay-core --features orchestrate -- mesh --nocapture
```
Both `test_mesh_mode_message_routing` and `test_mesh_mode_completed_not_dead` pass with no assertion failures. The routing test shows `messages_routed >= 1` in state.json.

## Test Cases

### 1. Message routing: outbox file arrives in peer inbox

1. Run `cargo test -p assay-core --features orchestrate -- test_mesh_mode_message_routing --nocapture`
2. **Expected:** Test passes; state.json contains `mesh_status.messages_routed >= 1`; `mesh/reader/inbox/msg.txt` exists in the temp dir

### 2. Completed sessions are not classified as Dead

1. Run `cargo test -p assay-core --features orchestrate -- test_mesh_mode_completed_not_dead --nocapture`
2. **Expected:** Test passes; state.json contains 2 members both with `state: "completed"` — not `"dead"` or `"alive"`

### 3. Schema snapshots stable

1. Run `cargo test -p assay-types --features orchestrate`
2. **Expected:** 61 schema snapshot tests pass; no `.snap.new` files created

### 4. Existing DAG tests unaffected (regression)

1. Run `cargo test -p assay-core --features orchestrate`
2. **Expected:** All 770+ unit tests and 5 orchestrate integration tests pass alongside the 2 new mesh integration tests

### 5. state.json mesh_status structure

1. After running `test_mesh_mode_message_routing`, locate the temp dir path from the `--nocapture` output (or add a `println!` to the test)
2. Read the state.json file: `cat <tmpdir>/.assay/orchestrator/<run_id>/state.json | jq .mesh_status`
3. **Expected:** JSON object with `members` array (2 entries, both `state: "completed"`) and `messages_routed: 1` (or greater)

## Edge Cases

### depends_on sessions in Mesh mode

1. Create a manifest with `mode = "mesh"` and a session with `depends_on = ["other"]`
2. Run via `assay run manifest.toml`
3. **Expected:** Warning logged (`depends_on is ignored in Mesh mode` for the affected session); all sessions still launch and complete normally

### Unrecognized outbox target

1. In a mesh session, write a file to `outbox/nonexistent-session/msg.txt`
2. **Expected:** Routing thread emits `tracing::warn!` with `unknown outbox target`; file is not moved; `messages_routed` does not increment

## Failure Signals

- `test_mesh_mode_message_routing` fails with "state.json must exist" — routing stub was re-introduced or persist_state broke
- `mesh_status` is `null` in state.json — mesh.rs is not writing status; check persist_state call in worker
- `messages_routed: 0` — routing thread exited before worker wrote the outbox file; increase sleep in writer runner or decrease routing poll interval
- `state: "dead"` for a session that succeeded — MeshMemberState update logic has a bug; check catch_unwind handling
- Snapshot test fails with `.snap.new` file — new optional field was added to OrchestratorStatus without regenerating the snapshot; run `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate`

## Requirements Proved By This UAT

- R035 (Mesh mode execution) — parallel launch with roster PromptLayer injection is proven by `test_mesh_mode_completed_not_dead`: both sessions receive their roster, run concurrently, and both complete normally
- R036 (Mesh peer messaging) — outbox→inbox file routing is proven by `test_mesh_mode_message_routing`: file written to writer's outbox arrives in reader's inbox; `messages_routed` counter is accurate; `MeshMemberState::Completed` distinguishes normal exit from crash

## Not Proven By This UAT

- Real Claude agents using the roster to actually read peer inboxes and act on messages — mock runners prove the routing mechanics but not agent comprehension; requires manual UAT with `claude -p` agents
- `Alive → Suspect → Dead` heartbeat transitions — Suspect state is unreachable in current implementation; heartbeat polling deferred to S04; only Alive/Running/Completed/Dead transitions at session boundaries are proven
- `orchestrate_status` MCP tool returning `mesh_status` — state.json is written and readable directly; MCP surfacing requires S04 to wire the response field
- CLI mode display showing "mesh" in run output — requires S04

## Notes for Tester

The integration tests use a bare tempdir (no git repo required). The writer runner discovers its outbox path by parsing the `mesh-roster` PromptLayer for a line starting with `"Outbox: "` — this is the machine-parseable contract (D058). If the roster format changes, both the writer runner test helper and the "Outbox: " parse must be updated together.

The routing thread polls every 50ms. The writer runner sleeps 200ms before writing the outbox file; the reader sleeps 300ms. These timings give the routing thread a ~150ms window to move the file before `run_mesh()` returns. If the test becomes flaky on a slow CI machine, increase the writer sleep to 500ms.
