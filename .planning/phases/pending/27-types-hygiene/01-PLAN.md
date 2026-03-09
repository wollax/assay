---
phase: 27
plan: 1
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - crates/assay-types/src/lib.rs
  - crates/assay-types/src/criterion.rs
  - crates/assay-types/src/enforcement.rs
  - crates/assay-types/src/gate.rs
  - crates/assay-types/src/gate_run.rs
  - crates/assay-types/src/gates_spec.rs
  - crates/assay-types/src/feature_spec.rs
  - crates/assay-types/src/session.rs
  - crates/assay-types/src/checkpoint.rs
  - crates/assay-types/src/context.rs
autonomous: true
source_issue: null
must_haves:
  truths:
    - "Every type without f64 fields derives both PartialEq and Eq"
    - "Types with f64 fields compile with explicit #[allow] annotations"
    - "GateSection::default() compiles and produces enforcement: Enforcement::Required"
    - "clippy::derive_partial_eq_without_eq is denied at workspace level"
  artifacts:
    - path: "Cargo.toml"
      provides: "Workspace-level clippy lint: derive_partial_eq_without_eq = deny"
    - path: "crates/assay-types/src/enforcement.rs"
      provides: "GateSection with Default derive"
    - path: "crates/assay-types/src/gate.rs"
      provides: "GateResult with Eq derive (dependency root for gate_run types)"
  key_links:
    - from: "gate.rs (GateResult Eq)"
      to: "gate_run.rs (CriterionResult, GateRunSummary, GateRunRecord Eq)"
      via: "CriterionResult contains Option<GateResult>, so GateResult must get Eq first"
    - from: "Cargo.toml (workspace lint)"
      to: "context.rs, checkpoint.rs (f64 types)"
      via: "Float types need per-type #[allow] to satisfy workspace deny lint"
---

<objective>
Add Eq derives to all types without f64 fields, add Default to GateSection, and establish the workspace clippy lint that enforces Eq alongside PartialEq. This is the foundation for TYPE-01 and TYPE-04 — all other plans can proceed independently except Plan 04 which needs Eq on Criterion.
</objective>

<context>
@crates/assay-types/src/lib.rs
@crates/assay-types/src/criterion.rs
@crates/assay-types/src/enforcement.rs
@crates/assay-types/src/gate.rs
@crates/assay-types/src/gate_run.rs
@crates/assay-types/src/gates_spec.rs
@crates/assay-types/src/feature_spec.rs
@crates/assay-types/src/session.rs
@crates/assay-types/src/checkpoint.rs
@crates/assay-types/src/context.rs
@Cargo.toml
</context>

<task type="auto">
  <name>Task 1: Add workspace clippy lint and Eq derives to all safe types</name>
  <files>
    - Cargo.toml
    - crates/assay-types/src/lib.rs
    - crates/assay-types/src/criterion.rs
    - crates/assay-types/src/enforcement.rs
    - crates/assay-types/src/gate.rs
    - crates/assay-types/src/gate_run.rs
    - crates/assay-types/src/gates_spec.rs
    - crates/assay-types/src/feature_spec.rs
    - crates/assay-types/src/session.rs
    - crates/assay-types/src/checkpoint.rs
    - crates/assay-types/src/context.rs
  </files>
  <action>
    1. Add `[workspace.lints.clippy]` section to root `Cargo.toml` with `derive_partial_eq_without_eq = "deny"`.
    2. Add `[lints] workspace = true` to `crates/assay-types/Cargo.toml` (and other workspace crates if they have a `[lints]` section already).
    3. Work in dependency order — GateResult FIRST, then types that contain it:
       - gate.rs: Add `Eq` to `GateKind` and `GateResult` derives
       - gate_run.rs: Add `Eq` to `CriterionResult`, `GateRunSummary`, `GateRunRecord`
       - criterion.rs: Add `Eq` to `Criterion`
       - enforcement.rs: Add `Eq` to `Enforcement` (already has it? verify), `GateSection`, `EnforcementSummary`. Add `Default` derive to `GateSection`.
       - gates_spec.rs: Add `Eq` to `GateCriterion`, `GatesSpec`
       - session.rs: Add `Eq` to `AgentEvaluation`, `AgentSession`
       - feature_spec.rs: Add `Eq` to all types: `SpecStatus`, `Obligation`, `Priority`, `VerificationMethod`, `AcceptanceCriterionType`, `AcceptanceCriterion`, `Requirement`, `FeatureOverview`, `Constraints`, `UserClass`, `QualityAttribute`, `QualityAttributes`, `Assumption`, `Dependency`, `ImpactLevel`, `LikelihoodLevel`, `Risk`, `VerificationStrategy`, `FeatureSpec`
       - lib.rs: Add `Eq` to `Spec`, `Gate`, `Review`, `Workflow`, `Config`, `GatesConfig`
       - checkpoint.rs: Add `Eq` to `AgentStatus`, `TaskStatus` (already have it? verify), `TeamCheckpoint`, `AgentState`, `TaskState`
       - context.rs: Add `Eq` to `ContextHealth` (already has it? verify), `BloatCategory` (already has it? verify), `PruneStrategy`, `PrescriptionTier`, `PruneSummary`, `PruneSample`, `PruneReport`, `SessionInfo`, `UsageData`, `BloatBreakdown`
    4. For the 5 types with f64 fields, add `#[allow(clippy::derive_partial_eq_without_eq)]` directly above their derive macro:
       - `GuardConfig` (lib.rs) — f64 fields: soft_threshold, hard_threshold
       - `ContextHealthSnapshot` (checkpoint.rs) — f64 field: utilization_pct
       - `BloatEntry` (context.rs) — f64 field: percentage
       - `DiagnosticsReport` (context.rs) — f64 field: context_utilization_pct
       - `TokenEstimate` (context.rs) — f64 field: context_utilization_pct
    5. Verify `GateSection::default()` compiles by checking that the single field `enforcement: Enforcement` already derives Default (it does — #[default] Required).
  </action>
  <verify>
    cargo clippy --workspace -- -D warnings 2>&1 | head -50
    cargo test -p assay-types 2>&1 | tail -20
  </verify>
  <done>
    - `cargo clippy --workspace` passes with no derive_partial_eq_without_eq warnings
    - All assay-types tests pass
    - `GateSection::default()` compiles (Enforcement::Required is default)
    - Five f64 types have explicit #[allow] annotations
  </done>
</task>

<verification>
```bash
just ready
```
</verification>

<success_criteria>
- [ ] Workspace lint `derive_partial_eq_without_eq = "deny"` present in root Cargo.toml
- [ ] All types without f64 fields derive both `PartialEq` and `Eq`
- [ ] Five f64-containing types have `#[allow(clippy::derive_partial_eq_without_eq)]`
- [ ] `GateSection` derives `Default`
- [ ] `just ready` passes
</success_criteria>
