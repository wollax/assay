---
phase: 27
plan: 2
wave: 1
depends_on: []
files_modified:
  - crates/assay-types/src/enforcement.rs
  - crates/assay-types/src/gate.rs
  - crates/assay-types/src/criterion.rs
  - crates/assay-types/src/session.rs
  - crates/assay-types/src/feature_spec.rs
  - crates/assay-types/src/context.rs
  - crates/assay-types/src/checkpoint.rs
autonomous: true
source_issue: null
must_haves:
  truths:
    - "All key user-facing enums implement Display with human-readable output"
    - "Display output does NOT change serde serialization format"
    - "No new dependencies added (hand-written impls only)"
  artifacts:
    - path: "crates/assay-types/src/enforcement.rs"
      provides: "Display for Enforcement"
    - path: "crates/assay-types/src/gate.rs"
      provides: "Display for GateKind (variant name only, no data)"
    - path: "crates/assay-types/src/feature_spec.rs"
      provides: "Display for SpecStatus, Obligation, Priority, VerificationMethod, AcceptanceCriterionType, ImpactLevel, LikelihoodLevel"
    - path: "crates/assay-types/src/context.rs"
      provides: "Display for BloatCategory, PruneStrategy, PrescriptionTier, ContextHealth"
    - path: "crates/assay-types/src/session.rs"
      provides: "Display for EvaluatorRole, Confidence"
    - path: "crates/assay-types/src/checkpoint.rs"
      provides: "Display for AgentStatus, TaskStatus"
    - path: "crates/assay-types/src/criterion.rs"
      provides: "Display for CriterionKind"
  key_links:
    - from: "BloatCategory::label()"
      to: "Display for BloatCategory"
      via: "Display delegates to existing label() method"
    - from: "PruneStrategy::label()"
      to: "Display for PruneStrategy"
      via: "Display delegates to existing label() method"
---

<objective>
Implement `Display` for all public enums in assay-types so they produce human-readable output suitable for CLI display and error messages. This fulfills TYPE-02. All impls are hand-written (zero new deps constraint).
</objective>

<context>
@crates/assay-types/src/enforcement.rs
@crates/assay-types/src/gate.rs
@crates/assay-types/src/criterion.rs
@crates/assay-types/src/session.rs
@crates/assay-types/src/feature_spec.rs
@crates/assay-types/src/context.rs
@crates/assay-types/src/checkpoint.rs
</context>

<task type="auto">
  <name>Task 1: Add Display impls to all public enums</name>
  <files>
    - crates/assay-types/src/enforcement.rs
    - crates/assay-types/src/gate.rs
    - crates/assay-types/src/criterion.rs
    - crates/assay-types/src/session.rs
    - crates/assay-types/src/feature_spec.rs
    - crates/assay-types/src/context.rs
    - crates/assay-types/src/checkpoint.rs
  </files>
  <action>
    Add `impl std::fmt::Display` for each public enum. Use human-readable strings that match the serde serialization form (kebab-case where serde uses kebab-case, etc.).

    **enforcement.rs:**
    - `Enforcement` → "required", "advisory" (matches serde kebab-case)

    **gate.rs:**
    - `GateKind` → "Command", "AlwaysPass", "FileExists", "AgentReport" (variant name only, no data fields)

    **criterion.rs:**
    - `CriterionKind` → "AgentReport"

    **session.rs:**
    - `EvaluatorRole` → "self", "independent", "human" (matches serde rename)
    - `Confidence` → "high", "medium", "low" (matches serde kebab-case)

    **feature_spec.rs (7 enums):**
    - `SpecStatus` → "draft", "proposed", "planned", "in-progress", "verified", "deprecated"
    - `Obligation` → "shall", "should", "may"
    - `Priority` → "must", "should", "could", "wont"
    - `VerificationMethod` → "test", "analysis", "inspection", "demonstration"
    - `AcceptanceCriterionType` → "gherkin", "ears", "plain"
    - `ImpactLevel` → "low", "medium", "high", "critical"
    - `LikelihoodLevel` → "low", "medium", "high"

    **context.rs:**
    - `BloatCategory` → delegate to `self.label()` (existing method)
    - `PruneStrategy` → delegate to `self.label()` (existing method)
    - `PrescriptionTier` → "gentle", "standard", "aggressive" (matches serde kebab-case)
    - `ContextHealth` → "healthy", "warning", "critical" (matches serde snake_case)

    **checkpoint.rs:**
    - `AgentStatus` → "active", "idle", "done", "unknown" (matches serde snake_case)
    - `TaskStatus` → "pending", "in_progress", "completed", "cancelled" (matches serde snake_case)
  </action>
  <verify>
    cargo test -p assay-types 2>&1 | tail -20
    cargo clippy -p assay-types -- -D warnings 2>&1 | head -30
  </verify>
  <done>
    - All 17 public enums implement Display
    - Display output matches serde serialization form
    - No new dependencies added
    - All tests pass
  </done>
</task>

<task type="auto">
  <name>Task 2: Add Display tests for key enums</name>
  <files>
    - crates/assay-types/src/enforcement.rs
    - crates/assay-types/src/gate.rs
    - crates/assay-types/src/context.rs
  </files>
  <action>
    Add focused tests for the most important Display impls to verify they produce expected output:

    1. In enforcement.rs tests: verify `Enforcement::Required.to_string() == "required"` and `Enforcement::Advisory.to_string() == "advisory"`
    2. In gate.rs tests: verify `GateKind::Command { cmd: "test".into() }.to_string() == "Command"` (data not included) and `GateKind::AlwaysPass.to_string() == "AlwaysPass"`
    3. In context.rs tests: verify `BloatCategory::ProgressTicks.to_string() == "Progress ticks"` (delegates to label())
  </action>
  <verify>
    cargo test -p assay-types -- display 2>&1 | tail -20
  </verify>
  <done>
    - Display tests exist for Enforcement, GateKind, and BloatCategory
    - All Display tests pass
  </done>
</task>

<verification>
```bash
just ready
```
</verification>

<success_criteria>
- [ ] All 17 public enums implement `Display`
- [ ] Display output is human-readable and matches serde form
- [ ] Zero new dependencies added
- [ ] Display tests pass for key enums
- [ ] `just ready` passes
</success_criteria>
