# S02: LocalFsBackend implementation and orchestrator wiring — UAT

**Milestone:** M010
**Written:** 2026-03-26

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S02's correctness is fully captured by automated tests — backward-compat round-trip tests, 16 LocalFsBackend contract tests, and 5 integration test suites covering orchestrate/mesh/gossip paths. No human interaction, live agent invocation, or real daemon is required to prove R072/R073. The slice plan explicitly set "Human/UAT required: no."

## Preconditions

- Rust toolchain installed (`cargo`, `just`)
- Working directory: `/Users/wollax/Git/personal/assay`
- All S01 artifacts in place: `StateBackend` trait, `CapabilitySet`, `LocalFsBackend` skeleton, `StateBackendConfig`

## Smoke Test

```bash
just ready
```

Expected: "All checks passed." with 1481+ tests passing.

## Test Cases

### 1. RunManifest backward-compat round-trip

```bash
cargo test -p assay-core --features orchestrate --test state_backend backward_compat
```

1. Run the backward-compat tests
2. **Expected:** 2 tests pass — manifest without `state_backend` deserializes to `None`; manifest with `Some(LocalFs)` survives TOML round-trip

### 2. LocalFsBackend contract tests

```bash
cargo test -p assay-core --features orchestrate --test state_backend
```

1. Run all 16 state_backend contract tests
2. **Expected:** 16/16 pass — push+read state, checkpoint save, send+poll messages, annotate_run, capabilities, backward-compat

### 3. Orchestrate integration unchanged

```bash
cargo test -p assay-core --features orchestrate --test orchestrate_integration
cargo test -p assay-core --features orchestrate --test mesh_integration
cargo test -p assay-core --features orchestrate --test gossip_integration
cargo test -p assay-core --features orchestrate --test orchestrate_spans
cargo test -p assay-core --features orchestrate --test integration_modes
```

1. Run all 5 integration test suites
2. **Expected:** All pass (5+2+2+5+3 = 17 tests) — behavior unchanged, routing now through LocalFsBackend

### 4. Schema snapshot includes state_backend

```bash
cargo test -p assay-types --features orchestrate --test schema_snapshots run_manifest_orchestrate
```

1. Run the orchestrate-gated schema snapshot test
2. **Expected:** Snapshot matches and includes `state_backend` field

### 5. No persist_state references remain

```bash
grep -rn "persist_state" crates/assay-core/src/orchestrate/
```

1. Search for any remaining direct persist_state calls
2. **Expected:** Empty output — all callsites replaced

## Edge Cases

### Manifest without state_backend field deserializes correctly

```bash
cargo test -p assay-core --features orchestrate --test state_backend backward_compat_no_state_backend
```

1. Run the specific no-field backward-compat test
2. **Expected:** `state_backend` deserializes as `None`; no parse error

### Default OrchestratorConfig provides a backend

Implicit in all `::default()` callsites in test files — they compile and the tests pass, confirming the `Default` impl provides a valid `LocalFsBackend`.

## Failure Signals

- `just ready` fails → compilation error or test regression; check which test suite fails
- `grep persist_state crates/assay-core/src/orchestrate/` returns matches → incomplete callsite replacement
- state_backend contract tests fail → LocalFsBackend method implementations have bugs (check path/operation context in AssayError)
- Schema snapshot mismatch → `RunManifest` schema changed without updating snapshot; run `INSTA_UPDATE=always cargo test` to accept

## Requirements Proved By This UAT

- R072 — Proved by: backward-compat round-trip tests confirm zero schema break; 16 LocalFsBackend contract tests confirm real persistence; all 17 integration tests pass without change; `just ready` green with 1481 tests
- R073 — Proved by: zero `persist_state` references in orchestrate src (grep-confirmed); all session/mesh/gossip/checkpoint writes route through `StateBackend` methods; LocalFsBackend retains filesystem behavior transparently

## Not Proven By This UAT

- Real multi-machine smelt worker interaction with the backend API — requires live smelt workers and network; UAT only
- Linear/GitHub/SSH remote backend implementations — deferred to M011+
- CapabilitySet degradation behavior when a backend lacks a capability — covered in S03
- smelt-agent plugin usability by a human author — covered in S04 UAT

## Notes for Tester

The test suite uses mock session runners (same as before M010), so no real AI agent invocation occurs. `just ready` is the authoritative single-command check. The schema split (orchestrate vs non-orchestrate) means both `cargo test -p assay-types --test schema_snapshots` and `cargo test -p assay-types --features orchestrate --test schema_snapshots` must pass — `just ready` runs `cargo nextest run --workspace` which covers both.
