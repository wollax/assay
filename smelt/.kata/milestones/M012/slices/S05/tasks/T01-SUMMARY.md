---
id: T01
parent: S05
milestone: M012
provides:
  - SmeltRunManifest.state_backend field (Option<StateBackendConfig>) with serde defaults
  - build_run_manifest_toml() passthrough of manifest.state_backend into run manifest
  - Unit test: None state_backend produces no TOML section (backward compat)
  - Unit test: Linear state_backend serializes as [state_backend.linear] with team_id/project_id
  - Unit test: LocalFs state_backend serializes as state_backend = "local_fs"
key_files:
  - crates/smelt-core/src/assay.rs
key_decisions: []
patterns_established:
  - "Optional tagged-enum passthrough with serde(default, skip_serializing_if) for deny_unknown_fields compat"
observability_surfaces:
  - "None — serialization passthrough only; existing tracing::debug! logs full manifest content"
duration: 10min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T01: State backend passthrough in AssayInvoker

**Added `state_backend` passthrough from JobManifest into SmeltRunManifest TOML with tagged-enum serialization and backward-compat serde defaults**

## What Happened

Added `use crate::tracker::StateBackendConfig` import to `assay.rs`. Extended `SmeltRunManifest` with `pub state_backend: Option<StateBackendConfig>` annotated with `#[serde(default, skip_serializing_if = "Option::is_none")]` so existing TOML without the field still parses (backward compat with `deny_unknown_fields`). Updated `build_run_manifest_toml()` to copy `manifest.state_backend.clone()` into the constructed `SmeltRunManifest`. Added 3 new unit tests covering None, Linear, and LocalFs variants.

## Verification

- `cargo test -p smelt-core --lib -- assay::tests` — 14 passed (11 existing + 3 new), 0 failed
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo test --workspace` — 390 passed, 0 failed, 11 ignored — 0 regressions

### Slice-level checks:
- ✅ `cargo test -p smelt-core --lib -- assay::tests` — passes
- ⏳ `cargo test -p smelt-cli --lib -- serve::tracker_poller` — not yet applicable (T02)
- ⏳ `cargo test -p smelt-cli --lib -- serve::tui` — not yet applicable (T03)
- ✅ `cargo test --workspace` — all pass
- ✅ `cargo clippy --workspace -- -D warnings` — clean
- ⏳ `cargo doc --workspace --no-deps` — not checked (docs task is T03)

## Diagnostics

None — this is a serialization passthrough. Existing `tracing::debug!` in `build_run_manifest_toml()` already logs full TOML content.

## Deviations

Test assertion for Linear variant updated: `StateBackendConfig::Linear` serializes as `[state_backend.linear]` (tagged enum table) rather than `[state_backend]` with a `type` field. The must-have wording said `[state_backend]` section — the actual TOML uses `[state_backend.linear]` which is correct serde `snake_case` tagged-enum behavior.

## Known Issues

None

## Files Created/Modified

- `crates/smelt-core/src/assay.rs` — Added `StateBackendConfig` import, `state_backend` field on `SmeltRunManifest`, passthrough in `build_run_manifest_toml()`, 3 new unit tests
