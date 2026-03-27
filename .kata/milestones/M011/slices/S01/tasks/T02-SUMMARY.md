---
id: T02
parent: S01
milestone: M011
provides:
  - Serde round-trip tests for all 5 StateBackendConfig variants (JSON + TOML)
  - Factory dispatch tests for backend_from_config() covering all 5 variants
  - Updated schema snapshots reflecting new variants
key_files:
  - crates/assay-core/tests/state_backend.rs
  - crates/assay-backends/src/factory.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap
key_decisions:
  - none
patterns_established:
  - Factory dispatch tests using tempdir + CapabilitySet equality assertions
observability_surfaces:
  - Schema snapshot files serve as locked contract for variant shapes; mismatches are immediate
duration: ~5 minutes
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T02: Write tests, regenerate schema snapshots, and pass `just ready`

**Added serde round-trip tests for all 5 StateBackendConfig variants, factory dispatch tests for backend_from_config(), regenerated schema snapshots, and passed `just ready` with 1497 tests.**

## What Happened

1. Added 7 new tests to `crates/assay-core/tests/state_backend.rs`:
   - JSON round-trip for Linear (full + minimal), GitHub, Ssh (full + minimal) variants
   - GitHub rename assertion verifying serialization as `"github"` (not `"git_hub"`)
   - TOML round-trip for RunManifest with Linear backend

2. Added 5 factory dispatch tests as inline `#[cfg(test)]` module in `crates/assay-backends/src/factory.rs`:
   - LocalFs → CapabilitySet::all()
   - Linear, GitHub, Ssh, Custom → CapabilitySet::none()

3. Ran `cargo insta accept` to update both schema snapshots reflecting the new variants.

4. Verified `github` rename in accepted snapshot (2 occurrences of "github", 0 of "git_hub").

## Verification

- `cargo test -p assay-backends` — 5 factory tests pass ✓
- `cargo test -p assay-core --features orchestrate -- serde_json_round_trip` — 5 round-trip tests pass ✓
- `cargo test -p assay-core --features orchestrate -- toml_round_trip_manifest_with_linear` — TOML test passes ✓
- `cargo test -p assay-types --features orchestrate -- schema_snapshot` — 70 tests pass (both updated snapshots) ✓
- `just ready` — 1497 tests, all passed, zero failures ✓

### Slice-level verification:
- `cargo build -p assay-backends` — compiles ✓
- `cargo test -p assay-backends` — factory dispatch tests pass ✓
- `cargo test -p assay-types --features orchestrate` — schema snapshots pass ✓
- `cargo test -p assay-core --features orchestrate` — round-trip tests pass ✓
- `just ready` — full workspace green with 1497 tests ✓

## Diagnostics

Schema snapshot files in `crates/assay-types/tests/snapshots/` serve as the locked contract. Any future variant shape change will produce an immediate snapshot mismatch with an exact diff.

## Deviations

None — followed plan exactly.

## Known Issues

Pre-existing: `cargo test -p assay-types` (without `orchestrate` feature) fails to compile `schema_roundtrip.rs` due to `state_backend` field being feature-gated. This is not a regression — same behavior before T01/T02.

## Files Created/Modified

- `crates/assay-core/tests/state_backend.rs` — added 7 serde round-trip tests for new variants
- `crates/assay-backends/src/factory.rs` — added inline test module with 5 factory dispatch tests
- `crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap` — regenerated with Linear/GitHub/Ssh variants
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap` — regenerated with new variants in RunManifest
