# S01: assay-backends crate scaffold and StateBackendConfig variants — UAT

**Milestone:** M011
**Written:** 2026-03-27

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is pure type and crate scaffolding — no runtime behavior, no network calls, no user-facing output. The verification surface is entirely compile-time and test-time: the crate either compiles or it doesn't, schema snapshots either match or they don't, serde round-trips either pass or they don't. All proof is machine-checkable.

## Preconditions

- Rust toolchain installed (`cargo`, `cargo-insta`)
- `just` installed
- Working directory: `crates/assay-backends/` parent repo root
- `just ready` was passing before this slice (1488+ tests)

## Smoke Test

```
cargo build -p assay-backends && cargo test -p assay-backends
```

Both must exit 0. If they do, the crate exists and the factory fn dispatches all variants.

## Test Cases

### 1. assay-backends crate compiles

```
cargo build -p assay-backends
```

**Expected:** exit 0, no errors (warnings are acceptable)

### 2. Factory dispatch tests pass

```
cargo test -p assay-backends
```

**Expected:** 5 tests pass — `local_fs_returns_all_capabilities`, `linear_returns_no_capabilities`, `github_returns_no_capabilities`, `ssh_returns_no_capabilities`, `custom_returns_no_capabilities`

### 3. StateBackendConfig serde round-trips

```
cargo test -p assay-core --features orchestrate -- serde_json_round_trip
```

**Expected:** All round-trip tests pass for Linear (full + minimal), GitHub, Ssh (full + minimal) variants, including GitHub rename assertion

### 4. Schema snapshots accepted

```
cargo test -p assay-types --features orchestrate -- schema_snapshot
```

**Expected:** 70 tests pass; snapshots include `"linear"`, `"github"`, `"ssh"` variants without diff

### 5. Full workspace green

```
just ready
```

**Expected:** 1497+ tests, all pass, zero failures, deny/lint/fmt all clean

## Edge Cases

### GitHub serde rename

Deserialize `{"type": "git_hub", ...}` — should fail (unknown variant). Deserialize `{"type": "github", ...}` — should succeed.

This is covered by the explicit rename assertion test in `crates/assay-core/tests/state_backend.rs`.

### Optional fields absent

```json
{"type": "linear", "team_id": "TEAM123"}
```

Should deserialize to `Linear { team_id: "TEAM123", project_id: None }` without error. Covered by minimal variant tests.

## Failure Signals

- `cargo build -p assay-backends` fails → crate was not added to workspace or has a dep resolution error
- `schema_snapshots__state-backend-config-schema.snap` test fails with mismatch → snapshot not regenerated or variant shape changed
- `serde_json_round_trip` tests fail → serde attributes (rename, default, skip_serializing_if) are wrong
- `just ready` exits non-zero → regression in existing tests or compilation error in any crate

## Requirements Proved By This UAT

- R079 — `assay-backends` crate exists with feature flags; `StateBackendConfig` has `Linear`, `GitHub`, `Ssh` variants; `backend_from_config()` factory fn dispatches all four variants; schema snapshots committed; `just ready` green. All S01 success criteria are machine-verified.

## Not Proven By This UAT

- R076 (LinearBackend) — backend stubs with NoopBackend; real LinearBackend not implemented until S02
- R077 (GitHubBackend) — backend stubs with NoopBackend; real GitHubBackend not implemented until S03
- R078 (SshSyncBackend) — backend stubs with NoopBackend; real SshSyncBackend not implemented until S04
- CLI/MCP construction site wiring — `backend_from_config()` is not yet wired into `assay-cli`/`assay-mcp`; that is S04 work
- Any real runtime behavior of Linear, GitHub, or SSH backends — all deferred to S02–S04

## Notes for Tester

This slice is entirely mechanical — all checks are automated and deterministic. No manual interaction needed. If `just ready` passes green, the slice is done. The only subtlety to be aware of: `cargo test -p assay-types` (without `--features orchestrate`) will fail to compile `schema_roundtrip.rs` — this is pre-existing behavior, not a regression from this slice.
