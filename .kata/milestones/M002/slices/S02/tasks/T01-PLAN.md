---
estimated_steps: 5
estimated_files: 6
---

# T01: Add orchestrator types to assay-types and assay-core

**Slice:** S02 — Parallel Session Executor
**Milestone:** M002

## Description

Create all serializable types needed for orchestrator state persistence (`SessionRunState`, `FailurePolicy`, `SessionStatus`, `OrchestratorStatus`, `OrchestratorPhase`) in `assay-types` and executor result types (`SessionOutcome`, `OrchestratorConfig`, `OrchestratorResult`) in `assay-core`. These types are the data contract consumed by S03 (merge runner), S06 (MCP tools), and the executor itself.

## Steps

1. Create `crates/assay-types/src/orchestrate.rs` with `SessionRunState` (enum: Pending/Running/Completed/Failed/Skipped), `FailurePolicy` (enum: SkipDependents/Abort), `OrchestratorPhase` (enum: Running/Completed/PartialFailure/Aborted), `SessionStatus` (struct with name, spec, state, timing, error, skip_reason), and `OrchestratorStatus` (struct with run_id, phase, failure_policy, sessions, timing). All with `Serialize, Deserialize, JsonSchema, deny_unknown_fields` where appropriate. Use `chrono::DateTime<Utc>` for timestamps. Feature-gate behind `#[cfg(feature = "orchestrate")]`.
2. Register types in assay-types `lib.rs`: add `#[cfg(feature = "orchestrate")] pub mod orchestrate;` and re-exports. Add `orchestrate` feature to `crates/assay-types/Cargo.toml`. Enable the feature in assay-core, assay-cli, and assay-mcp Cargo.toml files.
3. Add inventory schema submissions and run `cargo insta test --review` to accept new snapshots.
4. Create `crates/assay-core/src/orchestrate/executor.rs` with `SessionOutcome` enum (Completed/Failed/Skipped), `OrchestratorConfig` struct (pipeline config, max_concurrency, failure_policy), and `OrchestratorResult` struct (run_id, outcomes vec, timing, failure_policy). Add `pub mod executor;` to `orchestrate/mod.rs`.
5. Add unit tests in both modules: serde round-trip for all types in assay-types, construction tests for assay-core types. Verify `FailurePolicy` default is `SkipDependents`.

## Must-Haves

- [ ] `SessionRunState`, `FailurePolicy`, `OrchestratorPhase` enums with serde rename_all snake_case
- [ ] `SessionStatus` and `OrchestratorStatus` structs with deny_unknown_fields
- [ ] `SessionOutcome`, `OrchestratorConfig`, `OrchestratorResult` in assay-core behind orchestrate feature
- [ ] Schema snapshots locked via insta
- [ ] `FailurePolicy` defaults to `SkipDependents`
- [ ] `OrchestratorConfig.max_concurrency` defaults to 8

## Verification

- `cargo test -p assay-types --features orchestrate -- orchestrate` — round-trip tests pass
- `cargo test -p assay-core --features orchestrate` — compiles with new executor module
- `cargo insta test -p assay-types --review` — snapshots accepted
- `cargo check -p assay-types` (without feature) — compiles without orchestrate module

## Observability Impact

- Signals added/changed: `OrchestratorStatus` type establishes the schema for state persistence files read by future MCP tools
- How a future agent inspects this: `OrchestratorStatus` deserialized from `.assay/orchestrator/<run_id>/state.json`
- Failure state exposed: `SessionStatus.error` and `SessionStatus.skip_reason` fields carry structured failure context

## Inputs

- S01 summary: `DependencyGraph` exists in `orchestrate/dag.rs`; `orchestrate` feature gate pattern established on assay-core
- Research: type definitions from S02-RESEARCH.md New Types section
- `PipelineResult`, `PipelineError`, `PipelineStage` from `pipeline.rs` — referenced by `SessionOutcome`

## Expected Output

- `crates/assay-types/src/orchestrate.rs` — all 5 serializable types with full derives and tests
- `crates/assay-types/src/lib.rs` — feature-gated module and re-exports
- `crates/assay-types/Cargo.toml` — `orchestrate` feature added
- `crates/assay-core/src/orchestrate/executor.rs` — 3 result types with construction tests
- `crates/assay-core/src/orchestrate/mod.rs` — `pub mod executor` added
- `crates/assay-types/tests/snapshots/` — new schema snapshot files
