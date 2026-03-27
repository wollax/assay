# S03: CapabilitySet degradation paths — UAT

**Milestone:** M010
**Written:** 2026-03-26

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S03 has no UI surface, no daemon lifecycle, and no external system integration. All correctness claims are proven by integration tests using mock runners and the `NoopBackend` test helper. The plan explicitly states "Real runtime required: no" and "Human/UAT required: no". The integration tests assert on `OrchestratorResult` fields and `PromptLayer` contents — there is no observable behavior that requires human eyes to verify beyond running `just ready`.

## Preconditions

- Rust toolchain installed (`rustup show` reports stable toolchain)
- `just` installed (`just --version` succeeds)
- Working directory is the assay workspace root
- No uncommitted changes to `mesh.rs`, `gossip.rs`, or `state_backend.rs` that would shadow the guards

## Smoke Test

```
just ready
```

All 1486 tests pass, fmt + lint + deny all green.

## Test Cases

### 1. Mesh degradation — routing thread skipped when messaging unsupported

```
cargo test -p assay-core --features orchestrate --test mesh_integration test_mesh_degrades_gracefully_without_messaging -- --nocapture
```

1. Test creates an `OrchestratorConfig` with `backend: Arc::new(NoopBackend)` (supports_messaging = false)
2. Calls `run_mesh()` with mock session runners that complete successfully
3. **Expected:** Test passes. `OrchestratorResult.outcomes` shows all sessions completed. `mesh_status.messages_routed == 0`. No error returned. A `WARN` event with `capability="messaging"` and `mode="mesh"` appears in test output.

### 2. Gossip degradation — PromptLayer not injected when manifest unsupported

```
cargo test -p assay-core --features orchestrate --test gossip_integration test_gossip_degrades_gracefully_without_manifest -- --nocapture
```

1. Test creates an `OrchestratorConfig` with `backend: Arc::new(NoopBackend)` (supports_gossip_manifest = false)
2. Calls `run_gossip()` with mock session runners that capture their received PromptLayers
3. **Expected:** Test passes. All sessions complete. None of the sessions received a `gossip-knowledge-manifest` PromptLayer. No `knowledge.json` file is written to the run directory. A `WARN` event with `capability="gossip_manifest"` and `mode="gossip"` appears in test output.

### 3. Zero regression — existing mesh and gossip tests unchanged

```
cargo test -p assay-core --features orchestrate --test mesh_integration
cargo test -p assay-core --features orchestrate --test gossip_integration
```

1. Run all existing tests (not just the degradation tests)
2. **Expected:** `test_mesh_mode_completed_not_dead`, `test_mesh_mode_message_routing`, `test_gossip_mode_knowledge_manifest`, `test_gossip_mode_manifest_path_in_prompt_layer` all pass. No test output changes from the M009 baseline.

### 4. NoopBackend contract

```
cargo test -p assay-core --features orchestrate --test state_backend test_noop
```

1. Runs three contract tests: capabilities assertion, all-methods-return-Ok, Arc trait-object construction
2. **Expected:** 3 passed.

## Edge Cases

### NoopBackend does not write to disk

If the gossip degradation guard accidentally fires but the PromptLayer injection site has no guard, the manifest path won't be injected but `persist_knowledge_manifest` might still be called. The test asserts PromptLayer absence AND the test runs in a tempdir so any unexpected file writes would need to be checked.

To verify no `knowledge.json` is written:

```
# After running the gossip degradation test with a tempdir:
# knowledge.json should NOT exist under the run directory
find /tmp -name "knowledge.json" -newer /tmp -maxdepth 10 2>/dev/null | head -5
```

**Expected:** No `knowledge.json` files from the degradation test run.

### LocalFsBackend unaffected — normal mesh and gossip still write manifest

```
cargo test -p assay-core --features orchestrate --test gossip_integration test_gossip_mode_manifest_path_in_prompt_layer
```

**Expected:** Passes. The manifest path IS injected when using `LocalFsBackend` (capabilities all true). The guard does not incorrectly suppress normal behavior.

## Failure Signals

- Any test in `mesh_integration.rs` or `gossip_integration.rs` failing with "assertion failed" on PromptLayer contents or `messages_routed` — indicates a capability guard is missing or misplaced
- `just ready` returning non-zero — fmt, lint, or a test regression
- `test_gossip_mode_manifest_path_in_prompt_layer` failing — indicates the capability guard incorrectly suppresses LocalFsBackend behavior
- A `WARN` event appearing during `test_gossip_mode_knowledge_manifest` (normal test, not degradation) — indicates the guard fires for LocalFsBackend, which would be a bug

## Requirements Proved By This UAT

- R074 — CapabilitySet and graceful degradation: both capability guards exist, emit structured warn events, skip the guarded features without error, and are proven by integration tests with `NoopBackend`. Each degradation path has a dedicated test. No panic, no error return.

## Not Proven By This UAT

- Real `warn!` event visibility in a production tracing setup (Jaeger, JSON file export) — the structured fields exist but are only observable in integration test stderr; real exporter delivery is not exercised
- Degradation behavior under concurrent load (many sessions racing to access capability flags) — the capability bool is read once before `thread::scope` and is immutable afterward, so this is structurally safe but not load-tested
- Future backends (LinearBackend, GitHubBackend) implementing `CapabilitySet::none()` for some flags — M011+ concern
- The `supports_annotations` and `supports_checkpoints` capability flags — defined in `CapabilitySet` but no production code checks them yet (no guards needed until a slice adds annotation/checkpoint usage)

## Notes for Tester

- The `NoopBackend` produces no observable side effects — it does not write files, does not maintain state, and returns empty/zero results. Tests that use it must assert on `OrchestratorResult` fields and session runner interactions, not on filesystem state.
- `just ready` runs `cargo deny` which checks for RUSTSEC advisories; some "advisory not detected" warnings are expected and benign (they are pre-acknowledged advisories in `deny.toml`).
- The mesh degradation test passes without any production code changes (NoopBackend's no-op methods make routing a no-op already). Only the gossip degradation test required the T02 capability guard to pass.
