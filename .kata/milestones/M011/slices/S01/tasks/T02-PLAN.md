---
estimated_steps: 5
estimated_files: 6
---

# T02: Write tests, regenerate schema snapshots, and pass `just ready`

**Slice:** S01 — assay-backends crate scaffold and StateBackendConfig variants
**Milestone:** M011

## Description

Write serde round-trip tests for all five `StateBackendConfig` variants, factory dispatch tests for `backend_from_config()`, regenerate both schema snapshots affected by the new variants, and verify `just ready` green with 1488+ tests.

## Steps

1. **Add serde round-trip tests** in `crates/assay-core/tests/state_backend.rs`:
   - JSON round-trip for `StateBackendConfig::Linear { team_id: "TEAM".into(), project_id: Some("PROJ".into()) }`
   - JSON round-trip for `StateBackendConfig::GitHub { repo: "user/repo".into(), label: Some("assay".into()) }`
   - JSON round-trip for `StateBackendConfig::Ssh { host: "server.example.com".into(), remote_assay_dir: "/home/user/.assay".into(), user: Some("deploy".into()), port: Some(2222) }`
   - TOML round-trip for `RunManifest` with each new variant in `state_backend` field
   - Verify `GitHub` serializes as `"github"` (not `"git_hub"`) — this is the serde rename trap

2. **Add factory dispatch tests** in `crates/assay-backends/src/factory.rs` (inline `#[cfg(test)] mod tests`):
   - `backend_from_config(&StateBackendConfig::LocalFs, dir)` returns backend with `CapabilitySet::all()`
   - `backend_from_config(&StateBackendConfig::Linear { .. }, dir)` returns backend with `CapabilitySet::none()`
   - `backend_from_config(&StateBackendConfig::GitHub { .. }, dir)` returns backend with `CapabilitySet::none()`
   - `backend_from_config(&StateBackendConfig::Ssh { .. }, dir)` returns backend with `CapabilitySet::none()`
   - `backend_from_config(&StateBackendConfig::Custom { .. }, dir)` returns backend with `CapabilitySet::none()`

3. **Regenerate schema snapshots** — run both feature-flag states:
   - `cargo test -p assay-types` → snapshot `state-backend-config-schema` will need update
   - `cargo test -p assay-types --features orchestrate` → snapshot `run-manifest-orchestrate-schema` will need update
   - Run `cargo insta review` to accept both updated snapshots
   - Verify accepted snapshots reflect all five variants with correct field types

4. **Run `just ready`** — must pass with 1488+ tests, zero regression

5. **Verify the serde rename** — confirm the GitHub variant serializes as `"github"` not `"git_hub"` in the accepted schema snapshot JSON

## Must-Haves

- [ ] Serde round-trip tests for all five `StateBackendConfig` variants (JSON)
- [ ] TOML round-trip test for `RunManifest` with at least one new variant
- [ ] Factory dispatch tests for all five variants
- [ ] `state-backend-config-schema` snapshot updated and committed
- [ ] `run-manifest-orchestrate-schema` snapshot updated and committed
- [ ] `just ready` green with 1488+ tests

## Verification

- `cargo test -p assay-backends` — factory tests pass
- `cargo test -p assay-core --features orchestrate` — round-trip tests pass
- `cargo test -p assay-types` — base schema snapshots pass
- `cargo test -p assay-types --features orchestrate` — orchestrate schema snapshots pass
- `just ready` — full workspace green

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: Schema snapshot files in `crates/assay-types/tests/snapshots/` serve as the locked contract for variant shapes
- Failure state exposed: Snapshot mismatch failures are immediate and show the exact diff

## Inputs

- T01 output: `StateBackendConfig` with 5 variants, `backend_from_config()` in `assay-backends`
- Existing tests in `crates/assay-core/tests/state_backend.rs` — pattern to follow for round-trip tests
- Existing snapshots in `crates/assay-types/tests/snapshots/` — will be updated

## Expected Output

- `crates/assay-core/tests/state_backend.rs` — extended with 5+ new tests
- `crates/assay-backends/src/factory.rs` — inline test module with 5 dispatch tests
- `crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap` — updated
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap` — updated
- `just ready` green
