---
phase: 66
slug: evaluation-integration-validation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-11
---

# Phase 66 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` + `cargo test` |
| **Config file** | `Cargo.toml` workspace (no separate test config) |
| **Quick run command** | `cargo test -p assay-core --lib` |
| **Full suite command** | `just test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p assay-core --lib`
- **After every plan wave:** Run `just test`
- **Before `/kata:verify-work`:** Full suite must be green (`just ready`)
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 66-01-01 | 01 | 1 | PREC-03 | unit | `cargo test -p assay-types --lib gate_run::tests::gate_eval_outcome_variants` | ❌ W0 | ⬜ pending |
| 66-01-02 | 01 | 1 | PREC-01 | unit | `cargo test -p assay-core --lib gate::tests::precondition_requires_not_passed` | ❌ W0 | ⬜ pending |
| 66-01-03 | 01 | 1 | PREC-01 | unit | `cargo test -p assay-core --lib gate::tests::precondition_requires_no_history` | ❌ W0 | ⬜ pending |
| 66-01-04 | 01 | 1 | PREC-02 | unit | `cargo test -p assay-core --lib gate::tests::precondition_command_fails` | ❌ W0 | ⬜ pending |
| 66-01-05 | 01 | 1 | PREC-03 | unit | `cargo test -p assay-core --lib gate::tests::precondition_blocks_criteria` | ❌ W0 | ⬜ pending |
| 66-02-01 | 02 | 1 | PREC-01 | unit | `cargo test -p assay-core --lib gate::tests::evaluate_all_resolved` | ❌ W0 | ⬜ pending |
| 66-03-01 | 03 | 2 | SAFE-01 | unit | `cargo test -p assay-core --lib spec::validate::tests::composability_missing_parent` | ❌ W0 | ⬜ pending |
| 66-03-02 | 03 | 2 | SAFE-01 | unit | `cargo test -p assay-core --lib spec::validate::tests::composability_missing_library` | ❌ W0 | ⬜ pending |
| 66-03-03 | 03 | 2 | SAFE-01 | unit | `cargo test -p assay-core --lib spec::validate::tests::composability_cycle_extends` | ❌ W0 | ⬜ pending |
| 66-03-04 | 03 | 2 | SAFE-02 | unit | `cargo test -p assay-core --lib spec::validate::tests::composability_slug_path_traversal` | ❌ W0 | ⬜ pending |
| 66-03-05 | 03 | 2 | SAFE-02 | unit | `cargo test -p assay-core --lib spec::validate::tests::composability_valid_slug_passes` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Tests for `check_preconditions()` in `crates/assay-core/src/gate/mod.rs` (extend existing `#[cfg(test)]` at line 1131)
- [ ] Tests for `GateEvalOutcome` serde in `crates/assay-types/src/gate_run.rs`
- [ ] Tests for composability diagnostics in `crates/assay-core/src/spec/validate.rs` (extend existing `#[cfg(test)]` at line 372)
- [ ] Tests for `evaluate_all_resolved()` + `CriterionResult.source` in `crates/assay-core/src/gate/mod.rs`

*None of these require new files — all extend existing inline test modules.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
