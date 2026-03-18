# S01: Mode Infrastructure — UAT

**Milestone:** M004
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 delivers schema types, serde dispatch, and stub executors — no real agent invocation or filesystem coordination happens. All correctness signals are structural (schema snapshots, test coverage, TOML parsing). Automated tests in `cargo test` and `just ready` cover the full verification surface. No human interaction or live runtime is required until S02/S03 implement real execution.

## Preconditions

- `just ready` passes (fmt ✓, lint ✓, test ✓, deny ✓)
- `crates/assay-types/tests/snapshots/` contains `schema_snapshots__orchestrator-mode-schema.snap`, `schema_snapshots__mesh-config-schema.snap`, `schema_snapshots__gossip-config-schema.snap`
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` contains references to `mode`, `mesh_config`, and `gossip_config`

## Smoke Test

```
cargo test -p assay-types --features orchestrate manifest_without_mode_defaults_to_dag
```

Expected: `test manifest::tests::manifest_without_mode_defaults_to_dag ... ok`

## Test Cases

### 1. OrchestratorMode parses from TOML correctly

1. Run: `cargo test -p assay-types --features orchestrate orchestrator_mode`
2. **Expected:** `test orchestrate::tests::orchestrator_mode_default_is_dag ... ok`, `test orchestrate::tests::orchestrator_mode_serde_roundtrip ... ok`

### 2. RunManifest without mode field deserializes to Dag

1. Run: `cargo test -p assay-types --features orchestrate manifest_without_mode_defaults_to_dag`
2. **Expected:** ok — TOML `[[sessions]]\nspec = "auth"` (no `mode`) → `RunManifest { mode: Dag, mesh_config: None, gossip_config: None }`

### 3. RunManifest with mode = "mesh" parses correctly

1. Run: `cargo test -p assay-types --features orchestrate manifest_with_mode_mesh_parses`
2. **Expected:** ok — `mode = "mesh"` in TOML → `OrchestratorMode::Mesh`

### 4. RunManifest with mode = "gossip" parses correctly

1. Run: `cargo test -p assay-types --features orchestrate manifest_with_mode_gossip_parses`
2. **Expected:** ok — `mode = "gossip"` in TOML → `OrchestratorMode::Gossip`

### 5. Schema snapshots are locked for all new types

1. Run: `cargo test -p assay-types --features orchestrate -- --list | grep schema`
2. Confirm `orchestrator_mode_schema_snapshot`, `mesh_config_schema_snapshot`, `gossip_config_schema_snapshot`, `run_manifest_schema_snapshot` all appear
3. Run: `cargo test -p assay-types --features orchestrate`
4. **Expected:** all 58 tests pass including all four snapshot tests

### 6. CLI mode dispatch routes Mesh to stub

1. Run: `cargo test -p assay-cli mode_dispatch_mesh`
2. **Expected:** `test commands::run::tests::mode_dispatch_mesh_bypasses_needs_orchestration ... ok` (or similar test name)

### 7. MCP guard bypass for Mesh and Gossip

1. Run: `cargo test -p assay-mcp orchestrate_run_mesh_skips_session_count_guard orchestrate_run_gossip_skips_session_count_guard`
2. **Expected:** both tests pass — single-session Mesh/Gossip manifests are not rejected by the multi-session guard

### 8. Full just ready pass

1. Run: `just ready`
2. **Expected:** fmt ✓, lint ✓ (0 warnings), test ✓ (all 1222+ tests pass), deny ✓

## Edge Cases

### Unknown mode value in TOML

1. Create a TOML with `mode = "turbo"` in RunManifest
2. Parse with `RunManifest::from_str()`
3. **Expected:** serde deserialization error — unknown variant `turbo`, expected one of `dag`, `mesh`, `gossip`

### MeshConfig with unknown field

1. Create a TOML with `[mesh_config]\nheartbeat_interval_secs = 5\nunknown_field = true`
2. Parse with `RunManifest::from_str()`
3. **Expected:** serde deserialization error — `deny_unknown_fields` rejects `unknown_field`

### GossipConfig with unknown field

1. Create a TOML with `[gossip_config]\nunknown_field = 99`
2. **Expected:** serde deserialization error — `deny_unknown_fields` rejects `unknown_field`

### mesh_config present but mode = "dag" (silently ignored)

1. Create a manifest with `mode = "dag"` (or no mode) and `[mesh_config]\nheartbeat_interval_secs = 3`
2. Parse and dispatch
3. **Expected:** parses successfully; routes to DAG executor; `mesh_config` is present in `RunManifest` struct but ignored by the executor

## Failure Signals

- Any snapshot test failure (`insta` mismatch) — means a type changed without updating the locked snapshot; run `cargo insta review` to inspect the diff
- `just ready` lint failure with "unused import" or similar on `OrchestratorMode` — means import was added but not used in a dispatch branch
- MCP test `orchestrate_run_mesh_skips_session_count_guard` fails — guard condition regression
- `manifest_without_mode_defaults_to_dag` fails — serde default is broken; existing manifests would break

## Requirements Proved By This UAT

- R034 (OrchestratorMode selection) — `mode` field on `RunManifest` parses all three variants, defaults to `dag` when absent, dispatch routing sends Mesh/Gossip to stub executors, schema snapshots locked, backward-compatible with existing manifests, all 1222+ tests pass

## Not Proven By This UAT

- R035 (Mesh mode execution) — parallel launch, roster injection not implemented; stubs only
- R036 (Mesh peer messaging) — inbox/outbox directories, message routing, SWIM membership not implemented
- R037 (Gossip mode execution) — coordinator thread, knowledge manifest not implemented
- R038 (Gossip knowledge manifest injection) — manifest path injection at launch not implemented
- Real runtime behavior of Mesh/Gossip with actual agent processes — deferred to S02/S03 integration tests and manual UAT

## Notes for Tester

- The dispatch stubs return a real `OrchestratorResult` with a fresh ULID run_id. Calling `orchestrate_status` on this run_id will return not-found because the stubs do not persist state to disk. This is expected and correct at S01 — state persistence is an S02/S03 concern.
- `tracing::warn!` messages from Mesh/Gossip sessions with `depends_on` are visible with `RUST_LOG=warn`. These are the only observable runtime signals from S01 — they confirm the dispatch path was reached.
- All schema snapshot files must be committed (not just generated). `git status` should show no untracked or modified `.snap` files after `just ready`.
