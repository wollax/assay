---
id: S01
parent: M010
milestone: M010
provides:
  - StateBackend trait (7 sync methods, object-safe, Send+Sync) in assay_core::state_backend
  - CapabilitySet struct with all()/none() constructors, feature-gated behind orchestrate
  - LocalFsBackend skeleton implementing StateBackend with all-stub method bodies
  - _assert_object_safe compile guard proving Box<dyn StateBackend> works
  - StateBackendConfig enum (LocalFs / Custom { name, config: Value }) in assay_types::state_backend
  - Schema registration for state-backend-config via inventory::submit!
  - Locked snapshot at crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap
  - 6 passing contract tests in crates/assay-core/tests/state_backend.rs
requires:
  - slice: none
    provides: first slice — no upstream dependencies
affects:
  - S02
key_files:
  - crates/assay-types/src/state_backend.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap
  - crates/assay-core/src/state_backend.rs
  - crates/assay-core/src/lib.rs
  - crates/assay-core/tests/state_backend.rs
key_decisions:
  - D149 — StateBackend is a deliberate, scoped exception to D001 (zero-trait convention)
  - D150 — Trait methods are sync; async backends internalize their runtime (preserves D007)
  - D151 — Box<dyn StateBackend> in OrchestratorConfig, not a generic parameter (avoids viral generics)
  - D152 — Tier-1/Tier-2 split: heartbeats and per-tick routing are LocalFsBackend internals, not trait methods
  - D153 — StateBackendConfig as enum with LocalFs + Custom variants
  - D154 — state_backend module feature-gated behind orchestrate (consistent with existing module gate)
  - D155 — Object-safety compile guard fn _assert_object_safe(_: Box<dyn StateBackend>) at module level
patterns_established:
  - Feature-gated module: #[cfg(feature = "orchestrate")] pub mod state_backend + pub use at lib root
  - Object-safety compile guard: private fn taking Box<dyn Trait> at module level
  - New type module with inventory::submit! registration, pub mod + pub use in lib.rs — same pattern as all other assay-types modules
observability_surfaces:
  - backend.capabilities() returns CapabilitySet — all four bool fields inspectable at backend construction time
  - All trait methods return Result<_, AssayError> — failures carry structured context via existing error hierarchy
drill_down_paths:
  - .kata/milestones/M010/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M010/slices/S01/tasks/T02-SUMMARY.md
duration: 30min
verification_result: passed
completed_at: 2026-03-26T00:00:00Z
---

# S01: StateBackend trait and CapabilitySet

**`StateBackend` trait (7 sync methods, object-safe) + `CapabilitySet` flags struct + `LocalFsBackend` stub + `StateBackendConfig` schema-locked enum — API surface locked for S02 orchestrator wiring, all 1473 tests pass, `just ready` green.**

## What Happened

**T01** created `assay_types::state_backend::StateBackendConfig` — a schema-locked enum with `LocalFs` and `Custom { name: String, config: serde_json::Value }` variants. The type was registered in the schema inventory under `"state-backend-config"` and a JSON Schema snapshot was generated and committed. A pre-existing failure (`run_manifest_schema_snapshot`) was confirmed unrelated to this work.

**T02** created the full trait API surface in `assay_core::state_backend`, gated behind the `orchestrate` feature:
- `CapabilitySet` struct with 4 bool fields and `all()`/`none()` constructors. Required `Eq` derive alongside `PartialEq` (clippy lint — deviates from plan's note that this wouldn't be needed).
- `StateBackend` trait with 7 sync methods (`capabilities`, `push_session_event`, `read_run_state`, `send_message`, `poll_inbox`, `annotate_run`, `save_checkpoint_summary`), bounded `Send + Sync`.
- `_assert_object_safe(_: Box<dyn StateBackend>)` compile guard — required `#[allow(dead_code)]` attribute (clippy warned on unused function; plan said this wouldn't be needed).
- `LocalFsBackend { pub assay_dir: PathBuf }` with all-stub method bodies.

Six contract tests verified the full API surface without requiring real filesystem operations. `just ready` passed green (fmt + lint + test + deny) with 1473 total tests passing.

## Verification

- `cargo test -p assay-types --test schema_snapshots state_backend_config_schema_snapshot` — passes; snapshot locked
- `cargo test -p assay-core --features orchestrate --test state_backend` — 6/6 contract tests pass
- `cargo test --workspace` — 1473 tests, 0 failures (≥1466 requirement met)
- `just ready` — fmt + lint + test + deny all green
- `grep "_assert_object_safe"` confirms compile guard present in module

## Requirements Advanced

- R071 (StateBackend trait and CapabilitySet) — trait defined, CapabilitySet flags struct implemented, StateBackendConfig schema-locked; all contract tests pass

## Requirements Validated

- R071 — fully proven by this slice: trait exists with correct method signatures, object safety proven at compile time, CapabilitySet constructors verified, StateBackendConfig serde round-trip locked by snapshot, all 1473 tests pass

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- Plan said `#[allow(dead_code)]` not needed for `_assert_object_safe`; clippy required it to suppress the unused function warning — added the attribute.
- Plan said `CapabilitySet` needed only `PartialEq`; clippy's `derive_partial_eq_without_eq` required `Eq` to also be derived.
- `cargo fmt` collapsed multi-line `send_message` signatures to single-line in both trait and impl.

## Known Limitations

- All `LocalFsBackend` method bodies are stubs returning `Ok(())` / `Ok(None)` / `Ok(vec![])` — real implementations land in S02.
- `read_run_state` always returns `Ok(None)` — no filesystem reads yet.
- `run_manifest_schema_snapshot` test was pre-existing-failing on this branch before S01 work began. Not caused by this slice.

## Follow-ups

- S02: wire `LocalFsBackend` into `OrchestratorConfig`; implement real method bodies; add `RunManifest.state_backend` field; prove backward-compat round-trip.
- S02: check whether `RunManifest` has `deny_unknown_fields` before adding the optional `state_backend` field (risk noted in M010-ROADMAP.md).

## Files Created/Modified

- `crates/assay-types/src/state_backend.rs` — new: StateBackendConfig enum with schema registry entry
- `crates/assay-types/src/lib.rs` — added `pub mod state_backend` and `pub use state_backend::StateBackendConfig`
- `crates/assay-types/tests/schema_snapshots.rs` — added `state_backend_config_schema_snapshot` test
- `crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap` — locked JSON Schema snapshot
- `crates/assay-core/src/state_backend.rs` — new: StateBackend trait, CapabilitySet, LocalFsBackend, object-safety guard
- `crates/assay-core/src/lib.rs` — added feature-gated pub mod + re-exports
- `crates/assay-core/tests/state_backend.rs` — new: 6 contract tests

## Forward Intelligence

### What the next slice should know
- `LocalFsBackend` has `pub assay_dir: PathBuf` — S02 should populate this from the existing `assay_dir` in `OrchestratorConfig`; the field name is already the right shape.
- `StateBackend` methods take `&Path` arguments for directories, not stored config — S02 callers construct these paths at call sites, not at backend construction time.
- The `orchestrate` feature gate on the `state_backend` module means any test file using these types must include `#![cfg(feature = "orchestrate")]` at the top.

### What's fragile
- `run_manifest_schema_snapshot` — pre-existing failure unrelated to M010; S02 should not be blocked by it, but should not make it worse.
- `LocalFsBackend::read_run_state` returns `Ok(None)` unconditionally — S02 must replace this with real filesystem reads before orchestrate integration tests can pass.

### Authoritative diagnostics
- `cargo test -p assay-core --features orchestrate --test state_backend` — fastest check for state_backend API surface correctness
- `just ready` — authoritative green signal before committing

### What assumptions changed
- Plan assumed `_assert_object_safe` needed no dead_code annotation; clippy proved otherwise. The annotation is intentional and correct.
- Plan assumed `PartialEq` was sufficient for `CapabilitySet`; clippy's `derive_partial_eq_without_eq` required `Eq` too.
