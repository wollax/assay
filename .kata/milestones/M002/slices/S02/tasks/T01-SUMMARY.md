---
id: T01
parent: S02
milestone: M002
provides:
  - SessionRunState, FailurePolicy, OrchestratorPhase, SessionStatus, OrchestratorStatus enums/structs in assay-types
  - SessionOutcome, OrchestratorConfig, OrchestratorResult types in assay-core executor module
  - Schema snapshots for all 5 orchestrate types
key_files:
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-core/src/orchestrate/executor.rs
key_decisions:
  - Box<PipelineResult> in SessionOutcome::Completed to satisfy clippy large_enum_variant lint (PipelineResult is ~400 bytes)
patterns_established:
  - Feature-gated orchestrate types in assay-types with cfg(feature = "orchestrate") and re-exports
  - assay-core orchestrate feature propagates to assay-types/orchestrate via Cargo feature forwarding
observability_surfaces:
  - OrchestratorStatus schema establishes the contract for .assay/orchestrator/<run_id>/state.json persistence
  - SessionStatus.error and SessionStatus.skip_reason carry structured failure context
duration: 12m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add orchestrator types to assay-types and assay-core

**Added all serializable orchestrator status types (5 in assay-types, 3 in assay-core) with full serde round-trip tests and locked schema snapshots.**

## What Happened

Created `crates/assay-types/src/orchestrate.rs` with five types: `SessionRunState` (5-variant enum), `FailurePolicy` (enum defaulting to `SkipDependents`), `OrchestratorPhase` (4-variant enum), `SessionStatus` (struct with deny_unknown_fields), and `OrchestratorStatus` (struct with deny_unknown_fields). All types derive Serialize, Deserialize, JsonSchema with snake_case rename. Registered all 5 types in the inventory schema registry.

Created `crates/assay-core/src/orchestrate/executor.rs` with `SessionOutcome` (enum: Completed/Failed/Skipped), `OrchestratorConfig` (max_concurrency defaults to 8, failure_policy defaults to SkipDependents), and `OrchestratorResult` (run_id, outcomes vec, duration, failure_policy).

Added `orchestrate` feature to assay-types Cargo.toml. Configured assay-core's orchestrate feature to forward to `assay-types/orchestrate`. Enabled orchestrate feature in assay-mcp's assay-types dependency (assay-cli already had it).

## Verification

- `cargo test -p assay-types --features orchestrate -- orchestrate` — 9 tests pass (serde round-trip for all enums/structs, default checks, deny_unknown_fields rejection)
- `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — 6 tests pass (construction and default verification)
- `cargo insta test -p assay-types --features orchestrate` — all 40 snapshot tests pass, 5 new orchestrate snapshots accepted
- `cargo check -p assay-types` (without feature) — compiles, orchestrate module absent
- `cargo test -p assay-core` (without feature) — 700+ existing tests pass
- `just ready` — full suite green (fmt, lint, test, deny)

### Slice-level verification status (T01 of 5):
- ✅ `cargo test -p assay-types -- orchestrate` — type round-trip and schema tests pass
- ✅ `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — executor unit tests pass
- ✅ `cargo test -p assay-core --features orchestrate` — all core tests pass
- ✅ `cargo test -p assay-core` (without feature) — existing tests pass
- ✅ `just ready` — full suite green
- ⬜ state.json readability test — future task (T03/T04)
- ⬜ failed session dependents skipped test — future task (T03/T04)

## Diagnostics

`OrchestratorStatus` can be deserialized from JSON to inspect run state. Schema snapshots in `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` lock the wire format. `SessionStatus.error` and `SessionStatus.skip_reason` fields carry structured failure context for agent inspection.

## Deviations

- `SessionOutcome::Completed.result` uses `Box<PipelineResult>` instead of bare `PipelineResult` — required by clippy `large_enum_variant` lint since `PipelineResult` is ~400 bytes. No functional impact.
- Used `#[derive(Default)]` with `#[default]` attribute on `FailurePolicy::SkipDependents` instead of manual `impl Default` — required by clippy `derivable_impls` lint.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — new: 5 serializable types with schema registry and 9 unit tests
- `crates/assay-types/src/lib.rs` — feature-gated module declaration and re-exports
- `crates/assay-types/Cargo.toml` — added `orchestrate` feature
- `crates/assay-core/src/orchestrate/executor.rs` — new: 3 result types with 6 construction tests
- `crates/assay-core/src/orchestrate/mod.rs` — added `pub mod executor`
- `crates/assay-core/Cargo.toml` — orchestrate feature now forwards to assay-types/orchestrate
- `crates/assay-mcp/Cargo.toml` — enabled orchestrate feature on assay-types dependency
- `crates/assay-types/tests/schema_snapshots.rs` — 5 new feature-gated snapshot tests
- `crates/assay-types/tests/snapshots/` — 5 new schema snapshot files
